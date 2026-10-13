use super::{
    cache_control_for_route, http_client, response_status_is_retryable, UploadObjectInput,
    UploadObjectOutput,
};
use crate::error::{CommandResult, SafeError};
use crate::models::{Route, RouteCredentialInput};
use hmac::{Hmac, Mac};
use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, HOST,
};
use reqwest::StatusCode;
use sha1::{Digest as Sha1Digest, Sha1};
use std::fs;
use time::OffsetDateTime;
use url::Url;

type HmacSha1 = Hmac<Sha1>;

const SIGNATURE_TTL_SECONDS: i64 = 600;

pub fn upload_object(input: UploadObjectInput<'_>) -> CommandResult<UploadObjectOutput> {
    let body = fs::read(input.local_path).map_err(|_| {
        SafeError::new(
            "file_not_found",
            "找不到压缩后的本地文件，无法上传。",
            true,
            false,
        )
    })?;
    let signed = sign_put_object(
        input.route,
        input.credential,
        input.remote_key,
        input.content_type,
        OffsetDateTime::now_utc(),
    )?;

    let response = http_client()?
        .put(&signed.url)
        .headers(signed.headers)
        .body(body)
        .send()
        .map_err(network_error)?;

    let status = response.status();
    if !status.is_success() {
        return Err(SafeError::new(
            "upload_failed",
            format!("腾讯云 COS 上传失败，服务端返回 {}。", status),
            true,
            response_status_is_retryable(status),
        ));
    }

    Ok(UploadObjectOutput {
        public_url: public_url_for_key(input.route, input.remote_key),
    })
}

pub fn delete_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
) -> CommandResult<()> {
    let signed = sign_delete_object(route, credential, remote_key, OffsetDateTime::now_utc())?;
    let response = http_client()?
        .delete(&signed.url)
        .headers(signed.headers)
        .send()
        .map_err(network_error)?;

    let status = response.status();
    if status.is_success() || status == StatusCode::NOT_FOUND {
        return Ok(());
    }

    Err(SafeError::new(
        "upload_failed",
        format!("腾讯云 COS 清理测试对象失败，服务端返回 {}。", status),
        true,
        response_status_is_retryable(status),
    ))
}

struct SignedObjectRequest {
    url: String,
    headers: HeaderMap,
}

fn sign_put_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
    content_type: &str,
    now: OffsetDateTime,
) -> CommandResult<SignedObjectRequest> {
    signed_object_request(
        "put",
        route,
        credential,
        remote_key,
        Some(content_type),
        Some(cache_control_for_route(route)),
        now,
    )
}

fn sign_delete_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
    now: OffsetDateTime,
) -> CommandResult<SignedObjectRequest> {
    signed_object_request("delete", route, credential, remote_key, None, None, now)
}

fn signed_object_request(
    method: &str,
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
    content_type: Option<&str>,
    cache_control: Option<&str>,
    now: OffsetDateTime,
) -> CommandResult<SignedObjectRequest> {
    let secret_id = required_secret(credential.access_key_id.as_deref(), "SecretId")?;
    let secret_key = required_secret(credential.secret_access_key.as_deref(), "SecretKey")?;
    let endpoint = endpoint(route)?;
    let bucket = required_route_field(route.bucket.as_deref(), "Bucket")?;
    let (url, host, canonical_uri) = object_url(&endpoint, bucket, remote_key)?;
    let token = credential
        .session_token
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let key_time = key_time(now);
    let signed_headers = signed_headers(&host, content_type, cache_control, token);
    let authorization = cos_authorization(
        secret_id,
        secret_key,
        method,
        &canonical_uri,
        &signed_headers,
        &key_time,
    )?;

    let mut headers = HeaderMap::new();
    headers.insert(HOST, header_value(&host)?);
    headers.insert(AUTHORIZATION, header_value(&authorization)?);

    if let Some(content_type) = content_type {
        headers.insert(CONTENT_TYPE, header_value(content_type)?);
    }

    if let Some(cache_control) = cache_control {
        headers.insert(CACHE_CONTROL, header_value(cache_control)?);
    }

    if let Some(token) = token {
        headers.insert(
            HeaderName::from_static("x-cos-security-token"),
            header_value(token)?,
        );
    }

    Ok(SignedObjectRequest { url, headers })
}

