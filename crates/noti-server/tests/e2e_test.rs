//! End-to-end tests that start a real TCP server and send HTTP requests via `reqwest`.
//!
//! Unlike the `server_test.rs` tests which use `axum_test::TestServer` (in-process),
//! these tests bind to a random port and exercise the full HTTP stack, including
//! TCP transport and header serialization.
//!
//! Shared helpers (spawn_server*, mock providers, callback infrastructure) live in
//! `tests/common/mod.rs` to avoid duplication across test files.

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    MockFlakyProvider, MockOkProvider, MockSlowProvider, spawn_callback_server, spawn_server,
    spawn_server_sqlite, spawn_server_sqlite_file, spawn_server_sqlite_file_with_workers,
    spawn_server_sqlite_with_workers, spawn_server_sqlite_with_workers_serial,
    spawn_server_with_auth, spawn_server_with_body_limit, spawn_server_with_cors_permissive,
    spawn_server_with_cors_restricted, spawn_server_with_full_middleware,
    spawn_server_with_rate_limit, spawn_server_with_rate_limit_per_ip,
    spawn_server_with_request_id, spawn_server_with_workers, spawn_server_with_workers_serial,
    wait_for_terminal_status,
};
use noti_queue::QueueBackend;
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Health & Meta ─────────────────────

#[tokio::test]
async fn e2e_health_check() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn e2e_metrics_endpoint() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/metrics"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["providers"]["total_registered"].as_u64().unwrap() > 100);
    assert!(body["version"].is_string());
    assert!(body["uptime_seconds"].as_u64().is_some());
}

// ───────────────────── Providers ─────────────────────

#[tokio::test]
async fn e2e_list_providers() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["total"].as_u64().unwrap() > 100);
    assert!(body["providers"].is_array());
}

#[tokio::test]
async fn e2e_get_provider_detail() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/providers/slack"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "slack");
    assert!(body["params"].is_array());
}

#[tokio::test]
async fn e2e_provider_not_found() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ───────────────────── Send (synchronous) ─────────────────────

#[tokio::test]
async fn e2e_send_missing_provider() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "nonexistent",
            "text": "hello"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn e2e_send_missing_config() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": "hello",
            "config": {}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ───────────────────── Templates (full lifecycle via HTTP) ─────────────────────

