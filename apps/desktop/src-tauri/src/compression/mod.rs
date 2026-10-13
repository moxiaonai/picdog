use crate::error::{CommandResult, SafeError};
use crate::models::{AssetState, CompressionOptions, CompressionToolStatus};
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{
    CompressionType as PngCompressionType, FilterType as PngFilterType, PngEncoder,
};
use image::{GenericImageView, ImageEncoder};
#[cfg(windows)]
use std::ffi::OsString;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionOutcome {
    pub output_path: String,
    pub output_size: u64,
    pub saved_ratio: u8,
    pub output_format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub used_real_encoder: bool,
}

struct ResizeInput {
    path: PathBuf,
    width: u32,
    height: u32,
}

pub fn compression_tool_statuses(encoder_directory: Option<&str>) -> Vec<CompressionToolStatus> {
    let mut statuses = vec![CompressionToolStatus {
        name: "builtin-image".to_string(),
        available: true,
        path: None,
        purpose: "Built-in JPEG / PNG fallback and resize".to_string(),
    }];

    statuses.extend(
        [
            ("cjpeg", "JPEG / MozJPEG compatible encoding"),
            ("pngquant", "Lossy PNG palette compression"),
            ("oxipng", "Lossless PNG optimization"),
            ("gifsicle", "Animated GIF optimization"),
            ("cwebp", "WebP encoding"),
            ("gif2webp", "Animated GIF to WebP encoding"),
            ("avifenc", "AVIF encoding"),
            ("svgo", "SVG optimization"),
        ]
        .into_iter()
        .map(|(name, purpose)| {
            let path =
                find_binary(name, encoder_directory).map(|path| path.to_string_lossy().to_string());
            CompressionToolStatus {
                name: name.to_string(),
                available: path.is_some(),
                path,
                purpose: purpose.to_string(),
            }
        }),
    );

    statuses
}

pub fn compress_asset(
    asset: &AssetState,
    options: &CompressionOptions,
) -> CommandResult<CompressionOutcome> {
    let source_path = Path::new(&asset.source_path);

    if source_path.is_file() {
        let input_format =
            image_format(source_path).unwrap_or_else(|| format_from_mime(asset.mime.as_deref()));
        let output_format = choose_output_format(&input_format, options);

        if input_format == "original" {
            return Err(SafeError::new(
                "unsupported_format",
                "暂不支持该图片格式。",
                true,
                false,
            ));
        }

        if let Some(missing) = missing_encoder_hint(
            &input_format,
            &output_format,
            options.encoder_directory.as_deref(),
        ) {
            return Err(SafeError::new(
                "encoder_missing",
                format!(
                    "缺少图片压缩编码器：{}。请安装对应工具或更换输出格式。",
                    missing
                ),
                true,
                false,
            ));
        }

        let outcome = if should_prefer_builtin(&input_format, &output_format, options) {
            try_builtin_compression(asset, options, source_path)
                .or_else(|| try_real_compression(asset, options, source_path))
        } else {
            try_real_compression(asset, options, source_path)
                .or_else(|| try_builtin_compression(asset, options, source_path))
        };

        return outcome.ok_or_else(|| {
            SafeError::new(
                "compression_failed",
                "图片压缩失败，请检查文件是否损坏或更换压缩设置。",
                true,
                true,
            )
        });
    }

    Err(SafeError::new(
        "file_not_found",
        "找不到要压缩的源图片，请确认文件仍然存在。",
        true,
        false,
    ))
}

fn try_real_compression(
    asset: &AssetState,
    options: &CompressionOptions,
    source_path: &Path,
) -> Option<CompressionOutcome> {
    let source_size = source_path.metadata().ok()?.len();
    let input_format =
        image_format(source_path).unwrap_or_else(|| format_from_mime(asset.mime.as_deref()));
    let output_format = choose_output_format(&input_format, options);
    let output_path = output_path_for(source_path, &output_format);
    let resize_input =
        prepare_external_resize_input(source_path, &input_format, &output_format, options);
    let encoder_source_path = resize_input
        .as_ref()
        .map(|input| input.path.as_path())
        .unwrap_or(source_path);

    if output_path == source_path {
        return None;
    }

    let encoded = match output_format.as_str() {
        "jpg" | "jpeg" => run_cjpeg(encoder_source_path, &output_path, options),
        "png" => run_png_pipeline(encoder_source_path, &output_path, options),
        "gif" => run_gifsicle(encoder_source_path, &output_path, options),
        "svg" => run_svgo(
            encoder_source_path,
            &output_path,
            options.encoder_directory.as_deref(),
        ),
        "avif" => run_avifenc(
            encoder_source_path,
            &output_path,
            options.quality,
            options.encoder_directory.as_deref(),
        ),
        "webp" if input_format == "gif" => run_gif2webp(
            encoder_source_path,
            &output_path,
            options.quality,
            options.encoder_directory.as_deref(),
        ),
        "webp" => run_cwebp(encoder_source_path, &output_path, options),
        _ => None,
    }?;

    if !encoded {
        if let Some(input) = resize_input {
            let _ = std::fs::remove_file(input.path);
        }
        return None;
    }

    let output_size = output_path.metadata().ok()?.len();

    if options.fallback_when_larger && output_size >= source_size {
        let _ = std::fs::remove_file(&output_path);
        if let Some(input) = resize_input {
            let _ = std::fs::remove_file(input.path);
        }
        return Some(CompressionOutcome {
            output_path: source_path.to_string_lossy().to_string(),
            output_size: source_size,
            saved_ratio: 0,
            output_format: input_format,
            width: image_dimensions(source_path).map(|(width, _)| width),
            height: image_dimensions(source_path).map(|(_, height)| height),
            used_real_encoder: true,
        });
    }

    let resized_dimensions = resize_input
        .as_ref()
        .map(|input| (input.width, input.height));
    let dimensions = image_dimensions(&output_path)
        .or(resized_dimensions)
        .or_else(|| image_dimensions(source_path));

    if let Some(input) = resize_input {
        let _ = std::fs::remove_file(input.path);
    }

    Some(CompressionOutcome {
        output_path: output_path.to_string_lossy().to_string(),
        output_size,
        saved_ratio: saved_ratio(source_size, output_size),
        output_format,
        width: dimensions.map(|(width, _)| width),
        height: dimensions.map(|(_, height)| height),
        used_real_encoder: true,
    })
}

