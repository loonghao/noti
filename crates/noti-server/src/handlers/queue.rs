use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use noti_core::{Message, MessageFormat, Priority, ProviderConfig, RetryPolicy};
use noti_queue::{NotificationTask, QueueStats, TaskStatus};

use crate::state::AppState;

// ───────────────────── Request types ─────────────────────

/// Request body for async notification via the queue.
#[derive(Debug, Deserialize)]
pub struct AsyncSendRequest {
    /// Provider name (e.g. "slack", "email", "webhook").
    pub provider: String,
    /// Provider-specific configuration values.
    #[serde(default)]
    pub config: HashMap<String, String>,
    /// Message body text.
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

/// Retry configuration for the API.
#[derive(Debug, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries.
    pub max_retries: Option<u32>,
    /// Base delay in milliseconds.
    pub delay_ms: Option<u64>,
}

/// Query parameters for listing tasks.
#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    /// Filter by status: "queued", "processing", "completed", "failed", "cancelled".
    pub status: Option<String>,
    /// Maximum number of tasks to return (default: 50).
    pub limit: Option<usize>,
}

// ───────────────────── Response types ─────────────────────

/// Response for a successfully enqueued task.
#[derive(Debug, Serialize, Deserialize)]
pub struct EnqueueResponse {
    pub task_id: String,
    pub status: String,
    pub message: String,
}

/// Serializable task info for API responses.
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub queued: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total: usize,
}

/// Response for purge operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct PurgeResponse {
    pub purged: usize,
    pub message: String,
}

/// Response for cancel operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct CancelResponse {
    pub task_id: String,
    pub cancelled: bool,
    pub message: String,
}

// ───────────────────── Batch async types ─────────────────────

/// A single notification item within a batch async request.
#[derive(Debug, Deserialize)]
pub struct BatchAsyncItem {
    /// Provider name (e.g. "slack", "email", "webhook").
    pub provider: String,
    /// Provider-specific configuration values.
    #[serde(default)]
    pub config: HashMap<String, String>,
    /// Message body text.
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

/// Request body for batch async notification enqueue.
#[derive(Debug, Deserialize)]
pub struct BatchAsyncRequest {
    /// List of notifications to enqueue.
    pub items: Vec<BatchAsyncItem>,
}

/// Per-item result in a batch enqueue response.
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
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

fn build_message(
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

fn build_retry_policy(retry: Option<&RetryConfig>) -> RetryPolicy {
    match retry {
        Some(cfg) => {
            let max_retries = cfg.max_retries.unwrap_or(3);
            let delay = std::time::Duration::from_millis(cfg.delay_ms.unwrap_or(1000));
            RetryPolicy::fixed(max_retries, delay)
        }
        None => RetryPolicy::default(),
    }
}

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

fn queue_error_response(e: noti_queue::QueueError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, msg) = match &e {
        noti_queue::QueueError::QueueFull { .. } => {
            (StatusCode::SERVICE_UNAVAILABLE, e.to_string())
        }
        noti_queue::QueueError::NotFound(_) => (StatusCode::NOT_FOUND, e.to_string()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

// ───────────────────── Handlers ─────────────────────

/// POST /api/v1/send/async — Enqueue a notification for async processing.
pub async fn send_async(
    State(state): State<AppState>,
    Json(req): Json<AsyncSendRequest>,
) -> Result<(StatusCode, Json<EnqueueResponse>), (StatusCode, Json<serde_json::Value>)> {
    // Validate provider exists
    let _provider = state.registry.get_by_name(&req.provider).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("provider '{}' not found", req.provider)
            })),
        )
    })?;

    let config = ProviderConfig {
        values: req.config,
    };

    let msg = build_message(
        &req.text,
        req.title.as_deref(),
        req.format.as_deref(),
        req.priority.as_deref(),
        &req.extra,
    );

    let policy = build_retry_policy(req.retry.as_ref());

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
        .map_err(queue_error_response)?;

    Ok((
        StatusCode::ACCEPTED,
        Json(EnqueueResponse {
            task_id,
            status: "queued".to_string(),
            message: "Notification enqueued for async processing".to_string(),
        }),
    ))
}

/// POST /api/v1/send/async/batch — Enqueue multiple notifications for async processing.
pub async fn send_async_batch(
    State(state): State<AppState>,
    Json(req): Json<BatchAsyncRequest>,
) -> Result<(StatusCode, Json<BatchEnqueueResponse>), (StatusCode, Json<serde_json::Value>)> {
    if req.items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "items array must not be empty"
            })),
        ));
    }

    let total = req.items.len();
    let mut results = Vec::with_capacity(total);
    let mut enqueued = 0usize;
    let mut failed = 0usize;

    for (index, item) in req.items.into_iter().enumerate() {
        // Validate provider exists
        if state.registry.get_by_name(&item.provider).is_none() {
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

        let config = ProviderConfig {
            values: item.config,
        };

        let msg = build_message(
            &item.text,
            item.title.as_deref(),
            item.format.as_deref(),
            item.priority.as_deref(),
            &item.extra,
        );

        let policy = build_retry_policy(item.retry.as_ref());

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

/// GET /api/v1/queue/tasks/{task_id} — Get status of a queued task.
pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<TaskInfo>, (StatusCode, Json<serde_json::Value>)> {
    let task = state
        .queue
        .get_task(&task_id)
        .await
        .map_err(queue_error_response)?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("task '{}' not found", task_id)
                })),
            )
        })?;

    Ok(Json(task_to_info(&task)))
}

/// GET /api/v1/queue/tasks — List tasks with optional status filter.
pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<ListTasksQuery>,
) -> Result<Json<Vec<TaskInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let status_filter = query
        .status
        .as_deref()
        .and_then(parse_task_status);

    let limit = query.limit.unwrap_or(50).min(1000);

    let tasks = state
        .queue
        .list_tasks(status_filter, limit)
        .await
        .map_err(queue_error_response)?;

    let infos: Vec<TaskInfo> = tasks.iter().map(task_to_info).collect();
    Ok(Json(infos))
}

/// GET /api/v1/queue/stats — Get queue statistics.
pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let stats: QueueStats = state.queue.stats().await.map_err(queue_error_response)?;

    Ok(Json(StatsResponse {
        queued: stats.queued,
        processing: stats.processing,
        completed: stats.completed,
        failed: stats.failed,
        cancelled: stats.cancelled,
        total: stats.total(),
    }))
}

/// POST /api/v1/queue/tasks/{task_id}/cancel — Cancel a queued task.
pub async fn cancel_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<CancelResponse>, (StatusCode, Json<serde_json::Value>)> {
    let cancelled = state
        .queue
        .cancel(&task_id)
        .await
        .map_err(queue_error_response)?;

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

/// POST /api/v1/queue/purge — Purge completed/failed/cancelled tasks.
pub async fn purge_tasks(
    State(state): State<AppState>,
) -> Result<Json<PurgeResponse>, (StatusCode, Json<serde_json::Value>)> {
    let purged = state
        .queue
        .purge_completed()
        .await
        .map_err(queue_error_response)?;

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
    async fn test_batch_async_empty_items() {
        let server = TestServer::new(build_test_app());

        let body = serde_json::json!({
            "items": []
        });

        let resp = server.post("/api/v1/send/async/batch").json(&body).await;
        resp.assert_status(StatusCode::BAD_REQUEST);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "items array must not be empty");
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
