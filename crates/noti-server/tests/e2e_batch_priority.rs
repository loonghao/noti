mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    MockOkProvider, MockSlowProvider, spawn_callback_server, spawn_server_without_workers,
    test_client, wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Batch async with mixed priorities (e2e) ─────────────────────

dual_backend_test!(
    without_workers,
    e2e_batch_async_mixed_priorities_processed_in_order,
    e2e_sqlite_batch_async_mixed_priorities_processed_in_order,
    |spawn_without_workers, label| {
        // Batch-enqueue 4 tasks with different priorities via the async batch endpoint.
        // Use a single worker to ensure strict priority-ordered processing.
        // Verify via callback order that urgent is processed first, then high, normal, low.
        let (callback_base, payloads) = spawn_callback_server().await;

        let (base, state) = spawn_without_workers(vec![Arc::new(MockOkProvider)]).await;

        let client = test_client();
        let callback_url = format!("{callback_base}/callback");

        // Batch-enqueue: low, normal, high, urgent — all in one request
        let resp = client
            .post(format!("{base}/api/v1/send/async/batch"))
            .json(&json!({
                "items": [
                    {
                        "provider": "mock-ok",
                        "text": "batch-low",
                        "priority": "low",
                        "callback_url": &callback_url,
                        "metadata": {"order": "low"}
                    },
                    {
                        "provider": "mock-ok",
                        "text": "batch-normal",
                        "priority": "normal",
                        "callback_url": &callback_url,
                        "metadata": {"order": "normal"}
                    },
                    {
                        "provider": "mock-ok",
                        "text": "batch-high",
                        "priority": "high",
                        "callback_url": &callback_url,
                        "metadata": {"order": "high"}
                    },
                    {
                        "provider": "mock-ok",
                        "text": "batch-urgent",
                        "priority": "urgent",
                        "callback_url": &callback_url,
                        "metadata": {"order": "urgent"}
                    }
                ]
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["total"], 4);
        assert_eq!(body["enqueued"], 4);
        assert_eq!(body["failed"], 0);

        // Collect all task IDs
        let results = body["results"].as_array().unwrap();
        let task_ids: Vec<String> = results
            .iter()
            .map(|r| r["task_id"].as_str().unwrap().to_string())
            .collect();

        // NOW start a single worker so tasks are processed in strict priority order
        let worker_config = noti_queue::WorkerConfig::default()
            .with_concurrency(1)
            .with_poll_interval(Duration::from_millis(50));
        let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

        // Wait for all tasks to complete
        for task_id in &task_ids {
            wait_for_terminal_status(&client, &base, task_id).await;
        }

        // Give callbacks time to arrive
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Verify callback order: urgent → high → normal → low
        {
            let received = payloads.lock().unwrap();
            assert!(
                received.len() >= 4,
                "{label}expected at least 4 callbacks, got {}",
                received.len()
            );

            let expected_order = ["urgent", "high", "normal", "low"];
            for (i, expected) in expected_order.iter().enumerate() {
                assert_eq!(
                    received[i]["metadata"]["order"].as_str().unwrap(),
                    *expected,
                    "{label}callback #{i} should be '{expected}', got '{}'",
                    received[i]["metadata"]["order"]
                );
            }
        }

        worker_handle.shutdown_and_join().await;
    }
);

// ───────────────────── Graceful shutdown (e2e) ─────────────────────

// Verify that `shutdown_and_join()` waits for an in-flight slow task to complete
// before the worker pool exits, and the task reaches `completed` status.
dual_backend_test!(
    without_workers,
    e2e_graceful_shutdown_waits_for_inflight_task,
    e2e_sqlite_graceful_shutdown_waits_for_inflight_task,
    |spawn_without_workers, label| {
        let (callback_base, payloads) = spawn_callback_server().await;

        let slow: Arc<dyn noti_core::NotifyProvider> =
            Arc::new(MockSlowProvider::new(Duration::from_millis(500)));

        let (base, state) = spawn_without_workers(vec![slow]).await;

        // Start a single worker
        let worker_config = noti_queue::WorkerConfig::default()
            .with_concurrency(1)
            .with_poll_interval(Duration::from_millis(50));
        let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

        let client = test_client();
        let callback_url = format!("{callback_base}/callback");

        // Enqueue a task that takes 500ms to process
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-slow",
                "text": "slow-task",
                "callback_url": &callback_url,
                "metadata": {"test": "graceful-shutdown"}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        let task_id = body["task_id"].as_str().unwrap().to_string();

        // Wait a bit for the worker to pick up the task (but not finish it)
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Issue shutdown while the slow task is still in-flight
        worker_handle.shutdown_and_join().await;

        // After shutdown completes, the task should be completed (worker waited for it)
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let task: Value = resp.json().await.unwrap();
        assert_eq!(
            task["status"], "completed",
            "{label}in-flight task should complete before worker exits"
        );

        // Verify the callback was fired
        tokio::time::sleep(Duration::from_millis(200)).await;
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "{label}callback should have been fired for the completed slow task"
        );
        assert_eq!(received[0]["status"], "completed");
        assert_eq!(received[0]["metadata"]["test"], "graceful-shutdown");
    }
);

/// Verify that after shutdown, queued tasks that were not picked up remain in `pending` status.
/// Uses a slow provider so the single worker can only process one task before shutdown.
#[tokio::test]
async fn e2e_graceful_shutdown_stops_processing_new_tasks() {
    // Each task takes 200ms to complete
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(200)));

    let (base, state) = spawn_server_without_workers(vec![slow]).await;

    let client = test_client();

    // Enqueue 5 tasks BEFORE starting workers
    let mut task_ids = Vec::new();
    for i in 0..5 {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-slow",
                "text": format!("task-{i}"),
                "metadata": {"index": format!("{i}")}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        task_ids.push(body["task_id"].as_str().unwrap().to_string());
    }

    // Start a single worker
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    // Wait for the worker to pick up and start processing the first task (50ms poll + start)
    // but not long enough for 200ms send to finish
    tokio::time::sleep(Duration::from_millis(120)).await;

    // Issue shutdown — worker should finish the in-flight task but not start new ones
    worker_handle.shutdown_and_join().await;

    // Check task statuses
    let mut completed = 0;
    let mut pending = 0;
    for task_id in &task_ids {
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let body: Value = resp.json().await.unwrap();
        match body["status"].as_str().unwrap() {
            "completed" => completed += 1,
            "pending" | "queued" => pending += 1,
            _ => {}
        }
    }

    // The single worker with 200ms delay can process at most 1 task before shutdown
    // (picked up at ~50ms, finishes at ~250ms, shutdown at ~120ms waits for it).
    // Remaining tasks should still be pending.
    assert!(
        completed >= 1,
        "at least one task should have been completed (completed={completed})"
    );
    assert!(
        pending >= 1,
        "at least one task should remain pending after shutdown (pending={pending}, completed={completed})"
    );
    assert!(
        completed < 5,
        "not all tasks should be completed after immediate shutdown (completed={completed})"
    );
}