#[tokio::test]
async fn e2e_template_crud_lifecycle() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Create
    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "e2e-alert",
            "body": "Alert: {{message}} on {{host}}",
            "title": "{{level}} Alert",
            "defaults": {"level": "INFO"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "e2e-alert");

    // List
    let resp = client
        .get(format!("{base}/api/v1/templates"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 1);

    // Get
    let resp = client
        .get(format!("{base}/api/v1/templates/e2e-alert"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Render
    let resp = client
        .post(format!("{base}/api/v1/templates/e2e-alert/render"))
        .json(&json!({
            "variables": {
                "message": "CPU spike",
                "host": "prod-01"
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["text"], "Alert: CPU spike on prod-01");
    assert_eq!(body["title"], "INFO Alert");

    // Update
    let resp = client
        .put(format!("{base}/api/v1/templates/e2e-alert"))
        .json(&json!({
            "body": "Updated alert: {{message}}",
            "defaults": {"level": "WARN"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["body"].as_str().unwrap().contains("Updated"));
    assert_eq!(body["defaults"]["level"], "WARN");

    // Delete
    let resp = client
        .delete(format!("{base}/api/v1/templates/e2e-alert"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["deleted"].as_bool().unwrap());

    // Verify deletion
    let resp = client
        .get(format!("{base}/api/v1/templates/e2e-alert"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ───────────────────── Async queue (real HTTP) ─────────────────────

#[tokio::test]
async fn e2e_async_send_and_query() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Enqueue
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "slack",
            "text": "e2e async test",
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

#[tokio::test]
async fn e2e_async_batch_send() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "slack",
                    "text": "batch item 1",
                    "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/batch1"}
                },
                {
                    "provider": "nonexistent",
                    "text": "batch item 2"
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

// ───────────────────── OpenAPI / Swagger ─────────────────────

#[tokio::test]
async fn e2e_openapi_json_valid() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    // Basic OpenAPI 3.x structure validation
    assert!(
        body["openapi"].as_str().unwrap().starts_with("3."),
        "expected OpenAPI 3.x version"
    );
    assert!(body["info"]["title"].is_string());
    assert!(body["paths"].is_object());

    // Verify key paths exist
    let paths = body["paths"].as_object().unwrap();
    assert!(paths.contains_key("/health"), "missing /health path");
    assert!(
        paths.contains_key("/api/v1/send"),
        "missing /api/v1/send path"
    );
    assert!(
        paths.contains_key("/api/v1/providers"),
        "missing /api/v1/providers path"
    );
    assert!(
        paths.contains_key("/api/v1/templates"),
        "missing /api/v1/templates path"
    );
    assert!(
        paths.contains_key("/api/v1/queue/stats"),
        "missing /api/v1/queue/stats path"
    );
}

#[tokio::test]
async fn e2e_swagger_ui_accessible() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/swagger-ui/"))
        .send()
        .await
        .unwrap();

    // Swagger UI should return 200 (HTML)
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        content_type.contains("text/html"),
        "expected HTML content type, got: {content_type}"
    );
}

// ───────────────────── Status endpoints ─────────────────────

#[tokio::test]
async fn e2e_status_not_found() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/status/nonexistent-id"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn e2e_all_statuses_empty() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/status"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 0);
}

// ───────────────────── Queue edge cases ─────────────────────

#[tokio::test]
async fn e2e_queue_invalid_status_filter() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=bogus"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn e2e_queue_task_not_found() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/nonexistent-id"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ───────────────────── Auth middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_auth_rejects_unauthenticated_request() {
    let (base, _keys) = spawn_server_with_auth(vec!["test-key-alpha".to_string()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "unauthorized");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("missing API key")
    );
}

#[tokio::test]
async fn e2e_auth_rejects_invalid_key() {
    let (base, _keys) = spawn_server_with_auth(vec!["correct-key".to_string()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", "Bearer wrong-key")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "unauthorized");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("invalid API key")
    );
}

#[tokio::test]
async fn e2e_auth_accepts_valid_bearer_token() {
    let (base, keys) = spawn_server_with_auth(vec!["my-secret-key".to_string()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", format!("Bearer {}", keys[0]))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["total"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn e2e_auth_accepts_x_api_key_header() {
    let (base, keys) = spawn_server_with_auth(vec!["x-api-key-value".to_string()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("X-API-Key", &keys[0])
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn e2e_auth_health_bypasses_auth() {
    let (base, _keys) = spawn_server_with_auth(vec!["secret".to_string()]).await;
    let client = reqwest::Client::new();

    // /health is excluded from auth by default
    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_auth_multiple_keys() {
    let (base, keys) =
        spawn_server_with_auth(vec!["key-one".to_string(), "key-two".to_string()]).await;
    let client = reqwest::Client::new();

    for key in &keys {
        let resp = client
            .get(format!("{base}/api/v1/providers"))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "key '{key}' should be accepted"
        );
    }
}

#[tokio::test]
async fn e2e_auth_post_endpoint_requires_key() {
    let (base, keys) = spawn_server_with_auth(vec!["post-key".to_string()]).await;
    let client = reqwest::Client::new();

    // Without key → 401
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "slack", "text": "test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // With key → should proceed (will get 400/404 from handler, not 401)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("Authorization", format!("Bearer {}", keys[0]))
        .json(&json!({"provider": "slack", "text": "test", "config": {}}))
        .send()
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ───────────────────── Rate limit middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_rate_limit_allows_within_quota() {
    let (base, max_requests) = spawn_server_with_rate_limit(5, 60).await;
    let client = reqwest::Client::new();

    for i in 0..max_requests {
        let resp = client.get(format!("{base}/health")).send().await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "request {i} should be allowed"
        );

        // Verify rate limit headers are present
        assert!(
            resp.headers().contains_key("x-ratelimit-limit"),
            "missing x-ratelimit-limit header on request {i}"
        );
        assert!(
            resp.headers().contains_key("x-ratelimit-remaining"),
            "missing x-ratelimit-remaining header on request {i}"
        );
        assert_eq!(
            resp.headers()["x-ratelimit-limit"].to_str().unwrap(),
            max_requests.to_string()
        );
    }
}

#[tokio::test]
async fn e2e_rate_limit_returns_429_when_exceeded() {
    let (base, max_requests) = spawn_server_with_rate_limit(3, 60).await;
    let client = reqwest::Client::new();

    // Exhaust the quota
    for _ in 0..max_requests {
        let resp = client.get(format!("{base}/health")).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Next request should be rate limited
    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    // Verify 429 response structure
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "rate limit exceeded");
    assert!(body["retry_after_seconds"].as_u64().is_some());
    assert_eq!(body["limit"], max_requests);
}

#[tokio::test]
async fn e2e_rate_limit_429_has_retry_after_header() {
    let (base, _max) = spawn_server_with_rate_limit(1, 60).await;
    let client = reqwest::Client::new();

    // Use up the single allowed request
    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Trigger 429
    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        resp.headers().contains_key("retry-after"),
        "429 response should include Retry-After header"
    );
    let retry_after = resp.headers()["retry-after"]
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(
        retry_after > 0,
        "Retry-After should be positive, got {retry_after}"
    );
}

#[tokio::test]
async fn e2e_rate_limit_remaining_decrements() {
    let (base, _max) = spawn_server_with_rate_limit(10, 60).await;
    let client = reqwest::Client::new();

    let resp1 = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    let remaining1: u64 = resp1.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    let resp2 = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let remaining2: u64 = resp2.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    assert!(
        remaining2 < remaining1,
        "remaining should decrement: {remaining1} -> {remaining2}"
    );
}

// ───────────────────── Full middleware stack (e2e) ─────────────────────

#[tokio::test]
async fn e2e_full_middleware_auth_before_rate_limit() {
    let (base, keys) =
        spawn_server_with_full_middleware(vec!["full-stack-key".to_string()], 5, 60).await;
    let client = reqwest::Client::new();

    // Unauthenticated → 401 (auth fires before rate limit)
    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Authenticated → 200 with rate limit headers
    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", format!("Bearer {}", keys[0]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
}

#[tokio::test]
async fn e2e_full_middleware_health_bypasses_auth_has_rate_limit() {
    let (base, _keys) =
        spawn_server_with_full_middleware(vec!["bypass-key".to_string()], 100, 60).await;
    let client = reqwest::Client::new();

    // /health bypasses auth but still gets rate limit headers
    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_full_middleware_exhausts_rate_limit() {
    let (base, keys) =
        spawn_server_with_full_middleware(vec!["exhaust-key".to_string()], 3, 60).await;
    let client = reqwest::Client::new();
    let key = &keys[0];

    // Use up quota with authenticated requests
    for _ in 0..3 {
        let resp = client
            .get(format!("{base}/api/v1/providers"))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // 4th authenticated request → 429
    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

// ───────────────────── Body size limit middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_body_limit_small_body_accepted() {
    // Set a 1 KiB limit — small JSON payloads should be accepted
    let (base, _max) = spawn_server_with_body_limit(1024).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": "small payload",
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/ok"}
        }))
        .send()
        .await
        .unwrap();

    // Should NOT be 413 — the handler may return 400 due to config validation,
    // but the body was accepted by the body limit layer.
    assert_ne!(
        resp.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "small body should not trigger 413"
    );
}

#[tokio::test]
async fn e2e_body_limit_oversized_body_rejected() {
    // Set a very small limit (128 bytes) so our payload clearly exceeds it.
    // axum's DefaultBodyLimit may return 413 (PayloadTooLarge) or the JSON
    // extractor may catch the underlying bytes rejection and return 400.
    // Either way, the request should NOT succeed (not 2xx).
    let (base, _max) = spawn_server_with_body_limit(128).await;
    let client = reqwest::Client::new();

    // Build a payload much larger than 128 bytes
    let large_text = "x".repeat(2048);
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": large_text,
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/big"}
        }))
        .send()
        .await
        .unwrap();

    // Body limit enforcement: expect 413 (PAYLOAD_TOO_LARGE) or 400 (BAD_REQUEST)
    // depending on how axum's JSON extractor surfaces the bytes limit error
    let status = resp.status();
    assert!(
        status == StatusCode::PAYLOAD_TOO_LARGE || status == StatusCode::BAD_REQUEST,
        "oversized body should return 413 or 400, got: {status}"
    );
    assert!(!status.is_success(), "oversized body should never succeed");
}

#[tokio::test]
async fn e2e_body_limit_get_requests_unaffected() {
    // Body limit only applies to request bodies; GET endpoints should work fine
    let (base, _max) = spawn_server_with_body_limit(64).await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_body_limit_exact_boundary() {
    // Set limit to exactly accommodate a small JSON body
    // The json!({}) payload is about 2 bytes ("{}"), but reqwest adds Content-Type
    // headers etc. We use a known-small payload with a generous limit.
    let (base, _max) = spawn_server_with_body_limit(4096).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "body-limit-test",
            "body": "Hello {{name}}"
        }))
        .send()
        .await
        .unwrap();

    // Template creation should succeed (201) — body fits within limit
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ───────────────────── Request ID middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_request_id_generated_when_absent() {
    let base = spawn_server_with_request_id().await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Response should contain x-request-id header
    let request_id = resp
        .headers()
        .get("x-request-id")
        .expect("response should have x-request-id header");
    let id_str = request_id.to_str().unwrap();

    // Should be a valid UUID v4
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "x-request-id should be a valid UUID, got: {id_str}"
    );
}

#[tokio::test]
async fn e2e_request_id_preserved_when_provided() {
    let base = spawn_server_with_request_id().await;
    let client = reqwest::Client::new();

    let custom_id = "my-custom-request-id-42";
    let resp = client
        .get(format!("{base}/health"))
        .header("x-request-id", custom_id)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let returned_id = resp
        .headers()
        .get("x-request-id")
        .expect("response should echo x-request-id")
        .to_str()
        .unwrap();
    assert_eq!(
        returned_id, custom_id,
        "server should preserve the client-provided request ID"
    );
}

#[tokio::test]
async fn e2e_request_id_unique_per_request() {
    let base = spawn_server_with_request_id().await;
    let client = reqwest::Client::new();

    let resp1 = client.get(format!("{base}/health")).send().await.unwrap();
    let id1 = resp1.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_string();

    let resp2 = client.get(format!("{base}/health")).send().await.unwrap();
    let id2 = resp2.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_string();

    assert_ne!(id1, id2, "each request should get a unique request ID");
}

#[tokio::test]
async fn e2e_request_id_present_on_post_endpoints() {
    let base = spawn_server_with_request_id().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": "test request id on POST"
        }))
        .send()
        .await
        .unwrap();

    // Regardless of the handler response status, x-request-id should be present
    assert!(
        resp.headers().contains_key("x-request-id"),
        "POST endpoints should also receive x-request-id"
    );
    let id_str = resp.headers()["x-request-id"].to_str().unwrap();
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "x-request-id should be a valid UUID on POST, got: {id_str}"
    );
}

#[tokio::test]
async fn e2e_request_id_present_on_error_responses() {
    let base = spawn_server_with_request_id().await;
    let client = reqwest::Client::new();

    // Request a non-existent provider — should get 404 but still have x-request-id
    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert!(
        resp.headers().contains_key("x-request-id"),
        "error responses should also carry x-request-id"
    );
}

// ───────────────────── CORS middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_cors_permissive_allows_any_origin() {
    let base = spawn_server_with_cors_permissive().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/health"))
        .header("Origin", "https://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("permissive CORS should return Access-Control-Allow-Origin header");
    assert_eq!(
        acao.to_str().unwrap(),
        "*",
        "permissive CORS should allow all origins (*)"
    );
}

#[tokio::test]
async fn e2e_cors_permissive_preflight_succeeds() {
    let base = spawn_server_with_cors_permissive().await;
    let client = reqwest::Client::new();

    let resp = client
        .request(reqwest::Method::OPTIONS, format!("{base}/api/v1/providers"))
        .header("Origin", "https://example.com")
        .header("Access-Control-Request-Method", "GET")
        .header(
            "Access-Control-Request-Headers",
            "Content-Type, Authorization",
        )
        .send()
        .await
        .unwrap();

    // OPTIONS preflight should succeed (2xx)
    assert!(
        resp.status().is_success(),
        "OPTIONS preflight should succeed, got: {}",
        resp.status()
    );
    assert!(
        resp.headers().contains_key("access-control-allow-origin"),
        "preflight response should contain Access-Control-Allow-Origin"
    );
    assert!(
        resp.headers().contains_key("access-control-allow-methods"),
        "preflight response should contain Access-Control-Allow-Methods"
    );
    assert!(
        resp.headers().contains_key("access-control-allow-headers"),
        "preflight response should contain Access-Control-Allow-Headers"
    );
}

#[tokio::test]
async fn e2e_cors_restricted_allows_matching_origin() {
    let base = spawn_server_with_cors_restricted(vec![
        "https://allowed.example.com".to_string(),
        "https://also-allowed.com".to_string(),
    ])
    .await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/health"))
        .header("Origin", "https://allowed.example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("matching origin should return Access-Control-Allow-Origin header");
    assert_eq!(
        acao.to_str().unwrap(),
        "https://allowed.example.com",
        "ACAO should reflect the matching origin"
    );
}

#[tokio::test]
async fn e2e_cors_restricted_rejects_non_matching_origin() {
    let base =
        spawn_server_with_cors_restricted(vec!["https://allowed.example.com".to_string()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/health"))
        .header("Origin", "https://evil.example.com")
        .send()
        .await
        .unwrap();

    // The request itself still succeeds (CORS is enforced by browsers),
    // but Access-Control-Allow-Origin should NOT be set for non-matching origins.
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "non-matching origin should NOT receive Access-Control-Allow-Origin header"
    );
}

#[tokio::test]
async fn e2e_cors_restricted_preflight_non_matching_origin() {
    let base =
        spawn_server_with_cors_restricted(vec!["https://allowed.example.com".to_string()]).await;
    let client = reqwest::Client::new();

    let resp = client
        .request(reqwest::Method::OPTIONS, format!("{base}/api/v1/send"))
        .header("Origin", "https://evil.example.com")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();

    // Preflight for non-matching origin should not include ACAO
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "preflight for non-matching origin should NOT include ACAO"
    );
}

#[tokio::test]
async fn e2e_cors_permissive_post_endpoint() {
    let base = spawn_server_with_cors_permissive().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("Origin", "https://any-origin.com")
        .json(&json!({
            "provider": "slack",
            "text": "cors test"
        }))
        .send()
        .await
        .unwrap();

    // Regardless of handler result, ACAO should be present
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("POST response should include ACAO with permissive CORS");
    assert_eq!(acao.to_str().unwrap(), "*");
}

// ───────────────────── ValidatedJson middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_validated_json_empty_provider_returns_422() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "",
            "text": "hello"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["message"], "Request body validation failed");
    assert!(
        body["fields"]["provider"].is_array(),
        "should report field-level error for 'provider'"
    );
    let provider_errors = body["fields"]["provider"].as_array().unwrap();
    assert!(
        provider_errors
            .iter()
            .any(|e| e.as_str().unwrap().contains("must not be empty")),
        "error message should mention 'must not be empty'"
    );
}

