pub mod aliyun_oss;
pub mod custom_api;
pub mod github;
pub mod local_folder;
pub mod qiniu;
pub mod s3;
pub mod tencent_cos;

use crate::error::{CommandResult, SafeError};
use crate::models::{Route, RouteCredentialInput};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::path::PathBuf;
use std::time::Duration;

const CONNECT_TIMEOUT_SECONDS: u64 = 15;
const REQUEST_TIMEOUT_SECONDS: u64 = 60;
const DEFAULT_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

pub struct UploadObjectInput<'a> {
    pub route: &'a Route,
    pub credential: &'a RouteCredentialInput,
    pub local_path: &'a str,
    pub remote_key: &'a str,
    pub content_type: &'a str,
}

pub struct UploadObjectOutput {
    pub public_url: String,
}

const ROUTE_TEST_REMOTE_KEY: &str = "__picdog_route_test__/connection.txt";

pub fn cache_control_for_route(route: &Route) -> &str {
    let value = route.cache_control.trim();
    if value.is_empty() {
        DEFAULT_CACHE_CONTROL
    } else {
        value
    }
}

pub fn http_client() -> CommandResult<Client> {
    Client::builder()
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .build()
        .map_err(|error| {
            SafeError::new(
                "upload_failed",
                format!("初始化上传网络客户端失败：{}。", error),
                true,
                true,
            )
        })
}

pub fn response_status_is_retryable(status: StatusCode) -> bool {
    status.is_server_error()
        || matches!(
            status,
            StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
        )
}

#[cfg(test)]
mod tests {
    use super::response_status_is_retryable;
    use reqwest::StatusCode;

    #[test]
    fn retries_transient_http_statuses_only() {
        assert!(response_status_is_retryable(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(response_status_is_retryable(StatusCode::BAD_GATEWAY));
        assert!(response_status_is_retryable(StatusCode::REQUEST_TIMEOUT));
        assert!(response_status_is_retryable(StatusCode::TOO_MANY_REQUESTS));
        assert!(!response_status_is_retryable(StatusCode::UNAUTHORIZED));
        assert!(!response_status_is_retryable(StatusCode::FORBIDDEN));
        assert!(!response_status_is_retryable(StatusCode::BAD_REQUEST));
    }
}

pub fn upload_object(input: UploadObjectInput<'_>) -> CommandResult<UploadObjectOutput> {
    match input.route.provider.as_str() {
        "custom_api" => custom_api::upload_object(input),
        "github" => github::upload_object(input),
        "local_folder" => local_folder::upload_object(input),
        "qiniu" => qiniu::upload_object(input),
        "aliyun_oss" => aliyun_oss::upload_object(input),
        "s3" => s3::upload_object(input),
        "tencent_cos" => tencent_cos::upload_object(input),
        _ => Err(crate::error::SafeError::new(
            "upload_failed",
            "暂不支持该上传目标类型。",
            true,
            false,
        )),
    }
}

pub fn delete_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
) -> CommandResult<()> {
    match route.provider.as_str() {
        "custom_api" => custom_api::delete_object(route, credential, remote_key),
        "github" => github::delete_object(route, credential, remote_key),
        "local_folder" => local_folder::delete_object(route, remote_key),
        "qiniu" => qiniu::delete_object(route, credential, remote_key),
        "aliyun_oss" => aliyun_oss::delete_object(route, credential, remote_key),
        "s3" => s3::delete_object(route, credential, remote_key),
        "tencent_cos" => tencent_cos::delete_object(route, credential, remote_key),
        _ => Ok(()),
    }
}

pub fn test_route(route: &Route, credential: &RouteCredentialInput) -> CommandResult<String> {
    let local_path = route_test_file_path(route);
    std::fs::write(&local_path, b"PicDog route connectivity test\n").map_err(|_| {
        crate::error::SafeError::new("permission_denied", "无法创建上传测试文件。", true, true)
    })?;

    let local_path_string = local_path.to_string_lossy().to_string();
    let result = upload_object(UploadObjectInput {
        route,
        credential,
        local_path: &local_path_string,
        remote_key: ROUTE_TEST_REMOTE_KEY,
        content_type: "text/plain; charset=utf-8",
    });

    let _ = std::fs::remove_file(&local_path);

    result.map(|output| {
        let _ = delete_object(route, credential, ROUTE_TEST_REMOTE_KEY);
        output.public_url
    })
}

fn route_test_file_path(route: &Route) -> PathBuf {
    let safe_route_id = route
        .id
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();

    std::env::temp_dir().join(format!("picdog-route-test-{}.txt", safe_route_id))
}