fn endpoint(route: &Route) -> CommandResult<String> {
    if let Some(endpoint) = route
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(endpoint.to_string());
    }

    let region = required_route_field(route.region.as_deref(), "Region")?;
    Ok(format!("https://{{bucket}}.cos.{}.myqcloud.com", region))
}

fn object_url(
    endpoint: &str,
    bucket: &str,
    remote_key: &str,
) -> CommandResult<(String, String, String)> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    let encoded_bucket = uri_encode_path(bucket);
    let encoded_key = uri_encode_path(remote_key);
    let virtual_host_endpoint = endpoint.replace("{bucket}", &encoded_bucket);
    let parsed = Url::parse(&virtual_host_endpoint).map_err(upload_error)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| SafeError::new("upload_failed", "Endpoint 缺少主机名。", true, false))?
        .to_string();

    if endpoint.contains("{bucket}") {
        let canonical_uri = format!("/{}", encoded_key);
        return Ok((
            format!("{}{}", virtual_host_endpoint, canonical_uri),
            host,
            canonical_uri,
        ));
    }

    let canonical_uri = format!("/{}/{}", encoded_bucket, encoded_key);
    Ok((
        format!("{}{}", endpoint, canonical_uri),
        host,
        canonical_uri,
    ))
}

fn signed_headers(
    host: &str,
    content_type: Option<&str>,
    cache_control: Option<&str>,
    token: Option<&str>,
) -> Vec<(String, String)> {
    let mut headers = vec![("host".to_string(), host.trim().to_ascii_lowercase())];

    if let Some(cache_control) = cache_control {
        headers.push((
            "cache-control".to_string(),
            cache_control.trim().to_string(),
        ));
    }

    if let Some(content_type) = content_type {
        headers.push(("content-type".to_string(), content_type.trim().to_string()));
    }

    if let Some(token) = token {
        headers.push(("x-cos-security-token".to_string(), token.trim().to_string()));
    }

    headers.sort_by(|left, right| left.0.cmp(&right.0));
    headers
}

fn cos_authorization(
    secret_id: &str,
    secret_key: &str,
    method: &str,
    canonical_uri: &str,
    signed_headers: &[(String, String)],
    key_time: &str,
) -> CommandResult<String> {
    let header_list = signed_headers
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let http_headers = signed_headers
        .iter()
        .map(|(name, value)| format!("{}={}", percent_encode(name), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&");
    let http_string = format!("{}\n{}\n\n{}\n", method, canonical_uri, http_headers);
    let string_to_sign = format!("sha1\n{}\n{}\n", key_time, sha1_hex(http_string.as_bytes()));
    let sign_key = hmac_sha1_hex(secret_key.as_bytes(), key_time.as_bytes())?;
    let signature = hmac_sha1_hex(sign_key.as_bytes(), string_to_sign.as_bytes())?;

    Ok(format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list={}&q-url-param-list=&q-signature={}",
        secret_id, key_time, key_time, header_list, signature
    ))
}

fn key_time(now: OffsetDateTime) -> String {
    let start = now.unix_timestamp();
    format!("{};{}", start, start + SIGNATURE_TTL_SECONDS)
}

fn public_url_for_key(route: &Route, remote_key: &str) -> String {
    let key = remote_key
        .trim_start_matches('/')
        .split('/')
        .map(uri_encode_segment)
        .collect::<Vec<_>>()
        .join("/");

    format!("{}/{}", route.public_base_url.trim_end_matches('/'), key)
}

fn hmac_sha1_hex(key: &[u8], message: &[u8]) -> CommandResult<String> {
    let mut mac = HmacSha1::new_from_slice(key).map_err(upload_error)?;
    mac.update(message);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn sha1_hex(value: &[u8]) -> String {
    let digest = Sha1::digest(value);
    hex::encode(digest)
}

fn uri_encode_path(value: &str) -> String {
    value
        .trim_start_matches('/')
        .split('/')
        .map(uri_encode_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn uri_encode_segment(value: &str) -> String {
    let mut output = String::new();

    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(*byte as char)
            }
            _ => output.push_str(&format!("%{:02X}", byte)),
        }
    }

    output
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();

    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte.to_ascii_lowercase() as char)
            }
            _ => output.push_str(&format!("%{:02x}", byte)),
        }
    }

    output
}

fn required_secret<'a>(value: Option<&'a str>, label: &str) -> CommandResult<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "credential_missing",
                format!("腾讯云 COS 上传目标缺少 {}。", label),
                true,
                false,
            )
        })
}

