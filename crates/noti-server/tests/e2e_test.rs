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
    spawn_server_sqlite_with_workers, spawn_server_sqlite_with_workers_and_rate_limit,
    spawn_server_sqlite_with_workers_serial, spawn_server_sqlite_without_workers,
    spawn_server_with_auth, spawn_server_with_body_limit, spawn_server_with_cors_permissive,
    spawn_server_with_cors_restricted, spawn_server_with_full_middleware,
    spawn_server_with_rate_limit, spawn_server_with_rate_limit_per_ip,
    spawn_server_with_request_id, spawn_server_with_workers, spawn_server_with_workers_and_rate_limit,
    spawn_server_with_workers_serial, spawn_server_without_workers, wait_for_terminal_status,
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
async fn e2e_api_versions_endpoint() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api/versions"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    // Should have a versions array and a latest field
    let versions = body["versions"].as_array().expect("versions should be an array");
    assert!(!versions.is_empty(), "at least one version should be listed");

    // v1 should be present and stable
    let v1 = versions.iter().find(|v| v["version"] == "v1");
    assert!(v1.is_some(), "v1 should be listed");
    let v1 = v1.unwrap();
    assert_eq!(v1["status"], "stable");
    assert_eq!(v1["deprecated"], false);

    // latest should be v1
    assert_eq!(body["latest"], "v1");
}

#[tokio::test]
async fn e2e_api_versions_in_openapi_spec() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();

    let paths = body["paths"].as_object().unwrap();
    assert!(
        paths.contains_key("/api/versions"),
        "OpenAPI spec should include /api/versions path"
    );

    // Verify the Meta tag exists
    let tags = body["tags"].as_array().unwrap();
    let meta_tag = tags.iter().find(|t| t["name"] == "Meta");
    assert!(meta_tag.is_some(), "Meta tag should be present in OpenAPI spec");
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

#[tokio::test]
async fn e2e_openapi_schema_retry_config_has_backoff_fields() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    // Navigate to RetryConfig schema
    let schemas = &body["components"]["schemas"];
    assert!(
        schemas["RetryConfig"].is_object(),
        "RetryConfig schema should exist in OpenAPI components"
    );

    let retry_props = &schemas["RetryConfig"]["properties"];
    assert!(
        retry_props.is_object(),
        "RetryConfig should have properties"
    );

    // Verify all four fields are present
    assert!(
        retry_props["max_retries"].is_object(),
        "RetryConfig should have max_retries field"
    );
    assert!(
        retry_props["delay_ms"].is_object(),
        "RetryConfig should have delay_ms field"
    );
    assert!(
        retry_props["backoff_multiplier"].is_object(),
        "RetryConfig should have backoff_multiplier field"
    );
    assert!(
        retry_props["max_delay_ms"].is_object(),
        "RetryConfig should have max_delay_ms field"
    );

    // Verify types: backoff_multiplier should be number, max_delay_ms should be integer
    let bm_type = retry_props["backoff_multiplier"]["type"]
        .as_str()
        .unwrap_or("");
    assert!(
        bm_type == "number" || retry_props["backoff_multiplier"]["format"].is_string(),
        "backoff_multiplier should be a number type, got: {bm_type}"
    );

    let md_type = retry_props["max_delay_ms"]["type"].as_str().unwrap_or("");
    assert!(
        md_type == "integer" || retry_props["max_delay_ms"]["format"].is_string(),
        "max_delay_ms should be an integer type, got: {md_type}"
    );
}

#[tokio::test]
async fn e2e_openapi_schema_all_key_components_exist() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    let schemas = body["components"]["schemas"].as_object().unwrap();

    // Verify all key schema components are present
    let expected_schemas = [
        "ApiError",
        "RetryConfig",
        "SendRequest",
        "HealthResponse",
        "EnqueueResponse",
        "StatsResponse",
        "TaskInfo",
        "TemplateResponse",
        "ProviderInfo",
        "MetricsResponse",
    ];

    for name in &expected_schemas {
        assert!(
            schemas.contains_key(*name),
            "missing schema component: {name}"
        );
    }
}

#[tokio::test]
async fn e2e_openapi_schema_all_api_paths_exist() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    let paths = body["paths"].as_object().unwrap();

    // All API routes should be documented
    let expected_paths = [
        "/health",
        "/api/versions",
        "/api/v1/send",
        "/api/v1/send/batch",
        "/api/v1/send/async",
        "/api/v1/send/async/batch",
        "/api/v1/status/{notification_id}",
        "/api/v1/status",
        "/api/v1/status/purge",
        "/api/v1/templates",
        "/api/v1/templates/{name}",
        "/api/v1/templates/{name}/render",
        "/api/v1/providers",
        "/api/v1/providers/{name}",
        "/api/v1/queue/stats",
        "/api/v1/queue/tasks",
        "/api/v1/queue/tasks/{task_id}",
        "/api/v1/queue/tasks/{task_id}/cancel",
        "/api/v1/queue/purge",
        "/api/v1/metrics",
    ];

    for path in &expected_paths {
        assert!(
            paths.contains_key(*path),
            "missing API path in OpenAPI spec: {path}"
        );
    }
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

#[tokio::test]
async fn e2e_purge_statuses_empty() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/status/purge"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["purged"], 0);
    assert!(body["message"].as_str().unwrap().contains("Purged 0"));
}