fn try_builtin_compression(
    asset: &AssetState,
    options: &CompressionOptions,
    source_path: &Path,
) -> Option<CompressionOutcome> {
    let source_size = source_path.metadata().ok()?.len();
    let input_format =
        image_format(source_path).unwrap_or_else(|| format_from_mime(asset.mime.as_deref()));
    let output_format = choose_output_format(&input_format, options);

    if !builtin_encoder_available(&input_format, &output_format) {
        return None;
    }

    let mut image = image::open(source_path).ok()?;
    image = resize_for_limits(image, options.max_width, options.max_height);

    let output_path = output_path_for(source_path, &output_format);
    encode_builtin_image(&image, &output_path, &output_format, options.quality)?;

    let output_size = output_path.metadata().ok()?.len();

    if options.fallback_when_larger && output_size >= source_size {
        let _ = std::fs::remove_file(&output_path);
        let dimensions = image_dimensions(source_path).or_else(|| Some(image.dimensions()));
        return Some(CompressionOutcome {
            output_path: source_path.to_string_lossy().to_string(),
            output_size: source_size,
            saved_ratio: 0,
            output_format: input_format,
            width: dimensions.map(|(width, _)| width),
            height: dimensions.map(|(_, height)| height),
            used_real_encoder: true,
        });
    }

    let dimensions = image_dimensions(&output_path).or_else(|| Some(image.dimensions()));

    Some(CompressionOutcome {
        output_path: output_path.to_string_lossy().to_string(),
        output_size,
        saved_ratio: saved_ratio(source_size, output_size),
        output_format,
        width: dimensions.map(|(width, _)| width),
        height: dimensions.map(|(_, height)| height),
        used_real_encoder: true,
    })
}

