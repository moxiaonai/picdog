use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SafeError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    pub retryable: bool,
}

impl SafeError {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        recoverable: bool,
        retryable: bool,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
            retryable,
        }
    }
}

pub type CommandResult<T> = Result<T, SafeError>;
