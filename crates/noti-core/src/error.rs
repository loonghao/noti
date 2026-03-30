use thiserror::Error;

/// Core error type for the noti notification system.
#[derive(Error, Debug)]
pub enum NotiError {
    /// Configuration file read/write or parsing error.
    #[error("config error: {0}")]
    Config(String),

    /// Provider-specific error (e.g. API rejection, auth failure).
    #[error("provider error ({provider}): {message}")]
    Provider { provider: String, message: String },

    /// URL scheme parsing error.
    #[error("url parse error: {0}")]
    UrlParse(String),

    /// Network / HTTP transport error.
    #[error("network error: {0}")]
    Network(String),

    /// Parameter validation error.
    #[error("validation error: {0}")]
    Validation(String),

    /// File I/O error (e.g. reading an attachment).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl NotiError {
    /// Convenience constructor for provider errors.
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }
}
