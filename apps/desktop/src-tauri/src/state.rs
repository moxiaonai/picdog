use crate::error::{CommandResult, SafeError};
use crate::models::{
    default_settings, AppSettings, Batch, BatchState, HistoryItem, HistoryPage, HistoryQuery, Route,
};
use crate::storage::Storage;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Mutex;

pub struct AppState {
    pub batches: Mutex<HashMap<String, Batch>>,
    pub history: Mutex<Vec<HistoryItem>>,
    pub settings: Mutex<AppSettings>,
    pub storage: Mutex<Storage>,
    cancellations: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn open(db_path: impl AsRef<Path>) -> CommandResult<Self> {
        Self::with_storage(Storage::open(db_path)?)
    }

    pub fn in_memory() -> CommandResult<Self> {
        Self::with_storage(Storage::open_in_memory()?)
    }

    pub fn list_routes(&self) -> CommandResult<Vec<Route>> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage.list_routes()
    }

    pub fn get_route(&self, route_id: &str) -> CommandResult<Route> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage
            .get_route(route_id)?
            .ok_or_else(|| SafeError::new("route_not_found", "上传目标不存在。", true, false))
    }

    pub fn save_route(&self, route: &Route) -> CommandResult<()> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage.upsert_route(route)
    }

    pub fn delete_route(&self, route_id: &str) -> CommandResult<()> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage.delete_route(route_id)
    }

    pub fn get_settings(&self) -> CommandResult<AppSettings> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage.get_settings()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> CommandResult<()> {
        {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.save_settings(settings)?;
        }

        let mut current = self
            .settings
            .lock()
            .map_err(|_| state_error("无法访问设置。"))?;
        *current = settings.clone();

        Ok(())
    }

    pub fn upsert_batch(&self, batch: &Batch) -> CommandResult<()> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage.upsert_batch(batch)
    }

    pub fn get_batch(&self, batch_id: &str) -> CommandResult<Option<Batch>> {
        if let Some(batch) = self
            .batches
            .lock()
            .map_err(|_| state_error("无法访问批次状态。"))?
            .get(batch_id)
            .cloned()
        {
            return Ok(Some(batch));
        }

        let batch = {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.get_batch(batch_id)?
        };

        if let Some(batch) = batch.as_ref() {
            self.batches
                .lock()
                .map_err(|_| state_error("无法访问批次状态。"))?
                .insert(batch.id.clone(), batch.clone());
        }

        Ok(batch)
    }

    pub fn latest_batch(&self) -> CommandResult<Option<Batch>> {
        let batch = {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.latest_batch()?
        };

        if let Some(batch) = batch.as_ref() {
            self.batches
                .lock()
                .map_err(|_| state_error("无法访问批次状态。"))?
                .insert(batch.id.clone(), batch.clone());
        }

        Ok(batch)
    }

    pub fn ensure_batch_loaded(&self, batch_id: &str) -> CommandResult<()> {
        self.get_batch(batch_id)?
            .map(|_| ())
            .ok_or_else(|| SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false))
    }

    pub fn upsert_history_items(&self, items: &[HistoryItem]) -> CommandResult<()> {
        {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.upsert_history_items(items)?;
        }

        let mut history = self
            .history
            .lock()
            .map_err(|_| state_error("无法访问历史记录。"))?;
        for item in items {
            if let Some(existing) = history.iter_mut().find(|record| record.id == item.id) {
                *existing = item.clone();
            } else {
                history.insert(0, item.clone());
            }
        }

        Ok(())
    }

    pub fn list_history(&self, query: &HistoryQuery) -> CommandResult<HistoryPage> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage.list_history(query)
    }

    pub fn get_history_item(&self, asset_id: &str) -> CommandResult<HistoryItem> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage
            .get_history_item(asset_id)?
            .ok_or_else(|| SafeError::new("asset_not_found", "图片记录不存在。", true, false))
    }

    pub fn mark_history_copied(&self, asset_id: &str, _copied_at: &str) -> CommandResult<()> {
        let storage = self
            .storage
            .lock()
            .map_err(|_| state_error("无法访问本地数据库。"))?;
        storage.mark_history_copied(asset_id)
    }

    pub fn clear_batch(&self, batch_id: &str, delete_local_outputs: bool) -> CommandResult<()> {
        let output_pairs = if delete_local_outputs {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.output_pairs_for_batch(batch_id)?
        } else {
            Vec::new()
        };

        let deleted_output_ids = delete_local_output_files(&output_pairs)?;

        {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.clear_batch(batch_id, &deleted_output_ids)?;
        }

        self.batches
            .lock()
            .map_err(|_| state_error("无法访问批次状态。"))?
            .remove(batch_id);

        Ok(())
    }

    pub fn clear_history(&self, input: &crate::models::ClearHistoryInput) -> CommandResult<()> {
        let output_pairs = if input.clear_local_outputs {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.output_pairs_for_history_clear(input)?
        } else {
            Vec::new()
        };
        let deleted_output_ids = delete_local_output_files(&output_pairs)?;

        {
            let storage = self
                .storage
                .lock()
                .map_err(|_| state_error("无法访问本地数据库。"))?;
            storage.clear_history(input, &deleted_output_ids)?;
        }

        let mut history = self
            .history
            .lock()
            .map_err(|_| state_error("无法访问历史记录。"))?;
        match input.scope.as_str() {
            "selected" => {
                let ids = input.ids.clone().unwrap_or_default();
                history.retain(|item| !ids.iter().any(|id| id == &item.id));
            }
            "all" => history.clear(),
            _ => {
                return Err(SafeError::new(
                    "invalid_history_scope",
                    "历史清除范围无效。",
                    true,
                    false,
                ));
            }
        }

        Ok(())
    }

    pub fn request_cancel_task(&self, batch_id: &str) -> CommandResult<Batch> {
        let mut batches = self
            .batches
            .lock()
            .map_err(|_| state_error("无法访问批次状态。"))?;
        let batch = batches.get_mut(batch_id).ok_or_else(|| {
            SafeError::new("batch_not_found", "批次不存在或已被清除。", true, false)
        })?;

        if !matches!(
            batch.status,
            BatchState::Compressing | BatchState::Uploading | BatchState::Cancelling
        ) {
            return Ok(batch.clone());
        }

        self.cancellations
            .lock()
            .map_err(|_| state_error("无法访问任务取消状态。"))?
            .insert(batch_id.to_string());

        batch.status = BatchState::Cancelling;
        batch.updated_at = crate::models::now_string();
        let snapshot = batch.clone();
        drop(batches);

        self.upsert_batch(&snapshot)?;

        Ok(snapshot)
    }

    pub fn clear_cancel_request(&self, batch_id: &str) -> CommandResult<()> {
        self.cancellations
            .lock()
            .map_err(|_| state_error("无法访问任务取消状态。"))?
            .remove(batch_id);

        Ok(())
    }

    pub fn is_task_cancelled(&self, batch_id: &str) -> bool {
        self.cancellations
            .lock()
            .map(|cancellations| cancellations.contains(batch_id))
            .unwrap_or(false)
    }

    fn with_storage(storage: Storage) -> CommandResult<Self> {
        let settings = storage.get_settings()?;
        Ok(Self {
            batches: Mutex::new(HashMap::new()),
            history: Mutex::new(Vec::new()),
            settings: Mutex::new(settings),
            storage: Mutex::new(storage),
            cancellations: Mutex::new(HashSet::new()),
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::in_memory().unwrap_or_else(|_| Self {
            batches: Mutex::new(HashMap::new()),
            history: Mutex::new(Vec::new()),
            settings: Mutex::new(default_settings()),
            storage: Mutex::new(Storage::open_in_memory().expect("in-memory storage opens")),
            cancellations: Mutex::new(HashSet::new()),
        })
    }
}

pub fn state_error(message: &str) -> SafeError {
    SafeError::new("state_poisoned", message, true, true)
}

fn delete_local_output_files(
    output_pairs: &[(String, String, String)],
) -> CommandResult<Vec<String>> {
    let mut deleted_ids = Vec::new();

    for (asset_id, source_path, output_path) in output_pairs {
        if !is_safe_local_output_path(source_path, output_path) {
            continue;
        }

        match std::fs::remove_file(output_path) {
            Ok(()) => deleted_ids.push(asset_id.clone()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                deleted_ids.push(asset_id.clone())
            }
            Err(_) => {
                return Err(SafeError::new(
                    "permission_denied",
                    "无法删除本地压缩输出文件。",
                    true,
                    true,
                ));
            }
        }
    }

    Ok(deleted_ids)
}

fn is_safe_local_output_path(source_path: &str, output_path: &str) -> bool {
    if source_path == output_path {
        return false;
    }

    let output = Path::new(output_path);
    if !output.is_absolute() {
        return false;
    }

    output
        .file_name()
        .and_then(|value| value.to_str())
        .map(|name| name.contains(".picdog."))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::is_safe_local_output_path;

    #[test]
    fn only_picdog_outputs_are_safe_to_delete() {
        assert!(is_safe_local_output_path(
            "/tmp/hero.png",
            "/tmp/hero.picdog.webp"
        ));
        assert!(!is_safe_local_output_path("/tmp/hero.png", "/tmp/hero.png"));
        assert!(!is_safe_local_output_path(
            "/tmp/hero.png",
            "hero.picdog.webp"
        ));
        assert!(!is_safe_local_output_path(
            "/tmp/hero.png",
            "/tmp/hero.webp"
        ));
    }
}
