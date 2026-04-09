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

    /// Request timed out waiting for a response.
    #[error("timeout error: {0}")]
    Timeout(String),

    /// Rate limited by the provider (HTTP 429 or equivalent).
    #[error("rate limited by {provider}: retry after {retry_after_secs:?}s")]
    RateLimited {
        /// Provider that rate-limited the request.
        provider: String,
        /// Suggested retry-after duration in seconds, if provided by the API.
        retry_after_secs: Option<u64>,
    },
}

impl NotiError {
    /// Convenience constructor for provider errors.
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for timeout errors.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout(message.into())
    }

    /// Convenience constructor for rate-limited errors.
    pub fn rate_limited(provider: impl Into<String>, retry_after_secs: Option<u64>) -> Self {
        Self::RateLimited {
            provider: provider.into(),
            retry_after_secs,
        }
    }

    /// Whether this error is transient and worth retrying.
    ///
    /// Validation, config, and URL parse errors are permanent — retrying
    /// the same request will always produce the same result.
    /// Network, timeout, rate-limited, and some provider/Io errors
    /// are considered transient.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) | Self::Timeout(_) => true,
            Self::RateLimited { .. } => true,
            // Provider errors may be transient (5xx) or permanent (4xx auth).
            // Conservatively retry since the caller can inspect the status code
            // in SendResponse for finer-grained decisions.
            Self::Provider { .. } => true,
            // Io errors are often transient (broken pipe, connection reset).
            Self::Io(_) => true,
            Self::Validation(_) | Self::Config(_) | Self::UrlParse(_) => false,
        }
    }

    /// Whether this error is a rate-limit error.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, Self::RateLimited { .. })
    }

    /// Whether this error is a timeout error.
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout(_))
    }
}
