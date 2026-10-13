use crate::credentials;
use crate::error::{CommandResult, SafeError};
use crate::models::{
    AssetState, Batch, BatchState, HistoryItem, Route, RouteCredentialInput, UploadStatus,
};
use crate::state::AppState;
use crate::uploaders::{self, UploadObjectInput};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::HashSet, time::Duration};
use tauri::{AppHandle, Emitter, State};

const TASK_EVENT: &str = "picdog://task-event";
const UPLOAD_MAX_ATTEMPTS: usize = 3;

struct UploadedAsset {
    asset_id: String,
    public_url: String,
}

struct FailedUploadAsset {
    asset_id: String,
    error: SafeError,
}

struct CancelledUploadAsset {
    asset_id: String,
}

#[derive(Clone)]
struct UploadJob {
    asset_id: String,
    local_path: String,
    remote_key: String,
    content_type: &'static str,
}

enum UploadJobResult {
    Uploaded(UploadedAsset),
    Failed(FailedUploadAsset),
}

#[tauri::command(rename_all = "camelCase")]
pub async fn upload_assets(
    app: AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
    asset_ids: Vec<String>,
    route_id: String,
) -> CommandResult<Batch> {
    ensure_upload_enabled(state.inner())?;
    let route = state.get_route(&route_id)?;
    let credential = load_upload_credential(&route)?;
    state.ensure_batch_loaded(&batch_id)?;
    state.clear_cancel_request(&batch_id)?;
    emit_task_event(
        &app,
        json!({ "type": "batch.status.changed", "batchId": batch_id, "status": "uploading" }),
    );

    let targets = prepare_upload_targets(&state, &batch_id, &asset_ids)?;
    if targets.is_empty() {
        return state.get_batch(&batch_id)?.ok_or_else(|| {
            SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false)
        });
    }

    let mut jobs = Vec::new();
    for asset in &targets {
        let output_path = asset.output_path.as_deref().ok_or_else(|| {
            SafeError::new("upload_failed", "图片需要先压缩后才能上传。", true, false)
        })?;

        jobs.push(UploadJob {
            asset_id: asset.id.clone(),
            local_path: output_path.to_string(),
            remote_key: remote_key_for_asset(&asset.id, &asset.name, output_path, &route),
            content_type: content_type_for_path(output_path),
        });
    }

    let (uploaded, failed, cancelled) = upload_jobs(
        &app,
        &batch_id,
        &state,
        &route,
        &credential,
        jobs,
        route.concurrency.clamp(1, 8) as usize,
    );

    let (batch_snapshot, new_history) =
        apply_upload_results(&state, &batch_id, &route, uploaded, failed, cancelled)?;

    state.upsert_batch(&batch_snapshot)?;
    state.upsert_history_items(&new_history)?;
    state.clear_cancel_request(&batch_snapshot.id)?;
    emit_task_event(
        &app,
        json!({
            "type": "batch.status.changed",
            "batchId": batch_snapshot.id,
            "status": batch_status_to_event_value(&batch_snapshot.status)
        }),
    );
    emit_task_event(
        &app,
        json!({ "type": "queue.idle", "batchId": batch_snapshot.id }),
    );

    Ok(batch_snapshot)
}

fn ensure_upload_enabled(state: &AppState) -> CommandResult<()> {
    if state.get_settings()?.upload_enabled {
        return Ok(());
    }

    Err(SafeError::new(
        "upload_disabled",
        "上传功能已关闭，请先在 CDN 设置中开启。",
        true,
        false,
    ))
}

fn load_upload_credential(route: &Route) -> CommandResult<RouteCredentialInput> {
    match credentials::load_route_credential(&route.credential_ref) {
        Ok(credential) => Ok(credential),
        Err(error)
            if error.code == "credential_missing"
                && matches!(route.provider.as_str(), "custom_api" | "local_folder") =>
        {
            Ok(RouteCredentialInput::default())
        }
        Err(error) => Err(error),
    }
}

