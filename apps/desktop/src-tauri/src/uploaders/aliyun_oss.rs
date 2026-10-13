use super::{
    cache_control_for_route, http_client, response_status_is_retryable, UploadObjectInput,
    UploadObjectOutput,
};
use crate::error::{CommandResult, SafeError};
use crate::models::{Route, RouteCredentialInput};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::header::{
    HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, DATE, HOST,
};
use reqwest::StatusCode;
use sha1::Sha1;
use std::fs;
use time::{Month, OffsetDateTime, Weekday};
use url::Url;

type HmacSha1 = Hmac<Sha1>;

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
            format!("阿里云 OSS 上传失败，服务端返回 {}。", status),
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
        format!("阿里云 OSS 清理测试对象失败，服务端返回 {}。", status),
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
        "PUT",
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
    signed_object_request("DELETE", route, credential, remote_key, None, None, now)
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
    let access_key = required_secret(credential.access_key_id.as_deref(), "Access Key")?;
    let secret_key = required_secret(credential.secret_access_key.as_deref(), "Secret Key")?;
    let endpoint = required_route_field(route.endpoint.as_deref(), "Endpoint")?;
    let bucket = required_route_field(route.bucket.as_deref(), "Bucket")?;
    let (url, host, canonical_resource) = object_url(endpoint, bucket, remote_key)?;
    let date = http_date(now);
    let token = credential
        .session_token
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let oss_headers = canonical_oss_headers(token);
    let string_to_sign = format!(
        "{}\n\n{}\n{}\n{}{}",
        method,
        content_type.unwrap_or(""),
        date,
        oss_headers,
        canonical_resource
    );
    let authorization = format!(
        "OSS {}:{}",
        access_key,
        oss_signature(secret_key, &string_to_sign)?
    );

    let mut headers = HeaderMap::new();
    headers.insert(HOST, header_value(&host)?);
    headers.insert(DATE, header_value(&date)?);
    headers.insert(AUTHORIZATION, header_value(&authorization)?);

    if let Some(content_type) = content_type {
        headers.insert(CONTENT_TYPE, header_value(content_type)?);
    }

    if let Some(cache_control) = cache_control {
        headers.insert(CACHE_CONTROL, header_value(cache_control)?);
    }

    if let Some(token) = token {
        headers.insert(
            HeaderName::from_static("x-oss-security-token"),
            header_value(token)?,
        );
    }

    Ok(SignedObjectRequest { url, headers })
}

fn canonical_oss_headers(token: Option<&str>) -> String {
    token
        .map(|token| format!("x-oss-security-token:{}\n", token.trim()))
        .unwrap_or_default()
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

    let url = if endpoint.contains("{bucket}") {
        format!("{}/{}", virtual_host_endpoint, encoded_key)
    } else {
        format!("{}/{}/{}", endpoint, encoded_bucket, encoded_key)
    };
    let canonical_resource = format!("/{}/{}", encoded_bucket, encoded_key);

    Ok((url, host, canonical_resource))
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

fn oss_signature(secret_key: &str, string_to_sign: &str) -> CommandResult<String> {
    let mut mac = HmacSha1::new_from_slice(secret_key.as_bytes()).map_err(upload_error)?;
    mac.update(string_to_sign.as_bytes());
    Ok(STANDARD.encode(mac.finalize().into_bytes()))
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

fn http_date(now: OffsetDateTime) -> String {
    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        weekday_name(now.weekday()),
        now.day(),
        month_name(now.month()),
        now.year(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
}

fn month_name(month: Month) -> &'static str {
    match month {
        Month::January => "Jan",
        Month::February => "Feb",
        Month::March => "Mar",
        Month::April => "Apr",
        Month::May => "May",
        Month::June => "Jun",
        Month::July => "Jul",
        Month::August => "Aug",
        Month::September => "Sep",
        Month::October => "Oct",
        Month::November => "Nov",
        Month::December => "Dec",
    }
}

fn required_secret<'a>(value: Option<&'a str>, label: &str) -> CommandResult<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "credential_missing",
                format!("阿里云 OSS 上传目标缺少 {}。", label),
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
                format!("阿里云 OSS 上传目标缺少 {}。", label),
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
        format!("阿里云 OSS 上传准备失败：{}", error),
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
            id: "blog-oss".to_string(),
            name: "Blog CDN · OSS".to_string(),
            provider: "aliyun_oss".to_string(),
            credential_ref: "keychain://picdog/blog-oss".to_string(),
            endpoint: Some("https://oss-cn-hangzhou.aliyuncs.com".to_string()),
            bucket: Some("assets".to_string()),
            region: Some("cn-hangzhou".to_string()),
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
            access_key_id: Some("LTAI_TEST".to_string()),
            secret_access_key: Some("secret".to_string()),
            session_token: None,
            api_token: None,
        }
    }

    #[test]
    fn builds_path_style_object_url_and_canonical_resource() {
        let (url, host, resource) = object_url(
            "https://oss-cn-hangzhou.aliyuncs.com",
            "assets",
            "2026/10/产品 图.webp",
        )
        .unwrap();

        assert_eq!(
            url,
            "https://oss-cn-hangzhou.aliyuncs.com/assets/2026/10/%E4%BA%A7%E5%93%81%20%E5%9B%BE.webp"
        );
        assert_eq!(host, "oss-cn-hangzhou.aliyuncs.com");
        assert_eq!(
            resource,
            "/assets/2026/10/%E4%BA%A7%E5%93%81%20%E5%9B%BE.webp"
        );
    }

    #[test]
    fn builds_virtual_host_url_with_bucket_placeholder() {
        let (url, host, resource) = object_url(
            "https://{bucket}.oss-cn-hangzhou.aliyuncs.com",
            "assets",
            "2026/10/hero.webp",
        )
        .unwrap();

        assert_eq!(
            url,
            "https://assets.oss-cn-hangzhou.aliyuncs.com/2026/10/hero.webp"
        );
        assert_eq!(host, "assets.oss-cn-hangzhou.aliyuncs.com");
        assert_eq!(resource, "/assets/2026/10/hero.webp");
    }

    #[test]
    fn signs_put_object_with_oss_authorization() {
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

        assert!(authorization.starts_with("OSS LTAI_TEST:"));
        assert_eq!(
            signed.headers.get(DATE).unwrap().to_str().unwrap(),
            "Sun, 11 Oct 2026 12:00:00 GMT"
        );
        assert_eq!(
            signed.headers.get(CACHE_CONTROL).unwrap().to_str().unwrap(),
            "public, max-age=3600, stale-while-revalidate=60"
        );
    }

    #[test]
    fn signs_delete_object_with_sts_token_header() {
        let mut credential = credential();
        credential.session_token = Some("sts-token".to_string());
        let signed = sign_delete_object(
            &route(),
            &credential,
            "2026/10/hero.webp",
            datetime!(2026-10-11 12:00:00 UTC),
        )
        .unwrap();

        assert_eq!(
            signed
                .headers
                .get(HeaderName::from_static("x-oss-security-token"))
                .unwrap()
                .to_str()
                .unwrap(),
            "sts-token"
        );
        assert!(signed
            .headers
            .get(AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("OSS LTAI_TEST:"));
    }

    #[test]
    fn public_url_encodes_segments() {
        assert_eq!(
            public_url_for_key(&route(), "2026/10/产品 图.webp"),
            "https://cdn.picdog.app/2026/10/%E4%BA%A7%E5%93%81%20%E5%9B%BE.webp"
        );
    }
}
