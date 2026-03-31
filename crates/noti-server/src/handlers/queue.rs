use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use validator::Validate;

use noti_core::{ProviderConfig, RetryPolicy};
use noti_queue::{NotificationTask, QueueStats, TaskStatus};

use crate::handlers::common::{self, RetryConfig};
use crate::handlers::error::ApiError;
use crate::middleware::validated_json::ValidatedJson;
use crate::state::AppState;

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
}

impl From<QueueStats> for StatsResponse {
    fn from(stats: QueueStats) -> Self {
        Self {
            queued: stats.queued,
            processing: stats.processing,
            completed: stats.completed,
            failed: stats.failed,
            cancelled: stats.cancelled,
            total: stats.total(),
        }
    }
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

// ───────────────────── Helpers ─────────────────────

fn task_to_info(task: &NotificationTask) -> TaskInfo {
    TaskInfo {
        id: task.id.clone(),
        provider: task.provider.clone(),
        status: task.status.to_string(),
        attempts: task.attempts,
        last_error: task.last_error.clone(),
        priority: format!("{:?}", task.priority()),
        metadata: task.metadata.clone(),
    }
}

fn parse_task_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().as_str() {
        "queued" => Some(TaskStatus::Queued),
        "processing" => Some(TaskStatus::Processing),
        "completed" => Some(TaskStatus::Completed),
        "failed" => Some(TaskStatus::Failed),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

fn queue_error(e: noti_queue::QueueError) -> ApiError {
    match &e {
        noti_queue::QueueError::QueueFull { .. } => ApiError {
            error: "queue_full".to_string(),
            message: e.to_string(),
            status: StatusCode::SERVICE_UNAVAILABLE,
        },
        noti_queue::QueueError::NotFound(_) => ApiError::not_found(e.to_string()),
        _ => ApiError::internal(e.to_string()),
    }
}

// ───────────────────── Handlers ─────────────────────

/// Enqueue a notification for asynchronous processing.
#[utoipa::path(
    post,
    path = "/api/v1/send/async",
    tag = "Async Queue",
    request_body = AsyncSendRequest,
    responses(
        (status = 202, description = "Notification enqueued", body = EnqueueResponse),
        (status = 404, description = "Provider not found", body = ApiError),
        (status = 503, description = "Queue full", body = ApiError),
    )
)]
pub async fn send_async(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<AsyncSendRequest>,
) -> Result<(StatusCode, Json<EnqueueResponse>), ApiError> {
    // Validate provider exists and config is well-formed
    let provider = common::require_provider(&state.registry, &req.provider)?;

    let config = ProviderConfig {
        values: req.config,
    };

    if let Err(e) = provider.validate_config(&config) {
        return Err(ApiError::bad_request(e.to_string()));
    }

    let msg = common::build_message(
        &req.text,
        req.title.as_deref(),
        req.format.as_deref(),
        req.priority.as_deref(),
        &req.extra,
    );

    let policy = common::build_retry_policy(req.retry.as_ref(), RetryPolicy::default());

    let mut task = NotificationTask::new(&req.provider, config, msg)
        .with_retry_policy(policy);

    if let Some(url) = &req.callback_url {
        task = task.with_callback_url(url);
    }

    for (k, v) in &req.metadata {
        task = task.with_metadata(k, v);
    }

    let task_id = state
        .queue
        .enqueue(task)
        .await
        .map_err(queue_error)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(EnqueueResponse {
            task_id,
            status: "queued".to_string(),
            message: "Notification enqueued for async processing".to_string(),
        }),
    ))
}

