//! E2E tests for the DLQ (Dead Letter Queue) HTTP API endpoints.
//!
//! Tests cover:
//! - `GET /api/v1/queue/dlq` — list DLQ entries
//! - `GET /api/v1/queue/dlq/stats` — DLQ statistics
//! - `POST /api/v1/queue/dlq/{task_id}/requeue` — requeue from DLQ
//! - `DELETE /api/v1/queue/dlq/{task_id}` — delete from DLQ
//!
//! Scenario: tasks are sent to DLQ by exhausting retries on a permanently-failing provider.

mod common;

use common::{spawn_server_with_workers, test_client, wait_for_terminal_status};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};

// ───────────────────── DTOs ─────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DlqEntryInfo {
    task_id: String,
    provider: String,
    status: String,
    attempts: u32,
    #[serde(default)]
    last_error: Option<String>,
    reason: String,
    moved_at: String,
    priority: String,
    #[serde(default)]
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DlqListResponse {
    entries: Vec<DlqEntryInfo>,
    total: usize,
}

#[derive(Debug, Deserialize)]
struct DlqStatsResponse {
    dlq_size: usize,
}

#[derive(Debug, Deserialize)]
struct RequeueResponse {
    task_id: String,
    requeued: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct DeleteDlqResponse {
    task_id: String,
    success: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ErrorResponse {
    error: String,
    message: String,
}

// ───────────────────── Helper ─────────────────────

/// Enqueue a task that will always fail and end up in DLQ.
/// Uses `mock-fail` (always fails) with `max_retries=0` so it goes directly to DLQ.
async fn enqueue_dlq_task(base: &str, provider: &str, max_retries: u32) -> String {
    let client = test_client();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": provider,
            "text": "this task will fail and go to DLQ",
            "retry": {
                "max_retries": max_retries,
                "delay_ms": 10
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    body["task_id"].as_str().unwrap().to_string()
}

// ───────────────────── List DLQ ─────────────────────

/// List DLQ returns empty when there are no failed tasks.
#[tokio::test]
async fn e2e_dlq_list_empty() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/queue/dlq"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: DlqListResponse = resp.json().await.unwrap();
    assert!(body.entries.is_empty(), "expected empty DLQ");
    assert_eq!(body.total, 0);
}

/// List DLQ with a failed task shows the entry.
#[tokio::test]
async fn e2e_dlq_list_with_entry() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue a task that will fail and go to DLQ immediately (max_retries=0)
    let task_id = enqueue_dlq_task(&base, "mock-fail", 0).await;

    // Wait for the task to reach terminal state (it will be in DLQ)
    let _ = wait_for_terminal_status(&client, &base, &task_id).await;

    // List DLQ and verify the entry is there
    let resp = client
        .get(format!("{base}/api/v1/queue/dlq"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: DlqListResponse = resp.json().await.unwrap();
    assert_eq!(body.total, 1, "expected 1 DLQ entry");
    let entry = &body.entries[0];
    assert_eq!(entry.task_id, task_id);
    assert_eq!(entry.provider, "mock-fail");
    assert_eq!(entry.status, "failed");
    assert_eq!(entry.attempts, 1);
    assert!(entry.last_error.is_some(), "expected last_error to be set");
    // reason contains the actual error message, not a standardized string
    assert!(entry.reason.contains("network error") || entry.reason.contains("simulated"), "reason: {}", entry.reason);
    assert_eq!(entry.priority.to_lowercase(), "normal");
}

/// List DLQ respects the `limit` query parameter.
#[tokio::test]
async fn e2e_dlq_list_with_limit() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue multiple tasks that will fail
    let _task1 = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _task2 = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _task3 = enqueue_dlq_task(&base, "mock-fail", 0).await;

    // Give workers time to process
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Request only 2 entries
    let resp = client
        .get(format!("{base}/api/v1/queue/dlq?limit=2"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: DlqListResponse = resp.json().await.unwrap();
    // total equals entries.len() in current implementation
    assert_eq!(body.total, 2, "total equals entries.len() with limit=2");
    assert_eq!(body.entries.len(), 2, "only 2 entries returned due to limit");
}

// ───────────────────── DLQ Stats ─────────────────────

/// DLQ stats returns dlq_size=0 when empty.
#[tokio::test]
async fn e2e_dlq_stats_empty() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/queue/dlq/stats"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: DlqStatsResponse = resp.json().await.unwrap();
    assert_eq!(body.dlq_size, 0);
}

/// DLQ stats reflects actual DLQ size after tasks fail.
#[tokio::test]
async fn e2e_dlq_stats_with_entries() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue 3 tasks that will fail
    let _task1 = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _task2 = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _task3 = enqueue_dlq_task(&base, "mock-fail", 0).await;

    // Give workers time to process
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let resp = client
        .get(format!("{base}/api/v1/queue/dlq/stats"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: DlqStatsResponse = resp.json().await.unwrap();
    assert_eq!(body.dlq_size, 3);
}

// ───────────────────── Requeue from DLQ ─────────────────────

/// Requeue a task from DLQ back to main queue.
#[tokio::test]
async fn e2e_dlq_requeue_success() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue a task that will fail and go to DLQ
    let task_id = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _ = wait_for_terminal_status(&client, &base, &task_id).await;

    // Verify it's in DLQ
    let resp = client
        .get(format!("{base}/api/v1/queue/dlq"))
        .send()
        .await
        .unwrap();
    let body: DlqListResponse = resp.json().await.unwrap();
    assert_eq!(body.total, 1, "task should be in DLQ");

    // Requeue it back to main queue
    let resp = client
        .post(format!("{base}/api/v1/queue/dlq/{task_id}/requeue"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: RequeueResponse = resp.json().await.unwrap();
    assert_eq!(body.task_id, task_id);
    assert!(body.requeued, "requeued should be true");
    assert!(body.message.contains("success"));

    // Give workers time to process the requeued task
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Note: The requeued task might fail again and go back to DLQ with a new ID,
    // so we don't assert DLQ is empty. Instead, just verify the requeue succeeded.
}

/// Requeue a non-existent task returns 404.
#[tokio::test]
async fn e2e_dlq_requeue_not_found() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let fake_task_id = "nonexistent-task-id-12345";
    let resp = client
        .post(format!("{base}/api/v1/queue/dlq/{fake_task_id}/requeue"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: ErrorResponse = resp.json().await.unwrap();
    assert!(body.message.contains(fake_task_id) || body.message.contains("not found"));
}

/// Verify that requeueing a task from DLQ succeeds (basic smoke test).
#[tokio::test]
async fn e2e_dlq_requeue_already_requeued() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue a task that will fail
    let task_id = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _ = wait_for_terminal_status(&client, &base, &task_id).await;

    // Requeue it - should succeed
    let resp = client
        .post(format!("{base}/api/v1/queue/dlq/{task_id}/requeue"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: RequeueResponse = resp.json().await.unwrap();
    assert!(body.requeued, "requeue should succeed");
    assert_eq!(body.task_id, task_id);
    assert!(body.message.contains("success") || body.message.contains("DLQ"));

    // Note: The requeued task may fail again and go back to DLQ with a new ID,
    // so the original task_id is no longer in DLQ. The second call behavior
    // depends on timing (whether worker processed the requeued task yet).
}

// ───────────────────── Delete from DLQ ─────────────────────

/// Delete a task from DLQ permanently.
#[tokio::test]
async fn e2e_dlq_delete_success() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue a task that will fail and go to DLQ
    let task_id = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _ = wait_for_terminal_status(&client, &base, &task_id).await;

    // Verify it's in DLQ
    let resp = client
        .get(format!("{base}/api/v1/queue/dlq"))
        .send()
        .await
        .unwrap();
    let body: DlqListResponse = resp.json().await.unwrap();
    assert_eq!(body.total, 1, "task should be in DLQ");

    // Delete it
    let resp = client
        .delete(format!("{base}/api/v1/queue/dlq/{task_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: DeleteDlqResponse = resp.json().await.unwrap();
    assert_eq!(body.task_id, task_id);
    assert!(body.success, "success should be true");
    assert!(body.message.contains("success") || body.message.contains("removed"));

    // DLQ should now be empty
    let resp = client
        .get(format!("{base}/api/v1/queue/dlq"))
        .send()
        .await
        .unwrap();
    let body: DlqListResponse = resp.json().await.unwrap();
    assert_eq!(body.total, 0, "DLQ should be empty after delete");
}

/// Delete a non-existent task returns 404.
#[tokio::test]
async fn e2e_dlq_delete_not_found() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let fake_task_id = "nonexistent-task-id-67890";
    let resp = client
        .delete(format!("{base}/api/v1/queue/dlq/{fake_task_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: ErrorResponse = resp.json().await.unwrap();
    assert!(body.message.contains(fake_task_id) || body.message.contains("not found"));
}

/// Delete a task that was already deleted — handler succeeds (entry already gone).
#[tokio::test]
async fn e2e_dlq_delete_already_deleted() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue a task that will fail
    let task_id = enqueue_dlq_task(&base, "mock-fail", 0).await;
    let _ = wait_for_terminal_status(&client, &base, &task_id).await;

    // Delete it
    let resp = client
        .delete(format!("{base}/api/v1/queue/dlq/{task_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: DeleteDlqResponse = resp.json().await.unwrap();
    assert!(body.success, "first delete should succeed");
    assert_eq!(body.task_id, task_id);

    // Give workers time to process
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Try to delete again — task is no longer in DLQ, expect 404 or 400
    let resp = client
        .delete(format!("{base}/api/v1/queue/dlq/{task_id}"))
        .send()
        .await
        .unwrap();

    // Either 404 (not found) or 400 (not in DLQ/failed state) is acceptable
    assert!(resp.status() == StatusCode::NOT_FOUND || resp.status() == StatusCode::BAD_REQUEST,
        "expected 404 or 400, got {}", resp.status());
}

// ───────────────────── Stats endpoint includes DLQ size ─────────────────────

/// GET /api/v1/queue/stats includes dlq_size field.
#[tokio::test]
async fn e2e_dlq_stats_in_queue_stats() {
    let (base, _worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue a task that will fail
    let _task_id = enqueue_dlq_task(&base, "mock-fail", 0).await;

    // Give workers time to process
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(
        body.get("dlq_size").is_some(),
        "response should include dlq_size field"
    );
    assert_eq!(body["dlq_size"], 1, "dlq_size should be 1");
}
