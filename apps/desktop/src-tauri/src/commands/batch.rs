use crate::error::{CommandResult, SafeError};
use crate::models::{
    now_string, AssetState, Batch, BatchState, CompressStatus, FileInput, UploadStatus,
};
use crate::state::AppState;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Emitter, State};

const TASK_EVENT: &str = "picdog://task-event";

#[tauri::command]
pub async fn scan_dropped_paths(paths: Vec<String>) -> CommandResult<Vec<FileInput>> {
    let mut files = Vec::new();
    let mut seen = HashSet::new();

    for path in paths {
        collect_image_inputs(PathBuf::from(path), &mut files, &mut seen)?;
    }

    Ok(files)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn select_files(title: Option<String>) -> CommandResult<Vec<FileInput>> {
    let dialog_title = title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Select images");

    let Some(paths) = rfd::FileDialog::new()
        .set_title(dialog_title)
        .add_filter(
            "Images",
            &["jpg", "jpeg", "png", "gif", "webp", "svg", "avif"],
        )
        .pick_files()
    else {
        return Ok(Vec::new());
    };

    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for path in paths {
        collect_image_inputs(path, &mut files, &mut seen)?;
    }

    Ok(files)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn read_image_preview(path: String) -> CommandResult<Option<String>> {
    preview_data_url(Path::new(&path))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_batch(state: State<'_, AppState>, batch_id: String) -> CommandResult<Batch> {
    state
        .get_batch(&batch_id)?
        .ok_or_else(|| SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false))
}

#[tauri::command]
pub async fn get_latest_batch(state: State<'_, AppState>) -> CommandResult<Option<Batch>> {
    state.latest_batch()
}

#[tauri::command]
pub async fn create_batch(
    state: State<'_, AppState>,
    files: Vec<FileInput>,
) -> CommandResult<Batch> {
    let now = now_string();
    let batch_id = format!("batch-{}", now);

    let mut seen = HashSet::new();
    let assets = files
        .into_iter()
        .filter(|file| seen.insert(file.path.clone()))
        .enumerate()
        .map(|(index, file)| AssetState {
            id: format!("asset-{}-{}", batch_id, index),
            batch_id: batch_id.clone(),
            name: file.name,
            source_path: file.path,
            output_path: None,
            source_size: file.size,
            output_size: None,
            mime: file.mime,
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
        })
        .collect();

    let batch = Batch {
        id: batch_id.clone(),
        status: BatchState::Queue,
        assets,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut batches = state
        .batches
        .lock()
        .map_err(|_| SafeError::new("state_poisoned", "无法访问批次状态。", true, true))?;
    batches.insert(batch_id, batch.clone());

    state.upsert_batch(&batch)?;

    Ok(batch)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn clear_batch(
    state: State<'_, AppState>,
    batch_id: String,
    delete_local_outputs: Option<bool>,
) -> CommandResult<()> {
    state.clear_batch(&batch_id, delete_local_outputs.unwrap_or(false))?;

    let mut batches = state
        .batches
        .lock()
        .map_err(|_| SafeError::new("state_poisoned", "无法访问批次状态。", true, true))?;
    batches.remove(&batch_id);

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn cancel_task(
    app: AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
) -> CommandResult<Batch> {
    let batch = state.request_cancel_task(&batch_id)?;
    let _ = app.emit(
        TASK_EVENT,
        serde_json::json!({
            "type": "batch.status.changed",
            "batchId": batch.id,
            "status": "cancelling"
        }),
    );

    Ok(batch)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn reveal_output(state: State<'_, AppState>, asset_id: String) -> CommandResult<()> {
    let item = state.get_history_item(&asset_id)?;
    let output_path = item.output_path.ok_or_else(|| {
        SafeError::new(
            "file_not_found",
            "这张图片还没有可定位的压缩输出文件。",
            true,
            false,
        )
    })?;

    reveal_local_path(Path::new(&output_path))
}

fn collect_image_inputs(
    path: PathBuf,
    files: &mut Vec<FileInput>,
    seen: &mut HashSet<String>,
) -> CommandResult<()> {
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(()),
    };

    if metadata.is_dir() {
        let entries = fs::read_dir(&path)
            .map_err(|_| SafeError::new("permission_denied", "无法读取文件夹。", true, false))?;

        for entry in entries.flatten() {
            collect_image_inputs(entry.path(), files, seen)?;
        }

        return Ok(());
    }

    if !metadata.is_file() || !is_supported_image(&path) {
        return Ok(());
    }

    let canonical_path = path.canonicalize().unwrap_or(path);
    let source_path = canonical_path.to_string_lossy().to_string();
    if !seen.insert(source_path.clone()) {
        return Ok(());
    }
    let name = canonical_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("image")
        .to_string();

    files.push(FileInput {
        path: source_path,
        name,
        size: metadata.len(),
        mime: mime_for_path(&canonical_path).map(str::to_string),
        is_directory: Some(false),
    });

    Ok(())
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "avif"
            )
        })
        .unwrap_or(false)
}

fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("png") => Some("image/png"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        Some("svg") => Some("image/svg+xml"),
        Some("avif") => Some("image/avif"),
        _ => None,
    }
}

fn preview_data_url(path: &Path) -> CommandResult<Option<String>> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };

    if !metadata.is_file() || !is_supported_image(path) {
        return Ok(None);
    }

    let mime = mime_for_path(path).unwrap_or("application/octet-stream");

    if matches!(mime, "image/jpeg" | "image/png") {
        if let Ok(data_url) = thumbnail_data_url(path) {
            return Ok(Some(data_url));
        }
    }

    if metadata.len() > 8 * 1024 * 1024 {
        return Ok(None);
    }

    let bytes = fs::read(path)
        .map_err(|_| SafeError::new("file_not_found", "无法读取图片缩略图。", true, false))?;

    Ok(Some(format!(
        "data:{};base64,{}",
        mime,
        STANDARD.encode(bytes)
    )))
}

