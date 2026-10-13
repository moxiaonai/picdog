use crate::error::{CommandResult, SafeError};
use crate::models::{
    default_routes, default_settings, now_string, AppSettings, AssetState, Batch, BatchState,
    ClearHistoryInput, CompressStatus, HistoryItem, HistoryPage, HistoryQuery, Route, UploadStatus,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const INIT_SQL: &str = include_str!("../../migrations/0001_init.sql");

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>) -> CommandResult<Self> {
        let conn = Connection::open(path).map_err(storage_error)?;
        let storage = Self { conn };
        storage.initialize()?;
        Ok(storage)
    }

    pub fn open_in_memory() -> CommandResult<Self> {
        let conn = Connection::open_in_memory().map_err(storage_error)?;
        let storage = Self { conn };
        storage.initialize()?;
        Ok(storage)
    }

    pub fn get_settings(&self) -> CommandResult<AppSettings> {
        let value: Option<String> = self
            .conn
            .query_row("select value from settings where key = 'app'", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(storage_error)?;

        match value {
            Some(value) => serde_json::from_str(&value).map_err(storage_error),
            None => Ok(default_settings()),
        }
    }

    pub fn save_settings(&self, settings: &AppSettings) -> CommandResult<()> {
        let value = serde_json::to_string(settings).map_err(storage_error)?;
        self.conn
            .execute(
                "insert into settings(key, value, updated_at)
                 values('app', ?1, ?2)
                 on conflict(key) do update set value = excluded.value, updated_at = excluded.updated_at",
                params![value, now_string()],
            )
            .map_err(storage_error)?;

        Ok(())
    }

    pub fn list_routes(&self) -> CommandResult<Vec<Route>> {
        let mut statement = self
            .conn
            .prepare(
                "select id, name, provider, credential_ref, endpoint, bucket, region,
                        path_template, public_base_url, cache_control, default_copy_format,
                        custom_link_template, custom_api_method, custom_api_body,
                        custom_api_file_field, custom_api_headers, custom_api_url_path,
                        concurrency, created_at, updated_at
                   from routes
                  order by created_at asc",
            )
            .map_err(storage_error)?;

        let routes = statement
            .query_map([], route_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        Ok(routes)
    }

    pub fn get_route(&self, route_id: &str) -> CommandResult<Option<Route>> {
        self.conn
            .query_row(
                "select id, name, provider, credential_ref, endpoint, bucket, region,
                        path_template, public_base_url, cache_control, default_copy_format,
                        custom_link_template, custom_api_method, custom_api_body,
                        custom_api_file_field, custom_api_headers, custom_api_url_path,
                        concurrency, created_at, updated_at
                   from routes
                  where id = ?1",
                params![route_id],
                route_from_row,
            )
            .optional()
            .map_err(storage_error)
    }

    pub fn upsert_route(&self, route: &Route) -> CommandResult<()> {
        self.conn
            .execute(
                "insert into routes(
                   id, name, provider, credential_ref, endpoint, bucket, region,
                   path_template, public_base_url, cache_control, default_copy_format,
                   custom_link_template, custom_api_method, custom_api_body,
                   custom_api_file_field, custom_api_headers, custom_api_url_path,
                   concurrency, created_at, updated_at
                 )
                 values(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                 on conflict(id) do update set
                   name = excluded.name,
                   provider = excluded.provider,
                   credential_ref = excluded.credential_ref,
                   endpoint = excluded.endpoint,
                   bucket = excluded.bucket,
                   region = excluded.region,
                   path_template = excluded.path_template,
                   public_base_url = excluded.public_base_url,
                   cache_control = excluded.cache_control,
                   default_copy_format = excluded.default_copy_format,
                   custom_link_template = excluded.custom_link_template,
                   custom_api_method = excluded.custom_api_method,
                   custom_api_body = excluded.custom_api_body,
                   custom_api_file_field = excluded.custom_api_file_field,
                   custom_api_headers = excluded.custom_api_headers,
                   custom_api_url_path = excluded.custom_api_url_path,
                   concurrency = excluded.concurrency,
                   updated_at = excluded.updated_at",
                params![
                    route.id,
                    route.name,
                    route.provider,
                    route.credential_ref,
                    route.endpoint,
                    route.bucket,
                    route.region,
                    route.path_template,
                    route.public_base_url,
                    route.cache_control,
                    route.default_copy_format,
                    route.custom_link_template,
                    route.custom_api_method,
                    route.custom_api_body,
                    route.custom_api_file_field,
                    route.custom_api_headers,
                    route.custom_api_url_path,
                    route.concurrency,
                    route.created_at,
                    route.updated_at,
                ],
            )
            .map_err(storage_error)?;

        Ok(())
    }

    pub fn delete_route(&self, route_id: &str) -> CommandResult<()> {
        self.conn
            .execute("delete from routes where id = ?1", params![route_id])
            .map_err(storage_error)?;

        Ok(())
    }

    pub fn get_batch(&self, batch_id: &str) -> CommandResult<Option<Batch>> {
        let batch_header: Option<(String, String, String, String)> = self
            .conn
            .query_row(
                "select id, status, created_at, updated_at
                   from batches
                  where id = ?1
                    and cleared_at is null",
                params![batch_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(storage_error)?;

        let Some((id, status, created_at, updated_at)) = batch_header else {
            return Ok(None);
        };

        let mut statement = self
            .conn
            .prepare(
                "select id, batch_id, file_name, source_path, output_path,
                        source_size, output_size, saved_ratio, mime, width, height,
                        compress_status, upload_status, route_id, public_url,
                        error_code, error_message
                   from assets
                  where batch_id = ?1
                    and cleared_at is null
                  order by cast(created_at as integer) asc, id asc",
            )
            .map_err(storage_error)?;

        let assets = statement
            .query_map(params![batch_id], asset_state_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        Ok(Some(Batch {
            id,
            status: parse_batch_state(&status),
            assets,
            created_at,
            updated_at,
        }))
    }

    pub fn latest_batch(&self) -> CommandResult<Option<Batch>> {
        let batch_id: Option<String> = self
            .conn
            .query_row(
                "select id
                   from batches
                  where cleared_at is null
                  order by cast(updated_at as integer) desc, id desc
                  limit 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;

        match batch_id {
            Some(batch_id) => self.get_batch(&batch_id),
            None => Ok(None),
        }
    }

    pub fn upsert_batch(&self, batch: &Batch) -> CommandResult<()> {
        let asset_count = batch.assets.len() as i64;
        let uploaded_count = batch
            .assets
            .iter()
            .filter(|asset| matches!(asset.upload_status, UploadStatus::Done))
            .count() as i64;
        let source_total_size = batch
            .assets
            .iter()
            .map(|asset| asset.source_size)
            .sum::<u64>() as i64;
        let output_total_size = batch
            .assets
            .iter()
            .filter_map(|asset| asset.output_size)
            .sum::<u64>() as i64;
        let route_id = batch.assets.iter().find_map(|asset| asset.route_id.clone());

        self.conn
            .execute(
                "insert into batches(
                   id, status, asset_count, uploaded_count, source_total_size,
                   output_total_size, route_id, copy_format, cleared_at, created_at, updated_at
                 )
                 values(?1, ?2, ?3, ?4, ?5, ?6, ?7, null, null, ?8, ?9)
                 on conflict(id) do update set
                   status = excluded.status,
                   asset_count = excluded.asset_count,
                   uploaded_count = excluded.uploaded_count,
                   source_total_size = excluded.source_total_size,
                   output_total_size = excluded.output_total_size,
                   route_id = excluded.route_id,
                   updated_at = excluded.updated_at",
                params![
                    batch.id,
                    batch_state_to_str(&batch.status),
                    asset_count,
                    uploaded_count,
                    source_total_size,
                    if output_total_size > 0 {
                        Some(output_total_size)
                    } else {
                        None
                    },
                    route_id,
                    batch.created_at,
                    batch.updated_at,
                ],
            )
            .map_err(storage_error)?;

        for asset in &batch.assets {
            self.upsert_asset_state(batch, asset)?;
        }

        Ok(())
    }

    fn upsert_asset_state(&self, batch: &Batch, asset: &AssetState) -> CommandResult<()> {
        self.conn
            .execute(
                "insert into assets(
                   id, batch_id, source_path, output_path, file_name, source_size,
                   output_size, saved_ratio, output_format, width, height, mime,
                   compress_status, upload_status, remote_key, public_url,
                   route_id, route_name, copied_at, uploaded_at, history_hidden,
                   output_deleted_at, error_code, error_message, cleared_at, created_at, updated_at
                 )
                 values(
                   ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, null, ?9, ?10, ?11,
                   ?12, ?13, null, ?14, ?15, null, null, null, 0,
                   null, ?16, ?17, null, ?18, ?19
                 )
                 on conflict(id) do update set
                   source_path = excluded.source_path,
                   output_path = excluded.output_path,
                   file_name = excluded.file_name,
                   source_size = excluded.source_size,
                   output_size = excluded.output_size,
                   saved_ratio = excluded.saved_ratio,
                   width = excluded.width,
                   height = excluded.height,
                   mime = excluded.mime,
                   compress_status = excluded.compress_status,
                   upload_status = excluded.upload_status,
                   public_url = excluded.public_url,
                   route_id = excluded.route_id,
                   error_code = excluded.error_code,
                   error_message = excluded.error_message,
                   updated_at = excluded.updated_at,
                   history_hidden = 0,
                   cleared_at = null",
                params![
                    asset.id,
                    batch.id,
                    asset.source_path,
                    asset.output_path,
                    asset.name,
                    asset.source_size,
                    asset.output_size,
                    asset.saved_ratio,
                    asset.width,
                    asset.height,
                    asset.mime,
                    compress_status_to_str(&asset.compress_status),
                    upload_status_to_str(&asset.upload_status),
                    asset.public_url,
                    asset.route_id,
                    asset.error.as_ref().map(|error| error.code.as_str()),
                    asset.error.as_ref().map(|error| error.message.as_str()),
                    batch.created_at,
                    batch.updated_at,
                ],
            )
            .map_err(storage_error)?;

        Ok(())
    }

    pub fn list_history(&self, query: &HistoryQuery) -> CommandResult<HistoryPage> {
        let mut statement = self
            .conn
            .prepare(
                "select id, batch_id, file_name, source_path, output_path,
                        source_size, output_size, saved_ratio, output_format, width, height, route_name, route_id,
                        public_url, compress_status, upload_status,
                        created_at, uploaded_at, copied_at
                   from assets
                  where history_hidden = 0
                    and cleared_at is null
                    and compress_status != 'pending'
                  order by cast(created_at as integer) desc",
            )
            .map_err(storage_error)?;

        let mut items = statement
            .query_map([], history_from_row)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        if let Some(keyword) = query.keyword.as_ref().filter(|keyword| !keyword.is_empty()) {
            let keyword = keyword.to_lowercase();
            items.retain(|item| item.file_name.to_lowercase().contains(&keyword));
        }

        if let Some(route_id) = query
            .route_id
            .as_ref()
            .filter(|route_id| !route_id.is_empty())
        {
            items.retain(|item| item.route_id.as_deref() == Some(route_id.as_str()));
        }

        if let Some(status) = query.status.as_ref() {
            items.retain(|item| match status.as_str() {
                "uploaded" => item.can_copy,
                "compressed" => {
                    !item.can_copy && matches!(item.compress_status, CompressStatus::Done)
                }
                "failed" => {
                    matches!(
                        item.compress_status,
                        CompressStatus::Failed | CompressStatus::Cancelled
                    ) || matches!(
                        item.upload_status,
                        UploadStatus::Failed | UploadStatus::Cancelled
                    )
                }
                _ => true,
            });
        }

        let total = items.len() as u32;
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 100);
        let start = ((page - 1) * page_size) as usize;
        let end = (start + page_size as usize).min(items.len());
        let items = if start < items.len() {
            items[start..end].to_vec()
        } else {
            Vec::new()
        };

        Ok(HistoryPage {
            items,
            page,
            page_size,
            total,
        })
    }

    pub fn get_history_item(&self, asset_id: &str) -> CommandResult<Option<HistoryItem>> {
        self.conn
            .query_row(
                "select id, batch_id, file_name, source_path, output_path,
                        source_size, output_size, saved_ratio, output_format, width, height, route_name, route_id,
                        public_url, compress_status, upload_status,
                        created_at, uploaded_at, copied_at
                   from assets
                  where id = ?1
                    and history_hidden = 0
                    and cleared_at is null",
                params![asset_id],
                history_from_row,
            )
            .optional()
            .map_err(storage_error)
    }

    pub fn upsert_history_items(&self, items: &[HistoryItem]) -> CommandResult<()> {
        for item in items {
            self.conn
                .execute(
                    "insert into batches(
                       id, status, asset_count, uploaded_count, source_total_size,
                       output_total_size, route_id, copy_format, cleared_at, created_at, updated_at
                     )
                     values(?1, ?2, 1, ?3, ?4, ?5, null, null, null, ?6, ?7)
                     on conflict(id) do update set
                       uploaded_count = max(uploaded_count, excluded.uploaded_count),
                       output_total_size = excluded.output_total_size,
                       updated_at = excluded.updated_at",
                    params![
                        item.batch_id,
                        if item.can_copy {
                            "uploaded"
                        } else {
                            "compressed"
                        },
                        if item.can_copy { 1 } else { 0 },
                        item.source_size,
                        item.output_size,
                        item.created_at,
                        now_string(),
                    ],
                )
                .map_err(storage_error)?;

            self.conn
                .execute(
                    "insert into assets(
                       id, batch_id, source_path, output_path, file_name, source_size,
                       output_size, saved_ratio, output_format, width, height, mime,
                       compress_status, upload_status, remote_key, public_url,
                       route_id, route_name, copied_at, uploaded_at, history_hidden,
                       output_deleted_at, error_code, error_message, cleared_at, created_at, updated_at
                     )
                     values(
                       ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, null,
                       ?12, ?13, null, ?14, ?15, ?16, ?17, ?18, 0,
                       null, null, null, null, ?19, ?20
                     )
                     on conflict(id) do update set
                       output_size = excluded.output_size,
                       saved_ratio = excluded.saved_ratio,
                       output_format = excluded.output_format,
                       width = excluded.width,
                       height = excluded.height,
                       route_id = excluded.route_id,
                       route_name = excluded.route_name,
                       public_url = excluded.public_url,
                       compress_status = excluded.compress_status,
                       upload_status = excluded.upload_status,
                       copied_at = coalesce(excluded.copied_at, assets.copied_at),
                       uploaded_at = excluded.uploaded_at,
                       updated_at = excluded.updated_at,
                       history_hidden = 0,
                       cleared_at = null",
                    params![
                        item.id,
                        item.batch_id,
                        item.source_path.clone().unwrap_or_default(),
                        item.output_path,
                        item.file_name,
                        item.source_size,
                        item.output_size,
                        item.saved_ratio,
                        item.output_format,
                        item.width,
                        item.height,
                        compress_status_to_str(&item.compress_status),
                        upload_status_to_str(&item.upload_status),
                        item.public_url,
                        item.route_id,
                        item.route_name,
                        item.copied_at,
                        item.uploaded_at,
                        item.created_at,
                        now_string(),
                    ],
                )
                .map_err(storage_error)?;
        }

        Ok(())
    }

    pub fn mark_history_copied(&self, asset_id: &str) -> CommandResult<()> {
        self.conn
            .execute(
                "update assets set copied_at = ?1, updated_at = ?1 where id = ?2",
                params![now_string(), asset_id],
            )
            .map_err(storage_error)?;

        Ok(())
    }

    pub fn output_pairs_for_batch(
        &self,
        batch_id: &str,
    ) -> CommandResult<Vec<(String, String, String)>> {
        let mut statement = self
            .conn
            .prepare(
                "select id, source_path, output_path
                   from assets
                  where batch_id = ?1
                    and output_path is not null
                    and output_deleted_at is null",
            )
            .map_err(storage_error)?;

        let pairs = statement
            .query_map(params![batch_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        Ok(pairs)
    }

    pub fn output_pairs_for_history_clear(
        &self,
        input: &ClearHistoryInput,
    ) -> CommandResult<Vec<(String, String, String)>> {
        match input.scope.as_str() {
            "selected" => {
                let mut pairs = Vec::new();

                for id in input.ids.as_ref().into_iter().flatten() {
                    let pair = self
                        .conn
                        .query_row(
                            "select id, source_path, output_path
                               from assets
                              where id = ?1
                                and output_path is not null
                                and output_deleted_at is null",
                            params![id],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        )
                        .optional()
                        .map_err(storage_error)?;

                    if let Some(pair) = pair {
                        pairs.push(pair);
                    }
                }

                Ok(pairs)
            }
            "all" => {
                let mut statement = self
                    .conn
                    .prepare(
                        "select id, source_path, output_path
                           from assets
                          where history_hidden = 0
                            and cleared_at is null
                            and output_path is not null
                            and output_deleted_at is null",
                    )
                    .map_err(storage_error)?;

                let pairs = statement
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                    .map_err(storage_error)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(storage_error)?;

                Ok(pairs)
            }
            _ => Err(SafeError::new(
                "invalid_history_scope",
                "历史清除范围无效。",
                true,
                false,
            )),
        }
    }

    pub fn clear_batch(&self, batch_id: &str, deleted_output_ids: &[String]) -> CommandResult<()> {
        let cleared_at = now_string();

        self.conn
            .execute(
                "update batches set cleared_at = ?1, updated_at = ?1 where id = ?2",
                params![cleared_at, batch_id],
            )
            .map_err(storage_error)?;

        for id in deleted_output_ids {
            self.conn
                .execute(
                    "update assets
                        set output_deleted_at = ?1, updated_at = ?1
                      where id = ?2
                        and batch_id = ?3
                        and output_path is not null
                        and output_deleted_at is null",
                    params![cleared_at, id, batch_id],
                )
                .map_err(storage_error)?;
        }

        Ok(())
    }

    pub fn clear_history(
        &self,
        input: &ClearHistoryInput,
        deleted_output_ids: &[String],
    ) -> CommandResult<()> {
        let cleared_at = now_string();

        match input.scope.as_str() {
            "selected" => {
                for id in input.ids.as_ref().into_iter().flatten() {
                    self.conn
                        .execute(
                            "update assets
                                set history_hidden = 1,
                                    cleared_at = ?1,
                                    updated_at = ?1,
                                    output_deleted_at = case
                                      when ?2 then ?1
                                      else output_deleted_at
                                    end
                              where id = ?3",
                            params![
                                cleared_at,
                                deleted_output_ids.iter().any(|deleted_id| deleted_id == id),
                                id
                            ],
                        )
                        .map_err(storage_error)?;
                }
            }
            "all" => {
                self.conn
                    .execute(
                        "update assets
                            set history_hidden = 1,
                                cleared_at = ?1,
                                updated_at = ?1
                          where history_hidden = 0",
                        params![cleared_at],
                    )
                    .map_err(storage_error)?;
            }
            _ => {
                return Err(SafeError::new(
                    "invalid_history_scope",
                    "历史清除范围无效。",
                    true,
                    false,
                ));
            }
        }

        for id in deleted_output_ids {
            self.conn
                .execute(
                    "update assets
                        set output_deleted_at = ?1,
                            updated_at = ?1
                      where id = ?2
                        and output_path is not null
                        and output_deleted_at is null",
                    params![cleared_at, id],
                )
                .map_err(storage_error)?;
        }

        Ok(())
    }

    fn initialize(&self) -> CommandResult<()> {
        self.conn
            .execute_batch(
                "pragma foreign_keys = on;
                 pragma journal_mode = wal;",
            )
            .map_err(storage_error)?;
        self.conn.execute_batch(INIT_SQL).map_err(storage_error)?;
        self.ensure_route_custom_api_columns()?;
        self.seed_defaults()
    }

    fn seed_defaults(&self) -> CommandResult<()> {
        if self.get_settings_from_db()?.is_none() {
            self.save_settings(&default_settings())?;
        }

        self.remove_legacy_demo_routes()?;
        self.clear_missing_default_route()?;

        let route_count: i64 = self
            .conn
            .query_row("select count(*) from routes", [], |row| row.get(0))
            .map_err(storage_error)?;

        if route_count == 0 {
            for route in default_routes(&now_string()) {
                self.conn
                    .execute(
                        "insert into routes(
                           id, name, provider, credential_ref, endpoint, bucket, region,
                           path_template, public_base_url, cache_control, default_copy_format,
                           custom_link_template, custom_api_method, custom_api_body,
                           custom_api_file_field, custom_api_headers, custom_api_url_path,
                           concurrency, created_at, updated_at
                         )
                         values(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
                        params![
                            route.id,
                            route.name,
                            route.provider,
                            route.credential_ref,
                            route.endpoint,
                            route.bucket,
                            route.region,
                            route.path_template,
                            route.public_base_url,
                            route.cache_control,
                            route.default_copy_format,
                            route.custom_link_template,
                            route.custom_api_method,
                            route.custom_api_body,
                            route.custom_api_file_field,
                            route.custom_api_headers,
                            route.custom_api_url_path,
                            route.concurrency,
                            route.created_at,
                            route.updated_at,
                        ],
                    )
                    .map_err(storage_error)?;
            }
        }

        Ok(())
    }

    fn ensure_route_custom_api_columns(&self) -> CommandResult<()> {
        let existing_columns = {
            let mut statement = self
                .conn
                .prepare("pragma table_info(routes)")
                .map_err(storage_error)?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(storage_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage_error)?;
            columns
        };

        for (name, ddl) in [
            (
                "cache_control",
                "alter table routes add column cache_control text not null default 'public, max-age=31536000, immutable'",
            ),
            (
                "custom_api_method",
                "alter table routes add column custom_api_method text",
            ),
            (
                "custom_api_body",
                "alter table routes add column custom_api_body text",
            ),
            (
                "custom_api_file_field",
                "alter table routes add column custom_api_file_field text",
            ),
            (
                "custom_api_headers",
                "alter table routes add column custom_api_headers text",
            ),
            (
                "custom_api_url_path",
                "alter table routes add column custom_api_url_path text",
            ),
        ] {
            if !existing_columns.iter().any(|column| column == name) {
                self.conn.execute(ddl, []).map_err(storage_error)?;
            }
        }

        Ok(())
    }

    fn remove_legacy_demo_routes(&self) -> CommandResult<()> {
        self.conn
            .execute(
                "delete from routes
                  where (id = 'blog-r2'
                         and endpoint = 'https://example.r2.cloudflarestorage.com'
                         and public_base_url = 'https://cdn.picdog.app')
                     or (id = 'docs-oss'
                         and endpoint = 'https://oss.example.com'
                         and public_base_url = 'https://docs-cdn.picdog.app')",
                [],
            )
            .map_err(storage_error)?;

        Ok(())
    }

    fn clear_missing_default_route(&self) -> CommandResult<()> {
        let mut settings = self.get_settings()?;
        let Some(default_route_id) = settings.default_route_id.clone() else {
            return Ok(());
        };

        if self.get_route(&default_route_id)?.is_some() {
            return Ok(());
        }

        settings.default_route_id = None;
        self.save_settings(&settings)
    }

    fn get_settings_from_db(&self) -> CommandResult<Option<AppSettings>> {
        let value: Option<String> = self
            .conn
            .query_row("select value from settings where key = 'app'", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(storage_error)?;

        value
            .map(|value| serde_json::from_str(&value).map_err(storage_error))
            .transpose()
    }
}

fn route_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Route> {
    Ok(Route {
        id: row.get(0)?,
        name: row.get(1)?,
        provider: row.get(2)?,
        credential_ref: row.get(3)?,
        endpoint: row.get(4)?,
        bucket: row.get(5)?,
        region: row.get(6)?,
        path_template: row.get(7)?,
        public_base_url: row.get(8)?,
        cache_control: row.get(9)?,
        default_copy_format: row.get(10)?,
        custom_link_template: row.get(11)?,
        custom_api_method: row.get(12)?,
        custom_api_body: row.get(13)?,
        custom_api_file_field: row.get(14)?,
        custom_api_headers: row.get(15)?,
        custom_api_url_path: row.get(16)?,
        concurrency: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn history_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryItem> {
    let source_size: u64 = row.get(5)?;
    let output_size: Option<u64> = row.get(6)?;
    let saved_ratio: Option<u8> = row.get(7)?;
    let public_url: Option<String> = row.get(13)?;

    Ok(HistoryItem {
        id: row.get(0)?,
        batch_id: row.get(1)?,
        file_name: row.get(2)?,
        source_path: row.get(3)?,
        output_path: row.get(4)?,
        source_size,
        output_size,
        saved_ratio: saved_ratio.or_else(|| derive_saved_ratio(source_size, output_size)),
        output_format: row.get(8)?,
        width: row.get(9)?,
        height: row.get(10)?,
        route_name: row.get(11)?,
        route_id: row.get(12)?,
        public_url: public_url.clone(),
        compress_status: parse_compress_status(row.get::<_, String>(14)?.as_str()),
        upload_status: parse_upload_status(row.get::<_, String>(15)?.as_str()),
        can_copy: public_url.is_some(),
        created_at: row.get(16)?,
        uploaded_at: row.get(17)?,
        copied_at: row.get(18)?,
    })
}

fn asset_state_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetState> {
    let source_size: u64 = row.get(5)?;
    let output_size: Option<u64> = row.get(6)?;
    let saved_ratio: Option<u8> = row.get(7)?;
    let compress_status = parse_compress_status(row.get::<_, String>(11)?.as_str());
    let upload_status = parse_upload_status(row.get::<_, String>(12)?.as_str());

    Ok(AssetState {
        id: row.get(0)?,
        batch_id: row.get(1)?,
        name: row.get(2)?,
        source_path: row.get(3)?,
        output_path: row.get(4)?,
        source_size,
        output_size,
        mime: row.get(8)?,
        width: row.get(9)?,
        height: row.get(10)?,
        saved_ratio: saved_ratio.or_else(|| derive_saved_ratio(source_size, output_size)),
        compress_status: compress_status.clone(),
        upload_status: upload_status.clone(),
        compress_progress: if matches!(compress_status, CompressStatus::Done) {
            100
        } else {
            0
        },
        upload_progress: if matches!(upload_status, UploadStatus::Done) {
            100
        } else {
            0
        },
        selected: false,
        route_id: row.get(13)?,
        public_url: row.get(14)?,
        error: asset_error_from_row(row, 15, 16)?,
    })
}

fn asset_error_from_row(
    row: &rusqlite::Row<'_>,
    code_index: usize,
    message_index: usize,
) -> rusqlite::Result<Option<SafeError>> {
    let code: Option<String> = row.get(code_index)?;
    let message: Option<String> = row.get(message_index)?;

    Ok(code.map(|code| {
        SafeError::new(
            code,
            message.unwrap_or_else(|| "操作失败。".to_string()),
            true,
            true,
        )
    }))
}

fn derive_saved_ratio(source_size: u64, output_size: Option<u64>) -> Option<u8> {
    let output_size = output_size?;
    if source_size == 0 || output_size >= source_size {
        return Some(0);
    }

    Some((((source_size - output_size) * 100) / source_size) as u8)
}

fn batch_state_to_str(status: &BatchState) -> &'static str {
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

fn parse_batch_state(status: &str) -> BatchState {
    match status {
        "empty" => BatchState::Empty,
        "queue" => BatchState::Queue,
        "compressing" => BatchState::Compressing,
        "compressed" => BatchState::Compressed,
        "uploading" => BatchState::Uploading,
        "uploaded" => BatchState::Uploaded,
        "failed" => BatchState::Failed,
        "cancelling" => BatchState::Cancelling,
        _ => BatchState::Queue,
    }
}

fn compress_status_to_str(status: &CompressStatus) -> &'static str {
    match status {
        CompressStatus::Pending => "pending",
        CompressStatus::Running => "running",
        CompressStatus::Done => "done",
        CompressStatus::Failed => "failed",
        CompressStatus::Cancelled => "cancelled",
    }
}

fn upload_status_to_str(status: &UploadStatus) -> &'static str {
    match status {
        UploadStatus::None => "none",
        UploadStatus::Pending => "pending",
        UploadStatus::Running => "running",
        UploadStatus::Done => "done",
        UploadStatus::Failed => "failed",
        UploadStatus::Cancelled => "cancelled",
    }
}

fn parse_compress_status(status: &str) -> CompressStatus {
    match status {
        "running" => CompressStatus::Running,
        "done" => CompressStatus::Done,
        "failed" => CompressStatus::Failed,
        "cancelled" => CompressStatus::Cancelled,
        _ => CompressStatus::Pending,
    }
}

fn parse_upload_status(status: &str) -> UploadStatus {
    match status {
        "pending" => UploadStatus::Pending,
        "running" => UploadStatus::Running,
        "done" => UploadStatus::Done,
        "failed" => UploadStatus::Failed,
        "cancelled" => UploadStatus::Cancelled,
        _ => UploadStatus::None,
    }
}

fn storage_error(error: impl std::fmt::Display) -> SafeError {
    SafeError::new(
        "storage_error",
        format!("本地数据库访问失败：{}", error),
        true,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_item(id: &str) -> HistoryItem {
        HistoryItem {
            id: id.to_string(),
            batch_id: "batch-1".to_string(),
            file_name: "hero.webp".to_string(),
            source_path: Some("/tmp/hero.png".to_string()),
            output_path: Some("/tmp/hero.webp".to_string()),
            source_size: 100,
            output_size: Some(40),
            saved_ratio: Some(60),
            output_format: Some("webp".to_string()),
            width: Some(1200),
            height: Some(800),
            route_name: Some("Blog CDN · R2".to_string()),
            route_id: Some("blog-r2".to_string()),
            public_url: Some("https://cdn.picdog.app/hero.webp".to_string()),
            compress_status: CompressStatus::Done,
            upload_status: UploadStatus::Done,
            can_copy: true,
            created_at: "1".to_string(),
            uploaded_at: Some("2".to_string()),
            copied_at: None,
        }
    }

    fn route(id: &str) -> Route {
        Route {
            id: id.to_string(),
            name: format!("{} target", id),
            provider: "s3".to_string(),
            credential_ref: format!("keychain://picdog/{}", id),
            endpoint: Some("https://example.r2.cloudflarestorage.com".to_string()),
            bucket: Some("assets".to_string()),
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
    fn seeds_default_settings_and_routes() {
        let storage = Storage::open_in_memory().expect("storage opens");

        assert_eq!(storage.get_settings().unwrap().locale, "zh-CN");
        assert!(storage.get_settings().unwrap().default_route_id.is_none());
        assert!(storage.list_routes().unwrap().is_empty());
    }

    #[test]
    fn deletes_routes() {
        let storage = Storage::open_in_memory().expect("storage opens");
        storage.upsert_route(&route("docs-oss")).unwrap();
        assert!(storage.get_route("docs-oss").unwrap().is_some());

        storage.delete_route("docs-oss").unwrap();

        assert!(storage.get_route("docs-oss").unwrap().is_none());
        assert!(storage.list_routes().unwrap().is_empty());
    }

    #[test]
    fn persists_route_cache_control() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let mut route = route("blog-cache");
        route.cache_control = "public, max-age=3600, stale-while-revalidate=60".to_string();

        storage.upsert_route(&route).unwrap();
        let saved = storage.get_route("blog-cache").unwrap().unwrap();

        assert_eq!(
            saved.cache_control,
            "public, max-age=3600, stale-while-revalidate=60"
        );
    }

    #[test]
    fn persists_custom_api_route_options() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let mut route = route("custom-api");
        route.provider = "custom_api".to_string();
        route.custom_api_method = Some("PUT".to_string());
        route.custom_api_body = Some("multipart".to_string());
        route.custom_api_file_field = Some("image".to_string());
        route.custom_api_headers = Some("X-Key: {key}".to_string());
        route.custom_api_url_path = Some("data.url".to_string());

        storage.upsert_route(&route).unwrap();
        let saved = storage.get_route("custom-api").unwrap().unwrap();

        assert_eq!(saved.custom_api_method.as_deref(), Some("PUT"));
        assert_eq!(saved.custom_api_body.as_deref(), Some("multipart"));
        assert_eq!(saved.custom_api_file_field.as_deref(), Some("image"));
        assert_eq!(saved.custom_api_headers.as_deref(), Some("X-Key: {key}"));
        assert_eq!(saved.custom_api_url_path.as_deref(), Some("data.url"));
    }

    #[test]
    fn removes_legacy_demo_routes_and_clears_default_target() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let mut legacy = route("blog-r2");
        legacy.name = "Blog CDN · R2".to_string();
        legacy.endpoint = Some("https://example.r2.cloudflarestorage.com".to_string());
        legacy.public_base_url = "https://cdn.picdog.app".to_string();
        storage.upsert_route(&legacy).unwrap();

        let mut settings = storage.get_settings().unwrap();
        settings.default_route_id = Some("blog-r2".to_string());
        storage.save_settings(&settings).unwrap();

        storage.seed_defaults().unwrap();

        assert!(storage.get_route("blog-r2").unwrap().is_none());
        assert!(storage.get_settings().unwrap().default_route_id.is_none());
    }

    #[test]
    fn filters_history_by_route_id() {
        let storage = Storage::open_in_memory().expect("storage opens");

        let mut first = history_item("asset-route-1");
        first.route_id = Some("blog-r2".to_string());
        first.route_name = Some("Blog CDN · R2".to_string());

        let mut second = history_item("asset-route-2");
        second.route_id = Some("docs-oss".to_string());
        second.route_name = Some("Docs OSS".to_string());

        storage.upsert_history_items(&[first, second]).unwrap();

        let page = storage
            .list_history(&HistoryQuery {
                keyword: None,
                route_id: Some("docs-oss".to_string()),
                status: None,
                page: 1,
                page_size: 10,
            })
            .unwrap();

        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].route_id.as_deref(), Some("docs-oss"));
    }

    #[test]
    fn pending_batch_is_not_listed_as_history() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let batch = crate::models::Batch {
            id: "batch-pending".to_string(),
            status: crate::models::BatchState::Queue,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            assets: vec![crate::models::AssetState {
                id: "asset-pending".to_string(),
                batch_id: "batch-pending".to_string(),
                name: "draft.png".to_string(),
                source_path: "/tmp/draft.png".to_string(),
                output_path: None,
                source_size: 100,
                output_size: None,
                mime: Some("image/png".to_string()),
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
            }],
        };

        storage.upsert_batch(&batch).unwrap();

        let page = storage
            .list_history(&HistoryQuery {
                keyword: None,
                route_id: None,
                status: None,
                page: 1,
                page_size: 10,
            })
            .unwrap();

        assert_eq!(page.total, 0);
    }

    #[test]
    fn restores_batch_with_assets_from_storage() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let batch = crate::models::Batch {
            id: "batch-restore".to_string(),
            status: crate::models::BatchState::Uploaded,
            created_at: "1".to_string(),
            updated_at: "2".to_string(),
            assets: vec![crate::models::AssetState {
                id: "asset-restore".to_string(),
                batch_id: "batch-restore".to_string(),
                name: "hero.png".to_string(),
                source_path: "/tmp/hero.png".to_string(),
                output_path: Some("/tmp/hero.picdog.webp".to_string()),
                source_size: 100,
                output_size: Some(32),
                mime: Some("image/png".to_string()),
                width: Some(1200),
                height: Some(800),
                saved_ratio: Some(68),
                compress_status: CompressStatus::Done,
                upload_status: UploadStatus::Done,
                compress_progress: 100,
                upload_progress: 100,
                selected: true,
                route_id: Some("blog-r2".to_string()),
                public_url: Some("https://cdn.picdog.app/hero.webp".to_string()),
                error: None,
            }],
        };

        storage.upsert_batch(&batch).unwrap();

        let restored = storage
            .get_batch("batch-restore")
            .unwrap()
            .expect("batch exists");

        assert!(matches!(restored.status, BatchState::Uploaded));
        assert_eq!(restored.assets.len(), 1);
        assert_eq!(restored.assets[0].name, "hero.png");
        assert_eq!(restored.assets[0].compress_progress, 100);
        assert_eq!(restored.assets[0].upload_progress, 100);
        assert_eq!(restored.assets[0].route_id.as_deref(), Some("blog-r2"));
        assert!(!restored.assets[0].selected);
    }

    #[test]
    fn latest_batch_returns_recent_non_cleared_batch() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let older = crate::models::Batch {
            id: "batch-older".to_string(),
            status: crate::models::BatchState::Queue,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            assets: vec![crate::models::AssetState {
                id: "asset-older".to_string(),
                batch_id: "batch-older".to_string(),
                name: "older.png".to_string(),
                source_path: "/tmp/older.png".to_string(),
                output_path: None,
                source_size: 100,
                output_size: None,
                mime: Some("image/png".to_string()),
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
            }],
        };
        let mut newer = older.clone();
        newer.id = "batch-newer".to_string();
        newer.updated_at = "3".to_string();
        newer.assets[0].id = "asset-newer".to_string();
        newer.assets[0].batch_id = "batch-newer".to_string();
        newer.assets[0].name = "newer.png".to_string();

        storage.upsert_batch(&older).unwrap();
        storage.upsert_batch(&newer).unwrap();

        let latest = storage
            .latest_batch()
            .unwrap()
            .expect("latest batch exists");

        assert_eq!(latest.id, "batch-newer");
        assert_eq!(latest.assets.len(), 1);
        assert_eq!(latest.assets[0].name, "newer.png");
    }

    #[test]
    fn latest_batch_ignores_cleared_batches() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let active = crate::models::Batch {
            id: "batch-active".to_string(),
            status: crate::models::BatchState::Queue,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
            assets: vec![crate::models::AssetState {
                id: "asset-active".to_string(),
                batch_id: "batch-active".to_string(),
                name: "active.png".to_string(),
                source_path: "/tmp/active.png".to_string(),
                output_path: None,
                source_size: 100,
                output_size: None,
                mime: Some("image/png".to_string()),
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
            }],
        };
        let mut cleared = active.clone();
        cleared.id = "batch-cleared".to_string();
        cleared.updated_at = "4".to_string();
        cleared.assets[0].id = "asset-cleared".to_string();
        cleared.assets[0].batch_id = "batch-cleared".to_string();
        cleared.assets[0].name = "cleared.png".to_string();

        storage.upsert_batch(&active).unwrap();
        storage.upsert_batch(&cleared).unwrap();
        storage.clear_batch("batch-cleared", &[]).unwrap();

        let latest = storage
            .latest_batch()
            .unwrap()
            .expect("latest batch exists");

        assert_eq!(latest.id, "batch-active");
        assert_eq!(latest.assets[0].name, "active.png");
    }

    #[test]
    fn stores_history_and_soft_clears_it() {
        let storage = Storage::open_in_memory().expect("storage opens");
        let item = history_item("asset-1");

        storage.upsert_history_items(&[item]).unwrap();

        let page = storage
            .list_history(&HistoryQuery {
                keyword: None,
                route_id: None,
                status: Some("uploaded".to_string()),
                page: 1,
                page_size: 10,
            })
            .unwrap();
        assert_eq!(page.total, 1);

        storage
            .clear_history(
                &ClearHistoryInput {
                    scope: "all".to_string(),
                    ids: None,
                    clear_local_outputs: false,
                },
                &[],
            )
            .unwrap();

        let page = storage
            .list_history(&HistoryQuery {
                keyword: None,
                route_id: None,
                status: None,
                page: 1,
                page_size: 10,
            })
            .unwrap();
        assert_eq!(page.total, 0);
    }

    #[test]
    fn rejects_invalid_history_clear_scope() {
        let storage = Storage::open_in_memory().expect("storage opens");

        let error = storage
            .clear_history(
                &ClearHistoryInput {
                    scope: "everything".to_string(),
                    ids: None,
                    clear_local_outputs: false,
                },
                &[],
            )
            .expect_err("invalid scope is rejected");

        assert_eq!(error.code, "invalid_history_scope");
    }

    #[test]
    fn clear_history_marks_only_deleted_output_ids() {
        let storage = Storage::open_in_memory().expect("storage opens");
        storage
            .upsert_history_items(&[history_item("asset-1"), history_item("asset-2")])
            .unwrap();

        storage
            .clear_history(
                &ClearHistoryInput {
                    scope: "all".to_string(),
                    ids: None,
                    clear_local_outputs: true,
                },
                &["asset-1".to_string()],
            )
            .unwrap();

        let first_deleted_at: Option<String> = storage
            .conn
            .query_row(
                "select output_deleted_at from assets where id = 'asset-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let second_deleted_at: Option<String> = storage
            .conn
            .query_row(
                "select output_deleted_at from assets where id = 'asset-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(first_deleted_at.is_some());
        assert!(second_deleted_at.is_none());
    }
}