fn resize_for_limits(
    image: image::DynamicImage,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> image::DynamicImage {
    let (width, height) = image.dimensions();
    let Some((next_width, next_height)) =
        target_dimensions_for_limits(width, height, max_width, max_height)
    else {
        return image;
    };

    image.resize_exact(
        next_width,
        next_height,
        image::imageops::FilterType::Lanczos3,
    )
}

fn target_dimensions_for_limits(
    width: u32,
    height: u32,
    max_width: Option<u32>,
    max_height: Option<u32>,
) -> Option<(u32, u32)> {
    let width_scale = max_width
        .filter(|value| *value > 0 && width > *value)
        .map(|value| value as f32 / width as f32);
    let height_scale = max_height
        .filter(|value| *value > 0 && height > *value)
        .map(|value| value as f32 / height as f32);
    let scale = [width_scale, height_scale]
        .into_iter()
        .flatten()
        .fold(1.0_f32, f32::min);

    if scale >= 1.0 {
        return None;
    }

    let next_width = ((width as f32 * scale).round() as u32).max(1);
    let next_height = ((height as f32 * scale).round() as u32).max(1);

    Some((next_width, next_height))
}

fn prepare_external_resize_input(
    source_path: &Path,
    input_format: &str,
    output_format: &str,
    options: &CompressionOptions,
) -> Option<ResizeInput> {
    if !should_prepare_external_resize_input(input_format, output_format, options) {
        return None;
    }

    let image = image::open(source_path).ok()?;
    let (width, height) = image.dimensions();
    let (next_width, next_height) =
        target_dimensions_for_limits(width, height, options.max_width, options.max_height)?;
    let resized = image.resize_exact(
        next_width,
        next_height,
        image::imageops::FilterType::Lanczos3,
    );
    let temporary_format = if input_format == "jpg" || input_format == "jpeg" {
        "jpg"
    } else {
        "png"
    };
    let temporary_path = temporary_resize_path(source_path, temporary_format);

    encode_builtin_image(&resized, &temporary_path, temporary_format, options.quality)?;

    Some(ResizeInput {
        path: temporary_path,
        width: next_width,
        height: next_height,
    })
}

fn should_prepare_external_resize_input(
    input_format: &str,
    output_format: &str,
    options: &CompressionOptions,
) -> bool {
    (options.max_width.is_some() || options.max_height.is_some())
        && matches!(input_format, "jpg" | "jpeg" | "png")
        && matches!(output_format, "webp" | "avif")
}

fn external_tool_resize_dimensions(
    source_path: &Path,
    options: &CompressionOptions,
) -> Option<(u32, u32)> {
    if options.max_width.is_none() && options.max_height.is_none() {
        return None;
    }

    let (width, height) = image_dimensions(source_path)?;
    target_dimensions_for_limits(width, height, options.max_width, options.max_height)
}

fn cwebp_resize_args(source_path: &Path, options: &CompressionOptions) -> Vec<String> {
    external_tool_resize_dimensions(source_path, options)
        .map(|(width, height)| vec!["-resize".to_string(), width.to_string(), height.to_string()])
        .unwrap_or_default()
}

fn gifsicle_resize_args(source_path: &Path, options: &CompressionOptions) -> Vec<String> {
    external_tool_resize_dimensions(source_path, options)
        .map(|(width, height)| vec!["--resize-fit".to_string(), format!("{width}x{height}")])
        .unwrap_or_default()
}

fn temporary_resize_path(source_path: &Path, format: &str) -> PathBuf {
    let extension = normalize_extension(format);
    let parent = source_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("image");

    parent.join(format!("{stem}.picdog-resize.{extension}"))
}

fn encode_builtin_image(
    image: &image::DynamicImage,
    output_path: &Path,
    output_format: &str,
    quality: u8,
) -> Option<()> {
    let file = File::create(output_path).ok()?;
    let mut writer = BufWriter::new(file);

    match output_format {
        "jpg" | "jpeg" => {
            let rgb = image.to_rgb8();
            let mut encoder = JpegEncoder::new_with_quality(&mut writer, quality.clamp(1, 100));
            encoder
                .encode(
                    &rgb,
                    rgb.width(),
                    rgb.height(),
                    image::ColorType::Rgb8.into(),
                )
                .ok()?;
        }
        "png" => {
            let rgba = image.to_rgba8();
            let encoder = PngEncoder::new_with_quality(
                &mut writer,
                PngCompressionType::Best,
                PngFilterType::Adaptive,
            );
            encoder
                .write_image(
                    &rgba,
                    rgba.width(),
                    rgba.height(),
                    image::ColorType::Rgba8.into(),
                )
                .ok()?;
        }
        _ => return None,
    }

    Some(())
}

fn run_cjpeg(source_path: &Path, output_path: &Path, options: &CompressionOptions) -> Option<bool> {
    let cjpeg = find_binary("cjpeg", options.encoder_directory.as_deref())?;
    let status = Command::new(cjpeg)
        .arg("-quality")
        .arg(options.quality.clamp(1, 100).to_string())
        .arg("-progressive")
        .arg("-copy")
        .arg(jpeg_copy_mode(options))
        .arg("-outfile")
        .arg(output_path)
        .arg(source_path)
        .status()
        .ok()?;

    Some(status.success() && output_path.is_file())
}

fn run_cwebp(source_path: &Path, output_path: &Path, options: &CompressionOptions) -> Option<bool> {
    let cwebp = find_binary("cwebp", options.encoder_directory.as_deref())?;
    let mut command = Command::new(cwebp);
    command
        .arg("-quiet")
        .arg("-q")
        .arg(options.quality.clamp(1, 100).to_string())
        .arg("-metadata")
        .arg(webp_metadata_mode(options));
    for arg in cwebp_resize_args(source_path, options) {
        command.arg(arg);
    }
    let status = command
        .arg(source_path)
        .arg("-o")
        .arg(output_path)
        .status()
        .ok()?;

    Some(status.success() && output_path.is_file())
}

fn run_png_pipeline(
    source_path: &Path,
    output_path: &Path,
    options: &CompressionOptions,
) -> Option<bool> {
    if let Some(encoded) = run_pngquant(
        source_path,
        output_path,
        options.quality,
        options.encoder_directory.as_deref(),
    ) {
        if encoded {
            let _ = run_oxipng(
                output_path,
                options.strip_metadata,
                options.encoder_directory.as_deref(),
            );
        }

        return Some(encoded && output_path.is_file());
    }

    if find_binary("oxipng", options.encoder_directory.as_deref()).is_some()
        && std::fs::copy(source_path, output_path).is_ok()
    {
        return run_oxipng(
            output_path,
            options.strip_metadata,
            options.encoder_directory.as_deref(),
        )
        .map(|optimized| optimized && output_path.is_file());
    }

    None
}

fn run_pngquant(
    source_path: &Path,
    output_path: &Path,
    quality: u8,
    encoder_directory: Option<&str>,
) -> Option<bool> {
    let pngquant = find_binary("pngquant", encoder_directory)?;
    let max_quality = quality.clamp(1, 100);
    let min_quality = max_quality.saturating_sub(18).max(1);
    let quality_range = format!("{}-{}", min_quality, max_quality);
    let status = Command::new(pngquant)
        .arg("--force")
        .arg("--quality")
        .arg(quality_range)
        .arg("--output")
        .arg(output_path)
        .arg("--")
        .arg(source_path)
        .status()
        .ok()?;

    Some(status.success() && output_path.is_file())
}

fn run_oxipng(
    output_path: &Path,
    strip_metadata: bool,
    encoder_directory: Option<&str>,
) -> Option<bool> {
    let oxipng = find_binary("oxipng", encoder_directory)?;
    let mut command = Command::new(oxipng);
    command.arg("-o").arg("3");

    if strip_metadata {
        command.arg("--strip").arg("safe");
    }

    let status = command.arg(output_path).status().ok()?;

    Some(status.success() && output_path.is_file())
}

fn jpeg_copy_mode(options: &CompressionOptions) -> &'static str {
    if !options.strip_metadata {
        "all"
    } else if options.preserve_color_profile {
        "icc"
    } else {
        "none"
    }
}