fn upload_jobs(
    app: &AppHandle,
    batch_id: &str,
    state: &AppState,
    route: &Route,
    credential: &RouteCredentialInput,
    jobs: Vec<UploadJob>,
    concurrency: usize,
) -> (
    Vec<UploadedAsset>,
    Vec<FailedUploadAsset>,
    Vec<CancelledUploadAsset>,
) {
    let mut uploaded = Vec::new();
    let mut failed = Vec::new();
    let mut cancelled = Vec::new();
    let concurrency = concurrency.clamp(1, 8);

    for (chunk_index, chunk) in jobs.chunks(concurrency).enumerate() {
        if state.is_task_cancelled(batch_id) {
            for job in jobs.iter().skip(chunk_index * concurrency) {
                emit_task_event(
                    app,
                    json!({ "type": "asset.cancelled", "batchId": batch_id, "assetId": job.asset_id, "stage": "upload" }),
                );
                cancelled.push(CancelledUploadAsset {
                    asset_id: job.asset_id.clone(),
                });
            }
            break;
        }

        for job in chunk {
            emit_task_event(
                app,
                json!({ "type": "asset.upload.started", "batchId": batch_id, "assetId": job.asset_id }),
            );
            emit_task_event(
                app,
                json!({ "type": "asset.upload.progress", "batchId": batch_id, "assetId": job.asset_id, "progress": 24 }),
            );
        }

        let results = std::thread::scope(|scope| {
            let mut handles = Vec::new();

            for job in chunk.iter().cloned() {
                let asset_id = job.asset_id.clone();
                handles.push((
                    asset_id,
                    scope.spawn(move || upload_single_job(route, credential, job)),
                ));
            }

            handles
                .into_iter()
                .map(|(asset_id, handle)| {
                    handle.join().unwrap_or_else(|_| {
                        UploadJobResult::Failed(FailedUploadAsset {
                            asset_id,
                            error: SafeError::new(
                                "upload_failed",
                                "上传任务异常中断。",
                                true,
                                true,
                            ),
                        })
                    })
                })
                .collect::<Vec<_>>()
        });

        for result in results {
            match result {
                UploadJobResult::Uploaded(item) => {
                    emit_task_event(
                        app,
                        json!({ "type": "asset.upload.progress", "batchId": batch_id, "assetId": item.asset_id, "progress": 100 }),
                    );
                    emit_task_event(
                        app,
                        json!({ "type": "asset.upload.done", "batchId": batch_id, "assetId": item.asset_id, "url": item.public_url }),
                    );
                    uploaded.push(item);
                }
                UploadJobResult::Failed(item) => {
                    emit_task_event(
                        app,
                        json!({
                            "type": "asset.failed",
                            "batchId": batch_id,
                            "assetId": item.asset_id,
                            "stage": "upload",
                            "error": item.error.clone()
                        }),
                    );
                    failed.push(item);
                }
            }
        }
    }

    (uploaded, failed, cancelled)
}

fn upload_single_job(
    route: &Route,
    credential: &RouteCredentialInput,
    job: UploadJob,
) -> UploadJobResult {
    let mut last_error = None;

    for attempt in 1..=UPLOAD_MAX_ATTEMPTS {
        match uploaders::upload_object(UploadObjectInput {
            route,
            credential,
            local_path: &job.local_path,
            remote_key: &job.remote_key,
            content_type: job.content_type,
        }) {
            Ok(result) => {
                return UploadJobResult::Uploaded(UploadedAsset {
                    asset_id: job.asset_id,
                    public_url: result.public_url,
                });
            }
            Err(error) if should_retry_upload_error(&error, attempt) => {
                last_error = Some(error);
                std::thread::sleep(upload_retry_delay(attempt));
            }
            Err(error) => {
                return UploadJobResult::Failed(FailedUploadAsset {
                    asset_id: job.asset_id,
                    error,
                });
            }
        }
    }

    UploadJobResult::Failed(FailedUploadAsset {
        asset_id: job.asset_id,
        error: last_error.unwrap_or_else(|| {
            SafeError::new("upload_failed", "上传失败，请稍后重试。", true, true)
        }),
    })
}

fn should_retry_upload_error(error: &SafeError, attempt: usize) -> bool {
    attempt < UPLOAD_MAX_ATTEMPTS && error.retryable
}

fn upload_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(match attempt {
        1 => 250,
        2 => 700,
        _ => 1_200,
    })
}

