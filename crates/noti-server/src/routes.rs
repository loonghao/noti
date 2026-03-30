use axum::Router;
use axum::routing::{get, post};

use crate::handlers;
use crate::state::AppState;

/// Build the application router with all API routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))
        // Metrics endpoint
        .route("/api/v1/metrics", get(handlers::metrics::get_metrics))
        // Synchronous notification endpoints
        .route("/api/v1/send", post(handlers::send::send_notification))
        .route("/api/v1/send/batch", post(handlers::send::send_batch))
        // Async queue-based notification
        .route("/api/v1/send/async", post(handlers::queue::send_async))
        .route(
            "/api/v1/send/async/batch",
            post(handlers::queue::send_async_batch),
        )
        // Status endpoints
        .route(
            "/api/v1/status/{notification_id}",
            get(handlers::status::get_status),
        )
        .route("/api/v1/status", get(handlers::status::get_all_statuses))
        // Template endpoints
        .route("/api/v1/templates", post(handlers::templates::create_template))
        .route("/api/v1/templates", get(handlers::templates::list_templates))
        .route(
            "/api/v1/templates/{name}",
            get(handlers::templates::get_template),
        )
        .route(
            "/api/v1/templates/{name}/render",
            post(handlers::templates::render_template),
        )
        // Provider info endpoints
        .route("/api/v1/providers", get(handlers::providers::list_providers))
        .route(
            "/api/v1/providers/{name}",
            get(handlers::providers::get_provider),
        )
        // Queue management endpoints
        .route("/api/v1/queue/stats", get(handlers::queue::get_stats))
        .route("/api/v1/queue/tasks", get(handlers::queue::list_tasks))
        .route(
            "/api/v1/queue/tasks/{task_id}",
            get(handlers::queue::get_task),
        )
        .route(
            "/api/v1/queue/tasks/{task_id}/cancel",
            post(handlers::queue::cancel_task),
        )
        .route("/api/v1/queue/purge", post(handlers::queue::purge_tasks))
        .with_state(state)
}