fn webp_metadata_mode(options: &CompressionOptions) -> &'static str {
    if !options.strip_metadata {
        "all"
    } else if options.preserve_color_profile {
        "icc"
    } else {
        "none"
    }
}

fn run_gifsicle(
    source_path: &Path,
    output_path: &Path,
    options: &CompressionOptions,
) -> Option<bool> {
    let gifsicle = find_binary("gifsicle", options.encoder_directory.as_deref())?;
    let mut command = Command::new(gifsicle);
    command.arg("-O3");
    for arg in gifsicle_resize_args(source_path, options) {
        command.arg(arg);
    }
    let status = command
        .arg("-o")
        .arg(output_path)
        .arg(source_path)
        .status()
        .ok()?;

    Some(status.success() && output_path.is_file())
}

fn run_gif2webp(
    source_path: &Path,
    output_path: &Path,
    quality: u8,
    encoder_directory: Option<&str>,
) -> Option<bool> {
    let gif2webp = find_binary("gif2webp", encoder_directory)?;
    let status = Command::new(gif2webp)
        .arg("-q")
        .arg(quality.clamp(1, 100).to_string())
        .arg(source_path)
        .arg("-o")
        .arg(output_path)
        .status()
        .ok()?;

    Some(status.success() && output_path.is_file())
}

fn run_svgo(
    source_path: &Path,
    output_path: &Path,
    encoder_directory: Option<&str>,
) -> Option<bool> {
    let svgo = find_binary("svgo", encoder_directory)?;
    let status = Command::new(svgo)
        .arg("--multipass")
        .arg("--input")
        .arg(source_path)
        .arg("--output")
        .arg(output_path)
        .status()
        .ok()?;

    Some(status.success() && output_path.is_file())
}

fn run_avifenc(
    source_path: &Path,
    output_path: &Path,
    quality: u8,
    encoder_directory: Option<&str>,
) -> Option<bool> {
    let avifenc = find_binary("avifenc", encoder_directory)?;
    let status = Command::new(avifenc)
        .arg("--speed")
        .arg("6")
        .arg("--jobs")
        .arg("all")
        .arg("--qcolor")
        .arg(quality.clamp(1, 100).to_string())
        .arg("--qalpha")
        .arg(quality.clamp(1, 100).to_string())
        .arg(source_path)
        .arg(output_path)
        .status()
        .ok()?;

    Some(status.success() && output_path.is_file())
}

#[cfg(test)]
fn simulated_outcome(asset: &AssetState, options: &CompressionOptions) -> CompressionOutcome {
    let input_format = image_format(Path::new(&asset.source_path))
        .unwrap_or_else(|| format_from_mime(asset.mime.as_deref()));
    let output_format = choose_output_format(&input_format, options);
    let ratio = estimated_saved_ratio(asset, &output_format);
    let output_size = asset.source_size.saturating_mul(100 - ratio as u64) / 100;
    let output_path = output_path_for(Path::new(&asset.source_path), &output_format);

    CompressionOutcome {
        output_path: output_path.to_string_lossy().to_string(),
        output_size,
        saved_ratio: ratio,
        output_format,
        width: asset.width,
        height: asset.height,
        used_real_encoder: false,
    }
}

fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    let data = std::fs::read(path).ok()?;
    dimensions_from_bytes(&data)
}

fn dimensions_from_bytes(data: &[u8]) -> Option<(u32, u32)> {
    png_dimensions(data)
        .or_else(|| gif_dimensions(data))
        .or_else(|| jpeg_dimensions(data))
        .or_else(|| webp_dimensions(data))
}

fn png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }

    Some((
        u32::from_be_bytes(data[16..20].try_into().ok()?),
        u32::from_be_bytes(data[20..24].try_into().ok()?),
    ))
}

fn gif_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 10 || !matches!(&data[0..6], b"GIF87a" | b"GIF89a") {
        return None;
    }

    Some((
        u16::from_le_bytes(data[6..8].try_into().ok()?) as u32,
        u16::from_le_bytes(data[8..10].try_into().ok()?) as u32,
    ))
}

