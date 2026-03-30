//! Webhook callback support for task completion/failure notifications.
//!
//! When a task has a `callback_url` set, the worker will fire an HTTP POST
//! request to that URL with the task's final status as a JSON payload.

use serde::Serialize;
use std::collections::HashMap;

use crate::task::{NotificationTask, TaskStatus};

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
        "firing webhook callback"
    );

    // Use a short timeout to avoid blocking the worker
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                task_id = %task.id,
                error = %e,
                "failed to build HTTP client for callback"
            );
            return;
        }
    };

    match client.post(&url).json(&payload).send().await {
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
        let mut task = NotificationTask::new("slack", config, msg)
            .with_callback_url("not-a-valid-url");

        task.mark_processing();
        task.mark_completed();

        // Should handle gracefully without panic
        fire_callback(&task).await;
    }
}
