mod common;

use std::time::Duration;

use common::{
    spawn_server, spawn_server_sqlite_with_workers, spawn_server_with_cors_permissive,
    spawn_server_with_request_id, test_client, wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ═════════════════════════════════════════════════════════════════════════════
// Health Check structure e2e tests
// ═════════════════════════════════════════════════════════════════════════════

/// Verify `/health` returns expected JSON structure with all documented fields.
#[tokio::test]
async fn e2e_health_response_has_documented_structure() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    // Top-level fields
    assert!(body["status"].is_string(), "missing 'status' field");
    assert!(body["version"].is_string(), "missing 'version' field");
    assert!(
        body["uptime_seconds"].is_number(),
        "missing 'uptime_seconds' field"
    );
    // Dependencies
    let deps = &body["dependencies"];
    assert!(deps["queue"]["status"].is_string(), "missing queue.status");
    assert!(
        deps["providers"]["status"].is_string(),
        "missing providers.status"
    );
    // Providers should be "up" (125+ registered)
    assert_eq!(deps["providers"]["status"], "up");
    assert_eq!(body["status"], "ok");
}

// ═════════════════════════════════════════════════════════════════════════════
// Request ID generation e2e tests
// ═════════════════════════════════════════════════════════════════════════════

/// Verify the server generates a UUID v4 request ID when none is provided.
#[tokio::test]
async fn e2e_request_id_generated_is_valid_uuid() {
    let base = spawn_server_with_request_id().await;
    let client = test_client();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let id_header = resp
        .headers()
        .get("x-request-id")
        .expect("response should have x-request-id header");
    let id_str = id_header.to_str().unwrap();
    // UUID v4 format: 8-4-4-4-12 hex chars
    assert_eq!(id_str.len(), 36, "UUID should be 36 chars");
    assert!(id_str.contains('-'), "UUID should contain dashes");
}

/// Verify the server preserves a client-provided request ID.
#[tokio::test]
async fn e2e_request_id_preserves_client_provided() {
    let base = spawn_server_with_request_id().await;
    let client = test_client();
    let custom_id = "my-custom-trace-id-abc";

    let resp = client
        .get(format!("{base}/health"))
        .header("X-Request-Id", custom_id)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let id_header = resp
        .headers()
        .get("x-request-id")
        .expect("response should echo x-request-id");
    assert_eq!(id_header.to_str().unwrap(), custom_id);
}

// ═════════════════════════════════════════════════════════════════════════════
// CORS middleware e2e tests
// ═════════════════════════════════════════════════════════════════════════════

/// Verify permissive CORS returns wildcard origin and allows arbitrary origin header.
#[tokio::test]
async fn e2e_cors_permissive_returns_wildcard_for_arbitrary_origin() {
    let base = spawn_server_with_cors_permissive().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/health"))
        .header("Origin", "https://arbitrary.example.com")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let allow_origin = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("should have CORS allow-origin header");
    assert_eq!(allow_origin.to_str().unwrap(), "*");
}

// ═════════════════════════════════════════════════════════════════════════════
// Scheduled / delayed send e2e tests
// ═════════════════════════════════════════════════════════════════════════════

// Verify that `delay_seconds` causes the task to be held in the queue.
// A delay of 2 seconds should prevent immediate processing.
dual_backend_test!(
    with_workers,
    e2e_scheduled_send_delay_seconds_holds_task,
    e2e_sqlite_scheduled_send_delay_seconds_holds_task,
    |spawn_fn, label| {
        let (base, worker_handle) = spawn_fn().await;
        let client = test_client();

        let start = std::time::Instant::now();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": "delayed notification",
                "delay_seconds": 2
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        assert!(body["message"].as_str().unwrap().contains("scheduled"));

        let task_id = body["task_id"].as_str().unwrap().to_string();

        // The task should still be queued immediately after enqueueing
        tokio::time::sleep(Duration::from_millis(200)).await;
        let task_resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let task: Value = task_resp.json().await.unwrap();
        assert!(
            task["scheduled_at"].is_string(),
            "{label}delayed task should have scheduled_at in response"
        );

        // Wait for the task to complete (should take ~2 seconds)
        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        let elapsed = start.elapsed();

        assert_eq!(task["status"], "completed");
        assert!(
            elapsed >= Duration::from_millis(1800),
            "{label}delayed task should wait at least ~2s, but elapsed was {elapsed:?}"
        );

        worker_handle.shutdown_and_join().await;
    }
);

// Verify that `delay_seconds=0` is treated as immediate (no delay).
dual_backend_test!(
    with_workers,
    e2e_scheduled_send_delay_zero_is_immediate,
    e2e_sqlite_scheduled_send_delay_zero_is_immediate,
    |spawn_fn, label| {
        let (base, worker_handle) = spawn_fn().await;
        let client = test_client();

        let start = std::time::Instant::now();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": "immediate notification",
                "delay_seconds": 0
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        // Should say "enqueued" not "scheduled"
        assert!(body["message"].as_str().unwrap().contains("enqueued"));

        let task_id = body["task_id"].as_str().unwrap().to_string();
        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        let elapsed = start.elapsed();

        assert_eq!(task["status"], "completed");
        // Should complete quickly (well under 2s)
        assert!(
            elapsed < Duration::from_secs(2),
            "{label}delay_seconds=0 should not cause delay, elapsed was {elapsed:?}"
        );

        worker_handle.shutdown_and_join().await;
    }
);