fn jpeg_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 4 || data[0] != 0xff || data[1] != 0xd8 {
        return None;
    }

    let mut index = 2usize;
    while index + 9 < data.len() {
        while index < data.len() && data[index] == 0xff {
            index += 1;
        }
        if index >= data.len() {
            return None;
        }

        let marker = data[index];
        index += 1;

        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        if index + 2 > data.len() {
            return None;
        }

        let length = u16::from_be_bytes(data[index..index + 2].try_into().ok()?) as usize;
        if length < 2 || index + length > data.len() {
            return None;
        }

        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if length < 7 {
                return None;
            }
            let height = u16::from_be_bytes(data[index + 3..index + 5].try_into().ok()?) as u32;
            let width = u16::from_be_bytes(data[index + 5..index + 7].try_into().ok()?) as u32;
            return Some((width, height));
        }

        index += length;
    }

    None
}

fn webp_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 30 || &data[0..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return None;
    }

    match &data[12..16] {
        b"VP8X" if data.len() >= 30 => {
            let width =
                1 + u32::from(data[24]) + (u32::from(data[25]) << 8) + (u32::from(data[26]) << 16);
            let height =
                1 + u32::from(data[27]) + (u32::from(data[28]) << 8) + (u32::from(data[29]) << 16);
            Some((width, height))
        }
        b"VP8 " if data.len() >= 30 => {
            let width = u16::from_le_bytes(data[26..28].try_into().ok()?) as u32 & 0x3fff;
            let height = u16::from_le_bytes(data[28..30].try_into().ok()?) as u32 & 0x3fff;
            Some((width, height))
        }
        b"VP8L" if data.len() >= 25 => {
            let b0 = u32::from(data[21]);
            let b1 = u32::from(data[22]);
            let b2 = u32::from(data[23]);
            let b3 = u32::from(data[24]);
            let width = 1 + (((b1 & 0x3f) << 8) | b0);
            let height = 1 + ((b3 << 6) | (b2 << 2) | ((b1 & 0xc0) >> 6));
            Some((width, height))
        }
        _ => None,
    }
}

fn choose_output_format(input_format: &str, options: &CompressionOptions) -> String {
    match options.output_format.as_str() {
        "webp" => "webp".to_string(),
        "jpg" | "jpeg" => "jpg".to_string(),
        "png" => "png".to_string(),
        "avif" => {
            if find_binary("avifenc", options.encoder_directory.as_deref()).is_some() {
                "avif".to_string()
            } else if find_binary("cwebp", options.encoder_directory.as_deref()).is_some() {
                "webp".to_string()
            } else {
                input_format.to_string()
            }
        }
        _ if options.mode == "extreme_avif" => {
            if find_binary("avifenc", options.encoder_directory.as_deref()).is_some() {
                "avif".to_string()
            } else if find_binary("cwebp", options.encoder_directory.as_deref()).is_some() {
                "webp".to_string()
            } else {
                input_format.to_string()
            }
        }
        _ => input_format.to_string(),
    }
}

fn missing_encoder_hint(
    input_format: &str,
    output_format: &str,
    encoder_directory: Option<&str>,
) -> Option<String> {
    if builtin_encoder_available(input_format, output_format) {
        return None;
    }

    let missing = match output_format {
        "jpg" | "jpeg" => missing_all(&["cjpeg"], encoder_directory),
        "png" => missing_all(&["pngquant", "oxipng"], encoder_directory),
        "gif" => missing_all(&["gifsicle"], encoder_directory),
        "svg" => missing_all(&["svgo"], encoder_directory),
        "avif" => missing_all(&["avifenc"], encoder_directory),
        "webp" if input_format == "gif" => missing_all(&["gif2webp"], encoder_directory),
        "webp" => missing_all(&["cwebp"], encoder_directory),
        _ => Vec::new(),
    };

    if missing.is_empty() {
        None
    } else {
        Some(missing.join(" / "))
    }
}

fn builtin_encoder_available(input_format: &str, output_format: &str) -> bool {
    matches!(input_format, "jpg" | "jpeg" | "png")
        && matches!(output_format, "jpg" | "jpeg" | "png")
}

fn should_prefer_builtin(
    input_format: &str,
    output_format: &str,
    options: &CompressionOptions,
) -> bool {
    builtin_encoder_available(input_format, output_format)
        && (options.max_width.is_some() || options.max_height.is_some())
}

fn missing_all(names: &[&str], encoder_directory: Option<&str>) -> Vec<String> {
    if names
        .iter()
        .any(|name| find_binary(name, encoder_directory).is_some())
    {
        Vec::new()
    } else {
        names.iter().map(|name| name.to_string()).collect()
    }
}

#[cfg(test)]
fn estimated_saved_ratio(asset: &AssetState, output_format: &str) -> u8 {
    if output_format == "avif" {
        return 82;
    }
    if output_format == "webp" {
        return 76;
    }
    if asset.mime.as_deref().unwrap_or("").contains("svg") || output_format == "svg" {
        return 82;
    }
    if asset.mime.as_deref().unwrap_or("").contains("gif") || output_format == "gif" {
        return 61;
    }

    74
}

