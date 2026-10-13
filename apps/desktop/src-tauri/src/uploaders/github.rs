use super::{http_client, response_status_is_retryable, UploadObjectInput, UploadObjectOutput};
use crate::error::{CommandResult, SafeError};
use crate::models::{Route, RouteCredentialInput};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::fs;

const DEFAULT_GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitHubRepo {
    api_base: String,
    owner: String,
    repo: String,
}

#[derive(Debug, Deserialize)]
struct GitHubContentResponse {
    sha: Option<String>,
}

pub fn upload_object(input: UploadObjectInput<'_>) -> CommandResult<UploadObjectOutput> {
    let body = fs::read(input.local_path).map_err(|_| {
        SafeError::new(
            "file_not_found",
            "找不到压缩后的本地文件，无法上传。",
            true,
            false,
        )
    })?;
    let repo = parse_repo(required_route_field(
        input.route.endpoint.as_deref(),
        "Repository",
    )?)?;
    let branch = github_branch(input.route);
    let token = required_token(input.credential)?;
    let remote_key = normalize_remote_key(input.remote_key)?;
    let client = http_client()?;
    let existing_sha = content_sha(&client, &repo, &token, &remote_key, &branch)?;
    let request_url = content_url(&repo, &remote_key);
    let mut payload = json!({
        "message": format!("Upload {}", remote_key),
        "content": base64_encode(&body),
        "branch": branch,
    });

    if let Some(sha) = existing_sha {
        payload["sha"] = json!(sha);
    }

    let response = client
        .put(request_url)
        .headers(github_headers(&token)?)
        .body(payload.to_string())
        .send()
        .map_err(network_error)?;

    ensure_success(response, "GitHub 上传失败")?;

    Ok(UploadObjectOutput {
        public_url: public_url_for_key(input.route, &remote_key, &repo, &branch),
    })
}

pub fn delete_object(
    route: &Route,
    credential: &RouteCredentialInput,
    remote_key: &str,
) -> CommandResult<()> {
    let repo = parse_repo(required_route_field(
        route.endpoint.as_deref(),
        "Repository",
    )?)?;
    let branch = github_branch(route);
    let token = required_token(credential)?;
    let remote_key = normalize_remote_key(remote_key)?;
    let client = http_client()?;
    let Some(sha) = content_sha(&client, &repo, &token, &remote_key, &branch)? else {
        return Ok(());
    };
    let payload = json!({
        "message": format!("Delete {}", remote_key),
        "sha": sha,
        "branch": branch,
    });
    let response = client
        .request(Method::DELETE, content_url(&repo, &remote_key))
        .headers(github_headers(&token)?)
        .body(payload.to_string())
        .send()
        .map_err(network_error)?;

    ensure_success(response, "GitHub 清理测试对象失败")
}

pub fn default_public_base_url(endpoint: &str, branch: Option<&str>) -> Option<String> {
    let repo = parse_repo(endpoint).ok()?;
    let branch = branch
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("main");

    if repo.api_base == DEFAULT_GITHUB_API_BASE {
        return Some(format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            repo.owner, repo.repo, branch
        ));
    }

    None
}

pub fn repository_is_valid(endpoint: &str) -> bool {
    parse_repo(endpoint).is_ok()
}

