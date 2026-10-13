use super::{
    cache_control_for_route, http_client, response_status_is_retryable, UploadObjectInput,
    UploadObjectOutput,
};
use crate::error::{CommandResult, SafeError};
use crate::models::{Route, RouteCredentialInput};
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION, CACHE_CONTROL, HOST};
use reqwest::StatusCode;
use sha2::{Digest, Sha256};
use std::fs;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;
use url::Url;

type HmacSha256 = Hmac<Sha256>;

const AMZ_DATE_FORMAT: &[FormatItem<'_>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");
const DATE_SCOPE_FORMAT: &[FormatItem<'_>] = format_description!("[year][month][day]");
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
        &body,
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
            format!("上传失败，服务端返回 {}。", status),
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
        format!("清理 S3 测试对象失败，服务端返回 {}。", status),
        true,
        response_status_is_retryable(status),
    ))
}

struct SignedPutObject {
    url: String,
    headers: HeaderMap,
}

struct SignedDeleteObject {
    url: String,
    headers: HeaderMap,
}

fn sign_put_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
    content_type: &str,
    body: &[u8],
    now: OffsetDateTime,
) -> CommandResult<SignedPutObject> {
    let access_key = required_secret(credential.access_key_id.as_deref(), "Access Key")?;
    let secret_key = required_secret(credential.secret_access_key.as_deref(), "Secret Key")?;
    let endpoint = required_route_field(route.endpoint.as_deref(), "Endpoint")?;
    let bucket = required_route_field(route.bucket.as_deref(), "Bucket")?;
    let region = route.region.as_deref().unwrap_or("auto");

    let date_scope = now.format(DATE_SCOPE_FORMAT).map_err(upload_error)?;
    let amz_date = now.format(AMZ_DATE_FORMAT).map_err(upload_error)?;
    let payload_hash = sha256_hex(body);
    let (url, host, canonical_uri) = object_url(endpoint, bucket, remote_key)?;
    let token = credential
        .session_token
        .as_deref()
        .filter(|value| !value.trim().is_empty());

    let (canonical_headers, signed_headers) = canonical_headers(
        &host,
        &payload_hash,
        &amz_date,
        Some(content_type),
        Some(cache_control_for_route(route)),
        token,
    );
    let canonical_request = format!(
        "PUT\n{}\n\n{}\n{}\n{}",
        canonical_uri, canonical_headers, signed_headers, payload_hash
    );
    let credential_scope = format!("{}/{}/s3/aws4_request", date_scope, region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );
    let signature = signature_hex(secret_key, &date_scope, region, &string_to_sign)?;
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        access_key, credential_scope, signed_headers, signature
    );

    let mut headers = HeaderMap::new();
    headers.insert(HOST, header_value(&host)?);
    headers.insert(
        HeaderName::from_static("content-type"),
        header_value(content_type)?,
    );
    headers.insert(CACHE_CONTROL, header_value(cache_control_for_route(route))?);
    headers.insert(
        HeaderName::from_static("x-amz-content-sha256"),
        header_value(&payload_hash)?,
    );
    headers.insert(
        HeaderName::from_static("x-amz-date"),
        header_value(&amz_date)?,
    );
    headers.insert(AUTHORIZATION, header_value(&authorization)?);

    if let Some(token) = token {
        headers.insert(
            HeaderName::from_static("x-amz-security-token"),
            header_value(token)?,
        );
    }

    Ok(SignedPutObject { url, headers })
}

