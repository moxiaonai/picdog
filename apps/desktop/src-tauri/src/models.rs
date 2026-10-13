use crate::error::SafeError;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_string() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();

    millis.to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileInput {
    pub path: String,
    pub name: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_directory: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BatchState {
    Empty,
    Queue,
    Compressing,
    Compressed,
    Uploading,
    Uploaded,
    Failed,
    Cancelling,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CompressStatus {
    Pending,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    None,
    Pending,
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssetState {
    pub id: String,
    pub batch_id: String,
    pub name: String,
    pub source_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    pub source_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_ratio: Option<u8>,
    pub compress_status: CompressStatus,
    pub upload_status: UploadStatus,
    pub compress_progress: u8,
    pub upload_progress: u8,
    pub selected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SafeError>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    pub id: String,
    pub status: BatchState,
    pub assets: Vec<AssetState>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompressionOptions {
    pub mode: String,
    pub quality: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoder_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
    pub output_format: String,
    pub strip_metadata: bool,
    pub preserve_color_profile: bool,
    pub keep_original: bool,
    pub fallback_when_larger: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompressionToolStatus {
    pub name: String,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub purpose: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub credential_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub path_template: String,
    pub public_base_url: String,
    pub cache_control: String,
    pub default_copy_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_link_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_file_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_url_path: Option<String>,
    pub concurrency: u8,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouteDraft {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub path_template: String,
    pub public_base_url: String,
    #[serde(default = "default_route_cache_control")]
    pub cache_control: String,
    pub default_copy_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_link_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_file_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_headers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_api_url_path: Option<String>,
    pub concurrency: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<RouteCredentialInput>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RouteCredentialInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouteCredentialStatus {
    pub route_id: String,
    pub credential_ref: String,
    pub configured: bool,
    pub checked_at: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouteTestResult {
    pub route_id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_code: Option<String>,
    pub message: String,
    pub checked_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_base_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub locale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_route_id: Option<String>,
    #[serde(default)]
    pub upload_enabled: bool,
    pub copy_format: String,
    pub hide_local_path: bool,
    pub delete_local_outputs_on_clear: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoder_directory: Option<String>,
    #[serde(default = "default_compression_mode")]
    pub compression_mode: String,
    #[serde(default = "default_compression_quality")]
    pub compression_quality: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
    #[serde(default = "default_output_format")]
    pub output_format: String,
    #[serde(default = "default_true")]
    pub strip_metadata: bool,
    #[serde(default = "default_true")]
    pub preserve_color_profile: bool,
    #[serde(default = "default_true")]
    pub keep_original: bool,
    #[serde(default = "default_true")]
    pub fallback_when_larger: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompareImagePayload {
    pub asset_id: String,
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    pub source_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_ratio: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_data_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_data_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub batch_id: String,
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    pub source_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_ratio: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_url: Option<String>,
    pub compress_status: CompressStatus,
    pub upload_status: UploadStatus,
    pub can_copy: bool,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploaded_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copied_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub items: Vec<HistoryItem>,
    pub page: u32,
    pub page_size: u32,
    pub total: u32,
}

pub fn default_settings() -> AppSettings {
    AppSettings {
        locale: "zh-CN".to_string(),
        default_route_id: None,
        upload_enabled: false,
        copy_format: "markdown".to_string(),
        hide_local_path: false,
        delete_local_outputs_on_clear: false,
        encoder_directory: None,
        compression_mode: default_compression_mode(),
        compression_quality: default_compression_quality(),
        max_width: None,
        max_height: None,
        output_format: default_output_format(),
        strip_metadata: true,
        preserve_color_profile: true,
        keep_original: true,
        fallback_when_larger: true,
    }
}

fn default_compression_mode() -> String {
    "balanced".to_string()
}

fn default_compression_quality() -> u8 {
    78
}

fn default_output_format() -> String {
    "original".to_string()
}

pub fn default_route_cache_control() -> String {
    "public, max-age=31536000, immutable".to_string()
}

fn default_true() -> bool {
    true
}

pub fn default_routes(now: &str) -> Vec<Route> {
    let _ = now;
    Vec::new()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClearHistoryInput {
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
    pub clear_local_outputs: bool,
}

#[cfg(test)]
mod app_settings_tests {
    use super::*;

    #[test]
    fn legacy_settings_json_gets_compression_defaults() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
                "locale": "zh-CN",
                "defaultRouteId": "blog-r2",
                "copyFormat": "markdown",
                "hideLocalPath": false,
                "deleteLocalOutputsOnClear": false
            }"#,
        )
        .expect("legacy settings deserialize");

        assert_eq!(settings.compression_mode, "balanced");
        assert!(!settings.upload_enabled);
        assert_eq!(settings.encoder_directory, None);
        assert_eq!(settings.compression_quality, 78);
        assert_eq!(settings.max_width, None);
        assert_eq!(settings.max_height, None);
        assert_eq!(settings.output_format, "original");
        assert!(settings.strip_metadata);
        assert!(settings.preserve_color_profile);
        assert!(settings.keep_original);
        assert!(settings.fallback_when_larger);
    }
}