/// Verify that the HTTP server remains responsive during and after worker shutdown.
/// (Workers shutting down should not affect the server's ability to serve requests.)
#[tokio::test]
async fn e2e_http_server_responsive_during_worker_shutdown() {
    let slow: Arc<dyn noti_core::NotifyProvider> =
        Arc::new(MockSlowProvider::new(Duration::from_millis(300)));

    let (base, state) = spawn_server_without_workers(vec![slow, Arc::new(MockOkProvider)]).await;

    // Start worker
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    let client = test_client();
    let base_clone = base.clone();

    // Enqueue a slow task
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-slow",
            "text": "slow-during-shutdown"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);

    // Wait for worker to pick it up
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Spawn shutdown in the background (it will block until the slow task finishes)
    let shutdown_task = tokio::spawn(async move {
        worker_handle.shutdown_and_join().await;
    });

    // While shutdown is in progress, the HTTP server should still respond
    let resp = client
        .get(format!("{base_clone}/health"))
        .send()
        .await
        .expect("server should still respond during worker shutdown");
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client
        .get(format!("{base_clone}/api/v1/providers"))
        .send()
        .await
        .expect("provider listing should still work during worker shutdown");
    assert_eq!(resp.status(), StatusCode::OK);

    // Wait for shutdown to complete
    shutdown_task.await.unwrap();

    // Server should still respond after workers are fully stopped
    let resp = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("server should respond after worker shutdown");
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Verify that shutdown_and_join completes within a reasonable time
/// even when the queue is empty (no tasks to process).
#[tokio::test]
async fn e2e_graceful_shutdown_empty_queue_completes_quickly() {
    let (_base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    // Start workers with multiple concurrency
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(4)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    // Let workers run for a bit with empty queue
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shutdown should complete quickly (not hang waiting for tasks)
    let start = std::time::Instant::now();
    worker_handle.shutdown_and_join().await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(2),
        "shutdown of empty queue should complete quickly, took {:?}",
        elapsed
    );
}