fn thumbnail_data_url(path: &Path) -> Result<String, image::ImageError> {
    let image = image::open(path)?;
    let thumbnail = image.thumbnail(96, 96);
    let mut bytes = Cursor::new(Vec::new());
    thumbnail.write_to(&mut bytes, image::ImageFormat::Png)?;

    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(bytes.into_inner())
    ))
}

fn reveal_local_path(path: &Path) -> CommandResult<()> {
    if !path.exists() {
        return Err(SafeError::new(
            "file_not_found",
            "本地压缩输出文件不存在，可能已被清除。",
            true,
            false,
        ));
    }

    let status = platform_reveal_command(path)
        .status()
        .map_err(|_| SafeError::new("permission_denied", "无法打开系统文件管理器。", true, true))?;

    if status.success() {
        return Ok(());
    }

    Err(SafeError::new(
        "permission_denied",
        "无法定位本地压缩输出文件。",
        true,
        true,
    ))
}

fn platform_reveal_command(path: &Path) -> Command {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        command
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", path.to_string_lossy()));
        command
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let mut command = Command::new("xdg-open");
        command.arg(path.parent().unwrap_or(path));
        command
    }
}

#[cfg(test)]
mod reveal_tests {
    use super::{collect_image_inputs, platform_reveal_command};
    use std::collections::HashSet;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn reveal_command_targets_native_file_manager() {
        let command = platform_reveal_command(Path::new("/tmp/example.picdog.webp"));
        let program = command.get_program().to_string_lossy();

        #[cfg(target_os = "macos")]
        assert_eq!(program, "open");

        #[cfg(target_os = "windows")]
        assert_eq!(program, "explorer");

        #[cfg(all(unix, not(target_os = "macos")))]
        assert_eq!(program, "xdg-open");
    }

    #[test]
    fn collected_image_inputs_are_deduped_by_canonical_path() {
        let dir = std::env::temp_dir().join(format!(
            "picdog-dedupe-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let image = dir.join("hero.png");
        fs::write(&image, b"fake png").unwrap();

        let mut files = Vec::new();
        let mut seen = HashSet::new();
        collect_image_inputs(dir.clone(), &mut files, &mut seen).unwrap();
        collect_image_inputs(image.clone(), &mut files, &mut seen).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "hero.png");

        let _ = fs::remove_file(image);
        let _ = fs::remove_dir(dir);
    }
}
