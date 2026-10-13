use super::{http_client, response_status_is_retryable, UploadObjectInput, UploadObjectOutput};
use crate::error::{CommandResult, SafeError};
use crate::models::{Route, RouteCredentialInput};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::blocking::{multipart, Response};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use sha1::Sha1;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

const DEFAULT_UPLOAD_ENDPOINT: &str = "https://upload.qiniup.com";
const DEFAULT_RS_ENDPOINT: &str = "https://rs.qiniu.com";

pub fn upload_object(input: UploadObjectInput<'_>) -> CommandResult<UploadObjectOutput> {
    let body = fs::read(input.local_path).map_err(|_| {
        SafeError::new(
            "file_not_found",
            "找不到压缩后的本地文件，无法上传。",
            true,
            false,
        )
    })?;
    let remote_key = normalize_remote_key(input.remote_key)?;
    let token = upload_token(
        input.route,
        input.credential,
        &remote_key,
        unix_deadline_after_seconds(3600),
    )?;
    let file_name = std::path::Path::new(input.local_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    let file_part = multipart::Part::bytes(body)
        .file_name(file_name.to_string())
        .mime_str(input.content_type)
        .map_err(|_| upload_error("文件类型无效。"))?;
    let form = multipart::Form::new()
        .text("token", token)
        .text("key", remote_key.clone())
        .part("file", file_part);
    let response = http_client()?
        .post(upload_endpoint(input.route))
        .multipart(form)
        .send()
        .map_err(network_error)?;

    ensure_success(response, "七牛云上传失败")?;

    Ok(UploadObjectOutput {
        public_url: public_url_for_key(input.route, &remote_key),
    })
}

pub fn delete_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
) -> CommandResult<()> {
    let bucket = required_route_field(route.bucket.as_deref(), "Bucket")?;
    let access_key = required_secret(credential.access_key_id.as_deref(), "Access Key")?;
    let secret_key = required_secret(credential.secret_access_key.as_deref(), "Secret Key")?;
    let remote_key = normalize_remote_key(remote_key)?;
    let encoded_entry = base64_urlsafe(format!("{}:{}", bucket, remote_key).as_bytes());
    let path = format!("/delete/{}", encoded_entry);
    let authorization = qiniu_management_authorization(access_key, secret_key, &path)?;
    let response = http_client()?
        .post(format!("{}{}", rs_endpoint(route), path))
        .headers(qiniu_headers(&authorization)?)
        .send()
        .map_err(network_error)?;

    let status = response.status();
    if status.is_success() || status.as_u16() == 612 {
        return Ok(());
    }

    Err(SafeError::new(
        "upload_failed",
        format!("七牛云清理测试对象失败，服务端返回 {}。", status),
        true,
        response_status_is_retryable(status),
    ))
}

fn upload_token(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
    deadline: u64,
) -> CommandResult<String> {
    let bucket = required_route_field(route.bucket.as_deref(), "Bucket")?;
    let access_key = required_secret(credential.access_key_id.as_deref(), "Access Key")?;
    let secret_key = required_secret(credential.secret_access_key.as_deref(), "Secret Key")?;
    let policy = serde_json::json!({
        "scope": format!("{}:{}", bucket, remote_key),
        "deadline": deadline
    })
    .to_string();
    let encoded_policy = base64_urlsafe(policy.as_bytes());
    let encoded_sign = qiniu_hmac_sha1(secret_key, encoded_policy.as_bytes())?;

    Ok(format!(
        "{}:{}:{}",
        access_key, encoded_sign, encoded_policy
    ))
}

fn qiniu_management_authorization(
    access_key: &str,
    secret_key: &str,
    path: &str,
) -> CommandResult<String> {
    let signing_text = format!("{}\n", path);
    let encoded_sign = qiniu_hmac_sha1(secret_key, signing_text.as_bytes())?;

    Ok(format!("QBox {}:{}", access_key, encoded_sign))
}

fn qiniu_hmac_sha1(secret_key: &str, message: &[u8]) -> CommandResult<String> {
    let mut mac = HmacSha1::new_from_slice(secret_key.as_bytes()).map_err(|_| {
        SafeError::new("upload_failed", "七牛云 Secret Key 格式无效。", true, false)
    })?;
    mac.update(message);
    Ok(base64_urlsafe(&mac.finalize().into_bytes()))
}

