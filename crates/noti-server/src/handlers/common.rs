use std::collections::HashMap;

use serde::Deserialize;

use noti_core::{Message, MessageFormat, Priority, RetryPolicy};

/// Retry configuration for the API.
#[derive(Debug, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries.
    pub max_retries: Option<u32>,
    /// Base delay in milliseconds.
    pub delay_ms: Option<u64>,
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
pub fn build_retry_policy(retry: Option<&RetryConfig>, default_policy: RetryPolicy) -> RetryPolicy {
    match retry {
        Some(cfg) => {
            let max_retries = cfg.max_retries.unwrap_or(3);
            let delay = std::time::Duration::from_millis(cfg.delay_ms.unwrap_or(1000));
            RetryPolicy::fixed(max_retries, delay)
        }
        None => default_policy,
    }
}