#[tokio::test]
async fn e2e_purge_statuses_with_max_age() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/status/purge?max_age_secs=60"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["purged"], 0);
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
    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

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

    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

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
    // Enqueue 3 normal tasks, then 1 high-priority task on a server with NO
    // workers.  Start a single worker afterwards so dequeue order reflects
    // priority.  Verify via callback arrival order that the high-priority task
    // is processed before all normal tasks.
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue 3 normal tasks first
    let mut all_ids = Vec::new();
    for i in 0..3 {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("normal-{i}"),
                "priority": "normal",
                "callback_url": &callback_url,
                "metadata": {"order": format!("normal-{i}")}
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        all_ids.push(
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }

    // Enqueue 1 high-priority task (after the normals)
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "high-priority",
            "priority": "high",
            "callback_url": &callback_url,
            "metadata": {"order": "high"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    all_ids.push(
        resp.json::<Value>().await.unwrap()["task_id"]
            .as_str()
            .unwrap()
            .to_string(),
    );

    // Start a single worker — enforces serial processing in priority order.
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for all tasks to reach terminal state
    for id in &all_ids {
        wait_for_terminal_status(&client, &base, id).await;
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify callback order: high-priority task should arrive first
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 4,
            "expected at least 4 callbacks, got {}",
            received.len()
        );
        // First callback must be from the high-priority task
        assert_eq!(
            received[0]["metadata"]["order"], "high",
            "high-priority task should be processed first, but first callback was: {:?}",
            received[0]["metadata"]["order"]
        );
        // Remaining callbacks should all be normal tasks
        for i in 1..4 {
            let order = received[i]["metadata"]["order"].as_str().unwrap_or("");
            assert!(
                order.starts_with("normal"),
                "callback {i} should be a normal task, got: {order}"
            );
        }
    }

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
    let (base, state) =
        spawn_server_sqlite_without_workers(vec![Arc::new(MockOkProvider)]).await;

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

    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

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

    let (base, state) =
        spawn_server_sqlite_without_workers(vec![Arc::new(MockOkProvider)]).await;

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

    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(500)));

    let (base, state) = spawn_server_without_workers(vec![slow]).await;

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
    // Each task takes 200ms to complete
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(200)));

    let (base, state) = spawn_server_without_workers(vec![slow]).await;

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
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(300)));

    let (base, state) =
        spawn_server_without_workers(vec![slow, Arc::new(MockOkProvider)]).await;

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

    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(500)));

    let (base, state) = spawn_server_sqlite_without_workers(vec![slow]).await;

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
    let (_base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

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

// ───────────────────── Granular error codes (e2e) ─────────────────────

/// Verify that 404 error responses include the granular `code` field.
#[tokio::test]
async fn e2e_error_codes_not_found_responses_have_code() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Provider not found → PROVIDER_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "PROVIDER_NOT_FOUND");

    // Template not found → TEMPLATE_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "TEMPLATE_NOT_FOUND");

    // Notification status not found → NOTIFICATION_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/status/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "NOTIFICATION_NOT_FOUND");

    // Queue task not found → TASK_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "TASK_NOT_FOUND");
}

/// Verify that 400 error responses include the granular `code` field.
#[tokio::test]
async fn e2e_error_codes_bad_request_responses_have_code() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Config validation failure → CONFIG_VALIDATION_FAILED
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "slack", "text": "hello", "config": {}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["code"], "CONFIG_VALIDATION_FAILED");

    // Invalid status filter → INVALID_PARAMETER
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=bogus"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["code"], "INVALID_PARAMETER");

    // Template variable missing → TEMPLATE_VARIABLE_MISSING
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({"name": "code-test-tpl", "body": "{{a}} and {{b}}"}))
        .send()
        .await
        .unwrap();
    let resp = client
        .post(format!("{base}/api/v1/templates/code-test-tpl/render"))
        .json(&json!({"variables": {"a": "hello"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["code"], "TEMPLATE_VARIABLE_MISSING");
}

/// Verify that error responses without a code omit the field entirely (backward compat).
#[tokio::test]
async fn e2e_error_codes_absent_when_not_applicable() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

    // Invalid JSON body → no code field (handled by ValidatedJsonRejection, not ApiError)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body("not valid json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "invalid_json");
    assert!(
        body.get("code").is_none() || body["code"].is_null(),
        "invalid_json error should not have a code field"
    );
}

// ───────────────────── Batch async: mixed valid/invalid providers + priorities ─────────────────────

/// Batch-enqueue items with a mix of valid and invalid providers at different priorities.
/// Verify that invalid providers are rejected per-item (not failing the whole batch),
/// valid items are enqueued and processed in priority order, and the response counts are correct.
#[tokio::test]
async fn e2e_batch_async_mixed_providers_and_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Mix of valid (mock-ok) and invalid (nonexistent) providers at various priorities.
    // Valid items: urgent, normal, low — invalid items: high, low
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "nonexistent-a",
                    "text": "invalid-high",
                    "priority": "high",
                    "metadata": {"order": "invalid-high"}
                },
                {
                    "provider": "mock-ok",
                    "text": "valid-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "valid-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-urgent"}
                },
                {
                    "provider": "nonexistent-b",
                    "text": "invalid-low",
                    "priority": "low",
                    "metadata": {"order": "invalid-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "valid-normal",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-normal"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 5);
    assert_eq!(body["enqueued"], 3);
    assert_eq!(body["failed"], 2);

    // Verify per-item results
    let results = body["results"].as_array().unwrap();
    // index 0: invalid
    assert!(!results[0]["success"].as_bool().unwrap());
    assert_eq!(results[0]["provider"], "nonexistent-a");
    assert!(results[0]["error"].as_str().unwrap().contains("not found"));
    // index 1: valid
    assert!(results[1]["success"].as_bool().unwrap());
    assert!(results[1]["task_id"].is_string());
    // index 2: valid
    assert!(results[2]["success"].as_bool().unwrap());
    // index 3: invalid
    assert!(!results[3]["success"].as_bool().unwrap());
    assert_eq!(results[3]["provider"], "nonexistent-b");
    // index 4: valid
    assert!(results[4]["success"].as_bool().unwrap());

    // Collect task IDs of successfully enqueued items
    let task_ids: Vec<String> = results
        .iter()
        .filter(|r| r["success"].as_bool().unwrap_or(false))
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(task_ids.len(), 3);

    // Start a single worker AFTER enqueue to ensure strict priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for all valid tasks to complete
    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callback order: urgent → normal → low (only valid items)
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 3,
            "expected at least 3 callbacks, got {}",
            received.len()
        );

        let expected_order = ["valid-urgent", "valid-normal", "valid-low"];
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

/// Same as above but using SQLite queue backend.
#[tokio::test]
async fn e2e_sqlite_batch_async_mixed_providers_and_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, state) =
        spawn_server_sqlite_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Same mix: valid (mock-ok) and invalid (nonexistent) at various priorities
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite-valid-high",
                    "priority": "high",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-high"}
                },
                {
                    "provider": "nonexistent-x",
                    "text": "sqlite-invalid-urgent",
                    "priority": "urgent",
                    "metadata": {"order": "invalid-urgent"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-valid-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-valid-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-urgent"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 4);
    assert_eq!(body["enqueued"], 3);
    assert_eq!(body["failed"], 1);

    // Verify the failed item
    let results = body["results"].as_array().unwrap();
    assert!(results[0]["success"].as_bool().unwrap()); // valid-high
    assert!(!results[1]["success"].as_bool().unwrap()); // invalid-urgent
    assert!(results[2]["success"].as_bool().unwrap()); // valid-low
    assert!(results[3]["success"].as_bool().unwrap()); // valid-urgent

    // Collect valid task IDs
    let task_ids: Vec<String> = results
        .iter()
        .filter(|r| r["success"].as_bool().unwrap_or(false))
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(task_ids.len(), 3);

    // Start single worker after enqueue for strict priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callback order: urgent → high → low (only valid items, priority-sorted)
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 3,
            "SQLite: expected at least 3 callbacks, got {}",
            received.len()
        );

        let expected_order = ["valid-urgent", "valid-high", "valid-low"];
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

/// Batch async with ALL invalid providers — verify 202 response with all items failed.
#[tokio::test]
async fn e2e_batch_async_all_invalid_providers_returns_202() {
    let (base, _state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {"provider": "bad-1", "text": "a", "priority": "urgent"},
                {"provider": "bad-2", "text": "b", "priority": "high"},
                {"provider": "bad-3", "text": "c", "priority": "low"}
            ]
        }))
        .send()
        .await
        .unwrap();

    // Still 202 even though all items failed — partial success model
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["enqueued"], 0);
    assert_eq!(body["failed"], 3);

    let results = body["results"].as_array().unwrap();
    for (i, result) in results.iter().enumerate() {
        assert!(
            !result["success"].as_bool().unwrap(),
            "item {i} should fail"
        );
        assert!(
            result["error"].as_str().unwrap().contains("not found"),
            "item {i} error should mention 'not found'"
        );
        assert!(
            result["task_id"].is_null(),
            "item {i} should have no task_id"
        );
    }
}

