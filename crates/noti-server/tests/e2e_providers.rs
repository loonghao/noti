mod common;

use common::{spawn_server, test_client};
use reqwest::StatusCode;
use serde_json::Value;

// ───────────────────── Providers ─────────────────────

#[tokio::test]
async fn e2e_list_providers() {
    let base = spawn_server().await;
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