/// Enqueue multiple notifications for asynchronous processing.
#[utoipa::path(
    post,
    path = "/api/v1/send/async/batch",
    tag = "Async Queue",
    request_body = BatchAsyncRequest,
    responses(
        (status = 202, description = "Batch enqueued", body = BatchEnqueueResponse),
        (status = 400, description = "Invalid request", body = ApiError),
    )
)]
pub async fn send_async_batch(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<BatchAsyncRequest>,
) -> Result<(StatusCode, Json<BatchEnqueueResponse>), ApiError> {
    let total = req.items.len();
    let mut results = Vec::with_capacity(total);
    let mut enqueued = 0usize;
    let mut failed = 0usize;

    for (index, item) in req.items.into_iter().enumerate() {
        // Validate provider exists
        let provider = match state.registry.get_by_name(&item.provider) {
            Some(p) => p,
            None => {
                results.push(BatchEnqueueItemResult {
                    index,
                    provider: item.provider,
                    success: false,
                    task_id: None,
                    error: Some("provider not found".to_string()),
                });
                failed += 1;
                continue;
            }
        };

        let config = ProviderConfig {
            values: item.config,
        };

        // Validate config before enqueuing
        if let Err(e) = provider.validate_config(&config) {
            results.push(BatchEnqueueItemResult {
                index,
                provider: item.provider,
                success: false,
                task_id: None,
                error: Some(e.to_string()),
            });
            failed += 1;
            continue;
        }

        let msg = common::build_message(
            &item.text,
            item.title.as_deref(),
            item.format.as_deref(),
            item.priority.as_deref(),
            &item.extra,
        );

        let policy = common::build_retry_policy(item.retry.as_ref(), RetryPolicy::default());

        let mut task = NotificationTask::new(&item.provider, config, msg)
            .with_retry_policy(policy);

        if let Some(url) = &item.callback_url {
            task = task.with_callback_url(url);
        }

        for (k, v) in &item.metadata {
            task = task.with_metadata(k, v);
        }

        match state.queue.enqueue(task).await {
            Ok(task_id) => {
                results.push(BatchEnqueueItemResult {
                    index,
                    provider: item.provider,
                    success: true,
                    task_id: Some(task_id),
                    error: None,
                });
                enqueued += 1;
            }
            Err(e) => {
                results.push(BatchEnqueueItemResult {
                    index,
                    provider: item.provider,
                    success: false,
                    task_id: None,
                    error: Some(e.to_string()),
                });
                failed += 1;
            }
        }
    }

    Ok((
        StatusCode::ACCEPTED,
        Json(BatchEnqueueResponse {
            results,
            enqueued,
            failed,
            total,
        }),
    ))
}

/// Get status of a queued task.
#[utoipa::path(
    get,
    path = "/api/v1/queue/tasks/{task_id}",
    tag = "Async Queue",
    params(("task_id" = String, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Task info", body = TaskInfo),
        (status = 404, description = "Task not found", body = ApiError),
    )
)]
pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskInfo>, ApiError> {
    let task = state
        .queue
        .get_task(&task_id)
        .await
        .map_err(queue_error)?
        .ok_or_else(|| ApiError::not_found(format!("task '{}' not found", task_id)))?;

    Ok(Json(task_to_info(&task)))
}

/// List queued tasks with optional status filter.
#[utoipa::path(
    get,
    path = "/api/v1/queue/tasks",
    tag = "Async Queue",
    params(ListTasksQuery),
    responses(
        (status = 200, description = "Task list", body = Vec<TaskInfo>)
    )
)]
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<TaskInfo>>, ApiError> {
    let status_filter = match query.status.as_deref() {
        Some(s) => Some(parse_task_status(s).ok_or_else(|| {
            ApiError::bad_request(format!(
                "invalid status filter '{s}'; expected one of: queued, processing, completed, failed, cancelled"
            ))
        })?),
        None => None,
    };

    let limit = query.limit.unwrap_or(50).min(1000);

    let tasks = state
        .queue
        .list_tasks(status_filter, limit)
        .await
        .map_err(queue_error)?;

    let infos: Vec<TaskInfo> = tasks.iter().map(task_to_info).collect();
    Ok(Json(infos))
}

/// Get queue statistics.
#[utoipa::path(
    get,
    path = "/api/v1/queue/stats",
    tag = "Async Queue",
    responses(
        (status = 200, description = "Queue statistics", body = StatsResponse)
    )
)]
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, ApiError> {
    let stats: QueueStats = state.queue.stats().await.map_err(queue_error)?;
    Ok(Json(StatsResponse::from(stats)))
}

/// Cancel a queued task.
#[utoipa::path(
    post,
    path = "/api/v1/queue/tasks/{task_id}/cancel",
    tag = "Async Queue",
    params(("task_id" = String, Path, description = "Task ID")),
    responses(
        (status = 200, description = "Cancel result", body = CancelResponse)
    )
)]
pub async fn cancel_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<CancelResponse>, ApiError> {
    let cancelled = state
        .queue
        .cancel(&task_id)
        .await
        .map_err(queue_error)?;

    let message = if cancelled {
        "Task cancelled successfully".to_string()
    } else {
        "Task could not be cancelled (may already be processing or completed)".to_string()
    };

    Ok(Json(CancelResponse {
        task_id,
        cancelled,
        message,
    }))
}

