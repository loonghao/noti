use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use noti_core::{Message, MessageFormat, NotifyProvider, Priority, ProviderRegistry, RetryPolicy};

use super::error::{ApiError, codes};

/// Look up a provider by name, returning an `ApiError::not_found` if missing.
pub fn require_provider(
    registry: &ProviderRegistry,
    name: &str,
) -> Result<Arc<dyn NotifyProvider>, ApiError> {
    registry.get_by_name(name).cloned().ok_or_else(|| {
        ApiError::not_found(format!("provider '{name}' not found"))
            .with_code(codes::PROVIDER_NOT_FOUND)
    })
}

/// Retry configuration for the API.
///
/// When only `max_retries` and `delay_ms` are provided, a **fixed** delay
/// strategy is used.  Adding `backoff_multiplier` (> 1.0) switches to
/// **exponential** backoff where each retry delay is multiplied by this
/// factor, capped at `max_delay_ms`.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct RetryConfig {
    /// Maximum number of retries.
    pub max_retries: Option<u32>,
    /// Base delay in milliseconds (initial delay for exponential backoff).
    pub delay_ms: Option<u64>,
    /// Backoff multiplier applied after each retry (e.g. 2.0 for exponential).
    /// When absent or equal to 1.0, a fixed delay strategy is used.
    pub backoff_multiplier: Option<f64>,
    /// Maximum delay in milliseconds (caps exponential growth).
    /// Defaults to `delay_ms` when using fixed strategy, or 30 000 ms for exponential.
    pub max_delay_ms: Option<u64>,
}

/// Build a [`Message`] from raw API fields.
pub fn build_message(
    text: &str,
    title: Option<&str>,
    format: Option<&str>,
    priority: Option<&str>,
    extra: &HashMap<String, serde_json::Value>,
) -> Message {
    let mut msg = Message::text(text);

    if let Some(t) = title {
        msg = msg.with_title(t);
    }

    if let Some(f) = format {
        if let Ok(fmt) = f.parse::<MessageFormat>() {
            msg = msg.with_format(fmt);
        }
    }

    if let Some(p) = priority {
        if let Ok(pri) = p.parse::<Priority>() {
            msg = msg.with_priority(pri);
        }
    }

    for (k, v) in extra {
        msg = msg.with_extra(k, v.clone());
    }

    msg
}

