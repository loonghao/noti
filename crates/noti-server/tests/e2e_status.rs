mod common;

use common::{spawn_server, test_client};
use reqwest::StatusCode;
use serde_json::Value;

// ───────────────────── Status endpoints ─────────────────────

#[tokio::test]
async fn e2e_status_not_found() {
    let base = spawn_server().await;
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/nonexistent-id"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