fn prepare_upload_targets(
    state: &State<'_, AppState>,
    batch_id: &str,
    asset_ids: &[String],
) -> CommandResult<Vec<AssetState>> {
    let mut batches = state
        .batches
        .lock()
        .map_err(|_| SafeError::new("state_poisoned", "无法访问批次状态。", true, true))?;
    let batch = batches
        .get_mut(batch_id)
        .ok_or_else(|| SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false))?;

    if asset_ids.is_empty() {
        return Ok(Vec::new());
    }

    let requested = asset_ids.iter().map(String::as_str).collect::<HashSet<_>>();

    for asset_id in asset_ids {
        let Some(asset) = batch
            .assets
            .iter()
            .find(|asset| asset.id == asset_id.as_str())
        else {
            return Err(SafeError::new(
                "asset_not_found",
                "图片记录不存在。",
                true,
                false,
            ));
        };

        if asset.output_path.is_none() {
            return Err(SafeError::new(
                "upload_failed",
                "图片需要先压缩后才能上传。",
                true,
                false,
            ));
        }
    }

    let mut targets = Vec::new();

    for asset in &mut batch.assets {
        if !requested.contains(asset.id.as_str()) {
            continue;
        }

        asset.upload_status = UploadStatus::Running;
        asset.upload_progress = 24;
        asset.error = None;
        targets.push(asset.clone());
    }

    batch.status = BatchState::Uploading;
    batch.updated_at = crate::models::now_string();

    Ok(targets)
}

fn apply_upload_results(
    state: &State<'_, AppState>,
    batch_id: &str,
    route: &Route,
    uploaded: Vec<UploadedAsset>,
    failed: Vec<FailedUploadAsset>,
    cancelled: Vec<CancelledUploadAsset>,
) -> CommandResult<(Batch, Vec<HistoryItem>)> {
    let mut batches = state
        .batches
        .lock()
        .map_err(|_| SafeError::new("state_poisoned", "无法访问批次状态。", true, true))?;
    let batch = batches
        .get_mut(batch_id)
        .ok_or_else(|| SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false))?;

    let mut new_history = Vec::new();
    let uploaded_at = crate::models::now_string();

    for item in uploaded {
        if let Some(asset) = batch
            .assets
            .iter_mut()
            .find(|asset| asset.id == item.asset_id)
        {
            asset.upload_status = UploadStatus::Done;
            asset.upload_progress = 100;
            asset.selected = false;
            asset.route_id = Some(route.id.clone());
            asset.public_url = Some(item.public_url.clone());
            asset.error = None;

            new_history.push(HistoryItem {
                id: asset.id.clone(),
                batch_id: batch.id.clone(),
                file_name: asset.name.clone(),
                source_path: Some(asset.source_path.clone()),
                output_path: asset.output_path.clone(),
                source_size: asset.source_size,
                output_size: asset.output_size,
                saved_ratio: asset.saved_ratio,
                output_format: Some(
                    output_format_for_path(asset.output_path.as_deref().unwrap_or(&asset.name))
                        .to_string(),
                ),
                width: asset.width,
                height: asset.height,
                route_name: Some(route.name.clone()),
                route_id: Some(route.id.clone()),
                public_url: Some(item.public_url),
                compress_status: asset.compress_status.clone(),
                upload_status: UploadStatus::Done,
                can_copy: true,
                created_at: batch.created_at.clone(),
                uploaded_at: Some(uploaded_at.clone()),
                copied_at: None,
            });
        }
    }

    for item in failed {
        if let Some(asset) = batch
            .assets
            .iter_mut()
            .find(|asset| asset.id == item.asset_id)
        {
            asset.upload_status = UploadStatus::Failed;
            asset.upload_progress = 0;
            asset.selected = true;
            asset.route_id = Some(route.id.clone());
            asset.public_url = None;
            asset.error = Some(item.error.clone());

            new_history.push(HistoryItem {
                id: asset.id.clone(),
                batch_id: batch.id.clone(),
                file_name: asset.name.clone(),
                source_path: Some(asset.source_path.clone()),
                output_path: asset.output_path.clone(),
                source_size: asset.source_size,
                output_size: asset.output_size,
                saved_ratio: asset.saved_ratio,
                output_format: asset
                    .output_path
                    .as_deref()
                    .map(output_format_for_path)
                    .map(str::to_string),
                width: asset.width,
                height: asset.height,
                route_name: Some(route.name.clone()),
                route_id: Some(route.id.clone()),
                public_url: None,
                compress_status: asset.compress_status.clone(),
                upload_status: UploadStatus::Failed,
                can_copy: false,
                created_at: batch.created_at.clone(),
                uploaded_at: None,
                copied_at: None,
            });
        }
    }

    for item in cancelled {
        if let Some(asset) = batch
            .assets
            .iter_mut()
            .find(|asset| asset.id == item.asset_id)
        {
            asset.upload_status = UploadStatus::Cancelled;
            asset.upload_progress = 0;
            asset.selected = true;
            asset.route_id = Some(route.id.clone());
            asset.public_url = None;
            asset.error = Some(SafeError::new("cancelled", "上传已取消。", true, true));

            new_history.push(HistoryItem {
                id: asset.id.clone(),
                batch_id: batch.id.clone(),
                file_name: asset.name.clone(),
                source_path: Some(asset.source_path.clone()),
                output_path: asset.output_path.clone(),
                source_size: asset.source_size,
                output_size: asset.output_size,
                saved_ratio: asset.saved_ratio,
                output_format: asset
                    .output_path
                    .as_deref()
                    .map(output_format_for_path)
                    .map(str::to_string),
                width: asset.width,
                height: asset.height,
                route_name: Some(route.name.clone()),
                route_id: Some(route.id.clone()),
                public_url: None,
                compress_status: asset.compress_status.clone(),
                upload_status: UploadStatus::Cancelled,
                can_copy: false,
                created_at: batch.created_at.clone(),
                uploaded_at: None,
                copied_at: None,
            });
        }
    }

    let has_failed_upload = batch.assets.iter().any(|asset| {
        matches!(
            asset.upload_status,
            UploadStatus::Failed | UploadStatus::Cancelled
        )
    });
    let all_uploaded = batch
        .assets
        .iter()
        .filter(|asset| asset.output_path.is_some())
        .all(|asset| matches!(asset.upload_status, UploadStatus::Done));
    batch.status = if has_failed_upload {
        BatchState::Failed
    } else if all_uploaded {
        BatchState::Uploaded
    } else {
        BatchState::Compressed
    };
    batch.updated_at = crate::models::now_string();

    Ok((batch.clone(), new_history))
}

