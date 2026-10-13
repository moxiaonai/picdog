use crate::compression::{compress_asset, compression_tool_statuses};
use crate::error::{CommandResult, SafeError};
use crate::models::{
    Batch, BatchState, CompressStatus, CompressionOptions, CompressionToolStatus, HistoryItem,
    UploadStatus,
};
use crate::state::AppState;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

const TASK_EVENT: &str = "picdog://task-event";

#[tauri::command(rename_all = "camelCase")]
pub async fn check_compression_tools(
    encoder_directory: Option<String>,
) -> CommandResult<Vec<CompressionToolStatus>> {
    Ok(compression_tool_statuses(encoder_directory.as_deref()))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn start_compression(
    app: AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
    options: CompressionOptions,
) -> CommandResult<Batch> {
    state.ensure_batch_loaded(&batch_id)?;
    state.clear_cancel_request(&batch_id)?;
    emit_task_event(
        &app,
        json!({ "type": "batch.status.changed", "batchId": batch_id, "status": "compressing" }),
    );

    let assets_to_process = {
        let mut batches = state
            .batches
            .lock()
            .map_err(|_| SafeError::new("state_poisoned", "无法访问批次状态。", true, true))?;
        let batch = batches.get_mut(&batch_id).ok_or_else(|| {
            SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false)
        })?;

        batch.status = BatchState::Compressing;
        batch.updated_at = crate::models::now_string();
        batch
            .assets
            .iter()
            .filter(|asset| {
                !matches!(asset.compress_status, CompressStatus::Done)
                    || asset.output_path.is_none()
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    let mut outcomes = Vec::new();
    let mut cancelled_ids = Vec::new();

    for asset in &assets_to_process {
        if state.is_task_cancelled(&batch_id) {
            cancelled_ids.push(asset.id.clone());
            emit_task_event(
                &app,
                json!({ "type": "asset.cancelled", "batchId": batch_id, "assetId": asset.id, "stage": "compress" }),
            );
            continue;
        }

        emit_task_event(
            &app,
            json!({ "type": "asset.compress.started", "batchId": batch_id, "assetId": asset.id }),
        );
        emit_task_event(
            &app,
            json!({ "type": "asset.compress.progress", "batchId": batch_id, "assetId": asset.id, "progress": 12 }),
        );
        let outcome = compress_asset(asset, &options);
        if outcome.is_ok() {
            emit_task_event(
                &app,
                json!({ "type": "asset.compress.progress", "batchId": batch_id, "assetId": asset.id, "progress": 100 }),
            );
        }
        outcomes.push((asset.id.clone(), outcome));
    }

    let mut batches = state
        .batches
        .lock()
        .map_err(|_| SafeError::new("state_poisoned", "无法访问批次状态。", true, true))?;
    let batch = batches
        .get_mut(&batch_id)
        .ok_or_else(|| SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false))?;

    let mut new_history = Vec::new();
    let mut failed_count = 0usize;
    let mut cancelled_count = 0usize;

    for (asset_id, outcome) in outcomes {
        if let Some(asset) = batch.assets.iter_mut().find(|asset| asset.id == asset_id) {
            match outcome {
                Ok(outcome) => {
                    asset.output_path = Some(outcome.output_path);
                    asset.output_size = Some(outcome.output_size);
                    asset.saved_ratio = Some(outcome.saved_ratio);
                    asset.width = outcome.width;
                    asset.height = outcome.height;
                    asset.compress_status = CompressStatus::Done;
                    asset.compress_progress = 100;
                    asset.selected = false;
                    asset.error = None;

                    emit_task_event(
                        &app,
                        json!({
                            "type": "asset.compress.done",
                            "batchId": batch.id,
                            "assetId": asset.id,
                            "output": {
                                "outputPath": asset.output_path,
                                "outputSize": asset.output_size,
                                "mime": asset.mime,
                                "savedRatio": asset.saved_ratio,
                                "width": asset.width,
                                "height": asset.height
                            }
                        }),
                    );

                    new_history.push(HistoryItem {
                        id: asset.id.clone(),
                        batch_id: batch.id.clone(),
                        file_name: asset.name.clone(),
                        source_path: Some(asset.source_path.clone()),
                        output_path: asset.output_path.clone(),
                        source_size: asset.source_size,
                        output_size: asset.output_size,
                        saved_ratio: asset.saved_ratio,
                        output_format: Some(outcome.output_format),
                        width: asset.width,
                        height: asset.height,
                        route_name: Some("Local only".to_string()),
                        route_id: None,
                        public_url: None,
                        compress_status: CompressStatus::Done,
                        upload_status: UploadStatus::None,
                        can_copy: false,
                        created_at: batch.created_at.clone(),
                        uploaded_at: None,
                        copied_at: None,
                    });
                }
                Err(error) => {
                    failed_count += 1;
                    asset.output_path = None;
                    asset.output_size = None;
                    asset.saved_ratio = None;
                    asset.compress_status = CompressStatus::Failed;
                    asset.compress_progress = 100;
                    asset.selected = false;
                    asset.error = Some(error.clone());

                    emit_task_event(
                        &app,
                        json!({
                            "type": "asset.failed",
                            "batchId": batch.id,
                            "assetId": asset.id,
                            "stage": "compress",
                            "error": error
                        }),
                    );

                    new_history.push(HistoryItem {
                        id: asset.id.clone(),
                        batch_id: batch.id.clone(),
                        file_name: asset.name.clone(),
                        source_path: Some(asset.source_path.clone()),
                        output_path: None,
                        source_size: asset.source_size,
                        output_size: None,
                        saved_ratio: None,
                        output_format: None,
                        width: None,
                        height: None,
                        route_name: Some("Local only".to_string()),
                        route_id: None,
                        public_url: None,
                        compress_status: CompressStatus::Failed,
                        upload_status: UploadStatus::None,
                        can_copy: false,
                        created_at: batch.created_at.clone(),
                        uploaded_at: None,
                        copied_at: None,
                    });
                }
            }
        }
    }

    for asset_id in cancelled_ids {
        if let Some(asset) = batch.assets.iter_mut().find(|asset| asset.id == asset_id) {
            cancelled_count += 1;
            asset.output_path = None;
            asset.output_size = None;
            asset.saved_ratio = None;
            asset.compress_status = CompressStatus::Cancelled;
            asset.compress_progress = 0;
            asset.selected = false;
            asset.error = Some(SafeError::new("cancelled", "压缩已取消。", true, true));
        }
    }

    batch.status = if failed_count > 0 || cancelled_count > 0 {
        BatchState::Failed
    } else {
        BatchState::Compressed
    };
    batch.updated_at = crate::models::now_string();
    let batch_snapshot = batch.clone();

    drop(batches);

    state.upsert_batch(&batch_snapshot)?;
    state.upsert_history_items(&new_history)?;
    state.clear_cancel_request(&batch_snapshot.id)?;
    emit_task_event(
        &app,
        json!({
            "type": "batch.status.changed",
            "batchId": batch_snapshot.id,
            "status": if matches!(batch_snapshot.status, BatchState::Failed) { "failed" } else { "compressed" }
        }),
    );
    emit_task_event(
        &app,
        json!({ "type": "queue.idle", "batchId": batch_snapshot.id }),
    );

    Ok(batch_snapshot)
}

fn emit_task_event(app: &AppHandle, payload: serde_json::Value) {
    let _ = app.emit(TASK_EVENT, payload);
}
