use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use tracing::{info, warn};

use noti_core::{ProviderConfig, RetryPolicy};
use noti_queue::NotificationTask;

use crate::handlers::common;
use crate::handlers::error::ApiError;
use crate::middleware::validated_json::ValidatedJson;
use crate::state::AppState;

use super::dto::{
    AsyncSendRequest, BatchAsyncRequest, BatchEnqueueItemResult, BatchEnqueueResponse,
    CancelResponse, DeleteDlqResponse, DlqListResponse, DlqStatsResponse, EnqueueResponse,
    ListDlqQuery, ListTasksQuery, PurgeResponse, RequeueResponse, StatsResponse, TaskInfo,
};
use super::service::{
    dlq_entry_to_info, parse_scheduled_time, parse_task_status, queue_error, task_to_info,
};

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

    let config = ProviderConfig { values: req.config };

    if let Err(e) = provider.validate_config(&config) {
        use crate::handlers::error::codes;
        return Err(ApiError::bad_request(e.to_string()).with_code(codes::CONFIG_VALIDATION_FAILED));
    }

    let msg = common::build_message(
        &req.text,
        req.title.as_deref(),
        req.format.as_deref(),
        req.priority.as_deref(),
        &req.extra,
    );

    let policy = common::build_retry_policy(req.retry.as_ref(), RetryPolicy::default());

    // Parse schedule/delay
    let available_at = parse_scheduled_time(req.delay_seconds, req.scheduled_at.as_deref())?;

    let mut task = NotificationTask::new(&req.provider, config, msg).with_retry_policy(policy);

    if let Some(at) = available_at {
        task = task.with_available_at(at);
    }

    if let Some(url) = &req.callback_url {
        task = task.with_callback_url(url);
    }

    if let Some(secret) = &req.callback_secret {
        task = task.with_callback_secret(secret);
    }

    for (k, v) in &req.metadata {
        task = task.with_metadata(k, v);
    }

    let scheduled = available_at.is_some();
    let task_id = state.queue.enqueue(task).await.map_err(queue_error)?;

    let message = if scheduled {
        "Notification scheduled for delayed processing".to_string()
    } else {
        "Notification enqueued for async processing".to_string()
    };

    info!(
        task_id = %task_id,
        provider = %req.provider,
        scheduled,
        "task enqueued"
    );

    Ok((
        StatusCode::ACCEPTED,
        Json(EnqueueResponse {
            task_id,
            status: "queued".to_string(),
            message,
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

        // Parse schedule/delay for this item
        let available_at =
            match parse_scheduled_time(item.delay_seconds, item.scheduled_at.as_deref()) {
                Ok(at) => at,
                Err(e) => {
                    results.push(BatchEnqueueItemResult {
                        index,
                        provider: item.provider,
                        success: false,
                        task_id: None,
                        error: Some(e.message),
                    });
                    failed += 1;
                    continue;
                }
            };

        let mut task = NotificationTask::new(&item.provider, config, msg).with_retry_policy(policy);

        if let Some(at) = available_at {
            task = task.with_available_at(at);
        }

        if let Some(url) = &item.callback_url {
            task = task.with_callback_url(url);
        }

        if let Some(secret) = &item.callback_secret {
            task = task.with_callback_secret(secret);
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

    info!(total, enqueued, failed, "batch async enqueue completed");

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
    use crate::handlers::error::codes;
    let task = state
        .queue
        .get_task(&task_id)
        .await
        .map_err(queue_error)?
        .ok_or_else(|| {
            ApiError::not_found(format!("task '{}' not found", task_id))
                .with_code(codes::TASK_NOT_FOUND)
        })?;

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
    use crate::handlers::error::codes;
    let status_filter = match query.status.as_deref() {
        Some(s) => Some(parse_task_status(s).ok_or_else(|| {
            ApiError::bad_request(format!(
                "invalid status filter '{s}'; expected one of: queued, processing, completed, failed, cancelled"
            ))
            .with_code(codes::INVALID_PARAMETER)
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
pub async fn get_stats(State(state): State<AppState>) -> Result<Json<StatsResponse>, ApiError> {
    use noti_queue::QueueStats;
    let stats: QueueStats = state.queue.stats().await.map_err(queue_error)?;
    let dlq_stats = state.queue.dlq_stats().await.map_err(queue_error)?;

    let mut response = StatsResponse::from(stats);
    response.dlq_size = dlq_stats.dlq_size;

    Ok(Json(response))
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
    let cancelled = state.queue.cancel(&task_id).await.map_err(queue_error)?;

    if cancelled {
        info!(task_id = %task_id, "task cancelled");
    } else {
        warn!(task_id = %task_id, "task cancel failed (already processing or completed)");
    }

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
pub async fn purge_tasks(State(state): State<AppState>) -> Result<Json<PurgeResponse>, ApiError> {
    let purged = state.queue.purge_completed().await.map_err(queue_error)?;

    info!(purged, "queue purge completed");

    Ok(Json(PurgeResponse {
        purged,
        message: format!("Purged {} terminal tasks", purged),
    }))
}

// ───────────────────── DLQ handlers ─────────────────────

/// List all entries in the Dead Letter Queue (tasks that exhausted all retries).
#[utoipa::path(
    get,
    path = "/api/v1/queue/dlq",
    tag = "Async Queue",
    params(ListDlqQuery),
    responses(
        (status = 200, description = "DLQ entry list", body = DlqListResponse)
    )
)]
pub async fn list_dlq(
    State(state): State<AppState>,
    Query(query): Query<ListDlqQuery>,
) -> Result<Json<DlqListResponse>, ApiError> {
    let limit = query.limit.unwrap_or(50).min(1000);

    let entries = state.queue.list_dlq(limit).await.map_err(queue_error)?;
    let total = entries.len();

    let infos: Vec<_> = entries.iter().map(dlq_entry_to_info).collect();

    Ok(Json(DlqListResponse { entries: infos, total }))
}

/// Get dead letter queue statistics.
#[utoipa::path(
    get,
    path = "/api/v1/queue/dlq/stats",
    tag = "Async Queue",
    responses(
        (status = 200, description = "DLQ statistics", body = DlqStatsResponse)
    )
)]
pub async fn get_dlq_stats(State(state): State<AppState>) -> Result<Json<DlqStatsResponse>, ApiError> {
    let dlq_stats = state.queue.dlq_stats().await.map_err(queue_error)?;

    Ok(Json(DlqStatsResponse {
        dlq_size: dlq_stats.dlq_size,
    }))
}

/// Requeue a task from the DLQ back into the main queue.
#[utoipa::path(
    post,
    path = "/api/v1/queue/dlq/{task_id}/requeue",
    tag = "Async Queue",
    params(("task_id" = String, Path, description = "Task ID to requeue")),
    responses(
        (status = 200, description = "Requeue result", body = RequeueResponse),
        (status = 404, description = "Task not found or not in DLQ", body = ApiError),
    )
)]
pub async fn requeue_from_dlq(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<RequeueResponse>, ApiError> {
    use crate::handlers::error::codes;
    use noti_queue::TaskStatus;

    // Fetch the task and verify it is in the Failed state (which means it's in the DLQ)
    let task = state
        .queue
        .get_task(&task_id)
        .await
        .map_err(queue_error)?
        .ok_or_else(|| {
            ApiError::not_found(format!("task '{}' not found", task_id))
                .with_code(codes::TASK_NOT_FOUND)
        })?;

    if task.status != TaskStatus::Failed {
        return Err(ApiError::bad_request(format!(
            "task '{}' is not in the DLQ (status: {}); only failed tasks can be requeued",
            task_id, task.status
        ))
        .with_code(codes::INVALID_PARAMETER));
    }

    state
        .queue
        .requeue_from_dlq(&task_id)
        .await
        .map_err(queue_error)?;

    info!(task_id = %task_id, "task requeued from DLQ");

    Ok(Json(RequeueResponse {
        task_id,
        requeued: true,
        message: "Task requeued from DLQ successfully".to_string(),
    }))
}
#[utoipa::path(
    delete,
    path = "/api/v1/queue/dlq/{task_id}",
    tag = "Async Queue",
    params(("task_id" = String, Path, description = "Task ID to delete from DLQ")),
    responses(
        (status = 200, description = "Delete result", body = DeleteDlqResponse),
        (status = 404, description = "Task not found or not in DLQ", body = ApiError),
    )
)]
pub async fn delete_from_dlq(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<DeleteDlqResponse>, ApiError> {
    use crate::handlers::error::codes;
    use noti_queue::TaskStatus;

    // Verify task exists and is in Failed state (which means it's in the DLQ)
    let task = state
        .queue
        .get_task(&task_id)
        .await
        .map_err(queue_error)?
        .ok_or_else(|| {
            ApiError::not_found(format!("task '{}' not found", task_id))
                .with_code(codes::TASK_NOT_FOUND)
        })?;

    if task.status != TaskStatus::Failed {
        return Err(ApiError::bad_request(format!(
            "task '{}' is not in the DLQ (status: {}); only failed tasks can be deleted via DLQ endpoint",
            task_id, task.status
        ))
        .with_code(codes::INVALID_PARAMETER));
    }

    state
        .queue
        .delete_from_dlq(&task_id)
        .await
        .map_err(queue_error)?;

    info!(task_id = %task_id, "task deleted from DLQ");

    Ok(Json(DeleteDlqResponse {
        task_id,
        success: true,
        message: "Task permanently removed from DLQ".to_string(),
    }))
}
