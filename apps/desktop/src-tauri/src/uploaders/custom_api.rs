use super::{http_client, response_status_is_retryable, UploadObjectInput, UploadObjectOutput};
use crate::error::{CommandResult, SafeError};
use crate::models::{Route, RouteCredentialInput};
use reqwest::blocking::RequestBuilder;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use reqwest::Method;
use std::fs;
use std::path::Path;

const DEFAULT_FILE_FIELD: &str = "file";

pub fn upload_object(input: UploadObjectInput<'_>) -> CommandResult<UploadObjectOutput> {
    let body = fs::read(input.local_path).map_err(|_| {
        SafeError::new(
            "file_not_found",
            "找不到压缩后的本地文件，无法上传。",
            true,
            false,
        )
    })?;
    let endpoint = input
        .route
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new("upload_failed", "Custom API 缺少 Endpoint。", true, false)
        })?;

    let file_name = Path::new(input.local_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    let method = custom_api_method(input.route)?;
    let body_mode = custom_api_body(input.route)?;
    let mut headers = custom_headers(input.route, input.remote_key, input.content_type, file_name)?;
    headers.insert("x-picdog-key", header_value(input.remote_key)?);

    let request_body = match body_mode {
        CustomApiBody::Binary => {
            headers.insert(CONTENT_TYPE, header_value(input.content_type)?);
            body
        }
        CustomApiBody::Multipart => {
            let file_field = input
                .route
                .custom_api_file_field
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(DEFAULT_FILE_FIELD);
            let (multipart_body, content_type) = build_multipart_body(
                file_field,
                file_name,
                input.content_type,
                input.remote_key,
                &body,
            )?;
            headers.insert(CONTENT_TYPE, header_value(&content_type)?);
            multipart_body
        }
    };

    let request = http_client()?
        .request(method, endpoint)
        .headers(headers)
        .body(request_body);
    let request = apply_auth(request, input.credential);
    let response = request.send().map_err(network_error)?;
    let status = response.status();
    let body = response.text().unwrap_or_default();

    if !status.is_success() {
        return Err(SafeError::new(
            "upload_failed",
            format!("Custom API 上传失败，服务端返回 {}。", status),
            true,
            response_status_is_retryable(status),
        ));
    }

    Ok(UploadObjectOutput {
        public_url: public_url_from_response(&body, input.route.custom_api_url_path.as_deref())
            .unwrap_or_else(|| public_url_for_key(input.route, input.remote_key)),
    })
}

pub fn delete_object(
    _route: &Route,
    _credential: &RouteCredentialInput,
    _remote_key: &str,
) -> CommandResult<()> {
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CustomApiBody {
    Binary,
    Multipart,
}

fn custom_api_method(route: &Route) -> CommandResult<Method> {
    match route
        .custom_api_method
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("POST")
        .to_ascii_uppercase()
        .as_str()
    {
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "PATCH" => Ok(Method::PATCH),
        _ => Err(SafeError::new(
            "upload_failed",
            "Custom API 请求方法只支持 POST、PUT 或 PATCH。",
            true,
            false,
        )),
    }
}

fn custom_api_body(route: &Route) -> CommandResult<CustomApiBody> {
    match route
        .custom_api_body
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("binary")
    {
        "binary" => Ok(CustomApiBody::Binary),
        "multipart" => Ok(CustomApiBody::Multipart),
        _ => Err(SafeError::new(
            "upload_failed",
            "Custom API 请求体只支持 binary 或 multipart。",
            true,
            false,
        )),
    }
}

fn apply_auth(request: RequestBuilder, credential: &RouteCredentialInput) -> RequestBuilder {
    if let Some(token) = credential
        .api_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return request.bearer_auth(token);
    }

    request
}

