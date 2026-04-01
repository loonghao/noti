use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use noti_core::{Message, Priority, ProviderConfig, RetryPolicy};

/// Unique identifier for a queued notification task.
pub type TaskId = String;

/// Current status of a queued notification task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Waiting in queue to be picked up by a worker.
    Queued,
    /// Currently being processed by a worker.
    Processing,
    /// Successfully delivered.
    Completed,
    /// Failed after all retry attempts.
    Failed,
    /// Cancelled before delivery.
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Processing => write!(f, "processing"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A notification task to be processed asynchronously via the queue.
///
/// Contains all information needed to send a notification without
/// needing access to the original request context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTask {
    /// Unique task identifier.
    pub id: TaskId,

    /// Name of the provider to send through (e.g. "slack", "email").
    pub provider: String,

    /// Provider-specific configuration.
    pub config: ProviderConfig,

    /// The notification message to send.
    pub message: Message,

    /// Retry policy for this task.
    pub retry_policy: RetryPolicy,

    /// Current status of the task.
    pub status: TaskStatus,

    /// Number of delivery attempts made so far.
    pub attempts: u32,

    /// Error message from the last failed attempt, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,

    /// When the task was created.
    pub created_at: SystemTime,

    /// When the task was last updated.
    pub updated_at: SystemTime,

    /// Optional metadata for tracking/correlation.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,

    /// Optional webhook URL to call when the task reaches a terminal state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,

    /// Earliest time this task can be dequeued (used for retry backoff delays).
    /// When `None`, the task is immediately available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_at: Option<SystemTime>,
}

impl NotificationTask {
    /// Create a new notification task with default retry policy.
    pub fn new(provider: impl Into<String>, config: ProviderConfig, message: Message) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4().to_string(),
            provider: provider.into(),
            config,
            message,
            retry_policy: RetryPolicy::default(),
            status: TaskStatus::Queued,
            attempts: 0,
            last_error: None,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
            callback_url: None,
            available_at: None,
        }
    }

    /// Set a custom retry policy.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Add metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set a callback URL to be invoked when the task reaches a terminal state.
    pub fn with_callback_url(mut self, url: impl Into<String>) -> Self {
        self.callback_url = Some(url.into());
        self
    }

    /// Get the priority of the underlying message (used for queue ordering).
    pub fn priority(&self) -> Priority {
        self.message.priority
    }

    /// Mark task as processing.
    pub fn mark_processing(&mut self) {
        self.status = TaskStatus::Processing;
        self.attempts += 1;
        self.updated_at = SystemTime::now();
    }

    /// Mark task as completed.
    pub fn mark_completed(&mut self) {
        self.status = TaskStatus::Completed;
        self.updated_at = SystemTime::now();
    }

    /// Mark task as failed with an error message.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.last_error = Some(error.into());
        self.updated_at = SystemTime::now();
    }

    /// Mark task as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.updated_at = SystemTime::now();
    }

    /// Whether the task has reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled
        )
    }

    /// Whether the task should be retried based on its retry policy.
    pub fn should_retry(&self) -> bool {
        self.retry_policy
            .should_retry(self.attempts.saturating_sub(1))
    }

    /// Compute the backoff delay for the next retry based on the current attempt count.
    ///
    /// Returns `Duration::ZERO` when the task has no delay configured or for the first attempt.
    pub fn retry_delay(&self) -> std::time::Duration {
        self.retry_policy.delay_for_attempt(self.attempts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noti_core::Message;
    use std::time::Duration;

    #[test]
    fn test_task_new() {
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let task = NotificationTask::new("slack", config, msg);

        assert_eq!(task.provider, "slack");
        assert_eq!(task.status, TaskStatus::Queued);
        assert_eq!(task.attempts, 0);
        assert!(!task.id.is_empty());
        assert!(!task.is_terminal());
    }

    #[test]
    fn test_task_lifecycle() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task = NotificationTask::new("email", config, msg);

        assert_eq!(task.status, TaskStatus::Queued);

        task.mark_processing();
        assert_eq!(task.status, TaskStatus::Processing);
        assert_eq!(task.attempts, 1);

        task.mark_completed();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_failure() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task = NotificationTask::new("webhook", config, msg);

        task.mark_processing();
        task.mark_failed("connection timeout");

        assert_eq!(task.status, TaskStatus::Failed);
        assert_eq!(task.last_error.as_deref(), Some("connection timeout"));
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_cancellation() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task = NotificationTask::new("slack", config, msg);

        task.mark_cancelled();
        assert_eq!(task.status, TaskStatus::Cancelled);
        assert!(task.is_terminal());
    }

    #[test]
    fn test_task_retry_check() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let mut task = NotificationTask::new("slack", config, msg)
            .with_retry_policy(RetryPolicy::fixed(3, Duration::from_millis(100)));

        // Before any attempt
        assert!(task.should_retry());

        task.mark_processing(); // attempt 1
        assert!(task.should_retry());

        task.mark_processing(); // attempt 2
        assert!(task.should_retry());

        task.mark_processing(); // attempt 3
        assert!(task.should_retry());

        task.mark_processing(); // attempt 4 — exceeds max_retries=3
        assert!(!task.should_retry());
    }

    #[test]
    fn test_task_priority() {
        let msg = Message::text("urgent").with_priority(Priority::Urgent);
        let config = ProviderConfig::new();
        let task = NotificationTask::new("slack", config, msg);

        assert_eq!(task.priority(), Priority::Urgent);
    }

    #[test]
    fn test_task_metadata() {
        let msg = Message::text("test");
        let config = ProviderConfig::new();
        let task = NotificationTask::new("slack", config, msg)
            .with_metadata("correlation_id", "abc-123")
            .with_metadata("source", "api");

        assert_eq!(task.metadata.get("correlation_id").unwrap(), "abc-123");
        assert_eq!(task.metadata.get("source").unwrap(), "api");
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Queued.to_string(), "queued");
        assert_eq!(TaskStatus::Processing.to_string(), "processing");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_task_serde_roundtrip() {
        let msg = Message::text("test").with_priority(Priority::High);
        let config = ProviderConfig::new().set("webhook_url", "https://example.com");
        let task = NotificationTask::new("webhook", config, msg).with_metadata("key", "value");

        let json = serde_json::to_string(&task).unwrap();
        let parsed: NotificationTask = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, task.id);
        assert_eq!(parsed.provider, "webhook");
        assert_eq!(parsed.priority(), Priority::High);
        assert_eq!(parsed.metadata.get("key").unwrap(), "value");
    }
}