fn required_route_field<'a>(value: Option<&'a str>, label: &str) -> CommandResult<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "upload_failed",
                format!("腾讯云 COS 上传目标缺少 {}。", label),
                true,
                false,
            )
        })
}

fn header_value(value: &str) -> CommandResult<HeaderValue> {
    HeaderValue::from_str(value).map_err(upload_error)
}

fn network_error(error: impl std::fmt::Display) -> SafeError {
    SafeError::new(
        "network_timeout",
        format!("上传网络请求失败：{}", error),
        true,
        true,
    )
}

fn upload_error(error: impl std::fmt::Display) -> SafeError {
    SafeError::new(
        "upload_failed",
        format!("腾讯云 COS 上传准备失败：{}", error),
        true,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn route() -> Route {
        Route {
            id: "blog-cos".to_string(),
            name: "Blog CDN · COS".to_string(),
            provider: "tencent_cos".to_string(),
            credential_ref: "keychain://picdog/blog-cos".to_string(),
            endpoint: Some("https://{bucket}.cos.ap-shanghai.myqcloud.com".to_string()),
            bucket: Some("assets-1250000000".to_string()),
            region: Some("ap-shanghai".to_string()),
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

    fn credential() -> RouteCredentialInput {
        RouteCredentialInput {
            access_key_id: Some("AKID_TEST".to_string()),
            secret_access_key: Some("secret".to_string()),
            session_token: None,
            api_token: None,
        }
    }

    #[test]
    fn builds_virtual_host_style_object_url() {
        let (url, host, canonical_uri) = object_url(
            "https://{bucket}.cos.ap-shanghai.myqcloud.com",
            "assets-1250000000",
            "2026/10/产品 图.webp",
        )
        .unwrap();

        assert_eq!(
            url,
            "https://assets-1250000000.cos.ap-shanghai.myqcloud.com/2026/10/%E4%BA%A7%E5%93%81%20%E5%9B%BE.webp"
        );
        assert_eq!(host, "assets-1250000000.cos.ap-shanghai.myqcloud.com");
        assert_eq!(
            canonical_uri,
            "/2026/10/%E4%BA%A7%E5%93%81%20%E5%9B%BE.webp"
        );
    }

    #[test]
    fn builds_default_endpoint_from_region() {
        let mut route = route();
        route.endpoint = None;
        let value = endpoint(&route).unwrap();

        assert_eq!(value, "https://{bucket}.cos.ap-shanghai.myqcloud.com");
    }

    #[test]
    fn signs_put_object_with_cos_authorization() {
        let mut route = route();
        route.cache_control = "public, max-age=3600, stale-while-revalidate=60".to_string();
        let signed = sign_put_object(
            &route,
            &credential(),
            "2026/10/hero.webp",
            "image/webp",
            datetime!(2026-10-11 12:00:00 UTC),
        )
        .unwrap();
        let authorization = signed
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .unwrap();

        assert!(authorization.contains("q-sign-algorithm=sha1"));
        assert!(authorization.contains("q-ak=AKID_TEST"));
        assert!(authorization.contains("q-key-time=1791720000;1791720600"));
        assert!(authorization.contains("q-header-list=cache-control;content-type;host"));
        assert_eq!(
            signed.headers.get(CACHE_CONTROL).unwrap().to_str().unwrap(),
            "public, max-age=3600, stale-while-revalidate=60"
        );
    }

    #[test]
    fn signs_delete_object_with_temporary_token_header() {
        let mut credential = credential();
        credential.session_token = Some("token value".to_string());
        let signed = sign_delete_object(
            &route(),
            &credential,
            "2026/10/hero.webp",
            datetime!(2026-10-11 12:00:00 UTC),
        )
        .unwrap();

        assert!(signed
            .headers
            .get(AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("q-header-list=host;x-cos-security-token"));
        assert_eq!(
            signed
                .headers
                .get(HeaderName::from_static("x-cos-security-token"))
                .unwrap()
                .to_str()
                .unwrap(),
            "token value"
        );
    }

    #[test]
    fn public_url_encodes_segments() {
        assert_eq!(
            public_url_for_key(&route(), "2026/10/产品 图.webp"),
            "https://cdn.picdog.app/2026/10/%E4%BA%A7%E5%93%81%20%E5%9B%BE.webp"
        );
    }
}
