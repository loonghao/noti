mod common;

use std::time::Duration;

use common::{
    spawn_callback_server, spawn_server_sqlite_with_workers, spawn_server_with_workers,
    test_client, wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Concurrent task processing tests ─────────────────────

/// Multiple tasks enqueued concurrently are all processed to completion by workers.
#[tokio::test]
async fn e2e_concurrent_tasks_all_processed() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let task_count = 10;
    let mut task_ids = Vec::new();

    // Enqueue many tasks concurrently
    let mut handles = Vec::new();
    for i in 0..task_count {
        let c = client.clone();
        let b = base.clone();
        handles.push(tokio::spawn(async move {
            let resp = c
                .post(format!("{b}/api/v1/send/async"))
                .json(&json!({
                    "provider": "mock-ok",
                    "text": format!("concurrent task {i}")
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::ACCEPTED);
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string()
        }));
    }

    for h in handles {
        task_ids.push(h.await.unwrap());
    }

    assert_eq!(task_ids.len(), task_count);

    // Wait for all to reach terminal state
    for id in &task_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(
            result["status"].as_str().unwrap(),
            "completed",
            "task {id} should be completed"
        );
    }

    // Stats should show all completed
    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= task_count as u64);
    assert_eq!(stats["queued"].as_u64().unwrap(), 0);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);

    worker_handle.shutdown_and_join().await;
}

/// Each task is processed exactly once — no duplicates.
#[tokio::test]
async fn e2e_concurrent_tasks_no_duplicate_processing() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let (cb_base, payloads) = spawn_callback_server().await;
    let client = test_client();

    let task_count = 8;
    let mut task_ids = Vec::new();

    // Enqueue tasks with callback so we can count invocations
    for i in 0..task_count {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("dedup task {i}"),
                "callback_url": format!("{cb_base}/callback")
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        task_ids.push(
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }

    // Wait for all to complete
    for id in &task_ids {
        wait_for_terminal_status(&client, &base, id).await;
    }

    // Give callbacks a moment to arrive
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Each task should have produced exactly one callback
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            task_count,
            "expected exactly {task_count} callbacks, got {}",
            received.len()
        );

        // Verify all task IDs appear exactly once
        let callback_task_ids: Vec<&str> = received
            .iter()
            .map(|p| p["task_id"].as_str().unwrap())
            .collect();
        for id in &task_ids {
            let count = callback_task_ids.iter().filter(|&&cid| cid == id).count();
            assert_eq!(count, 1, "task {id} should have exactly 1 callback");
        }
    } // MutexGuard dropped before await

    worker_handle.shutdown_and_join().await;
}

/// Concurrent processing on SQLite backend also works correctly.
#[tokio::test]
async fn e2e_sqlite_concurrent_tasks_all_processed() {
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = test_client();

    let task_count = 10;
    let mut task_ids = Vec::new();

    let mut handles = Vec::new();
    for i in 0..task_count {
        let c = client.clone();
        let b = base.clone();
        handles.push(tokio::spawn(async move {
            let resp = c
                .post(format!("{b}/api/v1/send/async"))
                .json(&json!({
                    "provider": "mock-ok",
                    "text": format!("sqlite concurrent {i}")
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::ACCEPTED);
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string()
        }));
    }

    for h in handles {
        task_ids.push(h.await.unwrap());
    }

    for id in &task_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(
            result["status"].as_str().unwrap(),
            "completed",
            "SQLite task {id} should be completed"
        );
    }

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= task_count as u64);
    assert_eq!(stats["queued"].as_u64().unwrap(), 0);
    assert_eq!(stats["processing"].as_u64().unwrap(), 0);

    worker_handle.shutdown_and_join().await;
}

/// Mixed success/failure tasks processed concurrently all reach correct terminal states.
#[tokio::test]
async fn e2e_concurrent_mixed_success_failure() {
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let mut ok_ids = Vec::new();
    let mut fail_ids = Vec::new();

    // Enqueue 5 success + 5 failure tasks interleaved
    for i in 0..10 {
        let provider = if i % 2 == 0 { "mock-ok" } else { "mock-fail" };
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": provider,
                "text": format!("mixed task {i}")
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let id = resp.json::<Value>().await.unwrap()["task_id"]
            .as_str()
            .unwrap()
            .to_string();
        if i % 2 == 0 {
            ok_ids.push(id);
        } else {
            fail_ids.push(id);
        }
    }

    // Wait for all tasks
    for id in &ok_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(result["status"].as_str().unwrap(), "completed");
    }
    for id in &fail_ids {
        let result = wait_for_terminal_status(&client, &base, id).await;
        assert_eq!(result["status"].as_str().unwrap(), "failed");
    }

    let resp = client
        .get(format!("{base}/api/v1/queue/stats"))
        .send()
        .await
        .unwrap();
    let stats: Value = resp.json().await.unwrap();
    assert!(stats["completed"].as_u64().unwrap() >= 5);
    // Failed tasks that exhausted retries are now in DLQ, not in main queue's failed counter
    assert_eq!(stats["failed"].as_u64().unwrap(), 0);

    worker_handle.shutdown_and_join().await;
}
