mod common;

use common::{
    spawn_server, spawn_server_sqlite_with_workers, spawn_server_with_workers, test_client,
    wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Queue purge dedicated tests ─────────────────────

/// Purging an empty queue returns 0 purged.
#[tokio::test]
async fn e2e_purge_empty_queue_returns_zero() {
    let base = spawn_server().await;
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

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
    let client = test_client();

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