// ───────────────────── Batch async with MockFailProvider (always-failing valid provider) ─────────────────────

/// Batch-enqueue items where some use `mock-fail` (a registered provider that always errors at send time).
/// Unlike invalid/nonexistent providers which fail at enqueue time, `mock-fail` items are successfully
/// enqueued but fail during worker processing. Verify that:
/// 1. All items with registered providers (mock-ok and mock-fail) are enqueued successfully.
/// 2. Items with nonexistent providers fail at enqueue time (same partial-success model).
/// 3. mock-fail tasks reach `failed` status after worker processing.
/// 4. mock-ok tasks reach `completed` status.
/// 5. Callback order respects priority (urgent before normal before low).
#[tokio::test]
async fn e2e_batch_async_mock_fail_provider_with_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    // Use the serial helper which already registers MockOkProvider + MockFailProvider
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Batch: mix of mock-ok (always succeeds) and mock-fail (always fails at send time)
    // mock-fail items are valid providers — they should be enqueued successfully but fail during processing.
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-fail",
                    "text": "fail-urgent",
                    "priority": "urgent",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-urgent"}
                },
                {
                    "provider": "mock-ok",
                    "text": "ok-normal",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-normal"}
                },
                {
                    "provider": "mock-fail",
                    "text": "fail-low",
                    "priority": "low",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "ok-high",
                    "priority": "high",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-high"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    // All 4 items use registered providers — all should be enqueued
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 4);
    assert_eq!(body["enqueued"], 4, "all items use registered providers");
    assert_eq!(body["failed"], 0, "no enqueue-time failures");

    let results = body["results"].as_array().unwrap();
    let task_ids: Vec<String> = results
        .iter()
        .map(|r| {
            assert!(
                r["success"].as_bool().unwrap(),
                "all items should succeed at enqueue"
            );
            r["task_id"].as_str().unwrap().to_string()
        })
        .collect();
    assert_eq!(task_ids.len(), 4);

    // Wait for all tasks to reach terminal state
    let mut statuses = Vec::new();
    for task_id in &task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        statuses.push((
            task["metadata"]["order"].as_str().unwrap().to_string(),
            task["status"].as_str().unwrap().to_string(),
        ));
    }

    // Verify mock-fail tasks failed, mock-ok tasks completed
    for (order, status) in &statuses {
        if order.starts_with("fail-") {
            assert_eq!(status, "failed", "{order} should have failed status");
        } else {
            assert_eq!(status, "completed", "{order} should have completed status");
        }
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callbacks arrived in priority order: urgent → high → normal → low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 4,
            "expected at least 4 callbacks, got {}",
            received.len()
        );

        let expected_order = ["fail-urgent", "ok-high", "ok-normal", "fail-low"];
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

/// Same test as above but using SQLite queue backend.
#[tokio::test]
async fn e2e_sqlite_batch_async_mock_fail_provider_with_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite-ok-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-low"}
                },
                {
                    "provider": "mock-fail",
                    "text": "sqlite-fail-urgent",
                    "priority": "urgent",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-urgent"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-ok-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-urgent"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["enqueued"], 3);
    assert_eq!(body["failed"], 0);

    let results = body["results"].as_array().unwrap();
    let task_ids: Vec<String> = results
        .iter()
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();

    // Wait for terminal states
    let mut statuses = Vec::new();
    for task_id in &task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        statuses.push((
            task["metadata"]["order"].as_str().unwrap().to_string(),
            task["status"].as_str().unwrap().to_string(),
        ));
    }

    // mock-fail should fail, mock-ok should complete
    for (order, status) in &statuses {
        if order.starts_with("fail-") {
            assert_eq!(
                status, "failed",
                "SQLite: {order} should have failed status"
            );
        } else {
            assert_eq!(
                status, "completed",
                "SQLite: {order} should have completed status"
            );
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Callback order: urgent tasks first (fail-urgent, ok-urgent in enqueue order), then low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 3,
            "SQLite: expected at least 3 callbacks, got {}",
            received.len()
        );

        // Both urgent tasks should come before the low task
        let last_order = received[2]["metadata"]["order"].as_str().unwrap();
        assert_eq!(
            last_order, "ok-low",
            "SQLite: last callback should be the low-priority task"
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// Batch with mixed mock-ok, mock-fail, and nonexistent providers — verify that
/// nonexistent providers fail at enqueue time while mock-fail fails at processing time.
#[tokio::test]
async fn e2e_batch_async_mock_fail_mixed_with_nonexistent() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-fail",
                    "text": "fail-at-send",
                    "priority": "urgent",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-at-send"}
                },
                {
                    "provider": "nonexistent",
                    "text": "fail-at-enqueue",
                    "priority": "urgent",
                    "metadata": {"order": "fail-at-enqueue"}
                },
                {
                    "provider": "mock-ok",
                    "text": "succeeds",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "succeeds"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["enqueued"], 2, "mock-fail + mock-ok enqueued");
    assert_eq!(body["failed"], 1, "nonexistent fails at enqueue");

    let results = body["results"].as_array().unwrap();
    // index 0: mock-fail — enqueued
    assert!(results[0]["success"].as_bool().unwrap());
    assert!(results[0]["task_id"].is_string());
    // index 1: nonexistent — failed at enqueue
    assert!(!results[1]["success"].as_bool().unwrap());
    assert!(results[1]["error"].as_str().unwrap().contains("not found"));
    assert!(results[1]["task_id"].is_null());
    // index 2: mock-ok — enqueued
    assert!(results[2]["success"].as_bool().unwrap());
    assert!(results[2]["task_id"].is_string());

    // Collect enqueued task IDs
    let enqueued_ids: Vec<String> = results
        .iter()
        .filter(|r| r["success"].as_bool().unwrap_or(false))
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(enqueued_ids.len(), 2);

    // Wait for terminal states
    let fail_task = wait_for_terminal_status(&client, &base, &enqueued_ids[0]).await;
    let ok_task = wait_for_terminal_status(&client, &base, &enqueued_ids[1]).await;

    assert_eq!(fail_task["status"].as_str().unwrap(), "failed");
    assert_eq!(ok_task["status"].as_str().unwrap(), "completed");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callbacks: urgent (mock-fail) first, then normal (mock-ok)
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 2,
            "expected at least 2 callbacks, got {}",
            received.len()
        );
        assert_eq!(
            received[0]["metadata"]["order"].as_str().unwrap(),
            "fail-at-send"
        );
        assert_eq!(
            received[1]["metadata"]["order"].as_str().unwrap(),
            "succeeds"
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Concurrent batch async requests ─────────────────────

/// Fire multiple batch async requests concurrently and verify all are accepted
/// and all tasks eventually reach terminal states.
#[tokio::test]
async fn e2e_concurrent_batch_async_requests_all_accepted() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Fire 5 concurrent batch requests, each with 2 items
    let mut handles = Vec::new();
    for batch_idx in 0..5u32 {
        let client = client.clone();
        let base = base.clone();
        let callback_url = callback_url.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("batch-{batch_idx}-item-0"),
                            "priority": "normal",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "0"}
                        },
                        {
                            "provider": "mock-ok",
                            "text": format!("batch-{batch_idx}-item-1"),
                            "priority": "normal",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "1"}
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            let status = resp.status();
            let body: Value = resp.json().await.unwrap();
            (status, body)
        }));
    }

    // Collect all responses
    let mut all_task_ids = Vec::new();
    for handle in handles {
        let (status, body) = handle.await.unwrap();
        assert_eq!(
            status,
            StatusCode::ACCEPTED,
            "all batch requests should return 202"
        );
        assert_eq!(body["total"], 2);
        assert_eq!(body["enqueued"], 2);
        assert_eq!(body["failed"], 0);

        let results = body["results"].as_array().unwrap();
        for r in results {
            all_task_ids.push(r["task_id"].as_str().unwrap().to_string());
        }
    }

    assert_eq!(all_task_ids.len(), 10, "5 batches × 2 items = 10 tasks");

    // Wait for all tasks to reach terminal state
    for task_id in &all_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(
            task["status"].as_str().unwrap(),
            "completed",
            "task {task_id} should complete"
        );
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify all 10 callbacks arrived
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            10,
            "expected 10 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// Concurrent batch requests with a mix of mock-ok and mock-fail providers.
