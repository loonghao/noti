use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use crate::handlers::common::RetryConfig;

// ───────────────────── Request types ─────────────────────

/// Request body for async notification via the queue.
#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct AsyncSendRequest {
    /// Provider name (e.g. "slack", "email", "webhook").
    #[validate(length(min = 1, message = "provider must not be empty"))]
    pub provider: String,
    /// Provider-specific configuration values.
    #[serde(default)]
    pub config: HashMap<String, String>,
    /// Message body text.
    #[validate(length(min = 1, message = "text must not be empty"))]
    pub text: String,
    /// Optional message title/subject.
    pub title: Option<String>,
    /// Message format: "text", "markdown", or "html".
    #[serde(default)]
    pub format: Option<String>,
    /// Priority: "low", "normal", "high", "urgent".
    pub priority: Option<String>,
    /// Extra provider-specific parameters.
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
    /// Retry policy configuration.
    pub retry: Option<RetryConfig>,
    /// Optional metadata for tracking/correlation.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Optional webhook URL to call when the task completes or fails.
    pub callback_url: Option<String>,
    /// Optional HMAC secret for signing webhook callbacks.
    /// When set, the callback POST will include `X-Noti-Signature: sha256=<hmac>`.
    pub callback_secret: Option<String>,
    /// Delay in seconds before the notification is sent.
    /// Mutually exclusive with `scheduled_at`.
    pub delay_seconds: Option<u64>,
    /// ISO 8601 timestamp (RFC 3339) for when the notification should be sent.
    /// Mutually exclusive with `delay_seconds`.
    /// Example: `"2025-08-15T10:30:00Z"`.
    pub scheduled_at: Option<String>,
}

/// Query parameters for listing tasks.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListTasksQuery {
    /// Filter by status: "queued", "processing", "completed", "failed", "cancelled".
    pub status: Option<String>,
    /// Maximum number of tasks to return (default: 50).
    pub limit: Option<usize>,
}

// ───────────────────── Response types ─────────────────────

/// Response for a successfully enqueued task.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EnqueueResponse {
    pub task_id: String,
    pub status: String,
    pub message: String,
}

/// Serializable task info for API responses.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskInfo {
    pub id: String,
    pub provider: String,
    pub status: String,
    pub attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_error: Option<String>,
    pub priority: String,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,
    /// When the task is scheduled to be sent (ISO 8601 / RFC 3339).
    /// Present only for delayed/scheduled tasks.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scheduled_at: Option<String>,
}

/// Response for queue statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatsResponse {
    pub queued: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total: usize,
    /// Number of entries in the dead letter queue.
    pub dlq_size: usize,
}

/// Response for purge operation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PurgeResponse {
    pub purged: usize,
    pub message: String,
}

/// Response for cancel operation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CancelResponse {
    pub task_id: String,
    pub cancelled: bool,
    pub message: String,
}

// ───────────────────── Batch async types ─────────────────────

/// Request body for batch async notification enqueue.
///
/// Each item reuses [`AsyncSendRequest`] since the fields are identical.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct BatchAsyncRequest {
    /// List of notifications to enqueue.
    #[validate(length(min = 1, message = "items must not be empty"))]
    pub items: Vec<AsyncSendRequest>,
}

/// Per-item result in a batch enqueue response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchEnqueueItemResult {
    /// Index of the item in the request.
    pub index: usize,
    /// Provider name.
    pub provider: String,
    /// Whether the enqueue succeeded.
    pub success: bool,
    /// Task ID if successful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Error message if failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response for batch async enqueue.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchEnqueueResponse {
    /// Per-item results.
    pub results: Vec<BatchEnqueueItemResult>,
    /// Number of successfully enqueued items.
    pub enqueued: usize,
    /// Number of failed items.
    pub failed: usize,
    /// Total items in the request.
    pub total: usize,
}

// ───────────────────── QueueStats → StatsResponse conversion ─────────────────────

use noti_queue::QueueStats;

impl From<QueueStats> for StatsResponse {
    fn from(stats: QueueStats) -> Self {
        Self {
            queued: stats.queued,
            processing: stats.processing,
            completed: stats.completed,
            failed: stats.failed,
            cancelled: stats.cancelled,
            total: stats.total(),
            dlq_size: 0, // populated separately via dlq_stats()
        }
    }
}

// ───────────────────── DLQ types ─────────────────────

/// Query parameters for listing DLQ entries.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListDlqQuery {
    /// Maximum number of entries to return (default: 50, max: 1000).
    pub limit: Option<usize>,
}

/// Serializable DLQ entry info for API responses.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DlqEntryInfo {
    pub task_id: String,
    pub provider: String,
    pub status: String,
    pub attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_error: Option<String>,
    pub reason: String,
    pub moved_at: String,
    pub priority: String,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub metadata: HashMap<String, String>,
}

/// Response for DLQ statistics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DlqStatsResponse {
    pub dlq_size: usize,
}

/// Response for listing DLQ entries.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DlqListResponse {
    pub entries: Vec<DlqEntryInfo>,
    pub total: usize,
}

/// Response for requeue operation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RequeueResponse {
    pub task_id: String,
    pub requeued: bool,
    pub message: String,
}

/// Response for DLQ entry deletion.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteDlqResponse {
    pub task_id: String,
    pub success: bool,
    pub message: String,
}

/// Response for delete from DLQ operation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteFromDlqResponse {
    pub task_id: String,
    pub deleted: bool,
    pub message: String,
}
