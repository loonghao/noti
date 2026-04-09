//! Shared HTTP response helpers for provider implementations.
//!
//! Provides consistent handling of common HTTP status codes across all providers:
//! - 429 → [`NotiError::RateLimited`] with `Retry-After` header parsing
//! - Timeouts → [`NotiError::Timeout`]
//! - Other errors → [`NotiError::Provider`] or [`NotiError::Network`]

use noti_core::{NotiError, SendResponse};

/// Classify a `reqwest::Error` into the appropriate `NotiError` variant.
///
/// - Timeout errors → `NotiError::Timeout`
/// - Connection/redirect errors → `NotiError::Network`
/// - Other request errors → `NotiError::Network`
pub fn classify_reqwest_error(provider: &str, err: reqwest::Error) -> NotiError {
    if err.is_timeout() {
        NotiError::timeout(format!("request to {provider} timed out: {err}"))
    } else if err.is_connect() {
        NotiError::Network(format!("connection to {provider} failed: {err}"))
    } else {
        NotiError::Network(err.to_string())
    }
}

/// Handle a non-success HTTP response status code.
///
/// Returns the appropriate `NotiError` variant based on the status code:
/// - 429 → `NotiError::RateLimited` (parses `Retry-After` header if present)
/// - Other codes → `NotiError::Provider`
///
/// The `body_text` parameter is used as the error message for non-429 responses.
pub fn handle_http_error(
    provider: &str,
    status: u16,
    body_text: &str,
    retry_after: Option<&str>,
) -> NotiError {
    if status == 429 {
        let retry_after_secs = retry_after.and_then(|v| v.parse::<u64>().ok());
        NotiError::rate_limited(provider, retry_after_secs)
    } else {
        let msg = if body_text.is_empty() {
            format!("HTTP {status}")
        } else {
            format!("HTTP {status}: {}", truncate_body(body_text, 200))
        };
        NotiError::provider(provider, msg)
    }
}

/// Build a failure `SendResponse` from an HTTP error status code.
///
/// This is a convenience for providers that return `SendResponse::failure`
/// instead of `NotiError` for certain status codes (e.g., 4xx client errors
/// that should not be retried).
pub fn failure_from_status(provider: &str, status: u16, body_text: &str) -> SendResponse {
    SendResponse::failure(provider, format!("HTTP {status}: {}", truncate_body(body_text, 200)))
        .with_status_code(status)
}

/// Truncate a body string to `max_len` characters, appending "..." if truncated.
fn truncate_body(body: &str, max_len: usize) -> &str {
    if body.len() <= max_len {
        body
    } else {
        // Find a safe truncation point (don't split multi-byte chars)
        let mut end = max_len;
        while !body.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &body[..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_reqwest_timeout() {
        // Create a mock reqwest error by constructing from URL and kind
        let url = "https://api.slack.com";
        let err = reqwest::Error::new(
            reqwest::error::Kind::Builder(()),
            Some(url.parse().unwrap()),
        );
        // The is_timeout() won't be true for Builder errors, so test with Request
        // Instead, test the logic directly
        assert!(matches!(
            classify_reqwest_error("slack", make_timeout_error()),
            NotiError::Timeout(_)
        ));
    }

    #[test]
    fn test_classify_reqwest_connect_error() {
        let err = make_connect_error();
        let classified = classify_reqwest_error("slack", err);
        assert!(matches!(classified, NotiError::Network(_)));
        assert!(classified.to_string().contains("connection"));
    }

    #[test]
    fn test_handle_http_error_429_with_retry_after() {
        let err = handle_http_error("slack", 429, "rate limited", Some("60"));
        assert!(err.is_rate_limited());
        assert!(err.is_retryable());
        let msg = err.to_string();
        assert!(msg.contains("slack"));
        assert!(msg.contains("60"));
    }

    #[test]
    fn test_handle_http_error_429_without_retry_after() {
        let err = handle_http_error("discord", 429, "rate limited", None);
        assert!(err.is_rate_limited());
    }

    #[test]
    fn test_handle_http_error_429_invalid_retry_after() {
        let err = handle_http_error("slack", 429, "rate limited", Some("invalid"));
        assert!(err.is_rate_limited());
        // retry_after_secs should be None since "invalid" can't parse
        if let NotiError::RateLimited {
            retry_after_secs, ..
        } = err
        {
            assert_eq!(retry_after_secs, None);
        }
    }

    #[test]
    fn test_handle_http_error_500() {
        let err = handle_http_error("slack", 500, "internal server error", None);
        assert!(matches!(err, NotiError::Provider { .. }));
        assert!(err.is_retryable());
    }

    #[test]
    fn test_handle_http_error_401() {
        let err = handle_http_error("slack", 401, "unauthorized", None);
        assert!(matches!(err, NotiError::Provider { .. }));
        let msg = err.to_string();
        assert!(msg.contains("401"));
    }

    #[test]
    fn test_handle_http_error_empty_body() {
        let err = handle_http_error("slack", 500, "", None);
        assert!(err.to_string().contains("HTTP 500"));
    }

    #[test]
    fn test_failure_from_status() {
        let resp = failure_from_status("slack", 500, "internal server error");
        assert!(!resp.success);
        assert_eq!(resp.status_code, Some(500));
        assert!(resp.message.contains("500"));
    }

    #[test]
    fn test_truncate_body_short() {
        assert_eq!(truncate_body("hello", 200), "hello");
    }

    #[test]
    fn test_truncate_body_long() {
        let long = "a".repeat(300);
        let truncated = truncate_body(&long, 200);
        assert_eq!(truncated.len(), 200);
    }

    #[test]
    fn test_truncate_body_unicode() {
        let unicode = "日".repeat(100); // 300 bytes, 100 chars
        let truncated = truncate_body(&unicode, 200);
        // Should not split a multi-byte character
        assert!(truncated.chars().all(|c| c == '日'));
    }

    // Helper to create a timeout reqwest error
    fn make_timeout_error() -> reqwest::Error {
        use std::time::Duration;
        let url = "https://example.com".parse().unwrap();
        reqwest::Error::new(
            reqwest::error::Kind::Request(
                tokio::time::error::Elapsed::new(Duration::from_secs(30)),
            ),
            Some(url),
        )
    }

    // Helper to create a connect reqwest error
    fn make_connect_error() -> reqwest::Error {
        let url = "https://example.com".parse().unwrap();
        reqwest::Error::new(reqwest::error::Kind::Connect, Some(url))
    }
}