/// Verify all batches are accepted and tasks end in correct terminal states.
#[tokio::test]
async fn e2e_concurrent_batch_async_with_mixed_providers() {
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = reqwest::Client::new();

    // Fire 3 concurrent batches: each has 1 mock-ok and 1 mock-fail item
    let mut handles = Vec::new();
    for batch_idx in 0..3u32 {
        let client = client.clone();
        let base = base.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("batch-{batch_idx}-ok"),
                            "metadata": {"batch": format!("{batch_idx}"), "provider": "ok"}
                        },
                        {
                            "provider": "mock-fail",
                            "text": format!("batch-{batch_idx}-fail"),
                            "metadata": {"batch": format!("{batch_idx}"), "provider": "fail"}
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            let status = resp.status();
            let body: Value = resp.json().await.unwrap();
            (status, body)
        }));
    }

    let mut ok_task_ids = Vec::new();
    let mut fail_task_ids = Vec::new();

    for handle in handles {
        let (status, body) = handle.await.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["enqueued"], 2, "both providers are registered");

        let results = body["results"].as_array().unwrap();
        // First item is mock-ok, second is mock-fail
        ok_task_ids.push(results[0]["task_id"].as_str().unwrap().to_string());
        fail_task_ids.push(results[1]["task_id"].as_str().unwrap().to_string());
    }

    // Wait for all tasks
    for task_id in &ok_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(task["status"].as_str().unwrap(), "completed");
    }
    for task_id in &fail_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(task["status"].as_str().unwrap(), "failed");
    }

    worker_handle.shutdown_and_join().await;
}

/// Concurrent batch requests with SQLite backend — ensure no database contention issues.
#[tokio::test]
async fn e2e_sqlite_concurrent_batch_async_requests() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![]).await;

    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // Fire 4 concurrent batch requests, each with 3 items
    let mut handles = Vec::new();
    for batch_idx in 0..4u32 {
        let client = client.clone();
        let base = base.clone();
        let callback_url = callback_url.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("sqlite-b{batch_idx}-0"),
                            "priority": "high",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "0"}
                        },
                        {
                            "provider": "mock-ok",
                            "text": format!("sqlite-b{batch_idx}-1"),
                            "priority": "normal",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "1"}
                        },
                        {
                            "provider": "mock-fail",
                            "text": format!("sqlite-b{batch_idx}-fail"),
                            "priority": "low",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "fail"}
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            let status = resp.status();
            let body: Value = resp.json().await.unwrap();
            (status, body)
        }));
    }

    let mut all_task_ids = Vec::new();
    for handle in handles {
        let (status, body) = handle.await.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["total"], 3);
        assert_eq!(body["enqueued"], 3, "all providers are registered");
        assert_eq!(body["failed"], 0);

        let results = body["results"].as_array().unwrap();
        for r in results {
            all_task_ids.push(r["task_id"].as_str().unwrap().to_string());
        }
    }

    assert_eq!(all_task_ids.len(), 12, "4 batches × 3 items = 12 tasks");

    // Wait for all tasks to reach terminal state
    let mut completed = 0;
    let mut failed = 0;
    for task_id in &all_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        match task["status"].as_str().unwrap() {
            "completed" => completed += 1,
            "failed" => failed += 1,
            other => panic!("unexpected status: {other}"),
        }
    }

    // 4 batches × 2 mock-ok = 8 completed, 4 batches × 1 mock-fail = 4 failed
    assert_eq!(completed, 8, "expected 8 completed tasks");
    assert_eq!(failed, 4, "expected 4 failed tasks");

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // All 12 tasks should have generated callbacks
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            12,
            "SQLite: expected 12 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Batch async with retry policies (mock-flaky + retry config) ─────────────────────

/// Batch-enqueue items where some use `mock-flaky` (fails first N calls then succeeds).
/// With retry policy configured, flaky tasks should eventually succeed.
#[tokio::test]
async fn e2e_batch_async_flaky_with_retry_succeeds() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "flaky retry batch item 1",
                    "retry": {"max_retries": 3, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "high"
                },
                {
                    "provider": "mock-ok",
                    "text": "reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "normal"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);
    assert_eq!(body["failed"], 0);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "completed",
        "flaky task should succeed after retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 3,
        "flaky task should have taken multiple attempts"
    );

    assert_eq!(task_ok["status"], "completed");
    assert_eq!(task_ok["attempts"].as_u64().unwrap(), 1);

    // Both callbacks should arrive
    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "expected 2 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// Batch-enqueue items where flaky provider has too few retries configured —