/// Build a [`RetryPolicy`] from the optional API config.
///
/// `default_policy` controls what happens when no retry config is provided:
/// - For synchronous sends, pass `RetryPolicy::none()` (no retries by default).
/// - For async/queue sends, pass `RetryPolicy::default()` (retry by default).
///
/// Strategy selection:
/// - If `backoff_multiplier` is present and > 1.0 → exponential backoff.
/// - Otherwise → fixed delay.
pub fn build_retry_policy(retry: Option<&RetryConfig>, default_policy: RetryPolicy) -> RetryPolicy {
    match retry {
        Some(cfg) => {
            let max_retries = cfg.max_retries.unwrap_or(3);
            let delay_ms = cfg.delay_ms.unwrap_or(1000);
            let initial_delay = std::time::Duration::from_millis(delay_ms);

            match cfg.backoff_multiplier {
                Some(mult) if mult > 1.0 => {
                    let max_delay_ms = cfg.max_delay_ms.unwrap_or(30_000);
                    let max_delay = std::time::Duration::from_millis(max_delay_ms);
                    RetryPolicy {
                        max_retries,
                        initial_delay,
                        max_delay,
                        backoff_multiplier: mult,
                    }
                }
                _ => {
                    // Fixed delay: multiplier absent, null, 0, 1.0, or negative
                    RetryPolicy::fixed(max_retries, initial_delay)
                }
            }
        }
        None => default_policy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_build_retry_policy_none_config() {
        let policy = build_retry_policy(None, RetryPolicy::none());
        assert_eq!(policy.max_retries, 0);
    }

    #[test]
    fn test_build_retry_policy_default_config() {
        let policy = build_retry_policy(None, RetryPolicy::default());
        assert_eq!(policy.max_retries, 3);
        assert!((policy.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_build_retry_policy_fixed_explicit() {
        let cfg = RetryConfig {
            max_retries: Some(5),
            delay_ms: Some(200),
            backoff_multiplier: None,
            max_delay_ms: None,
        };
        let policy = build_retry_policy(Some(&cfg), RetryPolicy::none());
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(200));
        assert!((policy.backoff_multiplier - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_build_retry_policy_exponential() {
        let cfg = RetryConfig {
            max_retries: Some(4),
            delay_ms: Some(100),
            backoff_multiplier: Some(2.0),
            max_delay_ms: Some(5000),
        };
        let policy = build_retry_policy(Some(&cfg), RetryPolicy::none());
        assert_eq!(policy.max_retries, 4);
        assert_eq!(policy.initial_delay, Duration::from_millis(100));
        assert_eq!(policy.max_delay, Duration::from_millis(5000));
        assert!((policy.backoff_multiplier - 2.0).abs() < f64::EPSILON);

        // Verify delay progression: 100, 200, 400, 800
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(200));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(400));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(800));
    }

    #[test]
    fn test_build_retry_policy_exponential_capped() {
        let cfg = RetryConfig {
            max_retries: Some(5),
            delay_ms: Some(1000),
            backoff_multiplier: Some(3.0),
            max_delay_ms: Some(5000),
        };
        let policy = build_retry_policy(Some(&cfg), RetryPolicy::none());
        // 1000, 3000, 5000 (capped), 5000 (capped)
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(3000));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(5000));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_millis(5000));
    }

    #[test]
    fn test_build_retry_policy_multiplier_1_is_fixed() {
        let cfg = RetryConfig {
            max_retries: Some(3),
            delay_ms: Some(500),
            backoff_multiplier: Some(1.0),
            max_delay_ms: Some(10000),
        };
        let policy = build_retry_policy(Some(&cfg), RetryPolicy::none());
        // multiplier=1.0 should produce fixed delay
        assert!((policy.backoff_multiplier - 1.0).abs() < f64::EPSILON);
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(500));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(500));
    }

    #[test]
    fn test_build_retry_policy_exponential_default_max_delay() {
        let cfg = RetryConfig {
            max_retries: Some(3),
            delay_ms: Some(100),
            backoff_multiplier: Some(2.0),
            max_delay_ms: None, // should default to 30_000
        };
        let policy = build_retry_policy(Some(&cfg), RetryPolicy::none());
        assert_eq!(policy.max_delay, Duration::from_millis(30_000));
    }

    #[test]
    fn test_build_retry_policy_backward_compatible() {
        // Old-style config with only max_retries and delay_ms should still work
        let cfg = RetryConfig {
            max_retries: Some(2),
            delay_ms: Some(10),
            backoff_multiplier: None,
            max_delay_ms: None,
        };
        let policy = build_retry_policy(Some(&cfg), RetryPolicy::default());
        assert_eq!(policy.max_retries, 2);
        assert_eq!(policy.initial_delay, Duration::from_millis(10));
        assert!((policy.backoff_multiplier - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retry_config_serde_roundtrip() {
        let cfg = RetryConfig {
            max_retries: Some(3),
            delay_ms: Some(100),
            backoff_multiplier: Some(2.5),
            max_delay_ms: Some(10000),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let parsed: RetryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.max_retries, Some(3));
        assert_eq!(parsed.delay_ms, Some(100));
        assert!((parsed.backoff_multiplier.unwrap() - 2.5).abs() < f64::EPSILON);
        assert_eq!(parsed.max_delay_ms, Some(10000));
    }

    #[test]
    fn test_retry_config_serde_backward_compatible() {
        // Old JSON with only max_retries and delay_ms
        let json = r#"{"max_retries": 2, "delay_ms": 500}"#;
        let cfg: RetryConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.max_retries, Some(2));
        assert_eq!(cfg.delay_ms, Some(500));
        assert!(cfg.backoff_multiplier.is_none());
        assert!(cfg.max_delay_ms.is_none());
    }
}