fn emit_task_event(app: &AppHandle, payload: serde_json::Value) {
    let _ = app.emit(TASK_EVENT, payload);
}

fn batch_status_to_event_value(status: &BatchState) -> &'static str {
    match status {
        BatchState::Empty => "empty",
        BatchState::Queue => "queue",
        BatchState::Compressing => "compressing",
        BatchState::Compressed => "compressed",
        BatchState::Uploading => "uploading",
        BatchState::Uploaded => "uploaded",
        BatchState::Failed => "failed",
        BatchState::Cancelling => "cancelling",
    }
}

fn remote_key_for_asset(asset_id: &str, name: &str, output_path: &str, route: &Route) -> String {
    let (year, month, day) = current_date_parts();
    let stem = name.rsplit_once('.').map(|(left, _)| left).unwrap_or(name);
    let ext = output_format_for_path(output_path);
    let hash = stable_key_hash(asset_id);
    let safe_name = safe_key_segment(stem, &hash, !route.path_template.contains("{hash}"));
    render_key_template(
        &route.path_template,
        &safe_name,
        &hash,
        ext,
        year,
        month,
        day,
    )
}

fn render_key_template(
    template: &str,
    name: &str,
    hash: &str,
    ext: &str,
    year: i32,
    month: u32,
    day: u32,
) -> String {
    template
        .replace("{yyyy}", &format!("{:04}", year))
        .replace("{mm}", &format!("{:02}", month))
        .replace("{dd}", &format!("{:02}", day))
        .replace("{name}", name)
        .replace("{slug}", name)
        .replace("{hash}", hash)
        .replace("{ext}", ext)
}

fn current_date_parts() -> (i32, u32, u32) {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default();
    let days = seconds / 86_400;
    civil_from_days(days)
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

fn stable_key_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex::encode(digest)[..12].to_string()
}

fn safe_key_segment(value: &str, hash: &str, append_hash_when_sanitized: bool) -> String {
    let mut slug = String::new();
    let mut pending_separator = false;
    let mut was_sanitized = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            if pending_separator && !slug.is_empty() && !slug.ends_with(['-', '_']) {
                slug.push('-');
            }
            slug.push(ch);
            pending_separator = false;
        } else {
            was_sanitized = true;
            pending_separator = true;
        }
    }

    let mut slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        slug.push_str("image");
        was_sanitized = true;
    }

    if was_sanitized && append_hash_when_sanitized {
        slug.push('-');
        slug.push_str(&hash[..hash.len().min(8)]);
    }

    slug
}

fn output_format_for_path(path: &str) -> &str {
    path.rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or("original")
}