/// the task should fail after exhausting retries, while mock-ok still succeeds.
#[tokio::test]
async fn e2e_batch_async_flaky_retry_exhausted_fails() {
    let (callback_base, payloads) = spawn_callback_server().await;
    // MockFlakyProvider fails first 5 calls — with max_retries=1, only 2 total attempts → fails
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(5));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "flaky exhaustion batch",
                    "retry": {"max_retries": 1, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "urgent"
                },
                {
                    "provider": "mock-ok",
                    "text": "reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "low"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "failed",
        "flaky task should fail after exhausting retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 2,
        "flaky task should have attempted at least 2 times"
    );

    assert_eq!(task_ok["status"], "completed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "expected 2 callbacks, got {}",
            received.len()
        );

        // Verify the flaky callback reports failure
        let flaky_cb = received
            .iter()
            .find(|p| p["task_id"].as_str() == Some(task_id_flaky.as_str()))
            .expect("should find flaky task callback");
        assert_eq!(flaky_cb["status"], "failed");
    }

    worker_handle.shutdown_and_join().await;
}

/// Batch with mixed retry policies: flaky with sufficient retries, mock-fail with zero retries,
/// mock-ok with no retries, mock-fail with retries. Verifies each task gets its own retry behavior.
#[tokio::test]
async fn e2e_batch_async_mixed_retry_policies() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "flaky with enough retries",
                    "retry": {"max_retries": 5, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "always fails with zero retries",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-ok",
                    "text": "always succeeds",
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "always fails with retries",
                    "retry": {"max_retries": 2, "delay_ms": 10},
                    "callback_url": &callback_url
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 4);
    assert_eq!(body["failed"], 0);

    let task_ids: Vec<String> = (0..4)
        .map(|i| body["results"][i]["task_id"].as_str().unwrap().to_string())
        .collect();

    // Wait for all tasks to reach terminal state
    let mut terminal_statuses = Vec::new();
    for tid in &task_ids {
        let task = wait_for_terminal_status(&client, &base, tid).await;
        terminal_statuses.push(task);
    }

    // Item 0: mock-flaky with 5 retries → should succeed (fails 2, succeeds on 3rd)
    assert_eq!(
        terminal_statuses[0]["status"], "completed",
        "flaky with enough retries should complete"
    );

    // Item 1: mock-fail with 0 retries → should fail immediately (1 attempt only)
    assert_eq!(
        terminal_statuses[1]["status"], "failed",
        "mock-fail with zero retries should fail"
    );
    assert_eq!(
        terminal_statuses[1]["attempts"].as_u64().unwrap(),
        1,
        "zero-retry mock-fail should only try once"
    );

    // Item 2: mock-ok → always succeeds on first try
    assert_eq!(terminal_statuses[2]["status"], "completed");
    assert_eq!(terminal_statuses[2]["attempts"].as_u64().unwrap(), 1);

    // Item 3: mock-fail with 2 retries → should fail after 3 attempts (1 + 2 retries)
    assert_eq!(
        terminal_statuses[3]["status"], "failed",
        "mock-fail with retries should still fail"
    );
    assert!(
        terminal_statuses[3]["attempts"].as_u64().unwrap() >= 3,
        "mock-fail should exhaust all retries"
    );

    // All 4 callbacks should arrive
    tokio::time::sleep(Duration::from_millis(300)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            4,
            "expected 4 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Concurrent batch async with rate limiting ─────────────────────

/// Send multiple concurrent batch requests with rate limiting enabled.
/// Some requests should be accepted and some rejected per rate limit quota.
#[tokio::test]
async fn e2e_concurrent_batch_async_with_rate_limit_partial_reject() {
    let (callback_base, payloads) = spawn_callback_server().await;
    // Rate limit: 3 requests per 60s window. We'll send 5 concurrent batch requests.
    let (base, worker_handle, _max_requests) =
        spawn_server_with_workers_and_rate_limit(vec![], 3, 60).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let mut handles = Vec::new();
    for i in 0..5 {
        let client = client.clone();
        let base = base.clone();
        let cb_url = callback_url.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("rate-limited batch item {i}"),
                            "callback_url": &cb_url,
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            resp.status()
        }));
    }

    let mut accepted = 0u32;
    let mut rate_limited = 0u32;
    for handle in handles {
        let status = handle.await.unwrap();
        match status {
            StatusCode::ACCEPTED => accepted += 1,
            StatusCode::TOO_MANY_REQUESTS => rate_limited += 1,
            other => panic!("unexpected status: {other}"),
        }
    }

    // At most 3 should be accepted (rate limit), at least 2 should be rejected
    assert!(
        accepted <= 3,
        "at most 3 requests should pass rate limit, got {accepted}"
    );
    assert!(
        rate_limited >= 2,
        "at least 2 requests should be rate limited, got {rate_limited}"
    );

    // Wait for accepted tasks to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len() as u32,
            accepted,
            "callbacks should match accepted count: expected {accepted}, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// Rate limited server: a single batch request within quota should succeed normally.
#[tokio::test]
async fn e2e_batch_async_within_rate_limit_succeeds() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle, _max) =
        spawn_server_with_workers_and_rate_limit(vec![], 10, 60).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "rate limited batch 1",
                    "callback_url": &callback_url,
                },
                {
                    "provider": "mock-ok",
                    "text": "rate limited batch 2",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);
    assert_eq!(body["failed"], 0);

    // Verify rate limit headers are present on a separate request
    let health_resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert!(health_resp.headers().contains_key("x-ratelimit-limit"));

    let task_ids: Vec<String> = (0..2)
        .map(|i| body["results"][i]["task_id"].as_str().unwrap().to_string())
        .collect();

    for tid in &task_ids {
        let task = wait_for_terminal_status(&client, &base, tid).await;
        assert_eq!(task["status"], "completed");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(received.len(), 2);
    }

    worker_handle.shutdown_and_join().await;
}

/// Rate limit exhausted mid-sequence: first batch goes through, second batch gets 429.
#[tokio::test]
async fn e2e_sequential_batch_async_rate_limit_exhaustion() {
    let (callback_base, payloads) = spawn_callback_server().await;
    // Only 2 requests allowed per 60s
    let (base, worker_handle, _max) = spawn_server_with_workers_and_rate_limit(vec![], 2, 60).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // First batch — should succeed (request 1)
    let resp1 = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "first batch",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::ACCEPTED);

    // Second batch — should succeed (request 2)
    let resp2 = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "second batch",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::ACCEPTED);

    // Third batch — should be rate limited (request 3 > quota 2)
    let resp3 = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "third batch - should be rejected",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp3.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "third request should be rate limited"
    );
    let body_429: Value = resp3.json().await.unwrap();
    assert_eq!(body_429["error"], "rate limit exceeded");

    // Wait for the 2 accepted tasks to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "only 2 accepted tasks should produce callbacks"
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── SQLite batch async retry policy tests ─────────────────────

