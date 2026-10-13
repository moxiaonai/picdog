use crate::error::{CommandResult, SafeError};
use crate::models::now_string;
use crate::state::AppState;
use std::io::Write;
use std::process::{Command, Stdio};
use tauri::State;

#[tauri::command(rename_all = "camelCase")]
pub async fn copy_link(
    state: State<'_, AppState>,
    asset_id: String,
    format: String,
) -> CommandResult<String> {
    let from_batch = {
        let batches = state
            .batches
            .lock()
            .map_err(|_| SafeError::new("state_poisoned", "无法访问批次状态。", true, true))?;

        batches.values().find_map(|batch| {
            batch
                .assets
                .iter()
                .find(|asset| asset.id == asset_id)
                .map(|asset| {
                    (
                        asset.name.clone(),
                        asset.public_url.clone(),
                        asset.route_id.clone(),
                        asset.width,
                        asset.height,
                    )
                })
        })
    };

    if let Some((name, public_url, route_id, width, height)) = from_batch {
        let url = public_url.ok_or_else(|| {
            SafeError::new(
                "asset_not_uploaded",
                "上传完成后才能复制 CDN 链接。",
                true,
                false,
            )
        })?;

        let custom_template = custom_template_for_route(&state, route_id.as_deref());

        return Ok(format_link(
            &name,
            &url,
            &format,
            custom_template.as_deref(),
            width,
            height,
        ));
    }

    let item = state.get_history_item(&asset_id)?;
    let url = item.public_url.clone().ok_or_else(|| {
        SafeError::new(
            "asset_not_uploaded",
            "上传完成后才能复制 CDN 链接。",
            true,
            false,
        )
    })?;

    let custom_template = custom_template_for_route(&state, item.route_id.as_deref());

    Ok(format_link(
        &item.file_name,
        &url,
        &format,
        custom_template.as_deref(),
        item.width,
        item.height,
    ))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn write_clipboard_text(text: String) -> CommandResult<()> {
    write_system_clipboard(&text)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn mark_link_copied(state: State<'_, AppState>, asset_id: String) -> CommandResult<()> {
    let copied_at = now_string();
    state.mark_history_copied(&asset_id, &copied_at)
}

fn write_system_clipboard(text: &str) -> CommandResult<()> {
    #[cfg(target_os = "macos")]
    {
        return write_to_command("pbcopy", &[], text);
    }

    #[cfg(target_os = "windows")]
    {
        return write_to_command(
            "powershell",
            &[
                "-NoProfile",
                "-Command",
                "Set-Clipboard -Value ([Console]::In.ReadToEnd())",
            ],
            text,
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        write_to_command("wl-copy", &[], text)
            .or_else(|_| write_to_command("xclip", &["-selection", "clipboard"], text))
    }
}

fn write_to_command(program: &str, args: &[&str], text: &str) -> CommandResult<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|_| {
            SafeError::new(
                "clipboard_failed",
                "系统剪贴板不可用，请检查应用权限或手动复制。",
                true,
                true,
            )
        })?;

    let mut stdin = child.stdin.take().ok_or_else(|| {
        SafeError::new(
            "clipboard_failed",
            "系统剪贴板不可用，请检查应用权限或手动复制。",
            true,
            true,
        )
    })?;
    stdin.write_all(text.as_bytes()).map_err(|_| {
        SafeError::new(
            "clipboard_failed",
            "系统剪贴板写入失败，请手动复制。",
            true,
            true,
        )
    })?;
    drop(stdin);

    let status = child.wait().map_err(|_| {
        SafeError::new(
            "clipboard_failed",
            "系统剪贴板写入失败，请手动复制。",
            true,
            true,
        )
    })?;

    if status.success() {
        return Ok(());
    }

    Err(SafeError::new(
        "clipboard_failed",
        "系统剪贴板写入失败，请手动复制。",
        true,
        true,
    ))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn mark_links_copied(
    state: State<'_, AppState>,
    asset_ids: Vec<String>,
) -> CommandResult<()> {
    let copied_at = now_string();

    for asset_id in asset_ids {
        state.mark_history_copied(&asset_id, &copied_at)?;
    }

    Ok(())
}

fn custom_template_for_route(
    state: &State<'_, AppState>,
    route_id: Option<&str>,
) -> Option<String> {
    route_id
        .and_then(|route_id| state.get_route(route_id).ok())
        .and_then(|route| route.custom_link_template)
}

fn format_link(
    name: &str,
    url: &str,
    format: &str,
    custom_template: Option<&str>,
    width: Option<u32>,
    height: Option<u32>,
) -> String {
    match format {
        "markdown" => format!("![{}]({})", name, url),
        "html" => {
            let size = match (width, height) {
                (Some(width), Some(height)) => {
                    format!(" width=\"{}\" height=\"{}\"", width, height)
                }
                _ => String::new(),
            };
            format!(
                "<img src=\"{}\" alt=\"{}\"{} />",
                url,
                escape_attr(name),
                size
            )
        }
        "custom" => custom_template
            .map(|template| render_custom_template(template, name, url, width, height))
            .unwrap_or_else(|| url.to_string()),
        _ => url.to_string(),
    }
}

fn render_custom_template(
    template: &str,
    name: &str,
    url: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> String {
    let width = width.map(|value| value.to_string()).unwrap_or_default();
    let height = height.map(|value| value.to_string()).unwrap_or_default();

    template
        .replace("{name}", name)
        .replace("{alt}", name)
        .replace("{url}", url)
        .replace("{width}", &width)
        .replace("{height}", &height)
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::format_link;

    #[test]
    fn formats_copy_text_by_format() {
        assert_eq!(
            format_link(
                "hero.png",
                "https://cdn.example/hero.webp",
                "url",
                None,
                None,
                None
            ),
            "https://cdn.example/hero.webp"
        );
        assert_eq!(
            format_link(
                "hero.png",
                "https://cdn.example/hero.webp",
                "markdown",
                None,
                None,
                None
            ),
            "![hero.png](https://cdn.example/hero.webp)"
        );
        assert_eq!(
            format_link(
                "hero.png",
                "https://cdn.example/hero.webp",
                "html",
                None,
                None,
                None
            ),
            "<img src=\"https://cdn.example/hero.webp\" alt=\"hero.png\" />"
        );
    }

    #[test]
    fn escapes_html_alt_text() {
        assert_eq!(
            format_link(
                "a&b\"<.png",
                "https://cdn.example/a.webp",
                "html",
                None,
                None,
                None
            ),
            "<img src=\"https://cdn.example/a.webp\" alt=\"a&amp;b&quot;&lt;.png\" />"
        );
    }

    #[test]
    fn html_format_includes_dimensions_when_available() {
        assert_eq!(
            format_link(
                "hero.png",
                "https://cdn.example/hero.webp",
                "html",
                None,
                Some(1200),
                Some(800)
            ),
            "<img src=\"https://cdn.example/hero.webp\" alt=\"hero.png\" width=\"1200\" height=\"800\" />"
        );
    }

    #[test]
    fn renders_custom_template_when_available() {
        assert_eq!(
            format_link(
                "hero.png",
                "https://cdn.example/hero.webp",
                "custom",
                Some("<picture>{name}:{alt}:{url}:{width}x{height}</picture>"),
                Some(1200),
                Some(800)
            ),
            "<picture>hero.png:hero.png:https://cdn.example/hero.webp:1200x800</picture>"
        );
    }

    #[test]
    fn custom_template_uses_empty_dimensions_when_missing() {
        assert_eq!(
            format_link(
                "hero.png",
                "https://cdn.example/hero.webp",
                "custom",
                Some("{name}:{alt}:{width}x{height}"),
                None,
                None
            ),
            "hero.png:hero.png:x"
        );
    }

    #[test]
    fn custom_template_falls_back_to_url() {
        assert_eq!(
            format_link(
                "hero.png",
                "https://cdn.example/hero.webp",
                "custom",
                None,
                None,
                None
            ),
            "https://cdn.example/hero.webp"
        );
    }
}
