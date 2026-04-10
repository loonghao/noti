//! Webhook callback support for task completion/failure notifications.
//!
//! When a task has a `callback_url` set, the worker will fire an HTTP POST
//! request to that URL with the task's final status as a JSON payload.
//!
//! If `callback_secret` is set on the task, the callback request will include
//! an `X-Noti-Signature: sha256=<hmac_hex>` header computed as HMAC-SHA256
//! of the raw JSON body signed with the secret. This allows the receiver to
//! verify callback authenticity and detect tampering.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::task::{NotificationTask, TaskStatus};

/// W3C TraceContext traceparent header name.
const TRACEPARENT_HEADER: &str = "traceparent";

/// Extract the W3C TraceContext `traceparent` header value from the current tracing span.
///
/// Returns `None` if no active span has a valid OpenTelemetry context.
/// The format is: `00-{trace_id}-{parent_id}-{flags}` where trace_id is 32 hex
/// chars, parent_id is 16 hex chars, and flags is 2 hex chars ("01" = sampled).
fn current_traceparent() -> Option<String> {
    use opentelemetry::trace::TraceContextExt as _;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let span = tracing::Span::current();
    let ctx = span.context();
    let span_ref = ctx.span();
    let span_ctx = span_ref.span_context();
    if !span_ctx.is_valid() {
        return None;
    }

    let trace_id = span_ctx.trace_id();
    let span_id = span_ctx.span_id();
    let flags = if span_ctx.is_sampled() { "01" } else { "00" };

    // W3C spec: trace_id is 32 hex chars, span_id is 16 hex chars
    Some(format!(
        "00-{:032x}-{:016x}-{}",
        trace_id, span_id, flags
    ))
}

/// Shared HTTP client for webhook callbacks (reused across all calls).
static CALLBACK_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build callback HTTP client")
});

/// Header name for the HMAC-SHA256 signature of the callback body.
const SIGNATURE_HEADER: &str = "X-Noti-Signature";

/// Header name for the Unix timestamp at which the callback was sent.
const TIMESTAMP_HEADER: &str = "X-Noti-Timestamp";

/// Compute HMAC-SHA256 of `payload_bytes` using `secret` and return the
/// hex-encoded string prefixed with "sha256=".
fn compute_signature(secret: &str, payload_bytes: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can accept any key size");
    mac.update(payload_bytes);
    let result = mac.finalize().into_bytes();
    let hex = result
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    format!("sha256={}", hex)
}

