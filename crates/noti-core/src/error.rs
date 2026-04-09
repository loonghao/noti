use thiserror::Error;

/// Core error type for the noti notification system.
#[derive(Error, Debug)]
pub enum NotiError {
    /// Configuration file read/write or parsing error.
    #[error("config error: {0}")]
    Config(String),

    /// Provider-specific error (e.g. API rejection, auth failure).
    ///
    /// Contains an optional `retryable` flag to distinguish transient (5xx)
    /// from permanent (4xx) provider errors. When `None`, the caller should
    /// use its own heuristic (typically treat as non-retryable for safety).
    #[error("provider error ({provider}): {message}")]
    Provider {
        provider: String,
        message: String,
        /// Whether this error is likely transient and worth retrying.
        /// `None` means unknown — callers should conservatively treat as non-retryable.
        retryable: Option<bool>,
    },

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
    ///
    /// Defaults to `retryable: None` (unknown). Use [`Self::provider_retryable`]
    /// or [`Self::provider_permanent`] for explicit classification.
    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
            retryable: None,
        }
    }

    /// Convenience constructor for retryable provider errors (e.g. 5xx).
    pub fn provider_retryable(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
            retryable: Some(true),
        }
    }

    /// Convenience constructor for permanent provider errors (e.g. 4xx auth/validation).
    pub fn provider_permanent(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Provider {
            provider: provider.into(),
            message: message.into(),
            retryable: Some(false),
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
    ///
    /// For `Provider` errors, the `retryable` field distinguishes:
    /// - `Some(true)` — transient (5xx, upstream timeout)
    /// - `Some(false)` — permanent (4xx auth/validation/not-found)
    /// - `None` — unknown, conservatively treated as non-retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Network(_) | Self::Timeout(_) => true,
            Self::RateLimited { .. } => true,
            Self::Provider { retryable, .. } => retryable.unwrap_or(false),
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
