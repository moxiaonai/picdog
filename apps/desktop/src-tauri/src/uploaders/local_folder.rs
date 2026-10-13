use super::{UploadObjectInput, UploadObjectOutput};
use crate::error::{CommandResult, SafeError};
use crate::models::Route;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub fn upload_object(input: UploadObjectInput<'_>) -> CommandResult<UploadObjectOutput> {
    let destination_root = input
        .route
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "upload_failed",
                "Local Folder 缺少本地目录路径。",
                true,
                false,
            )
        })?;

    let source_path = Path::new(input.local_path);
    if !source_path.is_file() {
        return Err(SafeError::new(
            "file_not_found",
            "找不到压缩后的本地文件，无法复制到本地目录。",
            true,
            false,
        ));
    }

    let destination = safe_destination_path(destination_root, input.remote_key)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(write_error)?;
    }
    fs::copy(source_path, &destination).map_err(write_error)?;

    Ok(UploadObjectOutput {
        public_url: public_url_for_key(input.route, input.remote_key),
    })
}

pub fn delete_object(route: &Route, remote_key: &str) -> CommandResult<()> {
    let destination_root = route
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SafeError::new(
                "upload_failed",
                "Local Folder 缺少本地目录路径。",
                true,
                false,
            )
        })?;
    let destination = safe_destination_path(destination_root, remote_key)?;

    match fs::remove_file(destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(write_error(error)),
    }
}

fn safe_destination_path(root: &str, remote_key: &str) -> CommandResult<PathBuf> {
    let root = PathBuf::from(root);
    let mut destination = root.clone();

    for segment in remote_key.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." || segment.contains(['\\', ':'])
        {
            return Err(SafeError::new(
                "upload_failed",
                "本地目录目标路径不安全。",
                true,
                false,
            ));
        }
        destination.push(segment);
    }

    Ok(destination)
}

fn public_url_for_key(route: &Route, remote_key: &str) -> String {
    format!(
        "{}/{}",
        route.public_base_url.trim_end_matches('/'),
        remote_key.trim_start_matches('/')
    )
}

fn write_error(error: impl std::fmt::Display) -> SafeError {
    SafeError::new(
        "permission_denied",
        format!("无法写入本地发布目录：{}", error),
        true,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{delete_object, public_url_for_key, safe_destination_path};
    use crate::models::Route;

    fn route() -> Route {
        Route {
            id: "local".to_string(),
            name: "Local Site".to_string(),
            provider: "local_folder".to_string(),
            credential_ref: "keychain://picdog/local".to_string(),
            endpoint: Some("/tmp/site-public".to_string()),
            bucket: None,
            region: None,
            path_template: "{yyyy}/{mm}/{name}.{ext}".to_string(),
            public_base_url: "https://static.example.com".to_string(),
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
    fn builds_local_destination_under_root() {
        let path = safe_destination_path("/tmp/site-public", "2026/10/hero.webp").unwrap();
        assert_eq!(
            path,
            std::path::PathBuf::from("/tmp/site-public/2026/10/hero.webp")
        );
    }

    #[test]
    fn rejects_unsafe_local_key_segments() {
        assert!(safe_destination_path("/tmp/site-public", "../hero.webp").is_err());
        assert!(safe_destination_path("/tmp/site-public", "2026/../hero.webp").is_err());
    }

    #[test]
    fn builds_public_url_from_route_base_url() {
        assert_eq!(
            public_url_for_key(&route(), "2026/10/hero.webp"),
            "https://static.example.com/2026/10/hero.webp"
        );
    }

    #[test]
    fn deletes_local_test_object() {
        let root =
            std::env::temp_dir().join(format!("picdog-local-delete-test-{}", std::process::id()));
        let target = root.join("__picdog_route_test__/connection.txt");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, b"test").unwrap();

        let mut route = route();
        route.endpoint = Some(root.to_string_lossy().to_string());

        delete_object(&route, "__picdog_route_test__/connection.txt").unwrap();

        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