fn sign_delete_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
    now: OffsetDateTime,
) -> CommandResult<SignedDeleteObject> {
    let access_key = required_secret(credential.access_key_id.as_deref(), "Access Key")?;
    let secret_key = required_secret(credential.secret_access_key.as_deref(), "Secret Key")?;
    let endpoint = required_route_field(route.endpoint.as_deref(), "Endpoint")?;
    let bucket = required_route_field(route.bucket.as_deref(), "Bucket")?;
    let region = route.region.as_deref().unwrap_or("auto");

    let date_scope = now.format(DATE_SCOPE_FORMAT).map_err(upload_error)?;
    let amz_date = now.format(AMZ_DATE_FORMAT).map_err(upload_error)?;
    let payload_hash = sha256_hex(b"");
    let (url, host, canonical_uri) = object_url(endpoint, bucket, remote_key)?;
    let token = credential
        .session_token
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    let (canonical_headers, signed_headers) =
        canonical_headers(&host, &payload_hash, &amz_date, None, None, token);
    let canonical_request = format!(
        "DELETE\n{}\n\n{}\n{}\n{}",
        canonical_uri, canonical_headers, signed_headers, payload_hash
    );
    let credential_scope = format!("{}/{}/s3/aws4_request", date_scope, region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );
    let signature = signature_hex(secret_key, &date_scope, region, &string_to_sign)?;
    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        access_key, credential_scope, signed_headers, signature
    );

    let mut headers = HeaderMap::new();
    headers.insert(HOST, header_value(&host)?);
    headers.insert(
        HeaderName::from_static("x-amz-content-sha256"),
        header_value(&payload_hash)?,
    );
    headers.insert(
        HeaderName::from_static("x-amz-date"),
        header_value(&amz_date)?,
    );
    headers.insert(AUTHORIZATION, header_value(&authorization)?);

    if let Some(token) = token {
        headers.insert(
            HeaderName::from_static("x-amz-security-token"),
            header_value(token)?,
        );
    }

    Ok(SignedDeleteObject { url, headers })
}

fn canonical_headers(
    host: &str,
    payload_hash: &str,
    amz_date: &str,
    content_type: Option<&str>,
    cache_control: Option<&str>,
    token: Option<&str>,
) -> (String, String) {
    let mut headers = vec![
        ("host", host),
        ("x-amz-content-sha256", payload_hash),
        ("x-amz-date", amz_date),
    ];

    if let Some(content_type) = content_type {
        headers.push(("content-type", content_type));
    }

    if let Some(cache_control) = cache_control {
        headers.push(("cache-control", cache_control));
    }

    if let Some(token) = token {
        headers.push(("x-amz-security-token", token));
    }

    headers.sort_by_key(|(name, _)| *name);

    let canonical = headers
        .iter()
        .map(|(name, value)| format!("{}:{}\n", name, value.trim()))
        .collect::<String>();
    let signed = headers
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(";");

    (canonical, signed)
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
        let url = format!("{}{}", virtual_host_endpoint, canonical_uri);
        return Ok((url, host, canonical_uri));
    }

    let canonical_uri = format!("/{}/{}", encoded_bucket, encoded_key);
    let url = format!("{}{}", endpoint, canonical_uri);

    Ok((url, host, canonical_uri))
}

fn public_url_for_key(route: &Route, remote_key: &str) -> String {
    format!(
        "{}/{}",
        route.public_base_url.trim_end_matches('/'),
        remote_key.trim_start_matches('/')
    )
}