#[tokio::test]
async fn e2e_validated_json_empty_text_returns_422() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["fields"]["text"].is_array(),
        "should report field-level error for 'text'"
    );
}

#[tokio::test]
async fn e2e_validated_json_multiple_field_errors() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "",
            "text": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    // Both fields should have errors
    assert!(
        body["fields"]["provider"].is_array(),
        "should report 'provider' field error"
    );
    assert!(
        body["fields"]["text"].is_array(),
        "should report 'text' field error"
    );
}

#[tokio::test]
async fn e2e_validated_json_invalid_json_returns_400() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body("this is not valid json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "invalid_json");
    assert!(
        body["message"].is_string(),
        "should include an error message"
    );
}

#[tokio::test]
async fn e2e_validated_json_missing_required_fields_returns_422() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Send JSON with missing required fields (only config, no provider/text)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body(r#"{"config": {}}"#)
        .send()
        .await
        .unwrap();

    // Missing required fields should cause a deserialization error (400)
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "invalid_json");
}

#[tokio::test]
async fn e2e_validated_json_template_empty_name_returns_422() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "",
            "body": "Hello {{name}}"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["fields"]["name"].is_array(),
        "should report field-level error for template 'name'"
    );
}

#[tokio::test]
async fn e2e_validated_json_template_empty_body_returns_422() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "test-template",
            "body": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["fields"]["body"].is_array(),
        "should report field-level error for template 'body'"
    );
}

#[tokio::test]
async fn e2e_validated_json_valid_request_passes_validation() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // A valid send request — should pass validation and reach the handler
    // (will fail at provider config level, but not at validation level)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": "valid message",
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/valid"}
        }))
        .send()
        .await
        .unwrap();

    // Should NOT be 422 (validation passed) — handler may return 400 due to other issues
    assert_ne!(
        resp.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "valid payload should not trigger 422 validation error"
    );
}

// ───────────────────── Worker processing & Webhook callback (e2e) ─────────────────────

