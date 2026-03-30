use axum::http::StatusCode;
use axum_test::TestServer;
use serde_json::json;

use noti_core::ProviderRegistry;

fn build_test_server() -> TestServer {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);
    let state = noti_server::state::AppState::new(registry);
    let app = noti_server::routes::build_router(state);
    TestServer::new(app)
}

#[tokio::test]
async fn test_health_check() {
    let server = build_test_server();
    let response = server.get("/health").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_list_providers() {
    let server = build_test_server();
    let response = server.get("/api/v1/providers").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["total"].as_u64().unwrap() > 100);
    assert!(body["providers"].is_array());
}

#[tokio::test]
async fn test_get_provider_detail() {
    let server = build_test_server();
    let response = server.get("/api/v1/providers/slack").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "slack");
    assert!(body["params"].is_array());
}

#[tokio::test]
async fn test_get_provider_not_found() {
    let server = build_test_server();
    let response = server.get("/api/v1/providers/nonexistent").await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_send_notification_provider_not_found() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/send")
        .json(&json!({
            "provider": "nonexistent",
            "text": "hello"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_send_notification_missing_config() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/send")
        .json(&json!({
            "provider": "slack",
            "text": "hello",
            "config": {}
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_template_lifecycle() {
    let server = build_test_server();

    // Create a template
    let response = server
        .post("/api/v1/templates")
        .json(&json!({
            "name": "alert",
            "body": "Alert: {{message}} on {{host}}",
            "title": "{{level}} Alert",
            "defaults": {
                "level": "INFO"
            }
        }))
        .await;

    response.assert_status(StatusCode::CREATED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "alert");
    assert!(body["variables"].as_array().unwrap().len() >= 2);

    // List templates
    let response = server.get("/api/v1/templates").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 1);

    // Get template
    let response = server.get("/api/v1/templates/alert").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "alert");

    // Render template
    let response = server
        .post("/api/v1/templates/alert/render")
        .json(&json!({
            "variables": {
                "message": "disk full",
                "host": "server-01"
            }
        }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["text"], "Alert: disk full on server-01");
    assert_eq!(body["title"], "INFO Alert");
}

#[tokio::test]
async fn test_render_template_not_found() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/templates/nonexistent/render")
        .json(&json!({
            "variables": {}
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_status_not_found() {
    let server = build_test_server();
    let response = server
        .get("/api/v1/status/nonexistent-id")
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_all_statuses_empty() {
    let server = build_test_server();
    let response = server.get("/api/v1/status").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn test_batch_send_provider_not_found() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/send/batch")
        .json(&json!({
            "targets": [
                {"provider": "nonexistent", "config": {}}
            ],
            "text": "hello"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}
