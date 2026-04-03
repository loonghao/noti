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
async fn test_api_versions() {
    let server = build_test_server();
    let response = server.get("/api/versions").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["latest"], "v1");
    let versions = body["versions"].as_array().unwrap();
    assert!(!versions.is_empty());
    let v1 = &versions[0];
    assert_eq!(v1["version"], "v1");
    assert_eq!(v1["status"], "stable");
    assert_eq!(v1["deprecated"], false);
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
    let response = server.get("/api/v1/status/nonexistent-id").await;

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
async fn test_purge_statuses_empty() {
    let server = build_test_server();
    let response = server.post("/api/v1/status/purge").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["purged"], 0);
}

#[tokio::test]
async fn test_purge_statuses_with_max_age() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/status/purge?max_age_secs=3600")
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["purged"], 0);
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

#[tokio::test]
async fn test_template_update() {
    let server = build_test_server();

    // Create a template first
    server
        .post("/api/v1/templates")
        .json(&json!({
            "name": "greeting",
            "body": "Hello, {{name}}!",
            "title": "Greeting",
            "defaults": {"name": "World"}
        }))
        .await;

    // Update the template body
    let response = server
        .put("/api/v1/templates/greeting")
        .json(&json!({
            "body": "Hi there, {{name}}! Welcome to {{place}}.",
            "defaults": {"place": "Earth"}
        }))
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "greeting");
    assert!(body["body"].as_str().unwrap().contains("Hi there"));
    // Original default "name" should be preserved, plus new "place"
    assert_eq!(body["defaults"]["name"], "World");
    assert_eq!(body["defaults"]["place"], "Earth");
}

#[tokio::test]
async fn test_template_update_not_found() {
    let server = build_test_server();

    let response = server
        .put("/api/v1/templates/nonexistent")
        .json(&json!({
            "body": "new body"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_template_delete() {
    let server = build_test_server();

    // Create a template
    server
        .post("/api/v1/templates")
        .json(&json!({
            "name": "temp",
            "body": "temporary {{msg}}"
        }))
        .await;

    // Delete it
    let response = server.delete("/api/v1/templates/temp").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert!(body["deleted"].as_bool().unwrap());

    // Verify it's gone
    let response = server.get("/api/v1/templates/temp").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_template_delete_not_found() {
    let server = build_test_server();

    let response = server.delete("/api/v1/templates/nonexistent").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_template_full_crud_lifecycle() {
    let server = build_test_server();

    // Create
    let response = server
        .post("/api/v1/templates")
        .json(&json!({
            "name": "deploy-alert",
            "body": "Deployed {{service}} v{{version}} to {{env}}",
            "title": "Deploy: {{service}}",
            "defaults": {"env": "staging"}
        }))
        .await;
    response.assert_status(StatusCode::CREATED);

    // Read
    let response = server.get("/api/v1/templates/deploy-alert").await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["defaults"]["env"], "staging");

    // Update
    let response = server
        .put("/api/v1/templates/deploy-alert")
        .json(&json!({
            "defaults": {"env": "production"}
        }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["defaults"]["env"], "production");

    // Render with updated defaults
    let response = server
        .post("/api/v1/templates/deploy-alert/render")
        .json(&json!({
            "variables": {
                "service": "noti",
                "version": "1.0.0"
            }
        }))
        .await;
    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["text"], "Deployed noti v1.0.0 to production");

    // Delete
    let response = server.delete("/api/v1/templates/deploy-alert").await;
    response.assert_status_ok();

    // Verify deletion
    let response = server.get("/api/v1/templates").await;
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 0);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let server = build_test_server();
    let response = server.get("/api/v1/metrics").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body["providers"]["total_registered"].as_u64().unwrap() > 100);
    assert!(body["queue"]["total"].as_u64().is_some());
    assert!(body["version"].is_string());
    assert!(body["uptime_seconds"].as_u64().is_some());
}

// ───────────────────── Async send integration tests ─────────────────────

#[tokio::test]
async fn test_send_async_provider_not_found() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/send/async")
        .json(&json!({
            "provider": "nonexistent",
            "text": "hello"
        }))
        .await;

    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_send_async_missing_config() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/send/async")
        .json(&json!({
            "provider": "slack",
            "text": "hello",
            "config": {}
        }))
        .await;

    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_send_async_valid_provider() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/send/async")
        .json(&json!({
            "provider": "slack",
            "text": "hello",
            "config": {
                "webhook_url": "https://hooks.slack.com/services/T00/B00/xxx"
            },
            "metadata": {"trace_id": "abc123"},
            "callback_url": "https://example.com/callback"
        }))
        .await;

    response.assert_status(StatusCode::ACCEPTED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "queued");
    assert!(body["task_id"].is_string());
}

#[tokio::test]
async fn test_send_async_batch_mixed_providers() {
    let server = build_test_server();
    let response = server
        .post("/api/v1/send/async/batch")
        .json(&json!({
            "items": [
                {
                    "provider": "slack",
                    "text": "hello",
                    "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/xxx"}
                },
                {
                    "provider": "nonexistent",
                    "text": "world"
                }
            ]
        }))
        .await;

    response.assert_status(StatusCode::ACCEPTED);
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 2);
    assert_eq!(body["enqueued"], 1);
    assert_eq!(body["failed"], 1);

    let results = body["results"].as_array().unwrap();
    // First item should succeed (valid provider + config)
    assert!(results[0]["success"].as_bool().unwrap());
    assert!(results[0]["task_id"].is_string());
    // Second item should fail (unknown provider)
    assert!(!results[1]["success"].as_bool().unwrap());
    assert!(results[1]["error"].is_string());
}

