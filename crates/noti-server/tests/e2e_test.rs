//! End-to-end tests that start a real TCP server and send HTTP requests via `reqwest`.
//!
//! The bulk of e2e tests have been split into focused files by concern:
//! - `e2e_health_meta.rs`      — health check and meta endpoints
//! - `e2e_providers.rs`        — provider list / detail endpoints
//! - `e2e_send.rs`             — synchronous send and template lifecycle
//! - `e2e_openapi.rs`          — OpenAPI / Swagger endpoint validation
//! - `e2e_status.rs`           — notification status endpoints
//! - `e2e_middleware.rs`       — auth, rate-limit, CORS, body-limit, request-ID, validation
//! - `e2e_worker_callback.rs`  — worker processing and webhook callbacks
//! - `e2e_priority.rs`         — priority ordering and retry behavior
//! - `e2e_per_ip_rate.rs`      — per-IP rate limiting
//! - `e2e_sqlite_priority.rs`  — SQLite queue priority ordering
//! - `e2e_batch_priority.rs`   — batch async with priorities and graceful shutdown
//! - `e2e_stale_recovery.rs`   — SQLite stale task recovery after restart
//! - `e2e_queue_purge.rs`      — queue purge semantics
//! - `e2e_templates.rs`        — template CRUD and rendering depth tests
//! - `e2e_concurrent.rs`       — concurrent task processing
//! - `e2e_errors.rs`           — error response structure consistency
//! - `e2e_batch.rs`            — batch async with mixed providers and priorities
//! - `e2e_retry.rs`            — retry policies for batch and concurrent scenarios
//! - `e2e_backoff.rs`          — backoff delay timing and exponential backoff
//! - `e2e_scheduled.rs`        — scheduled / delayed send

mod common;

use common::test_client;
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Async queue (real HTTP) ─────────────────────

dual_backend_test!(
    basic,
    e2e_async_send_and_query,
    e2e_sqlite_async_send_query_cancel_purge,
    |spawn_fn, label| {
        let base = spawn_fn().await;
        let client = test_client();

        // Enqueue
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "slack",
                "text": format!("{label}async test"),
                "config": {
                    "webhook_url": "https://hooks.slack.com/services/T00/B00/e2e"
                }
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "queued");
        let task_id = body["task_id"].as_str().unwrap().to_string();

        // Get task
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["id"], task_id);
        assert_eq!(body["provider"], "slack");

        // Queue stats
        let resp = client
            .get(format!("{base}/api/v1/queue/stats"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.unwrap();
        assert!(body["total"].as_u64().unwrap() >= 1);

        // Cancel task
        let resp = client
            .post(format!("{base}/api/v1/queue/tasks/{task_id}/cancel"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.unwrap();
        assert!(body["cancelled"].as_bool().unwrap());

        // Purge
        let resp = client
            .post(format!("{base}/api/v1/queue/purge"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.unwrap();
        assert!(body["purged"].as_u64().unwrap() >= 1);
    }
);

dual_backend_test!(
    basic,
    e2e_async_batch_send,
    e2e_sqlite_batch_async_send,
    |spawn_fn, label| {
        let base = spawn_fn().await;
        let client = test_client();

        let resp = client
            .post(format!("{base}/api/v1/send/async/batch"))
            .json(&json!({
                "items": [
                    {
                        "provider": "slack",
                        "text": format!("{label}batch item 1"),
                        "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/batch1"}
                    },
                    {
                        "provider": "nonexistent",
                        "text": format!("{label}batch item 2")
                    }
                ]
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["total"], 2);
        assert_eq!(body["enqueued"], 1);
        assert_eq!(body["failed"], 1);
    }
);