/// SQLite mirror of `e2e_batch_async_flaky_with_retry_succeeds`.
/// Batch-enqueue items where some use `mock-flaky` on SQLite queue backend.
/// With retry policy configured, flaky tasks should eventually succeed.
#[tokio::test]
async fn e2e_sqlite_batch_async_flaky_with_retry_succeeds() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "sqlite flaky retry batch item 1",
                    "retry": {"max_retries": 3, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "high"
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "normal"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);
    assert_eq!(body["failed"], 0);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "completed",
        "SQLite: flaky task should succeed after retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 3,
        "SQLite: flaky task should have taken multiple attempts"
    );

    assert_eq!(task_ok["status"], "completed");
    assert_eq!(task_ok["attempts"].as_u64().unwrap(), 1);

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "SQLite: expected 2 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// SQLite mirror of `e2e_batch_async_flaky_retry_exhausted_fails`.
/// Batch-enqueue items where flaky provider has too few retries configured on SQLite backend —
/// the task should fail after exhausting retries, while mock-ok still succeeds.
#[tokio::test]
async fn e2e_sqlite_batch_async_flaky_retry_exhausted_fails() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(5));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "sqlite flaky exhaustion batch",
                    "retry": {"max_retries": 1, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "urgent"
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "low"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "failed",
        "SQLite: flaky task should fail after exhausting retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 2,
        "SQLite: flaky task should have attempted at least 2 times"
    );

    assert_eq!(task_ok["status"], "completed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "SQLite: expected 2 callbacks, got {}",
            received.len()
        );

        let flaky_cb = received
            .iter()
            .find(|p| p["task_id"].as_str() == Some(task_id_flaky.as_str()))
            .expect("SQLite: should find flaky task callback");
        assert_eq!(flaky_cb["status"], "failed");
    }

    worker_handle.shutdown_and_join().await;
}

/// SQLite mirror of `e2e_batch_async_mixed_retry_policies`.
/// Batch with mixed retry policies on SQLite backend: flaky with sufficient retries,
/// mock-fail with zero retries, mock-ok with no retries, mock-fail with retries.
#[tokio::test]
async fn e2e_sqlite_batch_async_mixed_retry_policies() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "sqlite flaky with enough retries",
                    "retry": {"max_retries": 5, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "sqlite always fails with zero retries",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite always succeeds",
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "sqlite always fails with retries",
                    "retry": {"max_retries": 2, "delay_ms": 10},
                    "callback_url": &callback_url
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 4);
    assert_eq!(body["failed"], 0);

    let task_ids: Vec<String> = (0..4)
        .map(|i| body["results"][i]["task_id"].as_str().unwrap().to_string())
        .collect();

    let mut terminal_statuses = Vec::new();
    for tid in &task_ids {
        let task = wait_for_terminal_status(&client, &base, tid).await;
        terminal_statuses.push(task);
    }

    assert_eq!(
        terminal_statuses[0]["status"], "completed",
        "SQLite: flaky with enough retries should complete"
    );

    assert_eq!(
        terminal_statuses[1]["status"], "failed",
        "SQLite: mock-fail with zero retries should fail"
    );
    assert_eq!(
        terminal_statuses[1]["attempts"].as_u64().unwrap(),
        1,
        "SQLite: zero-retry mock-fail should only try once"
    );

    assert_eq!(terminal_statuses[2]["status"], "completed");
    assert_eq!(terminal_statuses[2]["attempts"].as_u64().unwrap(), 1);

    assert_eq!(
        terminal_statuses[3]["status"], "failed",
        "SQLite: mock-fail with retries should still fail"
    );
    assert!(
        terminal_statuses[3]["attempts"].as_u64().unwrap() >= 3,
        "SQLite: mock-fail should exhaust all retries"
    );

    tokio::time::sleep(Duration::from_millis(300)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            4,
            "SQLite: expected 4 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── SQLite concurrent batch async with rate limiting ─────────────────────

/// SQLite mirror of `e2e_concurrent_batch_async_with_rate_limit_partial_reject`.
/// Send multiple concurrent batch requests with rate limiting enabled on SQLite backend.
/// Some requests should be accepted and some rejected per rate limit quota.
#[tokio::test]
async fn e2e_sqlite_concurrent_batch_async_with_rate_limit_partial_reject() {
    let (callback_base, payloads) = spawn_callback_server().await;
    // Rate limit: 3 requests per 60s window. We'll send 5 concurrent batch requests.
    let (base, worker_handle, _max_requests) =
        spawn_server_sqlite_with_workers_and_rate_limit(vec![], 3, 60).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let mut handles = Vec::new();
    for i in 0..5 {
        let client = client.clone();
        let base = base.clone();
        let cb_url = callback_url.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("sqlite rate-limited batch item {i}"),
                            "callback_url": &cb_url,
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            resp.status()
        }));
    }

    let mut accepted = 0u32;
    let mut rate_limited = 0u32;
    for handle in handles {
        let status = handle.await.unwrap();
        match status {
            StatusCode::ACCEPTED => accepted += 1,
            StatusCode::TOO_MANY_REQUESTS => rate_limited += 1,
            other => panic!("SQLite: unexpected status: {other}"),
        }
    }

    // At most 3 should be accepted (rate limit), at least 2 should be rejected
    assert!(
        accepted <= 3,
        "SQLite: at most 3 requests should pass rate limit, got {accepted}"
    );
    assert!(
        rate_limited >= 2,
        "SQLite: at least 2 requests should be rate limited, got {rate_limited}"
    );

    // Wait for accepted tasks to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len() as u32,
            accepted,
            "SQLite: callbacks should match accepted count: expected {accepted}, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// SQLite mirror of `e2e_batch_async_within_rate_limit_succeeds`.
/// Rate limited server with SQLite backend: a single batch request within quota should succeed.
#[tokio::test]
async fn e2e_sqlite_batch_async_within_rate_limit_succeeds() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle, _max) =
        spawn_server_sqlite_with_workers_and_rate_limit(vec![], 10, 60).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite rate limited batch 1",
                    "callback_url": &callback_url,
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite rate limited batch 2",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);
    assert_eq!(body["failed"], 0);

    // Verify rate limit headers are present on a separate request
    let health_resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert!(health_resp.headers().contains_key("x-ratelimit-limit"));

    let task_ids: Vec<String> = (0..2)
        .map(|i| body["results"][i]["task_id"].as_str().unwrap().to_string())
        .collect();

    for tid in &task_ids {
        let task = wait_for_terminal_status(&client, &base, tid).await;
        assert_eq!(task["status"], "completed");
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(received.len(), 2);
    }

    worker_handle.shutdown_and_join().await;
}