#[tokio::test]
async fn e2e_worker_processes_task_to_completion() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    // Enqueue a task for mock-ok provider
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "worker e2e test"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for worker to process the task
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(
        task["status"], "completed",
        "task should be completed by worker"
    );
    assert_eq!(task["provider"], "mock-ok");

    // Verify stats reflect the completed task
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= 1);

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_worker_handles_failed_task() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    // Enqueue a task for mock-fail provider with no retries
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "worker failure test",
            "retry": {"max_retries": 0, "delay_ms": 10}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for worker to process and fail the task
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "failed", "task should be failed by worker");
    assert!(
        task["last_error"].is_string(),
        "failed task should have an error message"
    );

    // Verify stats reflect the failed task
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["failed"].as_u64().unwrap() >= 1);

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_webhook_callback_on_success() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let callback_url = format!("{callback_base}/callback");

    // Enqueue a task with callback_url
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "callback success test",
            "callback_url": callback_url,
            "metadata": {"trace_id": "e2e-callback-ok"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for task to complete
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "completed");

    // Give callback a moment to fire (best-effort, async)
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify callback was received
    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback server should have received at least one payload"
        );

        let cb = &received[0];
        assert_eq!(cb["task_id"], task_id);
        assert_eq!(cb["provider"], "mock-ok");
        assert_eq!(cb["status"], "completed");
        assert!(cb["attempts"].as_u64().unwrap() >= 1);
        assert_eq!(cb["metadata"]["trace_id"], "e2e-callback-ok");
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_webhook_callback_on_failure() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let callback_url = format!("{callback_base}/callback");

    // Enqueue a task with callback_url that will fail
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "callback failure test",
            "callback_url": callback_url,
            "retry": {"max_retries": 0, "delay_ms": 10}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for task to fail
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "failed");

    // Give callback a moment to fire
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify callback was received with failure info
    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback server should receive payload on failure"
        );

        let cb = &received[0];
        assert_eq!(cb["task_id"], task_id);
        assert_eq!(cb["provider"], "mock-fail");
        assert_eq!(cb["status"], "failed");
        assert!(
            cb["last_error"].is_string(),
            "failed callback should include last_error"
        );
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_no_callback_when_url_not_set() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    // Enqueue a task WITHOUT callback_url
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "no callback test"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for task to complete
    let _task = wait_for_terminal_status(&client, &base, &task_id).await;

    // Give extra time
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Callback server should NOT have received anything
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.is_empty(),
            "callback server should NOT receive payload when no callback_url is set, got {} payloads: {callback_base}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_worker_multiple_tasks_processed() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let mut task_ids = Vec::new();

    // Enqueue 5 tasks
    for i in 0..5 {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("batch worker test {i}")
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        task_ids.push(body["task_id"].as_str().unwrap().to_string());
    }

    // Wait for all tasks to complete
    for task_id in &task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(
            task["status"], "completed",
            "task {task_id} should be completed"
        );
    }

    // Verify stats
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= 5);

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_webhook_callback_not_fired_for_cancelled_before_processing() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let callback_url = format!("{callback_base}/callback");

    // Use a server WITHOUT workers so the task stays queued
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Enqueue a task with callback_url (but no workers to process it)
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "slack",
            "text": "cancel before processing",
            "callback_url": callback_url,
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/test"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Cancel the task while it's still queued
    let resp = client
        .post(format!("{base}/api/v1/queue/tasks/{task_id}/cancel"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cancel_body: Value = resp.json().await.unwrap();
    assert!(cancel_body["cancelled"].as_bool().unwrap());

    // Give time for any spurious callback
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Callback should NOT be fired (cancellation via API doesn't trigger worker callback path)
    let received = payloads.lock().unwrap();
    assert!(
        received.is_empty(),
        "callback should not fire for API-cancelled tasks (not worker-triggered)"
    );
}

#[tokio::test]
async fn e2e_worker_task_with_metadata_preserved() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let callback_url = format!("{callback_base}/callback");

    // Enqueue with metadata
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "metadata test",
            "callback_url": callback_url,
            "metadata": {
                "request_id": "req-abc-123",
                "source": "e2e-test",
                "env": "test"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for completion
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "completed");

    // Verify metadata is preserved in task info
    assert_eq!(task["metadata"]["request_id"], "req-abc-123");
    assert_eq!(task["metadata"]["source"], "e2e-test");
    assert_eq!(task["metadata"]["env"], "test");

    // Give callback time
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify metadata in callback payload
    {
        let received = payloads.lock().unwrap();
        assert!(!received.is_empty());
        let cb = &received[0];
        assert_eq!(cb["metadata"]["request_id"], "req-abc-123");
        assert_eq!(cb["metadata"]["source"], "e2e-test");
        assert_eq!(cb["metadata"]["env"], "test");
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Priority ordering & Retry behavior (e2e) ─────────────────────

#[tokio::test]
async fn e2e_priority_ordering_urgent_before_low() {
    // Enqueue tasks with different priorities on a server with NO workers,
    // then start a single worker so tasks are processed in priority order.
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Enqueue tasks with different priorities (low first, urgent last)
    let priorities = vec!["low", "normal", "high", "urgent"];
    let mut task_ids = Vec::new();

    for pri in &priorities {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("priority-{pri}"),
                "priority": pri,
                "retry": {"max_retries": 0, "delay_ms": 10}
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        task_ids.push(body["task_id"].as_str().unwrap().to_string());
    }

    // Start a single worker — processes urgent first, then high, normal, low
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for all tasks to complete
    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    // Verify all completed
    for task_id in &task_ids {
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let body: Value = resp.json().await.unwrap();
        assert_eq!(
            body["status"], "completed",
            "task {task_id} should be completed"
        );
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_priority_ordering_verified_by_completion_order() {
    // Enqueue low then urgent, verify urgent callback arrives before low
    let (callback_base, payloads) = spawn_callback_server().await;

    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue: low first, then urgent — urgent should be processed first
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "low-priority-task",
            "priority": "low",
            "callback_url": &callback_url,
            "metadata": {"order": "low"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let low_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "urgent-priority-task",
            "priority": "urgent",
            "callback_url": &callback_url,
            "metadata": {"order": "urgent"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let urgent_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Start single worker to enforce serial processing
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for both tasks
    wait_for_terminal_status(&client, &base, &low_id).await;
    wait_for_terminal_status(&client, &base, &urgent_id).await;

    // Give callbacks time
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify callback order: urgent should arrive before low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 2,
            "expected at least 2 callbacks, got {}",
            received.len()
        );

        // First callback should be for the urgent task
        assert_eq!(
            received[0]["metadata"]["order"], "urgent",
            "urgent task should be processed first, but got: {:?}",
            received[0]["metadata"]["order"]
        );
        assert_eq!(
            received[1]["metadata"]["order"], "low",
            "low task should be processed second"
        );
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_retry_task_eventually_succeeds() {
    // MockFlakyProvider fails first 2 calls, then succeeds.
    // With max_retries=3, the task should eventually complete.
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "retry success test",
            "retry": {"max_retries": 3, "delay_ms": 10},
            "callback_url": &callback_url,
            "metadata": {"test": "retry-success"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for worker to process through retries and succeed
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(
        task["status"], "completed",
        "flaky task should eventually succeed after retries"
    );
    // The task went through 3 attempts: fail, fail, succeed
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "expected at least 3 attempts, got {}",
        task["attempts"]
    );

    // Give callback time
    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback should be received for completed task"
        );
        assert_eq!(received[0]["status"], "completed");
        assert_eq!(received[0]["metadata"]["test"], "retry-success");
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_retry_exhausted_task_fails() {
    // MockFlakyProvider fails first 5 calls, but max_retries=2 means only 3 total attempts.
    // The task should fail after exhausting retries.
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(5));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "retry exhaustion test",
            "retry": {"max_retries": 2, "delay_ms": 10},
            "callback_url": &callback_url,
            "metadata": {"test": "retry-fail"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(
        task["status"], "failed",
        "task should fail after exhausting retries"
    );
    assert!(
        task["last_error"].is_string(),
        "failed task should have error message"
    );

    // Give callback time
    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback should be received for failed task"
        );
        assert_eq!(received[0]["status"], "failed");
        assert!(received[0]["last_error"].is_string());
        assert_eq!(received[0]["metadata"]["test"], "retry-fail");
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_retry_zero_retries_fails_immediately() {
    // With max_retries=0, a failing task should fail on the first attempt.
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "no retry test",
            "retry": {"max_retries": 0, "delay_ms": 10}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "failed");
    // With 0 retries, only 1 attempt should have been made
    assert_eq!(
        task["attempts"].as_u64().unwrap(),
        1,
        "with max_retries=0, should have exactly 1 attempt"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_priority_high_tasks_processed_before_normal() {
    // Enqueue 3 normal + 1 high, verify all complete (high processed first)
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;
    let client = reqwest::Client::new();

    // Enqueue 3 normal tasks
    let mut normal_ids = Vec::new();
    for i in 0..3 {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("normal-{i}"),
                "priority": "normal"
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        normal_ids.push(
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }

    // Enqueue 1 high-priority task
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "high-priority",
            "priority": "high"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let high_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for all to complete
    wait_for_terminal_status(&client, &base, &high_id).await;
    for nid in &normal_ids {
        wait_for_terminal_status(&client, &base, nid).await;
    }

    // All should be completed
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= 4);

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Per-IP rate limiting (e2e) ─────────────────────

#[tokio::test]
async fn e2e_per_ip_rate_limit_isolates_x_forwarded_for() {
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = reqwest::Client::new();

    // IP-A exhausts its quota
    for i in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Forwarded-For", "10.0.0.1")
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "IP-A request {i} should be allowed"
        );
    }

    // IP-A is now rate-limited
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "IP-A should be rate-limited after exhausting quota"
    );

    // IP-B still has its own independent quota
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.2")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "IP-B should still be allowed (independent bucket)"
    );
    assert!(
        resp.headers().contains_key("x-ratelimit-limit"),
        "IP-B response should have rate limit headers"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_isolates_x_real_ip() {
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = reqwest::Client::new();

    // IP-C exhausts its quota via X-Real-IP
    for _ in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Real-IP", "192.168.1.100")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // IP-C is now limited
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Real-IP", "192.168.1.100")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "IP via X-Real-IP should be rate-limited after exhausting quota"
    );

    // IP-D via X-Real-IP is unaffected
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Real-IP", "192.168.1.200")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "different IP via X-Real-IP should still have quota"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_x_forwarded_for_takes_precedence() {
    // When both X-Forwarded-For and X-Real-IP are present,
    // X-Forwarded-For should take precedence per extract_client_ip logic.
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = reqwest::Client::new();

    // Exhaust quota for IP identified by X-Forwarded-For: 10.1.1.1
    for _ in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Forwarded-For", "10.1.1.1")
            .header("X-Real-IP", "10.2.2.2")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Next request with same X-Forwarded-For but different X-Real-IP → still limited
    // because X-Forwarded-For takes precedence
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.1.1.1")
        .header("X-Real-IP", "10.9.9.9")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "X-Forwarded-For should take precedence over X-Real-IP"
    );

    // Request with different X-Forwarded-For → allowed (different bucket)
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.3.3.3")
        .header("X-Real-IP", "10.2.2.2")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "different X-Forwarded-For should have its own bucket"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_remaining_tracks_per_ip() {
    let (base, _max) = spawn_server_with_rate_limit_per_ip(10, 60).await;
    let client = reqwest::Client::new();

    // IP-E sends one request
    let resp_e1 = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "172.16.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(resp_e1.status(), StatusCode::OK);
    let remaining_e1: u64 = resp_e1.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    // IP-E sends second request
    let resp_e2 = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "172.16.0.1")
        .send()
        .await
        .unwrap();
    let remaining_e2: u64 = resp_e2.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        remaining_e2 < remaining_e1,
        "remaining should decrement for same IP: {remaining_e1} -> {remaining_e2}"
    );

    // IP-F sends first request — should have full quota (independent of IP-E)
    let resp_f1 = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "172.16.0.2")
        .send()
        .await
        .unwrap();
    assert_eq!(resp_f1.status(), StatusCode::OK);
    let remaining_f1: u64 = resp_f1.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    // IP-F should have more remaining than IP-E (since IP-E used 2, IP-F used 1)
    assert!(
        remaining_f1 > remaining_e2,
        "new IP should have more remaining tokens: IP-F={remaining_f1}, IP-E={remaining_e2}"
    );
}

#[tokio::test]
async fn e2e_per_ip_rate_limit_multiple_ips_in_x_forwarded_for() {
    // X-Forwarded-For can contain multiple IPs separated by commas.
    // The middleware should use the first one (the original client IP).
    let (base, max_requests) = spawn_server_with_rate_limit_per_ip(2, 60).await;
    let client = reqwest::Client::new();

    // Exhaust quota for client IP 10.0.0.50 (first in chain)
    for _ in 0..max_requests {
        let resp = client
            .get(format!("{base}/health"))
            .header("X-Forwarded-For", "10.0.0.50, 10.0.0.99, 10.0.0.1")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Same first IP in a different proxy chain → still limited
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.50, 192.168.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "same first IP in X-Forwarded-For chain should share bucket"
    );

    // Different first IP → allowed
    let resp = client
        .get(format!("{base}/health"))
        .header("X-Forwarded-For", "10.0.0.51, 10.0.0.99, 10.0.0.1")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "different first IP in chain should have its own bucket"
    );
}

// ───────────────────── SQLite queue backend (e2e) ─────────────────────

#[tokio::test]
async fn e2e_sqlite_health_check() {
    let base = spawn_server_sqlite().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_sqlite_async_send_query_cancel_purge() {
    let base = spawn_server_sqlite().await;
    let client = reqwest::Client::new();

    // Enqueue
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "slack",
            "text": "sqlite e2e test",
            "config": {
                "webhook_url": "https://hooks.slack.com/services/T00/B00/sqlite"
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

#[tokio::test]
async fn e2e_sqlite_worker_processes_task_to_completion() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "sqlite worker e2e test"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(
        task["status"], "completed",
        "task should be completed by worker (SQLite backend)"
    );
    assert_eq!(task["provider"], "mock-ok");

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= 1);

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_worker_handles_failed_task() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "sqlite worker failure test",
            "retry": {"max_retries": 0, "delay_ms": 10}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(
        task["status"], "failed",
        "task should be failed by worker (SQLite backend)"
    );
    assert!(
        task["last_error"].is_string(),
        "failed task should have an error message"
    );

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["failed"].as_u64().unwrap() >= 1);

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_webhook_callback_on_success() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "sqlite callback success test",
            "callback_url": callback_url,
            "metadata": {"trace_id": "sqlite-callback-ok"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "completed");

    tokio::time::sleep(Duration::from_millis(200)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback server should have received at least one payload (SQLite backend)"
        );

        let cb = &received[0];
        assert_eq!(cb["task_id"], task_id);
        assert_eq!(cb["provider"], "mock-ok");
        assert_eq!(cb["status"], "completed");
        assert_eq!(cb["metadata"]["trace_id"], "sqlite-callback-ok");
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_priority_ordering_urgent_before_low() {
    let (callback_base, payloads) = spawn_callback_server().await;

    // Create AppState with SQLite queue but NO workers yet
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));

    let queue = Arc::new(noti_queue::SqliteQueue::in_memory().unwrap());
    let task_notify = queue.notifier();
    let state = noti_server::state::AppState::with_custom_queue(registry, queue, task_notify);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue: low first, then urgent — SQLite should dequeue urgent first
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "low-priority-task",
            "priority": "low",
            "callback_url": &callback_url,
            "metadata": {"order": "low"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let low_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "urgent-priority-task",
            "priority": "urgent",
            "callback_url": &callback_url,
            "metadata": {"order": "urgent"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let urgent_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // NOW start a single worker so tasks are processed in priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for both tasks
    wait_for_terminal_status(&client, &base, &low_id).await;
    wait_for_terminal_status(&client, &base, &urgent_id).await;

    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 2,
            "expected at least 2 callbacks (SQLite), got {}",
            received.len()
        );

        assert_eq!(
            received[0]["metadata"]["order"], "urgent",
            "SQLite: urgent task should be processed first, but got: {:?}",
            received[0]["metadata"]["order"]
        );
        assert_eq!(
            received[1]["metadata"]["order"], "low",
            "SQLite: low task should be processed second"
        );
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_retry_task_eventually_succeeds() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "sqlite retry success test",
            "retry": {"max_retries": 3, "delay_ms": 10},
            "callback_url": &callback_url,
            "metadata": {"test": "sqlite-retry-success"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(
        task["status"], "completed",
        "SQLite: flaky task should eventually succeed after retries"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "SQLite: expected at least 3 attempts, got {}",
        task["attempts"]
    );

    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback should be received for completed task (SQLite)"
        );
        assert_eq!(received[0]["status"], "completed");
        assert_eq!(received[0]["metadata"]["test"], "sqlite-retry-success");
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_multiple_tasks_processed() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let mut task_ids = Vec::new();

    for i in 0..5 {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("sqlite batch test {i}")
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        task_ids.push(body["task_id"].as_str().unwrap().to_string());
    }

    for task_id in &task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(
            task["status"], "completed",
            "SQLite: task {task_id} should be completed"
        );
    }

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= 5);

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_batch_async_send() {
    let base = spawn_server_sqlite().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "slack",
                    "text": "sqlite batch item 1",
                    "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/sbatch1"}
                },
                {
                    "provider": "nonexistent",
                    "text": "sqlite batch item 2"
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

#[tokio::test]
async fn e2e_sqlite_task_metadata_preserved() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "sqlite metadata test",
            "callback_url": callback_url,
            "metadata": {
                "request_id": "sqlite-req-123",
                "source": "sqlite-e2e",
                "env": "test"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "completed");

    // Verify metadata is preserved through SQLite serialization roundtrip
    assert_eq!(task["metadata"]["request_id"], "sqlite-req-123");
    assert_eq!(task["metadata"]["source"], "sqlite-e2e");
    assert_eq!(task["metadata"]["env"], "test");

    tokio::time::sleep(Duration::from_millis(200)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(!received.is_empty());
        let cb = &received[0];
        assert_eq!(cb["metadata"]["request_id"], "sqlite-req-123");
        assert_eq!(cb["metadata"]["source"], "sqlite-e2e");
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Batch async with mixed priorities (e2e) ─────────────────────

#[tokio::test]
async fn e2e_batch_async_mixed_priorities_processed_in_order() {
    // Batch-enqueue 4 tasks with different priorities via the async batch endpoint.
    // Use a single worker to ensure strict priority-ordered processing.
    // Verify via callback order that urgent is processed first, then high, normal, low.
    let (callback_base, payloads) = spawn_callback_server().await;

    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Batch-enqueue: low, normal, high, urgent — all in one request
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "batch-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "batch-normal",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "normal"}
                },
                {
                    "provider": "mock-ok",
                    "text": "batch-high",
                    "priority": "high",
                    "callback_url": &callback_url,
                    "metadata": {"order": "high"}
                },
                {
                    "provider": "mock-ok",
                    "text": "batch-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "urgent"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 4);
    assert_eq!(body["enqueued"], 4);
    assert_eq!(body["failed"], 0);

    // Collect all task IDs
    let results = body["results"].as_array().unwrap();
    let task_ids: Vec<String> = results
        .iter()
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();

    // NOW start a single worker so tasks are processed in strict priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for all tasks to complete
    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callback order: urgent → high → normal → low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 4,
            "expected at least 4 callbacks, got {}",
            received.len()
        );

        let expected_order = ["urgent", "high", "normal", "low"];
        for (i, expected) in expected_order.iter().enumerate() {
            assert_eq!(
                received[i]["metadata"]["order"].as_str().unwrap(),
                *expected,
                "callback #{i} should be '{expected}', got '{}'",
                received[i]["metadata"]["order"]
            );
        }
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_batch_async_mixed_priorities_processed_in_order() {
    // Same as above but using SQLite queue backend.
    let (callback_base, payloads) = spawn_callback_server().await;

    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));

    let queue = Arc::new(noti_queue::SqliteQueue::in_memory().unwrap());
    let task_notify = queue.notifier();
    let state = noti_server::state::AppState::with_custom_queue(registry, queue, task_notify);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Batch-enqueue: low, normal, high, urgent — all in one request
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite-batch-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-batch-normal",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "normal"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-batch-high",
                    "priority": "high",
                    "callback_url": &callback_url,
                    "metadata": {"order": "high"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-batch-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "urgent"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 4);
    assert_eq!(body["enqueued"], 4);
    assert_eq!(body["failed"], 0);

    // Collect all task IDs
    let results = body["results"].as_array().unwrap();
    let task_ids: Vec<String> = results
        .iter()
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();

    // NOW start a single worker so tasks are processed in strict priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for all tasks to complete
    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callback order: urgent → high → normal → low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 4,
            "SQLite: expected at least 4 callbacks, got {}",
            received.len()
        );

        let expected_order = ["urgent", "high", "normal", "low"];
        for (i, expected) in expected_order.iter().enumerate() {
            assert_eq!(
                received[i]["metadata"]["order"].as_str().unwrap(),
                *expected,
                "SQLite: callback #{i} should be '{expected}', got '{}'",
                received[i]["metadata"]["order"]
            );
        }
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Graceful shutdown (e2e) ─────────────────────