// Verify that `scheduled_at` with an RFC 3339 timestamp works.
dual_backend_test!(
    with_workers,
    e2e_scheduled_send_rfc3339_timestamp,
    e2e_sqlite_scheduled_send_rfc3339_timestamp,
    |spawn_fn, label| {
        let (base, worker_handle) = spawn_fn().await;
        let client = test_client();

        // Schedule 2 seconds from now
        let scheduled_time = std::time::SystemTime::now() + Duration::from_secs(2);
        let ts = humantime::format_rfc3339(scheduled_time).to_string();

        let start = std::time::Instant::now();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": "scheduled at timestamp",
                "scheduled_at": ts
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        let task_id = body["task_id"].as_str().unwrap().to_string();

        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        let elapsed = start.elapsed();

        assert_eq!(task["status"], "completed");
        assert!(
            elapsed >= Duration::from_millis(1800),
            "{label}scheduled_at task should wait at least ~2s, but elapsed was {elapsed:?}"
        );

        worker_handle.shutdown_and_join().await;
    }
);

// Verify that providing both `delay_seconds` and `scheduled_at` returns 400.
dual_backend_test!(
    basic,
    e2e_scheduled_send_mutually_exclusive_error,
    e2e_sqlite_scheduled_send_mutually_exclusive_error,
    |spawn_fn, label| {
        let base = spawn_fn().await;
        let client = test_client();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "slack",
                "text": "conflicting schedule params",
                "config": {"webhook_url": "https://hooks.slack.com/services/test"},
                "delay_seconds": 60,
                "scheduled_at": "2030-01-15T10:30:00Z"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: Value = resp.json().await.unwrap();
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("mutually exclusive"),
            "{label}expected mutually exclusive error"
        );
    }
);

// Verify that an invalid `scheduled_at` format returns 400.
dual_backend_test!(
    basic,
    e2e_scheduled_send_invalid_timestamp_format,
    e2e_sqlite_scheduled_send_invalid_timestamp_format,
    |spawn_fn, label| {
        let base = spawn_fn().await;
        let client = test_client();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "slack",
                "text": "bad timestamp",
                "config": {"webhook_url": "https://hooks.slack.com/services/test"},
                "scheduled_at": "not-a-valid-timestamp"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: Value = resp.json().await.unwrap();
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("invalid scheduled_at"),
            "{label}expected invalid scheduled_at error"
        );
    }
);

// Verify that `task_info.scheduled_at` is absent for non-delayed tasks.
dual_backend_test!(
    basic,
    e2e_scheduled_send_no_scheduled_at_for_immediate,
    e2e_sqlite_scheduled_send_no_scheduled_at_for_immediate,
    |spawn_fn, label| {
        let base = spawn_fn().await;
        let client = test_client();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "slack",
                "text": "immediate task",
                "config": {"webhook_url": "https://hooks.slack.com/services/test"}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        let task_id = body["task_id"].as_str().unwrap();

        let task_resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let task: Value = task_resp.json().await.unwrap();
        assert!(
            task["scheduled_at"].is_null()
                || !task.as_object().unwrap().contains_key("scheduled_at"),
            "{label}immediate task should not have scheduled_at"
        );
    }
);

/// Verify OpenAPI schema includes delay_seconds and scheduled_at fields.
#[tokio::test]
async fn e2e_openapi_schema_has_scheduled_send_fields() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let spec: Value = resp.json().await.unwrap();
    let async_schema = &spec["components"]["schemas"]["AsyncSendRequest"]["properties"];
    assert!(
        async_schema["delay_seconds"].is_object(),
        "AsyncSendRequest should have delay_seconds field in OpenAPI schema"
    );
    assert!(
        async_schema["scheduled_at"].is_object(),
        "AsyncSendRequest should have scheduled_at field in OpenAPI schema"
    );

    let task_schema = &spec["components"]["schemas"]["TaskInfo"]["properties"];
    assert!(
        task_schema["scheduled_at"].is_object(),
        "TaskInfo should have scheduled_at field in OpenAPI schema"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Scheduled / delayed send e2e tests — SQLite-only
// ═════════════════════════════════════════════════════════════════════════════

/// SQLite: Verify batch async with mixed delay_seconds per item.
#[tokio::test]
async fn e2e_sqlite_scheduled_send_batch_mixed_delays() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "immediate item",
                    "delay_seconds": 0
                },
                {
                    "provider": "mock-ok",
                    "text": "delayed item",
                    "delay_seconds": 1
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"].as_u64().unwrap(), 2);
    assert_eq!(body["failed"].as_u64().unwrap(), 0);

    let immediate_id = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let delayed_id = body["results"][1]["task_id"].as_str().unwrap().to_string();

    // The immediate task should complete first
    let immediate_task = wait_for_terminal_status(&client, &base, &immediate_id).await;
    assert_eq!(immediate_task["status"], "completed");

    // The delayed task should also eventually complete
    let delayed_task = wait_for_terminal_status(&client, &base, &delayed_id).await;
    assert_eq!(delayed_task["status"], "completed");

    worker_handle.shutdown_and_join().await;
}
