use crate::error::{CommandResult, SafeError};
use crate::models::RouteCredentialInput;

const SERVICE: &str = "PicDog";
const REF_PREFIX: &str = "keychain://picdog/";

pub fn credential_ref_for_route(route_id: &str) -> String {
    format!("{}{}", REF_PREFIX, route_id)
}

pub fn save_route_credential(
    credential_ref: &str,
    credential: &RouteCredentialInput,
) -> CommandResult<()> {
    if !has_secret_value(credential) {
        return Ok(());
    }

    let payload = serde_json::to_string(credential).map_err(credential_error)?;
    let entry = entry_for_ref(credential_ref)?;
    entry.set_password(&payload).map_err(credential_error)?;

    Ok(())
}

pub fn credential_configured(credential_ref: &str) -> CommandResult<bool> {
    let entry = entry_for_ref(credential_ref)?;

    match entry.get_password() {
        Ok(value) => Ok(!value.trim().is_empty()),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(credential_error(error)),
    }
}

pub fn delete_route_credential(credential_ref: &str) -> CommandResult<()> {
    let entry = entry_for_ref(credential_ref)?;

    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(credential_error(error)),
    }
}

pub fn load_route_credential(credential_ref: &str) -> CommandResult<RouteCredentialInput> {
    let entry = entry_for_ref(credential_ref)?;
    let value = entry.get_password().map_err(|error| match error {
        keyring::Error::NoEntry => SafeError::new(
            "credential_missing",
            "上传目标还没有保存凭据。",
            true,
            false,
        ),
        error => credential_error(error),
    })?;

    serde_json::from_str(&value).map_err(credential_error)
}

fn entry_for_ref(credential_ref: &str) -> CommandResult<keyring::Entry> {
    let account = account_from_ref(credential_ref)?;
    keyring::Entry::new(SERVICE, &account).map_err(credential_error)
}

fn account_from_ref(credential_ref: &str) -> CommandResult<String> {
    credential_ref
        .strip_prefix(REF_PREFIX)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| SafeError::new("credential_missing", "上传目标凭据引用无效。", true, false))
}

fn has_secret_value(credential: &RouteCredentialInput) -> bool {
    [
        credential.access_key_id.as_deref(),
        credential.secret_access_key.as_deref(),
        credential.session_token.as_deref(),
        credential.api_token.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty())
}

fn credential_error(error: impl std::fmt::Display) -> SafeError {
    SafeError::new(
        "credential_missing",
        format!("系统凭据存储访问失败：{}", error),
        true,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::credential_ref_for_route;

    #[test]
    fn builds_picdog_keychain_reference() {
        assert_eq!(
            credential_ref_for_route("blog-r2"),
            "keychain://picdog/blog-r2"
        );
    }
}
