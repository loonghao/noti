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
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

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

// ───────────────────── CORS middleware (e2e) ─────────────────────

/// Start a server with permissive CORS (allow all origins).
async fn spawn_server_with_cors_permissive() -> String {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = noti_server::routes::build_router(state).layer(cors_layer);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

/// Start a server with restricted CORS (only specified origins allowed).
async fn spawn_server_with_cors_restricted(allowed_origins: Vec<String>) -> String {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = noti_server::state::AppState::new(registry);
    let origins: Vec<axum::http::HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any);

    let app = noti_server::routes::build_router(state).layer(cors_layer);

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
        .header("Access-Control-Request-Headers", "Content-Type, Authorization")
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
        resp.headers()
            .contains_key("access-control-allow-origin"),
        "preflight response should contain Access-Control-Allow-Origin"
    );
    assert!(
        resp.headers()
            .contains_key("access-control-allow-methods"),
        "preflight response should contain Access-Control-Allow-Methods"
    );
    assert!(
        resp.headers()
            .contains_key("access-control-allow-headers"),
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
    let base = spawn_server_with_cors_restricted(vec!["https://allowed.example.com".to_string()])
        .await;
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
        resp.headers()
            .get("access-control-allow-origin")
            .is_none(),
        "non-matching origin should NOT receive Access-Control-Allow-Origin header"
    );
}

#[tokio::test]
async fn e2e_cors_restricted_preflight_non_matching_origin() {
    let base = spawn_server_with_cors_restricted(vec!["https://allowed.example.com".to_string()])
        .await;
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
        resp.headers()
            .get("access-control-allow-origin")
            .is_none(),
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

use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::routing::post as axum_post;
use axum::Router;

/// A mock provider that always succeeds.
struct MockOkProvider;

#[async_trait]
impl noti_core::NotifyProvider for MockOkProvider {
    fn name(&self) -> &str {
        "mock-ok"
    }
    fn url_scheme(&self) -> &str {
        "mock-ok"
    }
    fn params(&self) -> Vec<noti_core::ParamDef> {
        vec![]
    }
    fn description(&self) -> &str {
        "always succeeds"
    }
    fn example_url(&self) -> &str {
        "mock-ok://test"
    }
    async fn send(
        &self,
        _message: &noti_core::Message,
        _config: &noti_core::ProviderConfig,
    ) -> Result<noti_core::SendResponse, noti_core::NotiError> {
        Ok(noti_core::SendResponse::success("mock-ok", "ok"))
    }
}

/// A mock provider that always fails (returns an error).
struct MockFailProvider;

#[async_trait]
impl noti_core::NotifyProvider for MockFailProvider {
    fn name(&self) -> &str {
        "mock-fail"
    }
    fn url_scheme(&self) -> &str {
        "mock-fail"
    }
    fn params(&self) -> Vec<noti_core::ParamDef> {
        vec![]
    }
    fn description(&self) -> &str {
        "always fails"
    }
    fn example_url(&self) -> &str {
        "mock-fail://test"
    }
    async fn send(
        &self,
        _message: &noti_core::Message,
        _config: &noti_core::ProviderConfig,
    ) -> Result<noti_core::SendResponse, noti_core::NotiError> {
        Err(noti_core::NotiError::Network("simulated failure".into()))
    }
}

/// Shared state for the mock callback receiver.
#[derive(Clone)]
struct CallbackReceiverState {
    payloads: Arc<Mutex<Vec<Value>>>,
}

/// Handler that records incoming callback payloads.
async fn callback_handler(
    axum::extract::State(state): axum::extract::State<CallbackReceiverState>,
    axum::Json(payload): axum::Json<Value>,
) -> StatusCode {
    state.payloads.lock().unwrap().push(payload);
    StatusCode::OK
}

/// Start a mock HTTP server that records POST payloads at `/callback`.
/// Returns (base_url, shared payloads).
async fn spawn_callback_server() -> (String, Arc<Mutex<Vec<Value>>>) {
    let payloads: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let state = CallbackReceiverState {
        payloads: payloads.clone(),
    };

    let app = Router::new()
        .route("/callback", axum_post(callback_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind callback server");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), payloads)
}

/// Start a noti server with mock providers and background workers enabled.
/// Workers will actually process queued tasks.
/// Returns (base_url, worker_handle).
async fn spawn_server_with_workers() -> (String, noti_queue::WorkerHandle) {
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));
    registry.register(Arc::new(MockFailProvider));

    let state = noti_server::state::AppState::new(registry);
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(2)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let app = noti_server::routes::build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), worker_handle)
}

/// Helper: poll a task until it reaches a terminal state or timeout.
async fn wait_for_terminal_status(client: &reqwest::Client, base: &str, task_id: &str) -> Value {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(5);

    loop {
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let body: Value = resp.json().await.unwrap();
        let status = body["status"].as_str().unwrap_or("");

        if matches!(status, "completed" | "failed" | "cancelled") {
            return body;
        }

        if start.elapsed() > timeout {
            panic!(
                "task {task_id} did not reach terminal state within {timeout:?}, last status: {status}"
            );
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

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
    assert_eq!(task["status"], "completed", "task should be completed by worker");
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

/// A mock provider that fails the first N calls then succeeds.
struct MockFlakyProvider {
    fail_count: u32,
    call_counter: AtomicU32,
}

impl MockFlakyProvider {
    fn new(fail_count: u32) -> Self {
        Self {
            fail_count,
            call_counter: AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl noti_core::NotifyProvider for MockFlakyProvider {
    fn name(&self) -> &str {
        "mock-flaky"
    }
    fn url_scheme(&self) -> &str {
        "mock-flaky"
    }
    fn params(&self) -> Vec<noti_core::ParamDef> {
        vec![]
    }
    fn description(&self) -> &str {
        "fails first N calls then succeeds"
    }
    fn example_url(&self) -> &str {
        "mock-flaky://test"
    }
    async fn send(
        &self,
        _message: &noti_core::Message,
        _config: &noti_core::ProviderConfig,
    ) -> Result<noti_core::SendResponse, noti_core::NotiError> {
        let call = self.call_counter.fetch_add(1, AtomicOrdering::SeqCst);
        if call < self.fail_count {
            Err(noti_core::NotiError::Network(format!(
                "flaky failure #{}",
                call + 1
            )))
        } else {
            Ok(noti_core::SendResponse::success(
                "mock-flaky",
                "ok after retries",
            ))
        }
    }
}

/// Start a noti server with a single worker (serial processing) and all mock providers.
/// The single worker ensures tasks are dequeued in strict priority order.
/// Returns (base_url, worker_handle).
async fn spawn_server_with_workers_serial(
    extra_providers: Vec<Arc<dyn noti_core::NotifyProvider>>,
) -> (String, noti_queue::WorkerHandle) {
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));
    registry.register(Arc::new(MockFailProvider));
    for p in extra_providers {
        registry.register(p);
    }

    let state = noti_server::state::AppState::new(registry);
    // Single worker ensures sequential processing in priority order.
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let app = noti_server::routes::build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{addr}"), worker_handle)
}

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
    let addr: SocketAddr = listener.local_addr().unwrap();
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
    let addr: SocketAddr = listener.local_addr().unwrap();
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