// ───────────────────── Queue management integration tests ─────────────────────

#[tokio::test]
async fn test_queue_stats_empty() {
    let server = build_test_server();
    let response = server.get("/api/v1/queue/stats").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 0);
    assert_eq!(body["queued"], 0);
    assert_eq!(body["processing"], 0);
}

#[tokio::test]
async fn test_queue_tasks_list_empty() {
    let server = build_test_server();
    let response = server.get("/api/v1/queue/tasks").await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_queue_tasks_invalid_status_filter() {
    let server = build_test_server();
    let response = server.get("/api/v1/queue/tasks?status=bogus").await;

    response.assert_status(StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "bad_request");
}

#[tokio::test]
async fn test_queue_tasks_valid_status_filter() {
    let server = build_test_server();
    let response = server
        .get("/api/v1/queue/tasks?status=queued&limit=5")
        .await;

    response.assert_status_ok();
    let body: serde_json::Value = response.json();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_queue_task_lifecycle() {
    let server = build_test_server();

    // Enqueue a task
    let response = server
        .post("/api/v1/send/async")
        .json(&json!({
            "provider": "slack",
            "text": "lifecycle test",
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/xxx"}
        }))
        .await;
    response.assert_status(StatusCode::ACCEPTED);
    let enqueue_body: serde_json::Value = response.json();
    let task_id = enqueue_body["task_id"].as_str().unwrap();

    // Get the task
    let response = server.get(&format!("/api/v1/queue/tasks/{task_id}")).await;
    response.assert_status_ok();
    let task_body: serde_json::Value = response.json();
    assert_eq!(task_body["id"], task_id);
    assert_eq!(task_body["provider"], "slack");
    assert_eq!(task_body["status"], "queued");

    // Cancel the task
    let response = server
        .post(&format!("/api/v1/queue/tasks/{task_id}/cancel"))
        .await;
    response.assert_status_ok();
    let cancel_body: serde_json::Value = response.json();
    assert!(cancel_body["cancelled"].as_bool().unwrap());

    // Verify stats reflect the cancelled task
    let response = server.get("/api/v1/queue/stats").await;
    response.assert_status_ok();
    let stats_body: serde_json::Value = response.json();
    assert_eq!(stats_body["cancelled"], 1);

    // Purge completed/cancelled tasks
    let response = server.post("/api/v1/queue/purge").await;
    response.assert_status_ok();
    let purge_body: serde_json::Value = response.json();
    assert_eq!(purge_body["purged"], 1);
}

#[tokio::test]
async fn test_queue_task_not_found() {
    let server = build_test_server();
    let response = server.get("/api/v1/queue/tasks/nonexistent-id").await;

    response.assert_status(StatusCode::NOT_FOUND);
}
