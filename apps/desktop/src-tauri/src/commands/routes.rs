use crate::credentials;
use crate::error::CommandResult;
use crate::models::{
    default_route_cache_control, now_string, Route, RouteCredentialInput, RouteCredentialStatus,
    RouteDraft, RouteTestResult,
};
use crate::state::AppState;
use crate::uploaders;
use tauri::State;

#[tauri::command]
pub async fn list_routes(state: State<'_, AppState>) -> CommandResult<Vec<Route>> {
    state.list_routes()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn select_directory(title: Option<String>) -> CommandResult<Option<String>> {
    let dialog_title = title
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Select folder");

    Ok(rfd::FileDialog::new()
        .set_title(dialog_title)
        .pick_folder()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_route(state: State<'_, AppState>, draft: RouteDraft) -> CommandResult<Route> {
    let route = route_from_draft(&state, draft)?;
    state.save_route(&route)?;

    Ok(route)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn delete_route(
    state: State<'_, AppState>,
    route_id: String,
) -> CommandResult<Vec<Route>> {
    let route = state.get_route(&route_id)?;

    credentials::delete_route_credential(&route.credential_ref)?;
    state.delete_route(&route_id)?;

    let remaining_routes = state.list_routes()?;
    let mut settings = state.get_settings()?;
    if settings.default_route_id.as_deref() == Some(route_id.as_str()) {
        settings.default_route_id = remaining_routes.first().map(|route| route.id.clone());
        state.save_settings(&settings)?;
    }

    Ok(remaining_routes)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_route_credential(
    state: State<'_, AppState>,
    route_id: String,
    credential: RouteCredentialInput,
) -> CommandResult<RouteCredentialStatus> {
    let route = state.get_route(&route_id)?;
    credentials::save_route_credential(&route.credential_ref, &credential)?;

    credential_status_for_route(&route)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_route_credential_status(
    state: State<'_, AppState>,
    route_id: String,
) -> CommandResult<RouteCredentialStatus> {
    let route = state.get_route(&route_id)?;
    credential_status_for_route(&route)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn delete_route_credential(
    state: State<'_, AppState>,
    route_id: String,
) -> CommandResult<RouteCredentialStatus> {
    let route = state.get_route(&route_id)?;
    credentials::delete_route_credential(&route.credential_ref)?;

    credential_status_for_route(&route)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn test_route(
    state: State<'_, AppState>,
    route_id: String,
) -> CommandResult<RouteTestResult> {
    let route = state.get_route(&route_id)?;
    let (ok, message_code, message, public_base_url) = match load_route_test_credential(&route) {
        Ok(credential) => match uploaders::test_route(&route, &credential) {
            Ok(_) => (
                true,
                Some("route_test_success".to_string()),
                "上传目标连接成功，已完成一次测试上传。".to_string(),
                Some(route.public_base_url.clone()),
            ),
            Err(error) => (
                false,
                Some(error.code.clone()),
                error.message,
                Some(route.public_base_url.clone()),
            ),
        },
        Err(error) if error.code == "credential_missing" => (
            false,
            Some(error.code),
            "上传目标还没有保存凭据。".to_string(),
            None,
        ),
        Err(error) => (false, Some(error.code.clone()), error.message, None),
    };

    Ok(RouteTestResult {
        route_id: route.id.clone(),
        ok,
        message_code,
        message,
        checked_at: now_string(),
        public_base_url,
    })
}

fn load_route_test_credential(route: &Route) -> CommandResult<RouteCredentialInput> {
    match credentials::load_route_credential(&route.credential_ref) {
        Ok(credential) => Ok(credential),
        Err(error)
            if error.code == "credential_missing"
                && matches!(route.provider.as_str(), "custom_api" | "local_folder") =>
        {
            Ok(RouteCredentialInput::default())
        }
        Err(error) => Err(error),
    }
}

fn route_from_draft(state: &State<'_, AppState>, draft: RouteDraft) -> CommandResult<Route> {
    validate_route_draft(&draft)?;

    let id = draft
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(safe_route_id)
        .unwrap_or_else(|| {
            let from_name = safe_route_id(&draft.name);
            if from_name.is_empty() {
                format!("route-{}", now_string())
            } else {
                from_name
            }
        });

    let existing = state.get_route(&id).ok();
    let now = now_string();
    let credential_ref = existing
        .as_ref()
        .map(|route| route.credential_ref.clone())
        .unwrap_or_else(|| credentials::credential_ref_for_route(&id));

    let public_base_url = normalized_public_base_url(
        &draft.provider,
        draft.endpoint.as_deref(),
        draft.bucket.as_deref(),
        &draft.public_base_url,
    );

    let route = Route {
        id,
        name: draft.name.trim().to_string(),
        provider: draft.provider,
        credential_ref,
        endpoint: empty_to_none(draft.endpoint),
        bucket: empty_to_none(draft.bucket),
        region: empty_to_none(draft.region),
        path_template: non_empty_or(draft.path_template, "{yyyy}/{mm}/{hash}-{name}.{ext}"),
        public_base_url,
        cache_control: non_empty_or(draft.cache_control, &default_route_cache_control()),
        default_copy_format: non_empty_or(draft.default_copy_format, "markdown"),
        custom_link_template: empty_to_none(draft.custom_link_template),
        custom_api_method: normalized_custom_api_method(draft.custom_api_method),
        custom_api_body: normalized_custom_api_body(draft.custom_api_body),
        custom_api_file_field: empty_to_none(draft.custom_api_file_field),
        custom_api_headers: empty_to_none(draft.custom_api_headers),
        custom_api_url_path: empty_to_none(draft.custom_api_url_path),
        concurrency: draft.concurrency.clamp(1, 8),
        created_at: existing
            .as_ref()
            .map(|route| route.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    };

    if let Some(credential) = draft.credential.as_ref() {
        credentials::save_route_credential(&route.credential_ref, credential)?;
    }

    Ok(route)
}

fn validate_route_draft(draft: &RouteDraft) -> CommandResult<()> {
    if draft.name.trim().is_empty() {
        return Err(crate::error::SafeError::new(
            "route_name_required",
            "上传目标名称不能为空。",
            true,
            false,
        ));
    }

    if draft.public_base_url.trim().is_empty() && draft.provider != "github" {
        return Err(crate::error::SafeError::new(
            "route_public_base_url_required",
            "上传目标缺少 CDN 域名。",
            true,
            false,
        ));
    }

    match draft.provider.as_str() {
        "aliyun_oss" | "s3" | "tencent_cos" => {
            require_draft_field(
                draft.endpoint.as_deref(),
                "route_endpoint_required",
                "Endpoint",
            )?;
            require_draft_field(draft.bucket.as_deref(), "route_bucket_required", "Bucket")?;
        }
        "qiniu" => {
            require_draft_field(draft.bucket.as_deref(), "route_bucket_required", "Bucket")?;
        }
        "custom_api" => {
            require_draft_field(
                draft.endpoint.as_deref(),
                "route_endpoint_required",
                "Endpoint",
            )?;
            validate_custom_api_method(draft.custom_api_method.as_deref())?;
            validate_custom_api_body(draft.custom_api_body.as_deref())?;
        }
        "local_folder" => {
            require_draft_field(
                draft.endpoint.as_deref(),
                "route_local_folder_required",
                "本地目录",
            )?;
        }
        "github" => {
            require_draft_field(
                draft.endpoint.as_deref(),
                "route_repository_required",
                "Repository",
            )?;
            let endpoint = draft.endpoint.as_deref().unwrap_or("").trim();
            if !uploaders::github::repository_is_valid(endpoint) {
                return Err(crate::error::SafeError::new(
                    "github_repository_invalid",
                    "GitHub 仓库格式应为 owner/repo 或仓库 URL。",
                    true,
                    false,
                ));
            }
            if draft.public_base_url.trim().is_empty()
                && uploaders::github::default_public_base_url(endpoint, draft.bucket.as_deref())
                    .is_none()
            {
                return Err(crate::error::SafeError::new(
                    "github_enterprise_base_url_required",
                    "GitHub Enterprise 目标需要填写 Raw / CDN 域名。",
                    true,
                    false,
                ));
            }
        }
        _ => {
            return Err(crate::error::SafeError::new(
                "route_provider_unsupported",
                "暂不支持该上传目标类型。",
                true,
                false,
            ));
        }
    }

    Ok(())
}

fn require_draft_field(value: Option<&str>, code: &str, label: &str) -> CommandResult<()> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|_| ())
        .ok_or_else(|| {
            crate::error::SafeError::new(code, format!("上传目标缺少 {}。", label), true, false)
        })
}

fn credential_status_for_route(route: &Route) -> CommandResult<RouteCredentialStatus> {
    if route.provider == "local_folder" {
        return Ok(RouteCredentialStatus {
            route_id: route.id.clone(),
            credential_ref: route.credential_ref.clone(),
            configured: true,
            checked_at: now_string(),
        });
    }

    Ok(RouteCredentialStatus {
        route_id: route.id.clone(),
        credential_ref: route.credential_ref.clone(),
        configured: credentials::credential_configured(&route.credential_ref)?,
        checked_at: now_string(),
    })
}

fn validate_custom_api_method(value: Option<&str>) -> CommandResult<()> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some("POST" | "post" | "PUT" | "put" | "PATCH" | "patch") | None => Ok(()),
        _ => Err(crate::error::SafeError::new(
            "custom_api_method_invalid",
            "Custom API 请求方法只支持 POST、PUT 或 PATCH。",
            true,
            false,
        )),
    }
}

fn validate_custom_api_body(value: Option<&str>) -> CommandResult<()> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some("binary" | "multipart") | None => Ok(()),
        _ => Err(crate::error::SafeError::new(
            "custom_api_body_invalid",
            "Custom API 请求体只支持 binary 或 multipart。",
            true,
            false,
        )),
    }
}

fn normalized_custom_api_method(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| !value.is_empty())
}

fn normalized_custom_api_body(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

fn empty_to_none(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn non_empty_or(value: String, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn normalized_public_base_url(
    provider: &str,
    endpoint: Option<&str>,
    branch: Option<&str>,
    public_base_url: &str,
) -> String {
    let trimmed = public_base_url.trim().trim_end_matches('/');
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    if provider == "github" {
        return endpoint
            .and_then(|endpoint| uploaders::github::default_public_base_url(endpoint, branch))
            .unwrap_or_default();
    }

    String::new()
}

fn safe_route_id(value: &str) -> String {
    let mut output = String::new();

    for ch in value.trim().chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => output.push(ch.to_ascii_lowercase()),
            '-' | '_' => output.push(ch),
            _ => {
                if !output.ends_with('-') {
                    output.push('-');
                }
            }
        }
    }

    output.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::{normalized_public_base_url, safe_route_id, validate_route_draft};
    use crate::models::{default_route_cache_control, RouteDraft};

    #[test]
    fn creates_safe_route_ids_from_names() {
        assert_eq!(safe_route_id("Blog CDN · R2"), "blog-cdn-r2");
        assert_eq!(safe_route_id("  docs_oss  "), "docs_oss");
        assert_eq!(safe_route_id("中文 图床"), "");
    }

    fn route_draft(provider: &str) -> RouteDraft {
        RouteDraft {
            id: None,
            name: "Blog CDN".to_string(),
            provider: provider.to_string(),
            endpoint: Some("https://example.com".to_string()),
            bucket: Some("assets".to_string()),
            region: Some("auto".to_string()),
            path_template: "{yyyy}/{mm}/{name}.{ext}".to_string(),
            public_base_url: "https://cdn.example.com".to_string(),
            cache_control: default_route_cache_control(),
            default_copy_format: "markdown".to_string(),
            custom_link_template: None,
            custom_api_method: None,
            custom_api_body: None,
            custom_api_file_field: None,
            custom_api_headers: None,
            custom_api_url_path: None,
            concurrency: 3,
            credential: None,
        }
    }

    #[test]
    fn validates_provider_required_fields() {
        assert!(validate_route_draft(&route_draft("s3")).is_ok());
        assert!(validate_route_draft(&route_draft("aliyun_oss")).is_ok());
        assert!(validate_route_draft(&route_draft("tencent_cos")).is_ok());
        assert!(validate_route_draft(&route_draft("qiniu")).is_ok());
        assert!(validate_route_draft(&route_draft("custom_api")).is_ok());
        assert!(validate_route_draft(&route_draft("local_folder")).is_ok());
        let mut github = route_draft("github");
        github.endpoint = Some("picdog/assets".to_string());
        assert!(validate_route_draft(&github).is_ok());

        let mut missing_bucket = route_draft("s3");
        missing_bucket.bucket = None;
        let missing_bucket_error = validate_route_draft(&missing_bucket).unwrap_err();
        assert_eq!(missing_bucket_error.code, "route_bucket_required");
        assert_eq!(missing_bucket_error.message, "上传目标缺少 Bucket。");

        let mut github_without_cdn = route_draft("github");
        github_without_cdn.public_base_url = String::new();
        github_without_cdn.endpoint = Some("picdog/assets".to_string());
        github_without_cdn.bucket = Some("main".to_string());
        assert!(validate_route_draft(&github_without_cdn).is_ok());

        let mut invalid_github = route_draft("github");
        invalid_github.endpoint = Some("invalid".to_string());
        let invalid_github_error = validate_route_draft(&invalid_github).unwrap_err();
        assert_eq!(invalid_github_error.code, "github_repository_invalid");
        assert_eq!(
            invalid_github_error.message,
            "GitHub 仓库格式应为 owner/repo 或仓库 URL。"
        );

        let mut github_enterprise_without_cdn = route_draft("github");
        github_enterprise_without_cdn.endpoint =
            Some("https://ghe.example.com/acme/assets".to_string());
        github_enterprise_without_cdn.public_base_url = String::new();
        let github_enterprise_error =
            validate_route_draft(&github_enterprise_without_cdn).unwrap_err();
        assert_eq!(
            github_enterprise_error.code,
            "github_enterprise_base_url_required"
        );
        assert_eq!(
            github_enterprise_error.message,
            "GitHub Enterprise 目标需要填写 Raw / CDN 域名。"
        );

        let unsupported = route_draft("smms");
        let unsupported_error = validate_route_draft(&unsupported).unwrap_err();
        assert_eq!(unsupported_error.code, "route_provider_unsupported");
        assert_eq!(unsupported_error.message, "暂不支持该上传目标类型。");
    }

    #[test]
    fn builds_github_public_base_when_cdn_is_empty() {
        assert_eq!(
            normalized_public_base_url("github", Some("picdog/assets"), Some("main"), ""),
            "https://raw.githubusercontent.com/picdog/assets/main"
        );
        assert_eq!(
            normalized_public_base_url(
                "github",
                Some("picdog/assets"),
                Some("main"),
                "https://cdn.example.com/"
            ),
            "https://cdn.example.com"
        );
    }
}