/// SQLite mirror of `e2e_sequential_batch_async_rate_limit_exhaustion`.
/// Rate limit exhausted mid-sequence on SQLite backend: first batch goes through, second batch gets 429.
#[tokio::test]
async fn e2e_sqlite_sequential_batch_async_rate_limit_exhaustion() {
    let (callback_base, payloads) = spawn_callback_server().await;
    // Only 2 requests allowed per 60s
    let (base, worker_handle, _max) =
        spawn_server_sqlite_with_workers_and_rate_limit(vec![], 2, 60).await;
    let client = reqwest::Client::new();
    let callback_url = format!("{callback_base}/callback");

    // First batch — should succeed (request 1)
    let resp1 = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite first batch",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp1.status(), StatusCode::ACCEPTED);

    // Second batch — should succeed (request 2)
    let resp2 = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite second batch",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp2.status(), StatusCode::ACCEPTED);

    // Third batch — should be rate limited (request 3 > quota 2)
    let resp3 = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite third batch - should be rejected",
                    "callback_url": &callback_url,
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp3.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "SQLite: third request should be rate limited"
    );
    let body_429: Value = resp3.json().await.unwrap();
    assert_eq!(body_429["error"], "rate limit exceeded");

    // Wait for the 2 accepted tasks to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "SQLite: only 2 accepted tasks should produce callbacks"
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Backoff delay timing (e2e) ─────────────────────

#[tokio::test]
async fn e2e_backoff_delay_timing_flaky_task() {
    // MockFlakyProvider fails first 2 calls, then succeeds on the 3rd.
    // With delay_ms=200 (fixed), the queue should hold the task for ~200ms per retry.
    // Total expected wall-clock time >= 200ms * 2 retries = 400ms.
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "backoff timing test",
            "retry": {"max_retries": 3, "delay_ms": 200}
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "flaky task should eventually succeed after retries"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "expected at least 3 attempts, got {}",
        task["attempts"]
    );
    // 2 retries × 200ms delay = 400ms minimum (allow some slack for poll interval)
    assert!(
        elapsed >= Duration::from_millis(350),
        "backoff delay should enforce at least ~400ms total delay, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_backoff_delay_timing_exhausted_retries() {
    // MockFailProvider always fails. With max_retries=2, delay_ms=150,
    // the task should fail after 3 attempts with >= 300ms total delay.
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "backoff exhaustion timing test",
            "retry": {"max_retries": 2, "delay_ms": 150}
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "failed");
    // 2 retries × 150ms = 300ms minimum
    assert!(
        elapsed >= Duration::from_millis(250),
        "backoff delay should enforce at least ~300ms before final failure, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_backoff_delay_zero_delay_is_fast() {
    // With delay_ms=0, retries should happen immediately (no backoff delay).
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "zero delay test",
            "retry": {"max_retries": 3, "delay_ms": 0}
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "expected at least 3 attempts"
    );
    // With zero delay, should complete well under 2 seconds (just poll intervals)
    assert!(
        elapsed < Duration::from_secs(2),
        "zero delay retries should be fast, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_backoff_delay_timing_flaky_task() {
    // Same as e2e_backoff_delay_timing_flaky_task but with SQLite queue backend.
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "sqlite backoff timing test",
            "retry": {"max_retries": 3, "delay_ms": 200}
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "SQLite: flaky task should eventually succeed after retries"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "SQLite: expected at least 3 attempts, got {}",
        task["attempts"]
    );
    assert!(
        elapsed >= Duration::from_millis(350),
        "SQLite: backoff delay should enforce at least ~400ms total, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_backoff_delay_timing_exhausted_retries() {
    // Same as e2e_backoff_delay_timing_exhausted_retries but with SQLite queue backend.
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "sqlite backoff exhaustion timing test",
            "retry": {"max_retries": 2, "delay_ms": 150}
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "failed");
    assert!(
        elapsed >= Duration::from_millis(250),
        "SQLite: backoff delay should enforce at least ~300ms before final failure, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Exponential backoff via API (e2e) ─────────────────────

#[tokio::test]
async fn e2e_exponential_backoff_api_flaky_task() {
    // Test that backoff_multiplier in the API request produces exponential delays.
    // MockFlakyProvider fails first 2 calls, succeeds on 3rd.
    // With delay_ms=100 and backoff_multiplier=2.0:
    //   attempt 1 fails → wait 100ms
    //   attempt 2 fails → wait 200ms
    //   attempt 3 succeeds
    // Total backoff ≥ 250ms (100 + 200 = 300, minus timing slack)
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "exponential backoff test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 100,
                "backoff_multiplier": 2.0,
                "max_delay_ms": 5000
            }
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "flaky task with exponential backoff should eventually succeed"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "expected at least 3 attempts, got {}",
        task["attempts"]
    );
    // 100ms + 200ms = 300ms minimum backoff
    assert!(
        elapsed >= Duration::from_millis(250),
        "exponential backoff should take at least ~300ms, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_exponential_backoff_api_exhausted() {
    // Test exponential backoff with max_retries=2, always-fail provider.
    // delay_ms=100, backoff_multiplier=2.0 → waits 100ms + 200ms = 300ms total.
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "exponential exhaustion test",
            "retry": {
                "max_retries": 2,
                "delay_ms": 100,
                "backoff_multiplier": 2.0
            }
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "failed");
    assert!(
        elapsed >= Duration::from_millis(250),
        "exponential backoff exhaustion should take at least ~300ms, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_exponential_backoff_api_max_delay_caps() {
    // Test that max_delay_ms caps the exponential growth.
    // delay_ms=200, backoff_multiplier=10.0, max_delay_ms=300
    // attempt 1 fails → wait 200ms
    // attempt 2 fails → would be 2000ms but capped at 300ms → wait 300ms
    // Total ≥ 450ms (200 + 300 = 500, minus timing slack)
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "max delay cap test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 200,
                "backoff_multiplier": 10.0,
                "max_delay_ms": 300
            }
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    // 200ms + 300ms (capped) = 500ms minimum
    assert!(
        elapsed >= Duration::from_millis(450),
        "max_delay_ms should cap growth, expected ≥450ms, elapsed was {elapsed:?}"
    );
    // Without the cap it would be 200+2000=2200ms, so verify it was fast enough
    assert!(
        elapsed < Duration::from_millis(2000),
        "max_delay_ms cap should prevent 2s+ delays, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_exponential_backoff_api_flaky_task() {
    // Same as e2e_exponential_backoff_api_flaky_task but with SQLite queue backend.
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "sqlite exponential backoff test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 100,
                "backoff_multiplier": 2.0,
                "max_delay_ms": 5000
            }
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "SQLite: flaky task with exponential backoff should succeed"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "SQLite: expected at least 3 attempts, got {}",
        task["attempts"]
    );
    assert!(
        elapsed >= Duration::from_millis(250),
        "SQLite: exponential backoff should take at least ~300ms, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_backoff_multiplier_1_is_fixed() {
    // Verify that backoff_multiplier=1.0 behaves the same as fixed delay.
    // MockFlakyProvider fails first 2 calls, succeeds on 3rd.
    // With delay_ms=100 and backoff_multiplier=1.0 → fixed 100ms each retry.
    // Total backoff = 100ms + 100ms = 200ms
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "fixed delay via multiplier=1 test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 100,
                "backoff_multiplier": 1.0
            }
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    // Fixed 100ms × 2 retries = 200ms minimum
    assert!(
        elapsed >= Duration::from_millis(150),
        "fixed delay (multiplier=1) should take at least ~200ms, elapsed was {elapsed:?}"
    );
    // Should NOT grow beyond 200ms + overhead (not exponential)
    assert!(
        elapsed < Duration::from_millis(1000),
        "fixed delay should not grow exponentially, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

// ═════════════════════════════════════════════════════════════════════════════
// Health Check structure e2e tests
// ═════════════════════════════════════════════════════════════════════════════

/// Verify `/health` returns expected JSON structure with all documented fields.
#[tokio::test]
async fn e2e_health_response_has_documented_structure() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

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
    let client = reqwest::Client::new();

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
    let client = reqwest::Client::new();
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
    let client = reqwest::Client::new();

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

/// Verify that `delay_seconds` causes the task to be held in the queue.
/// A delay of 2 seconds should prevent immediate processing.
#[tokio::test]
async fn e2e_scheduled_send_delay_seconds_holds_task() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

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
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("scheduled"));

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
        "delayed task should have scheduled_at in response"
    );

    // Wait for the task to complete (should take ~2 seconds)
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    assert!(
        elapsed >= Duration::from_millis(1800),
        "delayed task should wait at least ~2s, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

/// Verify that `delay_seconds=0` is treated as immediate (no delay).
#[tokio::test]
async fn e2e_scheduled_send_delay_zero_is_immediate() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

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
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("enqueued"));

    let task_id = body["task_id"].as_str().unwrap().to_string();
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    // Should complete quickly (well under 2s)
    assert!(
        elapsed < Duration::from_secs(2),
        "delay_seconds=0 should not cause delay, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

/// Verify that `scheduled_at` with an RFC 3339 timestamp works.
#[tokio::test]
async fn e2e_scheduled_send_rfc3339_timestamp() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = reqwest::Client::new();

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
        "scheduled_at task should wait at least ~2s, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

/// Verify that providing both `delay_seconds` and `scheduled_at` returns 400.
#[tokio::test]
async fn e2e_scheduled_send_mutually_exclusive_error() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

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
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("mutually exclusive"));
}