fn content_type_for_path(path: &str) -> &'static str {
    match output_format_for_path(path).to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        civil_from_days, content_type_for_path, ensure_upload_enabled, remote_key_for_asset,
        render_key_template, safe_key_segment, should_retry_upload_error, stable_key_hash,
        upload_retry_delay, UPLOAD_MAX_ATTEMPTS,
    };
    use crate::error::SafeError;
    use crate::models::Route;
    use crate::state::AppState;

    fn route() -> Route {
        Route {
            id: "blog-r2".to_string(),
            name: "Blog CDN · R2".to_string(),
            provider: "s3".to_string(),
            credential_ref: "keychain://picdog/blog-r2".to_string(),
            endpoint: Some("https://example.r2.cloudflarestorage.com".to_string()),
            bucket: Some("blog-assets".to_string()),
            region: Some("auto".to_string()),
            path_template: "{yyyy}/{mm}/{name}.{ext}".to_string(),
            public_base_url: "https://cdn.picdog.app".to_string(),
            cache_control: crate::models::default_route_cache_control(),
            default_copy_format: "markdown".to_string(),
            custom_link_template: None,
            custom_api_method: None,
            custom_api_body: None,
            custom_api_file_field: None,
            custom_api_headers: None,
            custom_api_url_path: None,
            concurrency: 3,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }
    }

    #[test]
    fn keeps_safe_remote_key_characters() {
        assert_eq!(
            safe_key_segment("hero-cover_2", "123456789abc", true),
            "hero-cover_2"
        );
    }

    #[test]
    fn non_ascii_remote_key_names_get_stable_hash_suffixes() {
        let hash = stable_key_hash("/images/产品 图@2x.picdog.webp");
        assert_eq!(
            safe_key_segment("产品 图@2x", &hash, true),
            format!("2x-{}", &hash[..8])
        );
        assert_eq!(
            safe_key_segment("产品图", &hash, true),
            format!("image-{}", &hash[..8])
        );
        assert_eq!(
            safe_key_segment("hero cover", &hash, true),
            format!("hero-cover-{}", &hash[..8])
        );
        assert_eq!(safe_key_segment("产品图", &hash, false), "image");
    }

    #[test]
    fn renders_all_route_path_template_tokens() {
        assert_eq!(
            render_key_template(
                "{yyyy}/{mm}/{dd}/{hash}-{slug}-{name}.{ext}",
                "hero-cover",
                "abcdef123456",
                "webp",
                2026,
                10,
                12
            ),
            "2026/10/12/abcdef123456-hero-cover-hero-cover.webp"
        );
    }

    #[test]
    fn remote_key_uses_output_extension() {
        assert!(remote_key_for_asset(
            "asset-batch-1-0",
            "hero.png",
            "/tmp/hero.picdog.avif",
            &route()
        )
        .ends_with("/hero.avif"));
    }

    #[test]
    fn hash_token_disambiguates_same_named_assets() {
        let mut route = route();
        route.path_template = "{hash}-{name}.{ext}".to_string();

        let first = remote_key_for_asset(
            "asset-batch-1-0",
            "产品图.png",
            "/tmp/a/产品图.picdog.webp",
            &route,
        );
        let second = remote_key_for_asset(
            "asset-batch-1-1",
            "产品图.png",
            "/tmp/b/产品图.picdog.webp",
            &route,
        );

        assert_ne!(first, second);
        assert!(first.ends_with(".webp"));
        assert!(first.ends_with("-image.webp"));
    }

    #[test]
    fn maps_content_type_from_output_path() {
        assert_eq!(content_type_for_path("/tmp/hero.picdog.webp"), "image/webp");
        assert_eq!(content_type_for_path("/tmp/hero.picdog.avif"), "image/avif");
    }

    #[test]
    fn calculates_civil_year_month_from_unix_days() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_372), (2025, 10, 11));
    }

    #[test]
    fn retries_only_retryable_upload_errors_before_final_attempt() {
        let retryable = SafeError::new("network_timeout", "timeout", true, true);
        let fatal = SafeError::new("credential_missing", "missing", true, false);

        assert!(should_retry_upload_error(&retryable, 1));
        assert!(!should_retry_upload_error(&retryable, UPLOAD_MAX_ATTEMPTS));
        assert!(!should_retry_upload_error(&fatal, 1));
        assert!(upload_retry_delay(2) > upload_retry_delay(1));
    }

    #[test]
    fn rejects_uploads_when_feature_is_disabled_by_default() {
        let state = AppState::in_memory().expect("in-memory state opens");

        let error = ensure_upload_enabled(&state).expect_err("uploads should default to disabled");

        assert_eq!(error.code, "upload_disabled");
        assert!(error.recoverable);
        assert!(!error.retryable);
    }

    #[test]
    fn permits_uploads_after_feature_is_enabled() {
        let state = AppState::in_memory().expect("in-memory state opens");
        let mut settings = state.get_settings().expect("settings load");
        settings.upload_enabled = true;
        state.save_settings(&settings).expect("settings save");

        assert!(ensure_upload_enabled(&state).is_ok());
    }
}