fn custom_headers(
    route: &Route,
    remote_key: &str,
    content_type: &str,
    file_name: &str,
) -> CommandResult<HeaderMap> {
    let Some(raw) = route
        .custom_api_headers
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(HeaderMap::new());
    };

    if raw.starts_with('{') {
        return custom_headers_from_json(raw, remote_key, content_type, file_name);
    }

    let mut headers = HeaderMap::new();
    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or_else(|| {
            SafeError::new(
                "upload_failed",
                "Custom API 请求头格式应为 Header-Name: value。",
                true,
                false,
            )
        })?;
        headers.insert(
            header_name(name)?,
            header_value(&apply_header_placeholders(
                value.trim(),
                remote_key,
                content_type,
                file_name,
            ))?,
        );
    }

    Ok(headers)
}

fn custom_headers_from_json(
    raw: &str,
    remote_key: &str,
    content_type: &str,
    file_name: &str,
) -> CommandResult<HeaderMap> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| {
        SafeError::new(
            "upload_failed",
            "Custom API 请求头 JSON 无法解析。",
            true,
            false,
        )
    })?;
    let object = value.as_object().ok_or_else(|| {
        SafeError::new(
            "upload_failed",
            "Custom API 请求头 JSON 应为对象。",
            true,
            false,
        )
    })?;
    let mut headers = HeaderMap::new();

    for (name, value) in object {
        let value = value.as_str().ok_or_else(|| {
            SafeError::new(
                "upload_failed",
                "Custom API 请求头 JSON 的值必须是字符串。",
                true,
                false,
            )
        })?;
        headers.insert(
            header_name(name)?,
            header_value(&apply_header_placeholders(
                value,
                remote_key,
                content_type,
                file_name,
            ))?,
        );
    }

    Ok(headers)
}

fn apply_header_placeholders(
    value: &str,
    remote_key: &str,
    content_type: &str,
    file_name: &str,
) -> String {
    value
        .replace("{key}", remote_key)
        .replace("{name}", file_name)
        .replace("{contentType}", content_type)
}

fn build_multipart_body(
    file_field: &str,
    file_name: &str,
    content_type: &str,
    remote_key: &str,
    file: &[u8],
) -> CommandResult<(Vec<u8>, String)> {
    let file_field = safe_form_name(file_field)?;
    let file_name = safe_file_name(file_name)?;
    let boundary = format!("PicDogBoundary{}", crate::models::now_string());
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
            file_field, file_name
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", content_type).as_bytes());
    body.extend_from_slice(file);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"key\"\r\n\r\n");
    body.extend_from_slice(remote_key.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    Ok((body, format!("multipart/form-data; boundary={}", boundary)))
}

fn safe_form_name(value: &str) -> CommandResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains('\r')
        || trimmed.contains('\n')
        || trimmed.contains('"')
    {
        return Err(SafeError::new(
            "upload_failed",
            "Custom API 文件字段名无效。",
            true,
            false,
        ));
    }

    Ok(trimmed.to_string())
}

fn safe_file_name(value: &str) -> CommandResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains('\r')
        || trimmed.contains('\n')
        || trimmed.contains('"')
    {
        return Ok("image".to_string());
    }

    Ok(trimmed.to_string())
}

fn public_url_from_response(body: &str, configured_path: Option<&str>) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;

    if let Some(path) = configured_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        if let Some(url) = value_at_path(&value, path).and_then(|value| value.as_str()) {
            return http_url(url);
        }
    }

    [
        "url",
        "publicUrl",
        "public_url",
        "cdnUrl",
        "cdn_url",
        "data.url",
        "data.publicUrl",
        "data.public_url",
        "result.url",
    ]
    .iter()
    .find_map(|key| {
        value_at_path(&value, key)
            .and_then(|value| value.as_str())
            .and_then(http_url)
    })
}

fn value_at_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.trim().trim_start_matches("$.").split('.') {
        if segment.is_empty() {
            return None;
        }
        if let Ok(index) = segment.parse::<usize>() {
            current = current.get(index)?;
        } else {
            current = current.get(segment)?;
        }
    }

    Some(current)
}

