mod common;

use std::time::Duration;

use common::{spawn_server_sqlite_file, spawn_server_sqlite_file_with_workers, test_client};
use noti_queue::QueueBackend;
use reqwest::StatusCode;
use serde_json::Value;

/// Helper: backdate all 'processing' tasks so they appear stale (>5 min old).
/// This simulates the passage of time that would occur in a real crash/restart scenario.
fn backdate_processing_tasks(db_path: &str) {
    let conn = rusqlite::Connection::open(db_path).expect("open db for backdating");
    let six_min_ago_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        - 6 * 60 * 1000;
    conn.execute(
        "UPDATE tasks SET updated_at = ?1 WHERE status = 'processing'",
        rusqlite::params![six_min_ago_ms],
    )
    .expect("backdate processing tasks");
}

// ───────────────────── Stale task recovery (SQLite file) ─────────────────────

/// Enqueue tasks, dequeue them (leaving them in "processing" state), drop the
/// queue (simulating a crash), then start a new server against the same DB file.
/// `with_queue_backend` should recover the stale tasks back to "queued".
#[tokio::test]
async fn e2e_stale_recovery_processing_tasks_become_queued() {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let db_path = tmp.path().to_str().unwrap().to_string();

    // Phase 1: open queue, enqueue 2 tasks, dequeue them (→ processing), then drop
    {
        let queue = noti_queue::SqliteQueue::open(&db_path).expect("open sqlite queue");
        let task_a = noti_queue::NotificationTask::new(
            "slack",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("stale-task-a").with_priority(noti_core::Priority::Normal),
        );
        let task_b = noti_queue::NotificationTask::new(
            "slack",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("stale-task-b").with_priority(noti_core::Priority::Normal),
        );

        queue.enqueue(task_a).await.unwrap();
        queue.enqueue(task_b).await.unwrap();

        // Dequeue both → status becomes "processing"
        queue.dequeue().await.unwrap().expect("dequeue a");
        queue.dequeue().await.unwrap().expect("dequeue b");

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.processing, 2, "both tasks should be processing");
        // Drop queue — simulates crash
    }

    // Backdate processing tasks so they appear stale (>5 min threshold)
    backdate_processing_tasks(&db_path);

    // Phase 2: start HTTP server against the same DB — triggers recover_stale_tasks()
    let base = spawn_server_sqlite_file(&db_path).await;
    let client = test_client();

    // List tasks — recovered tasks should be "queued"
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=queued"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let tasks = body.as_array().expect("tasks should be an array");
    assert_eq!(
        tasks.len(),
        2,
        "both stale processing tasks should be recovered as queued"
    );

    // Stats should show 2 queued, 0 processing
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let stats: Value = resp.json().await.unwrap();
    assert_eq!(stats["queued"].as_u64().unwrap(), 2);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);
}

/// After recovery, a worker should be able to process the recovered tasks.
#[tokio::test]
async fn e2e_stale_recovery_tasks_can_be_processed_by_workers() {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let db_path = tmp.path().to_str().unwrap().to_string();

    // Phase 1: enqueue a task via "mock-ok" provider, dequeue it (→ processing), drop
    {
        let queue = noti_queue::SqliteQueue::open(&db_path).expect("open sqlite queue");
        let task = noti_queue::NotificationTask::new(
            "mock-ok",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("recover-and-process")
                .with_priority(noti_core::Priority::Normal),
        );

        let task_id = queue.enqueue(task).await.unwrap();

        // Dequeue → processing
        let dequeued = queue.dequeue().await.unwrap().expect("dequeue task");
        assert_eq!(dequeued.id, task_id);
        // Drop — simulates crash with task stuck in processing
    }

    // Backdate processing tasks so they appear stale (>5 min threshold)
    backdate_processing_tasks(&db_path);

    // Phase 2: start server with workers — recovery + worker processing
    let (base, worker_handle) = spawn_server_sqlite_file_with_workers(&db_path).await;
    let client = test_client();

    // Give workers time to pick up and process the recovered task
    tokio::time::sleep(Duration::from_millis(500)).await;

    // List all tasks — the task should now be "completed" (processed by mock-ok)
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=completed"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let tasks = body.as_array().expect("tasks array");
    assert_eq!(
        tasks.len(),
        1,
        "recovered task should be completed by worker"
    );
    assert_eq!(tasks[0]["provider"], "mock-ok");

    worker_handle.shutdown_and_join().await;
}

/// When there are no stale tasks, recovery is a no-op and the server starts normally.
#[tokio::test]
async fn e2e_stale_recovery_no_stale_tasks_is_noop() {
    let tmp = tempfile::NamedTempFile::new().expect("create temp file");
    let db_path = tmp.path().to_str().unwrap().to_string();

    // Phase 1: enqueue a task but do NOT dequeue it (stays queued, not processing)
    {
        let queue = noti_queue::SqliteQueue::open(&db_path).expect("open sqlite queue");
        let task = noti_queue::NotificationTask::new(
            "slack",
            noti_core::ProviderConfig::new(),
            noti_core::Message::text("not-stale").with_priority(noti_core::Priority::Normal),
        );
        queue.enqueue(task).await.unwrap();
    }

    // Phase 2: start server — no stale recovery needed
    let base = spawn_server_sqlite_file(&db_path).await;
    let client = test_client();

    // Task should still be queued (not touched by recovery)
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=queued"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    let tasks = body.as_array().expect("tasks array");
    assert_eq!(tasks.len(), 1);

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert_eq!(stats["queued"].as_u64().unwrap(), 1);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);
}
