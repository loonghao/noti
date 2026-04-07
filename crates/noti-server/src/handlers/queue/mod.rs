//! Async queue HTTP handlers, split across focused sub-modules:
//!
//! - [`dto`]      — request/response DTO types
//! - [`service`]  — mapping helpers and business logic (parse_scheduled_time, etc.)
//! - [`handlers`] — axum HTTP handler functions

pub mod dto;
pub mod handlers;
pub mod service;

// Re-export public types to maintain the same public API as the flat queue.rs.
pub use dto::{
    AsyncSendRequest, BatchAsyncRequest, BatchEnqueueItemResult, BatchEnqueueResponse,
    CancelResponse, DeleteFromDlqResponse, DlqEntryInfo, DlqListResponse, DlqStatsResponse,
    EnqueueResponse, ListDlqQuery, ListTasksQuery, PurgeResponse, RequeueResponse,
    StatsResponse, TaskInfo,
};
pub use handlers::{
    cancel_task, delete_from_dlq, get_dlq_stats, get_stats, get_task, list_dlq, list_tasks,
    purge_tasks, requeue_from_dlq, send_async, send_async_batch,
};

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::StatusCode;
    use axum::routing::{delete, get, post};
    use axum_test::TestServer;
    use noti_core::ProviderRegistry;

    use super::*;
    use crate::state::AppState;

    fn build_test_app() -> Router {
        let state = AppState::new(ProviderRegistry::new());
        Router::new()
            .route("/api/v1/send/async", post(send_async))
            .route("/api/v1/send/async/batch", post(send_async_batch))
            .route("/api/v1/queue/tasks", get(list_tasks))
            .route("/api/v1/queue/tasks/{task_id}", get(get_task))
            .route("/api/v1/queue/tasks/{task_id}/cancel", post(cancel_task))
            .route("/api/v1/queue/stats", get(get_stats))
            .route("/api/v1/queue/purge", post(purge_tasks))
            .route("/api/v1/queue/dlq", get(list_dlq))
            .route("/api/v1/queue/dlq/stats", get(get_dlq_stats))
            .route("/api/v1/queue/dlq/{task_id}/requeue", post(requeue_from_dlq))
            .route("/api/v1/queue/dlq/{task_id}", delete(delete_from_dlq))
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

        let resp = server.get("/api/v1/queue/tasks/nonexistent-id").await;
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

        let resp = server.get("/api/v1/queue/tasks?status=bogus").await;
        resp.assert_status(StatusCode::BAD_REQUEST);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "bad_request");
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("invalid status filter")
        );
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