/// Verify that an invalid `scheduled_at` format returns 400.
#[tokio::test]
async fn e2e_scheduled_send_invalid_timestamp_format() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

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
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("invalid scheduled_at"));
}

/// Verify that `task_info.scheduled_at` is absent for non-delayed tasks.
#[tokio::test]
async fn e2e_scheduled_send_no_scheduled_at_for_immediate() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

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
        task["scheduled_at"].is_null() || !task.as_object().unwrap().contains_key("scheduled_at"),
        "immediate task should not have scheduled_at"
    );
}

/// Verify OpenAPI schema includes delay_seconds and scheduled_at fields.
#[tokio::test]
async fn e2e_openapi_schema_has_scheduled_send_fields() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();

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
// Scheduled / delayed send e2e tests — SQLite backend
// ═════════════════════════════════════════════════════════════════════════════

/// SQLite: Verify that `delay_seconds` causes the task to be held in the queue.
#[tokio::test]
async fn e2e_sqlite_scheduled_send_delay_seconds_holds_task() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "sqlite delayed notification",
            "delay_seconds": 2
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("scheduled"));

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
        "delayed task should have scheduled_at in response (SQLite)"
    );

    // Wait for the task to complete (should take ~2 seconds)
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    assert!(
        elapsed >= Duration::from_millis(1800),
        "delayed task should wait at least ~2s (SQLite), but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

/// SQLite: Verify that `delay_seconds=0` is treated as immediate (no delay).
#[tokio::test]
async fn e2e_sqlite_scheduled_send_delay_zero_is_immediate() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "sqlite immediate notification",
            "delay_seconds": 0
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    // Should say "enqueued" not "scheduled"
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("enqueued"));

    let task_id = body["task_id"].as_str().unwrap().to_string();
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    // Should complete quickly (well under 2s)
    assert!(
        elapsed < Duration::from_secs(2),
        "delay_seconds=0 should not cause delay (SQLite), elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

/// SQLite: Verify that `scheduled_at` with an RFC 3339 timestamp works.
#[tokio::test]
async fn e2e_sqlite_scheduled_send_rfc3339_timestamp() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

    // Schedule 2 seconds from now
    let scheduled_time = std::time::SystemTime::now() + Duration::from_secs(2);
    let ts = humantime::format_rfc3339(scheduled_time).to_string();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "sqlite scheduled at timestamp",
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
        "scheduled_at task should wait at least ~2s (SQLite), but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

/// SQLite: Verify that providing both `delay_seconds` and `scheduled_at` returns 400.
#[tokio::test]
async fn e2e_sqlite_scheduled_send_mutually_exclusive_error() {
    let base = spawn_server_sqlite().await;
    let client = reqwest::Client::new();

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
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("mutually exclusive"));
}

/// SQLite: Verify that an invalid `scheduled_at` format returns 400.
#[tokio::test]
async fn e2e_sqlite_scheduled_send_invalid_timestamp_format() {
    let base = spawn_server_sqlite().await;
    let client = reqwest::Client::new();

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
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("invalid scheduled_at"));
}

/// SQLite: Verify that `task_info.scheduled_at` is absent for non-delayed tasks.
#[tokio::test]
async fn e2e_sqlite_scheduled_send_no_scheduled_at_for_immediate() {
    let base = spawn_server_sqlite().await;
    let client = reqwest::Client::new();

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
        task["scheduled_at"].is_null() || !task.as_object().unwrap().contains_key("scheduled_at"),
        "immediate task should not have scheduled_at (SQLite)"
    );
}

/// SQLite: Verify batch async with mixed delay_seconds per item.
#[tokio::test]
async fn e2e_sqlite_scheduled_send_batch_mixed_delays() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = reqwest::Client::new();

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