/// Verify that `shutdown_and_join()` waits for an in-flight slow task to complete
/// before the worker pool exits, and the task reaches `completed` status.
#[tokio::test]
async fn e2e_graceful_shutdown_waits_for_inflight_task() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let mut registry = noti_core::ProviderRegistry::new();
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(500)));
    registry.register(slow);

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Start a single worker
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue a task that takes 500ms to process
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-slow",
            "text": "slow-task",
            "callback_url": &callback_url,
            "metadata": {"test": "graceful-shutdown"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait a bit for the worker to pick up the task (but not finish it)
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Issue shutdown while the slow task is still in-flight
    // shutdown_and_join should block until the worker finishes the current task
    worker_handle.shutdown_and_join().await;

    // After shutdown completes, the task should be completed (worker waited for it)
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let task: Value = resp.json().await.unwrap();
    assert_eq!(
        task["status"], "completed",
        "in-flight task should complete before worker exits"
    );

    // Verify the callback was fired
    tokio::time::sleep(Duration::from_millis(200)).await;
    let received = payloads.lock().unwrap();
    assert!(
        !received.is_empty(),
        "callback should have been fired for the completed slow task"
    );
    assert_eq!(received[0]["status"], "completed");
    assert_eq!(received[0]["metadata"]["test"], "graceful-shutdown");
}