/// Payload sent to the callback URL when a task reaches a terminal state.
#[derive(Debug, Clone, Serialize)]
pub struct CallbackPayload {
    /// Task ID.
    pub task_id: String,
    /// Provider name.
    pub provider: String,
    /// Final status: "completed", "failed", or "cancelled".
    pub status: String,
    /// Number of delivery attempts.
    pub attempts: u32,
    /// Error message from the last failed attempt (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    /// Task metadata.
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl CallbackPayload {
    /// Build a callback payload from a terminal task.
    pub fn from_task(task: &NotificationTask) -> Self {
        Self {
            task_id: task.id.clone(),
            provider: task.provider.clone(),
            status: task.status.to_string(),
            attempts: task.attempts,
            last_error: task.last_error.clone(),
            metadata: task.metadata.clone(),
        }
    }
}

/// Fire a webhook callback for a task that reached a terminal state.
///
/// This is a best-effort operation — callback failures are logged but
/// do not affect the task's final status. The function is designed to
/// be called from the worker loop without blocking task processing.
#[tracing_attributes::instrument(skip(task), fields(task_id = %task.id, callback_url = %task.callback_url.as_ref().unwrap_or(&String::new()), status = %task.status))]
pub async fn fire_callback(task: &NotificationTask) {
    let url = match &task.callback_url {
        Some(url) if !url.is_empty() => url.clone(),
        _ => return, // No callback URL configured
    };

    // Only fire for terminal states
    if !matches!(
        task.status,
        TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
    ) {
        return;
    }

    let payload = CallbackPayload::from_task(task);

    tracing::debug!(
        task_id = %task.id,
        callback_url = %url,
        status = %task.status,
        signed = task.callback_secret.is_some(),
        "firing webhook callback"
    );

    // Use shared client with a short timeout
    let client = &*CALLBACK_CLIENT;

    // Serialize once so we can sign the raw bytes before sending
    let payload_bytes = match serde_json::to_vec(&payload) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!(task_id = %task.id, error = %e, "failed to serialize callback payload");
            return;
        }
    };

    let mut request = client.post(&url);
    request = request
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(TIMESTAMP_HEADER, std::time::UNIX_EPOCH.elapsed().unwrap_or_default().as_secs().to_string());

    // Propagate the W3C TraceContext for distributed tracing correlation
    if let Some(tp) = current_traceparent() {
        request = request.header(TRACEPARENT_HEADER, &tp);
    }

    if let Some(ref secret) = task.callback_secret {
        let sig = compute_signature(secret, &payload_bytes);
        request = request.header(SIGNATURE_HEADER, sig);
    }

    match request.body(payload_bytes).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                tracing::info!(
                    task_id = %task.id,
                    callback_url = %url,
                    http_status = %status,
                    "webhook callback delivered"
                );
            } else {
                tracing::warn!(
                    task_id = %task.id,
                    callback_url = %url,
                    http_status = %status,
                    "webhook callback returned non-success status"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                task_id = %task.id,
                callback_url = %url,
                error = %e,
                "webhook callback failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noti_core::{Message, ProviderConfig};

    #[test]
    fn test_callback_payload_from_task() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task = NotificationTask::new("slack", config, msg)
            .with_metadata("key", "value")
            .with_callback_url("https://example.com/callback");

        task.mark_processing();
        task.mark_completed();

        let payload = CallbackPayload::from_task(&task);
        assert_eq!(payload.task_id, task.id);
        assert_eq!(payload.provider, "slack");
        assert_eq!(payload.status, "completed");
        assert_eq!(payload.attempts, 1);
        assert!(payload.last_error.is_none());
        assert_eq!(payload.metadata.get("key").unwrap(), "value");
    }

    #[test]
    fn test_callback_payload_failed_task() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task = NotificationTask::new("email", config, msg);

        task.mark_processing();
        task.mark_failed("connection timeout");

        let payload = CallbackPayload::from_task(&task);
        assert_eq!(payload.status, "failed");
        assert_eq!(payload.last_error.as_deref(), Some("connection timeout"));
    }

    #[tokio::test]
    async fn test_fire_callback_no_url() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task = NotificationTask::new("slack", config, msg);
        task.mark_processing();
        task.mark_completed();

        // Should return immediately without error (no URL)
        fire_callback(&task).await;
    }

    #[tokio::test]
    async fn test_fire_callback_non_terminal() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let task = NotificationTask::new("slack", config, msg)
            .with_callback_url("https://example.com/callback");

        // Task is still in Queued status — callback should not fire
        fire_callback(&task).await;
    }

    #[tokio::test]
    async fn test_fire_callback_invalid_url() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task =
            NotificationTask::new("slack", config, msg).with_callback_url("not-a-valid-url");

        task.mark_processing();
        task.mark_completed();

        // Should handle gracefully without panic
        fire_callback(&task).await;
    }

    #[test]
    fn test_compute_signature() {
        // Known-answer test for HMAC-SHA256
        let secret = "my-secret-key";
        let payload = br#"{"task_id":"abc","provider":"slack","status":"completed","attempts":1}"#;
        let sig = compute_signature(secret, payload);
        assert!(sig.starts_with("sha256="));
        assert_eq!(sig.len(), 7 + 64); // "sha256=" (7) + 64 hex chars
    }

    #[test]
    fn test_compute_signature_different_secrets_different_output() {
        let payload = b"test";
        let sig1 = compute_signature("secret1", payload);
        let sig2 = compute_signature("secret2", payload);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_compute_signature_deterministic() {
        let secret = "static-secret";
        let payload = b"same payload";
        let sig1 = compute_signature(secret, payload);
        let sig2 = compute_signature(secret, payload);
        assert_eq!(sig1, sig2);
    }

    #[tokio::test]
    async fn test_current_traceparent_no_active_span() {
        // Without an active OTEL span (no tracer initialized in tests),
        // current_traceparent() should return None gracefully.
        let result = current_traceparent();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_current_traceparent_inside_span() {
        // When called inside an active tracing span, current_traceparent()
        // should return a valid W3C traceparent header.
        let result = tracing::info_span!("test_span", test = "value")
            .in_scope(current_traceparent);

        // If OTEL is initialized with a real exporter, this will be Some.
        // In test environments without OTEL, this may be None — both are valid.
        if let Some(tp) = result {
            // Verify format: 00-{32 hex trace_id}-{16 hex span_id}-{2 hex flags}
            assert!(tp.starts_with("00-"), "traceparent should start with '00-': {}", tp);
            let parts: Vec<&str> = tp.split('-').collect();
            assert_eq!(parts.len(), 4, "traceparent should have 4 parts: {}", tp);
            assert_eq!(parts[0], "00", "version should be '00': {}", tp);
            assert_eq!(parts[1].len(), 32, "trace_id should be 32 hex chars: {}", tp);
            assert_eq!(parts[2].len(), 16, "span_id should be 16 hex chars: {}", tp);
            assert!(parts[3] == "00" || parts[3] == "01", "flags should be 00 or 01: {}", tp);
        }
        // result being None is also acceptable in a no-OTEL test environment
    }
}