fn signature_hex(
    secret_key: &str,
    date_scope: &str,
    region: &str,
    string_to_sign: &str,
) -> CommandResult<String> {
    let date_key = hmac_sha256(
        format!("AWS4{}", secret_key).as_bytes(),
        date_scope.as_bytes(),
    )?;
    let region_key = hmac_sha256(&date_key, region.as_bytes())?;
    let service_key = hmac_sha256(&region_key, b"s3")?;
    let signing_key = hmac_sha256(&service_key, b"aws4_request")?;
    let signature = hmac_sha256(&signing_key, string_to_sign.as_bytes())?;

    Ok(hex::encode(signature))
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> CommandResult<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(upload_error)?;
    mac.update(message);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn sha256_hex(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

fn uri_encode_path(value: &str) -> String {
    value
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

fn required_secret<'a>(value: Option<&'a str>, label: &str) -> CommandResult<&'a str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "credential_missing",
                format!("上传目标缺少 {}。", label),
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
                format!("上传目标缺少 {}。", label),
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
        format!("上传准备失败：{}", error),
        true,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_headers, object_url, public_url_for_key, sign_delete_object, sign_put_object,
        uri_encode_path,
    };
    use crate::models::{Route, RouteCredentialInput};
    use reqwest::header::{AUTHORIZATION, CACHE_CONTROL};
    use time::macros::datetime;

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
    fn encodes_s3_path_segments() {
        assert_eq!(
            uri_encode_path("2026/10/产品 图.webp"),
            "2026/10/%E4%BA%A7%E5%93%81%20%E5%9B%BE.webp"
        );
    }

    #[test]
    fn builds_path_style_object_url() {
        let (url, host, canonical_uri) = object_url(
            "https://example.r2.cloudflarestorage.com",
            "blog-assets",
            "2026/10/hero.webp",
        )
        .unwrap();

        assert_eq!(
            url,
            "https://example.r2.cloudflarestorage.com/blog-assets/2026/10/hero.webp"
        );
        assert_eq!(host, "example.r2.cloudflarestorage.com");
        assert_eq!(canonical_uri, "/blog-assets/2026/10/hero.webp");
    }

    #[test]
    fn builds_virtual_host_style_object_url_from_bucket_placeholder() {
        let (url, host, canonical_uri) = object_url(
            "https://{bucket}.s3.amazonaws.com",
            "assets",
            "2026/10/hero image.webp",
        )
        .unwrap();

        assert_eq!(
            url,
            "https://assets.s3.amazonaws.com/2026/10/hero%20image.webp"
        );
        assert_eq!(host, "assets.s3.amazonaws.com");
        assert_eq!(canonical_uri, "/2026/10/hero%20image.webp");
    }

    #[test]
    fn builds_public_url_from_route_base_url() {
        assert_eq!(
            public_url_for_key(&route(), "2026/10/hero.webp"),
            "https://cdn.picdog.app/2026/10/hero.webp"
        );
    }

    #[test]
    fn canonical_headers_are_sorted_and_signed() {
        let (headers, signed) = canonical_headers(
            "example.r2.cloudflarestorage.com",
            "abc",
            "20261011T120000Z",
            Some("image/webp"),
            Some(crate::models::default_route_cache_control().as_str()),
            None,
        );

        assert_eq!(
            headers,
            "cache-control:public, max-age=31536000, immutable\ncontent-type:image/webp\nhost:example.r2.cloudflarestorage.com\nx-amz-content-sha256:abc\nx-amz-date:20261011T120000Z\n"
        );
        assert_eq!(
            signed,
            "cache-control;content-type;host;x-amz-content-sha256;x-amz-date"
        );
    }

    #[test]
    fn signs_put_object_with_route_cache_control() {
        let mut route = route();
        route.cache_control = "public, max-age=3600, stale-while-revalidate=60".to_string();
        let credential = RouteCredentialInput {
            access_key_id: Some("AKIA_TEST".to_string()),
            secret_access_key: Some("secret".to_string()),
            session_token: None,
            api_token: None,
        };

        let signed = sign_put_object(
            &route,
            &credential,
            "2026/10/hero.webp",
            "image/webp",
            b"image",
            datetime!(2026-10-11 12:00:00 UTC),
        )
        .unwrap();

        assert_eq!(
            signed.headers.get(CACHE_CONTROL).unwrap().to_str().unwrap(),
            "public, max-age=3600, stale-while-revalidate=60"
        );
        assert!(signed
            .headers
            .get(AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap()
            .contains(
                "SignedHeaders=cache-control;content-type;host;x-amz-content-sha256;x-amz-date"
            ));
    }

    #[test]
    fn signs_delete_object_without_content_type() {
        let credential = RouteCredentialInput {
            access_key_id: Some("AKIA_TEST".to_string()),
            secret_access_key: Some("secret".to_string()),
            session_token: None,
            api_token: None,
        };
        let signed = sign_delete_object(
            &route(),
            &credential,
            "__picdog_route_test__/connection.txt",
            datetime!(2026-10-11 12:00:00 UTC),
        )
        .unwrap();
        let authorization = signed
            .headers
            .get(reqwest::header::AUTHORIZATION)
            .unwrap()
            .to_str()
            .unwrap();

        assert!(signed
            .url
            .ends_with("/blog-assets/__picdog_route_test__/connection.txt"));
        assert!(authorization.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
    }
}