fn output_path_for(source_path: &Path, output_format: &str) -> PathBuf {
    let parent = source_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    parent.join(format!(
        "{}.picdog.{}",
        stem,
        normalize_extension(output_format)
    ))
}

fn image_format(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| normalize_extension(&value.to_ascii_lowercase()).to_string())
}

fn format_from_mime(mime: Option<&str>) -> String {
    match mime.unwrap_or("") {
        "image/jpeg" => "jpg".to_string(),
        "image/png" => "png".to_string(),
        "image/gif" => "gif".to_string(),
        "image/webp" => "webp".to_string(),
        "image/svg+xml" => "svg".to_string(),
        "image/avif" => "avif".to_string(),
        _ => "original".to_string(),
    }
}

fn normalize_extension(extension: &str) -> &str {
    match extension {
        "jpeg" => "jpg",
        other => other,
    }
}

fn saved_ratio(source_size: u64, output_size: u64) -> u8 {
    if source_size == 0 || output_size >= source_size {
        return 0;
    }

    (((source_size - output_size) * 100) / source_size).min(100) as u8
}

fn find_binary(name: &str, encoder_directory: Option<&str>) -> Option<PathBuf> {
    find_bundled_binary(name, encoder_directory).or_else(|| find_binary_on_path(name))
}

fn find_bundled_binary(name: &str, encoder_directory: Option<&str>) -> Option<PathBuf> {
    let mut roots = Vec::new();

    if let Some(path) = encoder_directory
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        roots.push(path);
    }

    if let Some(path) = std::env::var_os("PICDOG_ENCODER_DIR").map(PathBuf::from) {
        roots.push(path);
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            roots.push(exe_dir.join("../Resources/bin"));
            roots.push(exe_dir.join("resources/bin"));
            roots.push(exe_dir.join("../resources/bin"));
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir.join("resources/bin"));
        roots.push(current_dir.join("src-tauri/resources/bin"));
    }

    find_binary_in_dirs(name, roots)
}

fn find_binary_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_binary_in_dirs(name, std::env::split_paths(&path))
}

fn find_binary_in_dirs(name: &str, dirs: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    dirs.into_iter()
        .flat_map(|dir| binary_name_candidates(&dir.join(name)))
        .find(|candidate| candidate.is_file())
}

fn binary_name_candidates(candidate: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![candidate.to_path_buf()];
    candidates.extend(executable_extension_candidates(candidate));
    candidates
}