fn content_sha(
    client: &Client,
    repo: &GitHubRepo,
    token: &str,
    remote_key: &str,
    branch: &str,
) -> CommandResult<Option<String>> {
    let response = client
        .get(format!(
            "{}?ref={}",
            content_url(repo, remote_key),
            percent_encode(branch)
        ))
        .headers(github_headers(token)?)
        .send()
        .map_err(network_error)?;
    let status = response.status();

    if status == StatusCode::NOT_FOUND {
        return Ok(None);
    }

    if !status.is_success() {
        return Err(SafeError::new(
            "upload_failed",
            format!("GitHub 查询文件失败，服务端返回 {}。", status),
            true,
            response_status_is_retryable(status),
        ));
    }

    response
        .text()
        .map_err(network_error)
        .and_then(|body| {
            serde_json::from_str::<GitHubContentResponse>(&body).map_err(|_| {
                SafeError::new("upload_failed", "GitHub 返回内容无法解析。", true, true)
            })
        })
        .map(|content| content.sha)
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

fn github_headers(token: &str) -> CommandResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("PicDog"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        HeaderNameCompat::github_api_version(),
        HeaderValue::from_static(GITHUB_API_VERSION),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(AUTHORIZATION, header_value(&format!("Bearer {}", token))?);
    Ok(headers)
}

struct HeaderNameCompat;

impl HeaderNameCompat {
    fn github_api_version() -> reqwest::header::HeaderName {
        reqwest::header::HeaderName::from_static("x-github-api-version")
    }
}

fn header_value(value: &str) -> CommandResult<HeaderValue> {
    HeaderValue::from_str(value)
        .map_err(|_| SafeError::new("upload_failed", "GitHub Token 格式无效。", true, false))
}

fn parse_repo(value: &str) -> CommandResult<GitHubRepo> {
    let trimmed = value.trim().trim_end_matches(".git");
    if trimmed.is_empty() {
        return Err(invalid_repo_error());
    }

    if let Some(path) = trimmed.strip_prefix("git@github.com:") {
        return repo_from_path(DEFAULT_GITHUB_API_BASE, path);
    }

    if let Ok(url) = url::Url::parse(trimmed) {
        let segments = url
            .path_segments()
            .map(|segments| segments.collect::<Vec<_>>())
            .unwrap_or_default();
        if segments.len() >= 3 && segments[segments.len() - 3] == "repos" {
            let api_prefix = segments[..segments.len() - 3].join("/");
            let api_base = if api_prefix.is_empty() {
                url.origin().ascii_serialization()
            } else {
                format!("{}/{}", url.origin().ascii_serialization(), api_prefix)
            };
            let repo = repo_from_owner_repo(
                api_base,
                segments[segments.len() - 2],
                segments[segments.len() - 1],
            )?;
            return Ok(repo);
        }

        if segments.len() >= 2 {
            let api_base = if url.host_str() == Some("github.com") {
                DEFAULT_GITHUB_API_BASE.to_string()
            } else {
                format!("{}{}", url.origin().ascii_serialization(), "/api/v3")
            };
            return repo_from_owner_repo(api_base, segments[0], segments[1]);
        }
    }

    repo_from_path(DEFAULT_GITHUB_API_BASE, trimmed)
}

fn repo_from_path(api_base: &str, path: &str) -> CommandResult<GitHubRepo> {
    let cleaned = path.trim_matches('/');
    let parts = cleaned.split('/').collect::<Vec<_>>();

    if parts.len() < 2 {
        return Err(invalid_repo_error());
    }

    repo_from_owner_repo(api_base.to_string(), parts[0], parts[1])
}

fn repo_from_owner_repo(api_base: String, owner: &str, repo: &str) -> CommandResult<GitHubRepo> {
    let owner = owner.trim();
    let repo = repo.trim().trim_end_matches(".git");

    if owner.is_empty() || repo.is_empty() {
        return Err(invalid_repo_error());
    }

    Ok(GitHubRepo {
        api_base: api_base.trim_end_matches('/').to_string(),
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

fn invalid_repo_error() -> SafeError {
    SafeError::new(
        "upload_failed",
        "GitHub 仓库格式应为 owner/repo 或仓库 URL。",
        true,
        false,
    )
}

fn github_branch(route: &Route) -> String {
    route
        .bucket
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("main")
        .to_string()
}

fn required_token(credential: &RouteCredentialInput) -> CommandResult<&str> {
    credential
        .api_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SafeError::new("credential_missing", "GitHub Token 未配置。", true, false))
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

fn normalize_remote_key(remote_key: &str) -> CommandResult<String> {
    let cleaned = remote_key.trim().trim_start_matches('/');

    if cleaned.is_empty()
        || cleaned
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(SafeError::new(
            "upload_failed",
            "GitHub 文件路径无效。",
            true,
            false,
        ));
    }

    Ok(cleaned.to_string())
}

fn content_url(repo: &GitHubRepo, remote_key: &str) -> String {
    format!(
        "{}/repos/{}/{}/contents/{}",
        repo.api_base,
        percent_encode(&repo.owner),
        percent_encode(&repo.repo),
        percent_encode_path(remote_key)
    )
}

fn public_url_for_key(route: &Route, remote_key: &str, repo: &GitHubRepo, branch: &str) -> String {
    if route.public_base_url.trim().is_empty() && repo.api_base == DEFAULT_GITHUB_API_BASE {
        return format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            repo.owner,
            repo.repo,
            percent_encode(branch),
            percent_encode_path(remote_key)
        );
    }

    format!(
        "{}/{}",
        route.public_base_url.trim().trim_end_matches('/'),
        percent_encode_path(remote_key)
    )
}

fn percent_encode_path(value: &str) -> String {
    value
        .split('/')
        .map(percent_encode)
        .collect::<Vec<_>>()
        .join("/")
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);

        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }

        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }

    output
}

fn network_error(error: reqwest::Error) -> SafeError {
    if error.is_timeout() {
        return SafeError::new(
            "network_timeout",
            "GitHub 上传网络超时，请稍后重试。",
            true,
            true,
        );
    }

    SafeError::new(
        "upload_failed",
        format!("GitHub 上传请求失败：{}。", error),
        true,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{base64_encode, default_public_base_url, normalize_remote_key, parse_repo};

    #[test]
    fn parses_github_repository_inputs() {
        let repo = parse_repo("owner/repo").unwrap();
        assert_eq!(repo.api_base, "https://api.github.com");
        assert_eq!(repo.owner, "owner");
        assert_eq!(repo.repo, "repo");

        let repo = parse_repo("https://github.com/picdog/assets.git").unwrap();
        assert_eq!(repo.owner, "picdog");
        assert_eq!(repo.repo, "assets");

        let repo = parse_repo("https://ghe.example.com/api/v3/repos/acme/cdn").unwrap();
        assert_eq!(repo.api_base, "https://ghe.example.com/api/v3");
        assert_eq!(repo.owner, "acme");
        assert_eq!(repo.repo, "cdn");
    }

    #[test]
    fn builds_default_public_base_url_for_github_com() {
        assert_eq!(
            default_public_base_url("owner/repo", Some("assets")).unwrap(),
            "https://raw.githubusercontent.com/owner/repo/assets"
        );
    }

    #[test]
    fn validates_remote_key_segments() {
        assert_eq!(
            normalize_remote_key("/2026/10/a.png").unwrap(),
            "2026/10/a.png"
        );
        assert!(normalize_remote_key("../a.png").is_err());
        assert!(normalize_remote_key("2026//a.png").is_err());
    }

    #[test]
    fn encodes_base64_payload() {
        assert_eq!(base64_encode(b"PicDog"), "UGljRG9n");
        assert_eq!(base64_encode(b"hi"), "aGk=");
        assert_eq!(base64_encode(b"x"), "eA==");
    }
}
