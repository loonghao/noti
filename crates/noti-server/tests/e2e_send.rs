mod common;

use common::{spawn_server, test_client};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Send (synchronous) ─────────────────────

#[tokio::test]
async fn e2e_send_missing_provider() {
    let base = spawn_server().await;
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

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