fn qiniu_headers(authorization: &str) -> CommandResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(authorization).map_err(|_| upload_error("七牛云授权头格式无效。"))?,
    );
    Ok(headers)
}

fn upload_endpoint(route: &Route) -> String {
    route
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_UPLOAD_ENDPOINT)
        .trim_end_matches('/')
        .to_string()
}

fn rs_endpoint(route: &Route) -> String {
    route
        .region
        .as_deref()
        .map(str::trim)
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
        .unwrap_or(DEFAULT_RS_ENDPOINT)
        .trim_end_matches('/')
        .to_string()
}

fn public_url_for_key(route: &Route, remote_key: &str) -> String {
    let base = route.public_base_url.trim().trim_end_matches('/');
    let key = remote_key
        .split('/')
        .map(percent_encode_path_segment)
        .collect::<Vec<_>>()
        .join("/");

    format!("{}/{}", base, key)
}

fn normalize_remote_key(remote_key: &str) -> CommandResult<String> {
    let remote_key = remote_key.trim().trim_start_matches('/');
    if remote_key.is_empty()
        || remote_key
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(SafeError::new(
            "upload_failed",
            "七牛云存储路径无效。",
            true,
            false,
        ));
    }

    Ok(remote_key.to_string())
}

fn percent_encode_path_segment(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char);
            }
            _ => output.push_str(&format!("%{:02X}", byte)),
        }
    }
    output
}

fn unix_deadline_after_seconds(seconds: u64) -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() + seconds)
        .unwrap_or(seconds)
}

fn base64_urlsafe(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn ensure_success(response: Response, label: &str) -> CommandResult<()> {
    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    Err(SafeError::new(
        "upload_failed",
        format!("{}，服务端返回 {}。", label, status),
        true,
        response_status_is_retryable(status),
    ))
}

fn required_route_field<'a>(value: Option<&'a str>, label: &str) -> CommandResult<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "upload_failed",
                format!("七牛云上传目标缺少 {}。", label),
                true,
                false,
            )
        })
}

fn required_secret<'a>(value: Option<&'a str>, label: &str) -> CommandResult<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "credential_missing",
                format!("七牛云凭据缺少 {}。", label),
                true,
                false,
            )
        })
}

fn network_error(error: impl std::fmt::Display) -> SafeError {
    SafeError::new(
        "network_timeout",
        format!("七牛云网络请求失败：{}。", error),
        true,
        true,
    )
}

fn upload_error(message: &str) -> SafeError {
    SafeError::new("upload_failed", message, true, false)
}

#[cfg(test)]
mod tests {
    use super::{public_url_for_key, qiniu_management_authorization, upload_token};
    use crate::models::{Route, RouteCredentialInput};

    fn route() -> Route {
        Route {
            id: "qiniu".to_string(),
            name: "Qiniu Kodo".to_string(),
            provider: "qiniu".to_string(),
            credential_ref: "keychain://picdog/qiniu".to_string(),
            endpoint: None,
            bucket: Some("assets".to_string()),
            region: None,
            path_template: "{yyyy}/{mm}/{name}.{ext}".to_string(),
            public_base_url: "https://cdn.example.com".to_string(),
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

    fn credential() -> RouteCredentialInput {
        RouteCredentialInput {
            access_key_id: Some("ak".to_string()),
            secret_access_key: Some("sk".to_string()),
            session_token: None,
            api_token: None,
        }
    }

    #[test]
    fn upload_token_contains_access_key_and_policy() {
        let token = upload_token(&route(), &credential(), "2026/10/hero.webp", 1800).unwrap();
        let parts = token.split(':').collect::<Vec<_>>();

        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "ak");
        assert!(!parts[1].is_empty());
        assert!(parts[2].contains("eyJkZWFkbGluZSI"));
    }

    #[test]
    fn public_url_for_key_encodes_segments() {
        assert_eq!(
            public_url_for_key(&route(), "2026/10/hero image.webp"),
            "https://cdn.example.com/2026/10/hero%20image.webp"
        );
    }

    #[test]
    fn management_authorization_uses_qbox_prefix() {
        let authorization = qiniu_management_authorization("ak", "sk", "/delete/entry").unwrap();

        assert!(authorization.starts_with("QBox ak:"));
    }
}