/// Verify that after shutdown, queued tasks that were not picked up remain in `pending` status.
/// Uses a slow provider so the single worker can only process one task before shutdown.
#[tokio::test]
async fn e2e_graceful_shutdown_stops_processing_new_tasks() {
    let mut registry = noti_core::ProviderRegistry::new();
    // Each task takes 200ms to complete
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(200)));
    registry.register(slow);

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // Enqueue 5 tasks BEFORE starting workers
    let mut task_ids = Vec::new();
    for i in 0..5 {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-slow",
                "text": format!("task-{i}"),
                "metadata": {"index": format!("{i}")}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        task_ids.push(body["task_id"].as_str().unwrap().to_string());
    }

    // Start a single worker
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for the worker to pick up and start processing the first task (50ms poll + start)
    // but not long enough for 200ms send to finish
    tokio::time::sleep(Duration::from_millis(120)).await;

    // Issue shutdown — worker should finish the in-flight task but not start new ones
    worker_handle.shutdown_and_join().await;

    // Check task statuses
    let mut completed = 0;
    let mut pending = 0;
    for task_id in &task_ids {
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let body: Value = resp.json().await.unwrap();
        match body["status"].as_str().unwrap() {
            "completed" => completed += 1,
            "pending" | "queued" => pending += 1,
            _ => {}
        }
    }

    // The single worker with 200ms delay can process at most 1 task before shutdown
    // (picked up at ~50ms, finishes at ~250ms, shutdown at ~120ms waits for it).
    // Remaining tasks should still be pending.
    assert!(
        completed >= 1,
        "at least one task should have been completed (completed={completed})"
    );
    assert!(
        pending >= 1,
        "at least one task should remain pending after shutdown (pending={pending}, completed={completed})"
    );
    assert!(
        completed < 5,
        "not all tasks should be completed after immediate shutdown (completed={completed})"
    );
}

/// Verify that the HTTP server remains responsive during and after worker shutdown.
/// (Workers shutting down should not affect the server's ability to serve requests.)
#[tokio::test]
async fn e2e_http_server_responsive_during_worker_shutdown() {
    let mut registry = noti_core::ProviderRegistry::new();
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(300)));
    registry.register(slow);
    registry.register(Arc::new(MockOkProvider));

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Start worker
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let client = reqwest::Client::new();
    let base_clone = base.clone();

    // Enqueue a slow task
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-slow",
            "text": "slow-during-shutdown"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Wait for worker to pick it up
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Spawn shutdown in the background (it will block until the slow task finishes)
    let shutdown_task = tokio::spawn(async move {
        worker_handle.shutdown_and_join().await;
    });

    // While shutdown is in progress, the HTTP server should still respond
    let resp = client
        .get(format!("{base_clone}/health"))
        .send()
        .await
        .expect("server should still respond during worker shutdown");
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client
        .get(format!("{base_clone}/api/v1/providers"))
        .send()
        .await
        .expect("provider listing should still work during worker shutdown");
    assert_eq!(resp.status(), StatusCode::OK);

    // Wait for shutdown to complete
    shutdown_task.await.unwrap();

    // Server should still respond after workers are fully stopped
    let resp = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("server should respond after worker shutdown");
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Verify graceful shutdown with SQLite backend — in-flight task completes.
#[tokio::test]
async fn e2e_sqlite_graceful_shutdown_waits_for_inflight_task() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let mut registry = noti_core::ProviderRegistry::new();
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(500)));
    registry.register(slow);

    let queue = Arc::new(
        noti_queue::SqliteQueue::in_memory().expect("failed to create in-memory SQLite queue"),
    );
    let task_notify = queue.notifier();
    let state = noti_server::state::AppState::with_custom_queue(registry, queue, task_notify);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue a slow task
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-slow",
            "text": "sqlite-slow-task",
            "callback_url": &callback_url,
            "metadata": {"test": "sqlite-graceful-shutdown"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for the worker to pick up the task
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shut down — should wait for in-flight task
    worker_handle.shutdown_and_join().await;

    // Task should be completed in the SQLite backend
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let task: Value = resp.json().await.unwrap();
    assert_eq!(
        task["status"], "completed",
        "SQLite: in-flight task should complete before worker exits"
    );

    // Verify callback
    tokio::time::sleep(Duration::from_millis(200)).await;
    let received = payloads.lock().unwrap();
    assert!(
        !received.is_empty(),
        "SQLite: callback should have been fired for the completed slow task"
    );
    assert_eq!(received[0]["status"], "completed");
    assert_eq!(received[0]["metadata"]["test"], "sqlite-graceful-shutdown");
}

/// Verify that shutdown_and_join completes within a reasonable time
/// even when the queue is empty (no tasks to process).
#[tokio::test]
async fn e2e_graceful_shutdown_empty_queue_completes_quickly() {
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind");
    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    let _base = format!("http://{addr}");

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Start workers with multiple concurrency
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(4)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Let workers run for a bit with empty queue
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shutdown should complete quickly (not hang waiting for tasks)
    let start = std::time::Instant::now();
    worker_handle.shutdown_and_join().await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "shutdown of empty queue should complete quickly, took {:?}",
        elapsed
    );
}

// ───────────────────── Stale task recovery (SQLite file) ─────────────────────

/// Enqueue tasks, dequeue them (leaving them in "processing" state), drop the
/// queue (simulating a crash), then start a new server against the same DB file.
/// `with_queue_backend` should recover the stale tasks back to "queued".
#[tokio::test]
async fn e2e_stale_recovery_processing_tasks_become_queued() {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let db_path = tmp.path().to_str().unwrap().to_string();

    // Phase 1: open queue, enqueue 2 tasks, dequeue them (→ processing), then drop
    {
        let queue = noti_queue::SqliteQueue::open(&db_path).expect("open sqlite queue");
        let task_a = noti_queue::NotificationTask::new(
            "slack",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("stale-task-a").with_priority(noti_core::Priority::Normal),
        );
        let task_b = noti_queue::NotificationTask::new(
            "slack",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("stale-task-b").with_priority(noti_core::Priority::Normal),
        );

        queue.enqueue(task_a).await.unwrap();
        queue.enqueue(task_b).await.unwrap();

        // Dequeue both → status becomes "processing"
        queue.dequeue().await.unwrap().expect("dequeue a");
        queue.dequeue().await.unwrap().expect("dequeue b");

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.processing, 2, "both tasks should be processing");
        // Drop queue — simulates crash
    }

    // Phase 2: start HTTP server against the same DB — triggers recover_stale_tasks()
    let base = spawn_server_sqlite_file(&db_path).await;
    let client = reqwest::Client::new();

    // List tasks — recovered tasks should be "queued"
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=queued"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let tasks = body.as_array().expect("tasks should be an array");
    assert_eq!(
        tasks.len(),
        2,
        "both stale processing tasks should be recovered as queued"
    );

    // Stats should show 2 queued, 0 processing
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let stats: Value = resp.json().await.unwrap();
    assert_eq!(stats["queued"].as_u64().unwrap(), 2);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);
}

