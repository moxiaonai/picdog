use crate::error::{CommandResult, SafeError};
use crate::models::{
    ClearHistoryInput, CompressStatus, HistoryItem, HistoryPage, HistoryQuery, UploadStatus,
};
use crate::state::AppState;
use std::fs;
use tauri::State;

#[tauri::command]
pub async fn list_history(
    state: State<'_, AppState>,
    query: HistoryQuery,
) -> CommandResult<HistoryPage> {
    state.list_history(&query)
}

#[tauri::command]
pub async fn clear_history(
    state: State<'_, AppState>,
    input: ClearHistoryInput,
) -> CommandResult<()> {
    state.clear_history(&input)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn export_history(
    state: State<'_, AppState>,
    query: HistoryQuery,
    title: Option<String>,
) -> CommandResult<String> {
    let dialog_title = title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Export PicDog history");

    let Some(path) = rfd::FileDialog::new()
        .set_title(dialog_title)
        .set_file_name("picdog-history.csv")
        .add_filter("CSV", &["csv"])
        .save_file()
    else {
        return Err(SafeError::new(
            "cancelled",
            "已取消导出历史记录。",
            true,
            false,
        ));
    };

    let items = collect_history_items(&state, &query)?;
    let settings = state.get_settings()?;
    let csv = render_history_csv(&items, settings.hide_local_path);

    fs::write(&path, csv)
        .map_err(|_| SafeError::new("permission_denied", "无法写入历史导出文件。", true, true))?;

    Ok(path.to_string_lossy().to_string())
}

fn collect_history_items(
    state: &AppState,
    query: &HistoryQuery,
) -> CommandResult<Vec<HistoryItem>> {
    let mut page_number = 1;
    let page_size = 100;
    let mut items = Vec::new();

    loop {
        let mut page_query = query.clone();
        page_query.page = page_number;
        page_query.page_size = page_size;

        let page = state.list_history(&page_query)?;
        let total = page.total as usize;
        items.extend(page.items);

        if items.len() >= total || total == 0 {
            break;
        }

        page_number += 1;
    }

    Ok(items)
}

fn render_history_csv(items: &[HistoryItem], hide_local_path: bool) -> String {
    let mut lines = vec![[
        "file_name",
        "source_path",
        "output_path",
        "source_size",
        "output_size",
        "saved_ratio",
        "width",
        "height",
        "route_name",
        "route_id",
        "public_url",
        "compress_status",
        "upload_status",
        "created_at",
        "uploaded_at",
        "copied_at",
    ]
    .join(",")];

    for item in items {
        lines.push(
            [
                csv_cell(&item.file_name),
                csv_cell(if hide_local_path {
                    ""
                } else {
                    item.source_path.as_deref().unwrap_or_default()
                }),
                csv_cell(if hide_local_path {
                    ""
                } else {
                    item.output_path.as_deref().unwrap_or_default()
                }),
                item.source_size.to_string(),
                item.output_size
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                item.saved_ratio
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                item.width
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                item.height
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                csv_cell(item.route_name.as_deref().unwrap_or_default()),
                csv_cell(item.route_id.as_deref().unwrap_or_default()),
                csv_cell(item.public_url.as_deref().unwrap_or_default()),
                compress_status_label(&item.compress_status).to_string(),
                upload_status_label(&item.upload_status).to_string(),
                csv_cell(&item.created_at),
                csv_cell(item.uploaded_at.as_deref().unwrap_or_default()),
                csv_cell(item.copied_at.as_deref().unwrap_or_default()),
            ]
            .join(","),
        );
    }

    lines.push(String::new());
    lines.join("\n")
}

fn csv_cell(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn compress_status_label(status: &CompressStatus) -> &'static str {
    match status {
        CompressStatus::Pending => "pending",
        CompressStatus::Running => "running",
        CompressStatus::Done => "done",
        CompressStatus::Failed => "failed",
        CompressStatus::Cancelled => "cancelled",
    }
}

fn upload_status_label(status: &UploadStatus) -> &'static str {
    match status {
        UploadStatus::None => "none",
        UploadStatus::Pending => "pending",
        UploadStatus::Running => "running",
        UploadStatus::Done => "done",
        UploadStatus::Failed => "failed",
        UploadStatus::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::csv_cell;

    #[test]
    fn csv_cell_escapes_commas_quotes_and_newlines() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("hero,large"), "\"hero,large\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_cell("line\nbreak"), "\"line\nbreak\"");
    }
}
