//! End-to-end tests that start a real TCP server and send HTTP requests via `reqwest`.
//!
//! Unlike the `server_test.rs` tests which use `axum_test::TestServer` (in-process),
//! these tests bind to a random port and exercise the full HTTP stack, including
//! TCP transport and header serialization.

use std::net::SocketAddr;

use noti_core::ProviderRegistry;
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