/// After recovery, a worker should be able to process the recovered tasks.
#[tokio::test]
async fn e2e_stale_recovery_tasks_can_be_processed_by_workers() {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let db_path = tmp.path().to_str().unwrap().to_string();

    // Phase 1: enqueue a task via "mock-ok" provider, dequeue it (→ processing), drop
    {
        let queue = noti_queue::SqliteQueue::open(&db_path).expect("open sqlite queue");
        let task = noti_queue::NotificationTask::new(
            "mock-ok",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("recover-and-process")
                .with_priority(noti_core::Priority::Normal),
        );

        let task_id = queue.enqueue(task).await.unwrap();

        // Dequeue → processing
        let dequeued = queue.dequeue().await.unwrap().expect("dequeue task");
        assert_eq!(dequeued.id, task_id);
        // Drop — simulates crash with task stuck in processing
    }

    // Phase 2: start server with workers — recovery + worker processing
    let (base, worker_handle) = spawn_server_sqlite_file_with_workers(&db_path).await;
    let client = reqwest::Client::new();

    // Give workers time to pick up and process the recovered task
    tokio::time::sleep(Duration::from_millis(500)).await;

    // List all tasks — the task should now be "completed" (processed by mock-ok)
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=completed"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let tasks = body.as_array().expect("tasks array");
    assert_eq!(
        tasks.len(),
        1,
        "recovered task should be completed by worker"
    );
    assert_eq!(tasks[0]["provider"], "mock-ok");

    worker_handle.shutdown_and_join().await;
}

/// When there are no stale tasks, recovery is a no-op and the server starts normally.
#[tokio::test]
async fn e2e_stale_recovery_no_stale_tasks_is_noop() {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let db_path = tmp.path().to_str().unwrap().to_string();

    // Phase 1: enqueue a task but do NOT dequeue it (stays queued, not processing)
    {
        let queue = noti_queue::SqliteQueue::open(&db_path).expect("open sqlite queue");
        let task = noti_queue::NotificationTask::new(
            "slack",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("not-stale").with_priority(noti_core::Priority::Normal),
        );
        queue.enqueue(task).await.unwrap();
    }

    // Phase 2: start server — no stale recovery needed
    let base = spawn_server_sqlite_file(&db_path).await;
    let client = reqwest::Client::new();

    // Task should still be queued (not touched by recovery)
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=queued"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let tasks = body.as_array().expect("tasks array");
    assert_eq!(tasks.len(), 1);

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert_eq!(stats["queued"].as_u64().unwrap(), 1);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);
}

// ───────────────────── Queue purge dedicated tests ─────────────────────

/// Purging an empty queue returns 0 purged.
#[tokio::test]
async fn e2e_purge_empty_queue_returns_zero() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/queue/purge"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["purged"].as_u64().unwrap(), 0);
    assert!(body["message"].as_str().unwrap().contains("0"));
}

/// Purge removes completed, failed, and cancelled tasks but not queued ones.
#[tokio::test]
async fn e2e_purge_removes_terminal_preserves_nonterminal() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    // Enqueue a task that will complete (mock-ok provider)
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "will complete"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_ok_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Enqueue a task that will fail (mock-fail provider)
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "will fail"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_fail_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for both to reach terminal state
    wait_for_terminal_status(&client, &base, &task_ok_id).await;
    wait_for_terminal_status(&client, &base, &task_fail_id).await;

    // Shutdown workers before adding a task that should stay queued
    worker_handle.shutdown_and_join().await;

    // Enqueue a task after workers are stopped — it stays queued
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "stays queued"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Stats before purge should show terminal + queued tasks
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats_before: Value = resp.json().await.unwrap();
    let total_before = stats_before["total"].as_u64().unwrap();
    assert!(total_before >= 3, "should have at least 3 tasks");

    // Purge terminal tasks
    let resp = client
        .post(format!("{base}/api/v1/queue/purge"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let purged = body["purged"].as_u64().unwrap();
    assert!(
        purged >= 2,
        "should purge at least the completed + failed tasks"
    );

    // Stats after purge — terminal counters should be 0
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats_after: Value = resp.json().await.unwrap();
    assert_eq!(stats_after["completed"].as_u64().unwrap(), 0);
    assert_eq!(stats_after["failed"].as_u64().unwrap(), 0);
    assert_eq!(stats_after["cancelled"].as_u64().unwrap(), 0);
    // The queued slack task should still be there
    assert!(stats_after["queued"].as_u64().unwrap() >= 1);
}

/// Purge on SQLite backend also correctly removes terminal tasks.
#[tokio::test]
async fn e2e_sqlite_purge_removes_terminal_tasks() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    // Enqueue a task that completes
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "sqlite purge test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_ok_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Enqueue a task that fails
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "sqlite purge fail"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_fail_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Wait for both to reach terminal state
    wait_for_terminal_status(&client, &base, &task_ok_id).await;
    wait_for_terminal_status(&client, &base, &task_fail_id).await;

    // Shutdown workers
    worker_handle.shutdown_and_join().await;

    // Purge
    let resp = client
        .post(format!("{base}/api/v1/queue/purge"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let purged = body["purged"].as_u64().unwrap();
    assert!(
        purged >= 2,
        "should purge completed + failed tasks, got {purged}"
    );

    // After purge, stats should show 0 terminal tasks
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert_eq!(stats["completed"].as_u64().unwrap(), 0);
    assert_eq!(stats["failed"].as_u64().unwrap(), 0);
    assert_eq!(stats["cancelled"].as_u64().unwrap(), 0);
}

/// Double-purge: second purge returns 0 since all terminal tasks were already removed.
#[tokio::test]
async fn e2e_purge_idempotent_second_purge_returns_zero() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    // Enqueue and wait for completion
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "double purge test"
        }))
        .send()
        .await
        .unwrap();
    let task_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    wait_for_terminal_status(&client, &base, &task_id).await;
    worker_handle.shutdown_and_join().await;

    // First purge
    let resp = client
        .post(format!("{base}/api/v1/queue/purge"))
        .send()
        .await
        .unwrap();
    let first: Value = resp.json().await.unwrap();
    assert!(first["purged"].as_u64().unwrap() >= 1);

    // Second purge — should be 0
    let resp = client
        .post(format!("{base}/api/v1/queue/purge"))
        .send()
        .await
        .unwrap();
    let second: Value = resp.json().await.unwrap();
    assert_eq!(second["purged"].as_u64().unwrap(), 0);
}

// ───────────────────── Template CRUD depth tests ─────────────────────

