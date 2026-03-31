//! End-to-end tests that start a real TCP server and send HTTP requests via `reqwest`.
//!
//! Unlike the `server_test.rs` tests which use `axum_test::TestServer` (in-process),
//! these tests bind to a random port and exercise the full HTTP stack, including
//! TCP transport and header serialization.

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use noti_core::ProviderRegistry;
use noti_server::middleware::auth::{AuthConfig, AuthState, auth_middleware};
use noti_server::middleware::rate_limit::{
    RateLimitConfig, RateLimiterState, rate_limit_middleware,
};
use noti_server::middleware::request_id::request_id_middleware;
use reqwest::StatusCode;
use serde_json::{Value, json};

/// Start a real HTTP server on a random port and return the base URL.
async fn spawn_server() -> String {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

/// Start a server with auth middleware enabled.
/// Returns (base_url, valid_api_keys).
async fn spawn_server_with_auth(api_keys: Vec<String>) -> (String, Vec<String>) {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let auth_config = AuthConfig::new(api_keys.clone());
    let auth_state = AuthState::new(auth_config);

    let app = noti_server::routes::build_router(state).layer(axum::middleware::from_fn_with_state(
        auth_state,
        auth_middleware,
    ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), api_keys)
}

/// Start a server with rate limit middleware enabled (global mode).
/// Returns (base_url, max_requests).
async fn spawn_server_with_rate_limit(max_requests: u64, window_secs: u64) -> (String, u64) {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let rate_config =
        RateLimitConfig::new(max_requests, Duration::from_secs(window_secs)).with_per_ip(false);
    let rate_state = RateLimiterState::new(rate_config);

    let app = noti_server::routes::build_router(state).layer(axum::middleware::from_fn_with_state(
        rate_state,
        rate_limit_middleware,
    ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), max_requests)
}

/// Start a server with both auth and rate limit middleware (production-like stack).
/// Middleware order: Auth → Rate-limit → BodyLimit → Router
async fn spawn_server_with_full_middleware(
    api_keys: Vec<String>,
    max_requests: u64,
    window_secs: u64,
) -> (String, Vec<String>) {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let auth_config = AuthConfig::new(api_keys.clone());
    let auth_state = AuthState::new(auth_config);
    let rate_config =
        RateLimitConfig::new(max_requests, Duration::from_secs(window_secs)).with_per_ip(false);
    let rate_state = RateLimiterState::new(rate_config);

    let app = noti_server::routes::build_router(state)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            rate_state,
            rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth_middleware,
        ));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), api_keys)
}

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

/// Start a server with a custom body size limit.
/// Returns (base_url, max_bytes).
async fn spawn_server_with_body_limit(max_bytes: usize) -> (String, usize) {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state).layer(DefaultBodyLimit::max(max_bytes));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), max_bytes)
}

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

/// Start a server with request ID middleware enabled.
async fn spawn_server_with_request_id() -> String {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state)
        .layer(axum::middleware::from_fn(request_id_middleware));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

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