/// Purge completed, failed, and cancelled tasks from the queue.
#[utoipa::path(
    post,
    path = "/api/v1/queue/purge",
    tag = "Async Queue",
    responses(
        (status = 200, description = "Purge result", body = PurgeResponse)
    )
)]
pub async fn purge_tasks(
    State(state): State<AppState>,
) -> Result<Json<PurgeResponse>, ApiError> {
    let purged = state
        .queue
        .purge_completed()
        .await
        .map_err(queue_error)?;

    Ok(Json(PurgeResponse {
        purged,
        message: format!("Purged {} terminal tasks", purged),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::{get, post};
    use axum_test::TestServer;
    use noti_core::ProviderRegistry;

    fn build_test_app() -> Router {
        let state = AppState::new(ProviderRegistry::new());
        Router::new()
            .route("/api/v1/send/async", post(send_async))
            .route("/api/v1/send/async/batch", post(send_async_batch))
            .route("/api/v1/queue/tasks", get(list_tasks))
            .route("/api/v1/queue/tasks/{task_id}", get(get_task))
            .route(
                "/api/v1/queue/tasks/{task_id}/cancel",
                post(cancel_task),
            )
            .route("/api/v1/queue/stats", get(get_stats))
            .route("/api/v1/queue/purge", post(purge_tasks))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_send_async_unknown_provider() {
        let server = TestServer::new(build_test_app());

        let body = serde_json::json!({
            "provider": "nonexistent",
            "text": "hello"
        });

        let resp = server.post("/api/v1/send/async").json(&body).await;
        resp.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_stats_empty() {
        let server = TestServer::new(build_test_app());

        let resp = server.get("/api/v1/queue/stats").await;
        resp.assert_status_ok();

        let stats: StatsResponse = resp.json();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.queued, 0);
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let server = TestServer::new(build_test_app());

        let resp = server.get("/api/v1/queue/tasks").await;
        resp.assert_status_ok();

        let tasks: Vec<TaskInfo> = resp.json();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_get_task_not_found() {
        let server = TestServer::new(build_test_app());

        let resp = server
            .get("/api/v1/queue/tasks/nonexistent-id")
            .await;
        resp.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_cancel_nonexistent_task() {
        let server = TestServer::new(build_test_app());

        let resp = server
            .post("/api/v1/queue/tasks/nonexistent-id/cancel")
            .await;
        resp.assert_status_ok();

        let result: CancelResponse = resp.json();
        assert!(!result.cancelled);
    }

    #[tokio::test]
    async fn test_purge_empty_queue() {
        let server = TestServer::new(build_test_app());

        let resp = server.post("/api/v1/queue/purge").await;
        resp.assert_status_ok();

        let result: PurgeResponse = resp.json();
        assert_eq!(result.purged, 0);
    }

    #[tokio::test]
    async fn test_list_tasks_with_status_filter() {
        let server = TestServer::new(build_test_app());

        let resp = server
            .get("/api/v1/queue/tasks?status=queued&limit=10")
            .await;
        resp.assert_status_ok();

        let tasks: Vec<TaskInfo> = resp.json();
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_list_tasks_invalid_status_filter() {
        let server = TestServer::new(build_test_app());

        let resp = server
            .get("/api/v1/queue/tasks?status=bogus")
            .await;
        resp.assert_status(StatusCode::BAD_REQUEST);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "bad_request");
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("invalid status filter"));
    }

    #[tokio::test]
    async fn test_batch_async_empty_items() {
        let server = TestServer::new(build_test_app());

        let body = serde_json::json!({
            "items": []
        });

        let resp = server.post("/api/v1/send/async/batch").json(&body).await;
        resp.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "validation_failed");
        assert!(body["fields"]["items"].is_array());
    }

    #[tokio::test]
    async fn test_batch_async_all_unknown_providers() {
        let server = TestServer::new(build_test_app());

        let body = serde_json::json!({
            "items": [
                {"provider": "nonexistent1", "text": "hello"},
                {"provider": "nonexistent2", "text": "world"}
            ]
        });

        let resp = server.post("/api/v1/send/async/batch").json(&body).await;
        resp.assert_status(StatusCode::ACCEPTED);

        let result: BatchEnqueueResponse = resp.json();
        assert_eq!(result.total, 2);
        assert_eq!(result.enqueued, 0);
        assert_eq!(result.failed, 2);
        assert!(!result.results[0].success);
        assert!(result.results[0].error.is_some());
    }

    #[tokio::test]
    async fn test_batch_async_response_structure() {
        let server = TestServer::new(build_test_app());

        let body = serde_json::json!({
            "items": [
                {"provider": "unknown", "text": "test"}
            ]
        });

        let resp = server.post("/api/v1/send/async/batch").json(&body).await;
        resp.assert_status(StatusCode::ACCEPTED);

        let result: BatchEnqueueResponse = resp.json();
        assert_eq!(result.total, 1);
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].index, 0);
        assert_eq!(result.results[0].provider, "unknown");
    }
}