fn executable_extension_candidates(candidate: &Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        ["exe", "cmd", "bat"]
            .into_iter()
            .map(|extension| {
                let mut value = OsString::from(candidate);
                value.push(".");
                value.push(extension);
                PathBuf::from(value)
            })
            .collect()
    }

    #[cfg(not(windows))]
    {
        let _ = candidate;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_IMAGE_COUNTER: AtomicU64 = AtomicU64::new(0);
    use crate::models::{CompressStatus, UploadStatus};

    fn asset(path: &str, mime: &str) -> AssetState {
        AssetState {
            id: "asset-1".to_string(),
            batch_id: "batch-1".to_string(),
            name: "hero.png".to_string(),
            source_path: path.to_string(),
            output_path: None,
            source_size: 1_000,
            output_size: None,
            mime: Some(mime.to_string()),
            width: None,
            height: None,
            saved_ratio: None,
            compress_status: CompressStatus::Pending,
            upload_status: UploadStatus::None,
            compress_progress: 0,
            upload_progress: 0,
            selected: false,
            route_id: None,
            public_url: None,
            error: None,
        }
    }

    fn options(output_format: &str) -> CompressionOptions {
        CompressionOptions {
            mode: "balanced".to_string(),
            quality: 78,
            encoder_directory: None,
            max_width: None,
            max_height: None,
            output_format: output_format.to_string(),
            strip_metadata: true,
            preserve_color_profile: true,
            keep_original: true,
            fallback_when_larger: true,
        }
    }

    fn temp_image_path(extension: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEMP_IMAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "picdog-compression-test-{}-{}-{}.{}",
            std::process::id(),
            nanos,
            counter,
            extension
        ))
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        let image = image::RgbImage::from_pixel(width, height, image::Rgb([244, 89, 89]));
        image.save(path).unwrap();
    }

    fn write_test_gif_header(path: &Path, width: u16, height: u16) {
        let mut data = Vec::from(*b"GIF89a");
        data.extend(width.to_le_bytes());
        data.extend(height.to_le_bytes());
        data.extend([0, 0, 0, 0]);
        std::fs::write(path, data).unwrap();
    }

    fn write_test_webp_vp8x_header(path: &Path, width: u32, height: u32) {
        let mut data = vec![0u8; 30];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&22u32.to_le_bytes());
        data[8..12].copy_from_slice(b"WEBP");
        data[12..16].copy_from_slice(b"VP8X");

        let encoded_width = width.saturating_sub(1);
        let encoded_height = height.saturating_sub(1);
        data[24] = (encoded_width & 0xff) as u8;
        data[25] = ((encoded_width >> 8) & 0xff) as u8;
        data[26] = ((encoded_width >> 16) & 0xff) as u8;
        data[27] = (encoded_height & 0xff) as u8;
        data[28] = ((encoded_height >> 8) & 0xff) as u8;
        data[29] = ((encoded_height >> 16) & 0xff) as u8;

        std::fs::write(path, data).unwrap();
    }

    fn options_with_metadata(
        strip_metadata: bool,
        preserve_color_profile: bool,
    ) -> CompressionOptions {
        CompressionOptions {
            strip_metadata,
            preserve_color_profile,
            ..options("original")
        }
    }

    #[test]
    fn simulated_outcome_uses_picdog_suffix() {
        let outcome = simulated_outcome(&asset("/tmp/hero.png", "image/png"), &options("original"));

        assert_eq!(outcome.output_path, "/tmp/hero.picdog.png");
        assert_eq!(outcome.output_format, "png");
        assert!(!outcome.used_real_encoder);
    }

    #[test]
    fn compress_asset_rejects_missing_source_file() {
        let error = compress_asset(
            &asset("/tmp/picdog-missing-source.png", "image/png"),
            &options("original"),
        )
        .unwrap_err();

        assert_eq!(error.code, "file_not_found");
    }

    #[test]
    fn builtin_compression_encodes_jpeg_without_external_tool() {
        let source = temp_image_path("png");
        write_test_png(&source, 16, 12);
        let mut asset = asset(source.to_str().unwrap(), "image/png");
        asset.source_size = source.metadata().unwrap().len();
        let mut options = options("jpg");
        options.fallback_when_larger = false;

        let outcome = try_builtin_compression(&asset, &options, &source).unwrap();

        assert_eq!(outcome.output_format, "jpg");
        assert_eq!(outcome.width, Some(16));
        assert_eq!(outcome.height, Some(12));
        assert!(Path::new(&outcome.output_path).is_file());

        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(outcome.output_path);
    }

    #[test]
    fn builtin_compression_applies_resize_limits() {
        let source = temp_image_path("png");
        write_test_png(&source, 20, 10);
        let mut asset = asset(source.to_str().unwrap(), "image/png");
        asset.source_size = source.metadata().unwrap().len();
        let mut options = options("png");
        options.max_width = Some(10);
        options.fallback_when_larger = false;

        let outcome = try_builtin_compression(&asset, &options, &source).unwrap();

        assert_eq!(outcome.width, Some(10));
        assert_eq!(outcome.height, Some(5));

        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(outcome.output_path);
    }

    #[test]
    fn compress_asset_applies_resize_limits_for_jpeg_png() {
        let source = temp_image_path("png");
        write_test_png(&source, 20, 10);
        let mut asset = asset(source.to_str().unwrap(), "image/png");
        asset.source_size = source.metadata().unwrap().len();
        let mut options = options("original");
        options.max_width = Some(10);
        options.fallback_when_larger = false;

        let outcome = compress_asset(&asset, &options).unwrap();

        assert_eq!(outcome.width, Some(10));
        assert_eq!(outcome.height, Some(5));

        let _ = std::fs::remove_file(source);
        let _ = std::fs::remove_file(outcome.output_path);
    }

    #[test]
    fn resize_limits_preserve_aspect_ratio_without_upscaling() {
        assert_eq!(
            target_dimensions_for_limits(2000, 1000, Some(1000), None),
            Some((1000, 500))
        );
        assert_eq!(
            target_dimensions_for_limits(2000, 1000, Some(1200), Some(400)),
            Some((800, 400))
        );
        assert_eq!(
            target_dimensions_for_limits(800, 600, Some(1200), None),
            None
        );
    }

    #[test]
    fn external_resize_input_prepares_bounded_intermediate_for_webp_and_avif() {
        let source = temp_image_path("png");
        write_test_png(&source, 20, 10);
        let mut options = options("webp");
        options.max_width = Some(10);

        let webp_input = prepare_external_resize_input(&source, "png", "webp", &options)
            .expect("webp resize input");

        assert_eq!((webp_input.width, webp_input.height), (10, 5));
        assert_eq!(image_dimensions(&webp_input.path), Some((10, 5)));

        let _ = std::fs::remove_file(webp_input.path);

        options.output_format = "avif".to_string();
        options.max_width = None;
        options.max_height = Some(4);
        let avif_input = prepare_external_resize_input(&source, "png", "avif", &options)
            .expect("avif resize input");

        assert_eq!((avif_input.width, avif_input.height), (8, 4));
        assert_eq!(image_dimensions(&avif_input.path), Some((8, 4)));

        let _ = std::fs::remove_file(avif_input.path);
        let _ = std::fs::remove_file(source);
    }

    #[test]
    fn external_tool_resize_args_cover_webp_and_gif_sources() {
        let webp = temp_image_path("webp");
        write_test_webp_vp8x_header(&webp, 1024, 512);
        let mut webp_options = options("webp");
        webp_options.max_width = Some(320);

        assert_eq!(
            cwebp_resize_args(&webp, &webp_options),
            vec!["-resize", "320", "160"]
        );

        webp_options.max_width = Some(2000);
        assert!(cwebp_resize_args(&webp, &webp_options).is_empty());

        let gif = temp_image_path("gif");
        write_test_gif_header(&gif, 800, 600);
        let mut gif_options = options("original");
        gif_options.max_height = Some(120);

        assert_eq!(
            gifsicle_resize_args(&gif, &gif_options),
            vec!["--resize-fit", "160x120"]
        );

        let _ = std::fs::remove_file(webp);
        let _ = std::fs::remove_file(gif);
    }

    #[test]
    fn finds_binary_in_candidate_directories_before_path_lookup() {
        let dir = temp_image_path("dir");
        std::fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("cwebp");
        std::fs::write(&binary, b"").unwrap();

        assert_eq!(find_binary_in_dirs("cwebp", [dir.clone()]).unwrap(), binary);

        let _ = std::fs::remove_file(binary);
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn compression_tool_statuses_use_configured_encoder_directory_first() {
        let dir = temp_image_path("encoders");
        std::fs::create_dir_all(&dir).unwrap();
        let binary = dir.join("avifenc");
        std::fs::write(&binary, b"").unwrap();
        let dir_string = dir.to_string_lossy().to_string();

        let statuses = compression_tool_statuses(Some(&dir_string));
        let avifenc = statuses
            .iter()
            .find(|tool| tool.name == "avifenc")
            .expect("avifenc status");

        assert!(avifenc.available);
        assert_eq!(avifenc.path.as_deref(), Some(binary.to_str().unwrap()));

        let _ = std::fs::remove_file(binary);
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn webp_output_changes_extension() {
        let outcome = simulated_outcome(&asset("hero.jpg", "image/jpeg"), &options("webp"));

        assert_eq!(outcome.output_path, "hero.picdog.webp");
        assert_eq!(outcome.output_format, "webp");
        assert_eq!(outcome.saved_ratio, 76);
    }

    #[test]
    fn original_mode_keeps_svg_and_gif_formats() {
        let svg = simulated_outcome(&asset("icon.svg", "image/svg+xml"), &options("original"));
        let gif = simulated_outcome(&asset("motion.gif", "image/gif"), &options("original"));

        assert_eq!(svg.output_format, "svg");
        assert_eq!(svg.output_path, "icon.picdog.svg");
        assert_eq!(gif.output_format, "gif");
        assert_eq!(gif.output_path, "motion.picdog.gif");
    }

    #[test]
    fn requested_png_output_uses_png_suffix() {
        let outcome = simulated_outcome(&asset("hero.jpg", "image/jpeg"), &options("png"));

        assert_eq!(outcome.output_format, "png");
        assert_eq!(outcome.output_path, "hero.picdog.png");
    }

    #[test]
    fn saved_ratio_never_underflows() {
        assert_eq!(saved_ratio(100, 120), 0);
        assert_eq!(saved_ratio(100, 25), 75);
    }

    #[test]
    fn metadata_preferences_map_to_encoder_flags() {
        let strip_keep_color = options_with_metadata(true, true);
        let strip_all = options_with_metadata(true, false);
        let keep_all = options_with_metadata(false, false);

        assert_eq!(jpeg_copy_mode(&strip_keep_color), "icc");
        assert_eq!(webp_metadata_mode(&strip_keep_color), "icc");
        assert_eq!(jpeg_copy_mode(&strip_all), "none");
        assert_eq!(webp_metadata_mode(&strip_all), "none");
        assert_eq!(jpeg_copy_mode(&keep_all), "all");
        assert_eq!(webp_metadata_mode(&keep_all), "all");
    }

    #[test]
    fn reads_png_gif_and_jpeg_dimensions_from_headers() {
        let png = [
            b"\x89PNG\r\n\x1a\n".as_slice(),
            &[0, 0, 0, 13],
            b"IHDR".as_slice(),
            &1200u32.to_be_bytes(),
            &800u32.to_be_bytes(),
        ]
        .concat();
        assert_eq!(dimensions_from_bytes(&png), Some((1200, 800)));

        let gif = *b"GIF89a\x40\x01\xf0\x00";
        assert_eq!(dimensions_from_bytes(&gif), Some((320, 240)));

        let webp = {
            let mut data = vec![0u8; 30];
            data[0..4].copy_from_slice(b"RIFF");
            data[8..12].copy_from_slice(b"WEBP");
            data[12..16].copy_from_slice(b"VP8X");
            data[24] = 0xff;
            data[25] = 0x03;
            data[27] = 0xff;
            data[28] = 0x01;
            data
        };
        assert_eq!(dimensions_from_bytes(&webp), Some((1024, 512)));

        let jpeg = [
            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00, 0xff, 0xc0, 0x00, 0x08, 0x08, 0x01,
            0x2c, 0x02, 0x58, 0x03,
        ];
        assert_eq!(dimensions_from_bytes(&jpeg), Some((600, 300)));
    }
}