fn http_url(value: &str) -> Option<String> {
    let value = value.trim();
    (value.starts_with("http://") || value.starts_with("https://")).then(|| value.to_string())
}

fn public_url_for_key(route: &Route, remote_key: &str) -> String {
    format!(
        "{}/{}",
        route.public_base_url.trim_end_matches('/'),
        remote_key.trim_start_matches('/')
    )
}

fn header_name(value: &str) -> CommandResult<HeaderName> {
    HeaderName::from_bytes(value.trim().as_bytes()).map_err(|error| {
        SafeError::new(
            "upload_failed",
            format!("Custom API 请求头名称无效：{}", error),
            true,
            false,
        )
    })
}

fn header_value(value: &str) -> CommandResult<HeaderValue> {
    HeaderValue::from_str(value).map_err(|error| {
        SafeError::new(
            "upload_failed",
            format!("Custom API 请求头生成失败：{}", error),
            true,
            false,
        )
    })
}

fn network_error(error: impl std::fmt::Display) -> SafeError {
    SafeError::new(
        "network_timeout",
        format!("Custom API 网络请求失败：{}", error),
        true,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_multipart_body, custom_headers, public_url_for_key, public_url_from_response,
    };
    use crate::models::Route;

    fn route() -> Route {
        Route {
            id: "custom-api".to_string(),
            name: "Custom API".to_string(),
            provider: "custom_api".to_string(),
            credential_ref: "keychain://picdog/custom-api".to_string(),
            endpoint: Some("https://upload.example.com/images".to_string()),
            bucket: None,
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
            concurrency: 2,
            created_at: "1".to_string(),
            updated_at: "1".to_string(),
        }
    }

    #[test]
    fn extracts_public_url_from_common_response_keys() {
        assert_eq!(
            public_url_from_response(r#"{ "publicUrl": "https://cdn.example.com/a.webp" }"#, None),
            Some("https://cdn.example.com/a.webp".to_string())
        );
        assert_eq!(
            public_url_from_response(r#"{ "url": "notaurl" }"#, None),
            None
        );
    }

    #[test]
    fn extracts_public_url_from_configured_response_path() {
        assert_eq!(
            public_url_from_response(
                r#"{ "data": { "links": [{ "href": "https://cdn.example.com/a.png" }] } }"#,
                Some("data.links.0.href"),
            ),
            Some("https://cdn.example.com/a.png".to_string())
        );
    }

    #[test]
    fn parses_custom_headers_with_placeholders() {
        let mut route = route();
        route.custom_api_headers =
            Some("X-Key: {key}\nX-Name: {name}\nX-Type: {contentType}".to_string());
        let headers = custom_headers(&route, "2026/10/a.webp", "image/webp", "a.webp").unwrap();

        assert_eq!(headers.get("x-key").unwrap(), "2026/10/a.webp");
        assert_eq!(headers.get("x-name").unwrap(), "a.webp");
        assert_eq!(headers.get("x-type").unwrap(), "image/webp");
    }

    #[test]
    fn builds_multipart_body_with_file_and_key_fields() {
        let (body, content_type) =
            build_multipart_body("image", "a.webp", "image/webp", "2026/10/a.webp", b"abc")
                .unwrap();
        let body = String::from_utf8(body).unwrap();

        assert!(content_type.starts_with("multipart/form-data; boundary=PicDogBoundary"));
        assert!(body.contains("name=\"image\"; filename=\"a.webp\""));
        assert!(body.contains("Content-Type: image/webp"));
        assert!(body.contains("name=\"key\""));
        assert!(body.contains("2026/10/a.webp"));
    }

    #[test]
    fn falls_back_to_route_public_url() {
        assert_eq!(
            public_url_for_key(&route(), "2026/10/hero.webp"),
            "https://cdn.example.com/2026/10/hero.webp"
        );
    }
}