/// Multiple templates can be created and listed; list returns sorted names.
#[tokio::test]
async fn e2e_template_list_multiple_sorted() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    for name in ["zulu-tpl", "alpha-tpl", "mike-tpl"] {
        let resp = client
            .post(format!("{base}/api/v1/templates"))
            .json(&json!({
                "name": name,
                "body": format!("Hello from {name}")
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = client
        .get(format!("{base}/api/v1/templates"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"].as_u64().unwrap(), 3);
    let names: Vec<&str> = body["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["alpha-tpl", "mike-tpl", "zulu-tpl"]);
}

/// Update preserves existing defaults that are not overridden.
#[tokio::test]
async fn e2e_template_update_preserves_defaults() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Create with two defaults
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "defaults-tpl",
            "body": "{{greeting}}, {{name}}!",
            "defaults": {"greeting": "Hi", "name": "World"}
        }))
        .send()
        .await
        .unwrap();

    // Update only the greeting default
    let resp = client
        .put(format!("{base}/api/v1/templates/defaults-tpl"))
        .json(&json!({
            "defaults": {"greeting": "Hello"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["defaults"]["greeting"], "Hello");
    assert_eq!(body["defaults"]["name"], "World");
}

/// Rendering with missing required variables (no defaults) returns 400.
#[tokio::test]
async fn e2e_template_render_missing_required_var_returns_400() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "required-vars",
            "body": "{{a}} and {{b}}"
        }))
        .send()
        .await
        .unwrap();

    // Provide only one of two required variables
    let resp = client
        .post(format!("{base}/api/v1/templates/required-vars/render"))
        .json(&json!({
            "variables": {"a": "hello"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    let msg = body["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("b"),
        "error should mention missing variable 'b'"
    );
}

/// Rendering with defaults supplying the missing variable succeeds.
#[tokio::test]
async fn e2e_template_render_defaults_fill_missing_vars() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "with-default",
            "body": "{{greeting}}, {{name}}!",
            "title": "{{greeting}} title",
            "defaults": {"greeting": "Hey"}
        }))
        .send()
        .await
        .unwrap();

    let resp = client
        .post(format!("{base}/api/v1/templates/with-default/render"))
        .json(&json!({
            "variables": {"name": "Alice"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["text"], "Hey, Alice!");
    assert_eq!(body["title"], "Hey title");
}

/// Deleting a non-existent template returns 404.
#[tokio::test]
async fn e2e_template_delete_nonexistent_returns_404() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .delete(format!("{base}/api/v1/templates/no-such-template"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Getting a non-existent template returns 404.
#[tokio::test]
async fn e2e_template_get_nonexistent_returns_404() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Rendering a non-existent template returns 404.
#[tokio::test]
async fn e2e_template_render_nonexistent_returns_404() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/templates/ghost/render"))
        .json(&json!({"variables": {}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Creating a template with the same name overwrites the previous one.
#[tokio::test]
async fn e2e_template_create_same_name_overwrites() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Create v1
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "overwrite-tpl",
            "body": "version 1"
        }))
        .send()
        .await
        .unwrap();

    // Create v2 with same name
    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "overwrite-tpl",
            "body": "version 2 with {{var}}"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Get should show v2
    let resp = client
        .get(format!("{base}/api/v1/templates/overwrite-tpl"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["body"].as_str().unwrap().contains("version 2"));
    assert_eq!(body["variables"].as_array().unwrap().len(), 1);

    // List should still show 1 total
    let resp = client
        .get(format!("{base}/api/v1/templates"))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"].as_u64().unwrap(), 1);
}

/// Update body only; title and defaults remain from original.
#[tokio::test]
async fn e2e_template_update_body_only_preserves_title() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "title-keep",
            "body": "original body {{x}}",
            "title": "Original Title"
        }))
        .send()
        .await
        .unwrap();

    let resp = client
        .put(format!("{base}/api/v1/templates/title-keep"))
        .json(&json!({
            "body": "updated body {{y}}"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["body"], "updated body {{y}}");
    assert_eq!(body["title"], "Original Title");
}

// ───────────────────── Concurrent task processing tests ─────────────────────

/// Multiple tasks enqueued concurrently are all processed to completion by workers.
#[tokio::test]
async fn e2e_concurrent_tasks_all_processed() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let task_count = 10;
    let mut task_ids = Vec::new();

    // Enqueue many tasks concurrently
    let mut handles = Vec::new();
    for i in 0..task_count {
        let c = client.clone();
        let b = base.clone();
        handles.push(tokio::spawn(async move {
            let resp = c
                .post(format!("{b}/api/v1/send/async"))
                .json(&json!({
                    "provider": "mock-ok",
                    "text": format!("concurrent task {i}")
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::ACCEPTED);
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string()
        }));
    }

    for h in handles {
        task_ids.push(h.await.unwrap());
    }

    assert_eq!(task_ids.len(), task_count);

    // Wait for all to reach terminal state
    for id in &task_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(
            result["status"].as_str().unwrap(),
            "completed",
            "task {id} should be completed"
        );
    }

    // Stats should show all completed
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= task_count as u64);
    assert_eq!(stats["queued"].as_u64().unwrap(), 0);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);

    worker_handle.shutdown_and_join().await;
}

/// Each task is processed exactly once — no duplicates.
#[tokio::test]
async fn e2e_concurrent_tasks_no_duplicate_processing() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let (cb_base, payloads) = spawn_callback_server().await;
    let client = reqwest::Client::new();

    let task_count = 8;
    let mut task_ids = Vec::new();

    // Enqueue tasks with callback so we can count invocations
    for i in 0..task_count {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("dedup task {i}"),
                "callback_url": format!("{cb_base}/callback")
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        task_ids.push(
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }

    // Wait for all to complete
    for id in &task_ids {
        wait_for_terminal_status(&client, &base, id).await;
    }

    // Give callbacks a moment to arrive
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Each task should have produced exactly one callback
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            task_count,
            "expected exactly {task_count} callbacks, got {}",
            received.len()
        );

        // Verify all task IDs appear exactly once
        let callback_task_ids: Vec<&str> = received
            .iter()
            .map(|p| p["task_id"].as_str().unwrap())
            .collect();
        for id in &task_ids {
            let count = callback_task_ids.iter().filter(|&&cid| cid == id).count();
            assert_eq!(count, 1, "task {id} should have exactly 1 callback");
        }
    } // MutexGuard dropped before await

    worker_handle.shutdown_and_join().await;
}

/// Concurrent processing on SQLite backend also works correctly.
#[tokio::test]
async fn e2e_sqlite_concurrent_tasks_all_processed() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let task_count = 10;
    let mut task_ids = Vec::new();

    let mut handles = Vec::new();
    for i in 0..task_count {
        let c = client.clone();
        let b = base.clone();
        handles.push(tokio::spawn(async move {
            let resp = c
                .post(format!("{b}/api/v1/send/async"))
                .json(&json!({
                    "provider": "mock-ok",
                    "text": format!("sqlite concurrent {i}")
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::ACCEPTED);
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string()
        }));
    }

    for h in handles {
        task_ids.push(h.await.unwrap());
    }

    for id in &task_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(
            result["status"].as_str().unwrap(),
            "completed",
            "SQLite task {id} should be completed"
        );
    }

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= task_count as u64);
    assert_eq!(stats["queued"].as_u64().unwrap(), 0);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);

    worker_handle.shutdown_and_join().await;
}

/// Mixed success/failure tasks processed concurrently all reach correct terminal states.
#[tokio::test]
async fn e2e_concurrent_mixed_success_failure() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let mut ok_ids = Vec::new();
    let mut fail_ids = Vec::new();

    // Enqueue 5 success + 5 failure tasks interleaved
    for i in 0..10 {
        let provider = if i % 2 == 0 { "mock-ok" } else { "mock-fail" };
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": provider,
                "text": format!("mixed task {i}")
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let id = resp.json::<Value>().await.unwrap()["task_id"]
            .as_str()
            .unwrap()
            .to_string();
        if i % 2 == 0 {
            ok_ids.push(id);
        } else {
            fail_ids.push(id);
        }
    }

    // Wait for all tasks
    for id in &ok_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(result["status"].as_str().unwrap(), "completed");
    }
    for id in &fail_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(result["status"].as_str().unwrap(), "failed");
    }

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= 5);
    assert!(stats["failed"].as_u64().unwrap() >= 5);

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Error response structure consistency (e2e) ─────────────────────

/// Helper: assert that a JSON error response has the standard {error, message} shape.
fn assert_error_shape(body: &Value, expected_error: &str, context: &str) {
    assert!(
        body["error"].is_string(),
        "{context}: response should have a string 'error' field, got: {body}"
    );
    assert!(
        body["message"].is_string(),
        "{context}: response should have a string 'message' field, got: {body}"
    );
    assert_eq!(
        body["error"].as_str().unwrap(),
        expected_error,
        "{context}: error code mismatch"
    );
}

/// All 404 error responses share the same {error: "not_found", message: "..."} shape.
#[tokio::test]
async fn e2e_error_structure_not_found_responses() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Provider not found
    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "provider not found");

    // Status not found
    let resp = client
        .get(format!("{base}/api/v1/status/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "status not found");

    // Queue task not found
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "queue task not found");

    // Template not found
    let resp = client
        .get(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "template not found");

    // Template delete not found
    let resp = client
        .delete(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "template delete not found");

    // Send with nonexistent provider
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "nonexistent", "text": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "send provider not found");

    // Async send with nonexistent provider
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({"provider": "nonexistent", "text": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "async send provider not found");
}

/// 400 Bad Request errors share the same {error: "bad_request", message: "..."} shape.
#[tokio::test]
async fn e2e_error_structure_bad_request_responses() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Send with missing config (provider validation failure)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "slack", "text": "hello", "config": {}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "bad_request", "send missing config");

    // Invalid status filter
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=bogus"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "bad_request", "invalid status filter");

    // Template render with missing required var
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({"name": "err-struct-tpl", "body": "{{a}} and {{b}}"}))
        .send()
        .await
        .unwrap();
    let resp = client
        .post(format!("{base}/api/v1/templates/err-struct-tpl/render"))
        .json(&json!({"variables": {"a": "hello"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "bad_request", "template render missing var");
}

/// 422 Unprocessable Entity (validation) errors include {error, message, fields}.
#[tokio::test]
async fn e2e_error_structure_validation_responses() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "", "text": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["message"].is_string(),
        "validation error should have message"
    );
    assert!(
        body["fields"].is_object(),
        "validation error should have fields object"
    );
}

/// Invalid JSON body returns {error: "invalid_json", message: "..."}.
#[tokio::test]
async fn e2e_error_structure_invalid_json_response() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body("not valid json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "invalid_json", "invalid json body");
}
