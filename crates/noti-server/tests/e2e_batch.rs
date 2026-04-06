mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    MockOkProvider, spawn_callback_server, spawn_server_sqlite_with_workers_serial,
    spawn_server_sqlite_without_workers, spawn_server_with_workers_serial,
    spawn_server_without_workers, test_client, wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Batch async: mixed valid/invalid providers + priorities ─────────────────────

/// Batch-enqueue items with a mix of valid and invalid providers at different priorities.
/// Verify that invalid providers are rejected per-item (not failing the whole batch),
/// valid items are enqueued and processed in priority order, and the response counts are correct.
#[tokio::test]
async fn e2e_batch_async_mixed_providers_and_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Mix of valid (mock-ok) and invalid (nonexistent) providers at various priorities.
    // Valid items: urgent, normal, low — invalid items: high, low
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "nonexistent-a",
                    "text": "invalid-high",
                    "priority": "high",
                    "metadata": {"order": "invalid-high"}
                },
                {
                    "provider": "mock-ok",
                    "text": "valid-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "valid-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-urgent"}
                },
                {
                    "provider": "nonexistent-b",
                    "text": "invalid-low",
                    "priority": "low",
                    "metadata": {"order": "invalid-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "valid-normal",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-normal"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 5);
    assert_eq!(body["enqueued"], 3);
    assert_eq!(body["failed"], 2);

    // Verify per-item results
    let results = body["results"].as_array().unwrap();
    // index 0: invalid
    assert!(!results[0]["success"].as_bool().unwrap());
    assert_eq!(results[0]["provider"], "nonexistent-a");
    assert!(results[0]["error"].as_str().unwrap().contains("not found"));
    // index 1: valid
    assert!(results[1]["success"].as_bool().unwrap());
    assert!(results[1]["task_id"].is_string());
    // index 2: valid
    assert!(results[2]["success"].as_bool().unwrap());
    // index 3: invalid
    assert!(!results[3]["success"].as_bool().unwrap());
    assert_eq!(results[3]["provider"], "nonexistent-b");
    // index 4: valid
    assert!(results[4]["success"].as_bool().unwrap());

    // Collect task IDs of successfully enqueued items
    let task_ids: Vec<String> = results
        .iter()
        .filter(|r| r["success"].as_bool().unwrap_or(false))
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(task_ids.len(), 3);

    // Start a single worker AFTER enqueue to ensure strict priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    // Wait for all valid tasks to complete
    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callback order: urgent → normal → low (only valid items)
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 3,
            "expected at least 3 callbacks, got {}",
            received.len()
        );

        let expected_order = ["valid-urgent", "valid-normal", "valid-low"];
        for (i, expected) in expected_order.iter().enumerate() {
            assert_eq!(
                received[i]["metadata"]["order"].as_str().unwrap(),
                *expected,
                "callback #{i} should be '{expected}', got '{}'",
                received[i]["metadata"]["order"]
            );
        }
    }

    worker_handle.shutdown_and_join().await;
}

/// Same as above but using SQLite queue backend.
#[tokio::test]
async fn e2e_sqlite_batch_async_mixed_providers_and_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, state) = spawn_server_sqlite_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Same mix: valid (mock-ok) and invalid (nonexistent) at various priorities
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite-valid-high",
                    "priority": "high",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-high"}
                },
                {
                    "provider": "nonexistent-x",
                    "text": "sqlite-invalid-urgent",
                    "priority": "urgent",
                    "metadata": {"order": "invalid-urgent"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-valid-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-valid-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "valid-urgent"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 4);
    assert_eq!(body["enqueued"], 3);
    assert_eq!(body["failed"], 1);

    // Verify the failed item
    let results = body["results"].as_array().unwrap();
    assert!(results[0]["success"].as_bool().unwrap()); // valid-high
    assert!(!results[1]["success"].as_bool().unwrap()); // invalid-urgent
    assert!(results[2]["success"].as_bool().unwrap()); // valid-low
    assert!(results[3]["success"].as_bool().unwrap()); // valid-urgent

    // Collect valid task IDs
    let task_ids: Vec<String> = results
        .iter()
        .filter(|r| r["success"].as_bool().unwrap_or(false))
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(task_ids.len(), 3);

    // Start single worker after enqueue for strict priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callback order: urgent → high → low (only valid items, priority-sorted)
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 3,
            "SQLite: expected at least 3 callbacks, got {}",
            received.len()
        );

        let expected_order = ["valid-urgent", "valid-high", "valid-low"];
        for (i, expected) in expected_order.iter().enumerate() {
            assert_eq!(
                received[i]["metadata"]["order"].as_str().unwrap(),
                *expected,
                "SQLite: callback #{i} should be '{expected}', got '{}'",
                received[i]["metadata"]["order"]
            );
        }
    }

    worker_handle.shutdown_and_join().await;
}

/// Batch async with ALL invalid providers — verify 202 response with all items failed.
#[tokio::test]
async fn e2e_batch_async_all_invalid_providers_returns_202() {
    let (base, _state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {"provider": "bad-1", "text": "a", "priority": "urgent"},
                {"provider": "bad-2", "text": "b", "priority": "high"},
                {"provider": "bad-3", "text": "c", "priority": "low"}
            ]
        }))
        .send()
        .await
        .unwrap();

    // Still 202 even though all items failed — partial success model
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["enqueued"], 0);
    assert_eq!(body["failed"], 3);

    let results = body["results"].as_array().unwrap();
    for (i, result) in results.iter().enumerate() {
        assert!(
            !result["success"].as_bool().unwrap(),
            "item {i} should fail"
        );
        assert!(
            result["error"].as_str().unwrap().contains("not found"),
            "item {i} error should mention 'not found'"
        );
        assert!(
            result["task_id"].is_null(),
            "item {i} should have no task_id"
        );
    }
}

// ───────────────────── Batch async with MockFailProvider (always-failing valid provider) ─────────────────────

/// Batch-enqueue items where some use `mock-fail` (a registered provider that always errors at send time).
/// Unlike invalid/nonexistent providers which fail at enqueue time, `mock-fail` items are successfully
/// enqueued but fail during worker processing. Verify that:
/// 1. All items with registered providers (mock-ok and mock-fail) are enqueued successfully.
/// 2. Items with nonexistent providers fail at enqueue time (same partial-success model).
/// 3. mock-fail tasks reach `failed` status after worker processing.
/// 4. mock-ok tasks reach `completed` status.
/// 5. Callback order respects priority (urgent before normal before low).
#[tokio::test]
async fn e2e_batch_async_mock_fail_provider_with_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    // Use the serial helper which already registers MockOkProvider + MockFailProvider
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Batch: mix of mock-ok (always succeeds) and mock-fail (always fails at send time)
    // mock-fail items are valid providers — they should be enqueued successfully but fail during processing.
    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-fail",
                    "text": "fail-urgent",
                    "priority": "urgent",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-urgent"}
                },
                {
                    "provider": "mock-ok",
                    "text": "ok-normal",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-normal"}
                },
                {
                    "provider": "mock-fail",
                    "text": "fail-low",
                    "priority": "low",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-low"}
                },
                {
                    "provider": "mock-ok",
                    "text": "ok-high",
                    "priority": "high",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-high"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    // All 4 items use registered providers — all should be enqueued
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 4);
    assert_eq!(body["enqueued"], 4, "all items use registered providers");
    assert_eq!(body["failed"], 0, "no enqueue-time failures");

    let results = body["results"].as_array().unwrap();
    let task_ids: Vec<String> = results
        .iter()
        .map(|r| {
            assert!(
                r["success"].as_bool().unwrap(),
                "all items should succeed at enqueue"
            );
            r["task_id"].as_str().unwrap().to_string()
        })
        .collect();
    assert_eq!(task_ids.len(), 4);

    // Wait for all tasks to reach terminal state
    let mut statuses = Vec::new();
    for task_id in &task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        statuses.push((
            task["metadata"]["order"].as_str().unwrap().to_string(),
            task["status"].as_str().unwrap().to_string(),
        ));
    }

    // Verify mock-fail tasks failed, mock-ok tasks completed
    for (order, status) in &statuses {
        if order.starts_with("fail-") {
            assert_eq!(status, "failed", "{order} should have failed status");
        } else {
            assert_eq!(status, "completed", "{order} should have completed status");
        }
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callbacks arrived in priority order: urgent → high → normal → low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 4,
            "expected at least 4 callbacks, got {}",
            received.len()
        );

        let expected_order = ["fail-urgent", "ok-high", "ok-normal", "fail-low"];
        for (i, expected) in expected_order.iter().enumerate() {
            assert_eq!(
                received[i]["metadata"]["order"].as_str().unwrap(),
                *expected,
                "callback #{i} should be '{expected}', got '{}'",
                received[i]["metadata"]["order"]
            );
        }
    }

    worker_handle.shutdown_and_join().await;
}

/// Same test as above but using SQLite queue backend.
#[tokio::test]
async fn e2e_sqlite_batch_async_mock_fail_provider_with_priorities() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-ok",
                    "text": "sqlite-ok-low",
                    "priority": "low",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-low"}
                },
                {
                    "provider": "mock-fail",
                    "text": "sqlite-fail-urgent",
                    "priority": "urgent",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-urgent"}
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite-ok-urgent",
                    "priority": "urgent",
                    "callback_url": &callback_url,
                    "metadata": {"order": "ok-urgent"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["enqueued"], 3);
    assert_eq!(body["failed"], 0);

    let results = body["results"].as_array().unwrap();
    let task_ids: Vec<String> = results
        .iter()
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();

    // Wait for terminal states
    let mut statuses = Vec::new();
    for task_id in &task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        statuses.push((
            task["metadata"]["order"].as_str().unwrap().to_string(),
            task["status"].as_str().unwrap().to_string(),
        ));
    }

    // mock-fail should fail, mock-ok should complete
    for (order, status) in &statuses {
        if order.starts_with("fail-") {
            assert_eq!(
                status, "failed",
                "SQLite: {order} should have failed status"
            );
        } else {
            assert_eq!(
                status, "completed",
                "SQLite: {order} should have completed status"
            );
        }
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Callback order: urgent tasks first (fail-urgent, ok-urgent in enqueue order), then low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 3,
            "SQLite: expected at least 3 callbacks, got {}",
            received.len()
        );

        // Both urgent tasks should come before the low task
        let last_order = received[2]["metadata"]["order"].as_str().unwrap();
        assert_eq!(
            last_order, "ok-low",
            "SQLite: last callback should be the low-priority task"
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// Batch with mixed mock-ok, mock-fail, and nonexistent providers — verify that
/// nonexistent providers fail at enqueue time while mock-fail fails at processing time.
#[tokio::test]
async fn e2e_batch_async_mock_fail_mixed_with_nonexistent() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-fail",
                    "text": "fail-at-send",
                    "priority": "urgent",
                    "retry": {"max_retries": 0},
                    "callback_url": &callback_url,
                    "metadata": {"order": "fail-at-send"}
                },
                {
                    "provider": "nonexistent",
                    "text": "fail-at-enqueue",
                    "priority": "urgent",
                    "metadata": {"order": "fail-at-enqueue"}
                },
                {
                    "provider": "mock-ok",
                    "text": "succeeds",
                    "priority": "normal",
                    "callback_url": &callback_url,
                    "metadata": {"order": "succeeds"}
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 3);
    assert_eq!(body["enqueued"], 2, "mock-fail + mock-ok enqueued");
    assert_eq!(body["failed"], 1, "nonexistent fails at enqueue");

    let results = body["results"].as_array().unwrap();
    // index 0: mock-fail — enqueued
    assert!(results[0]["success"].as_bool().unwrap());
    assert!(results[0]["task_id"].is_string());
    // index 1: nonexistent — failed at enqueue
    assert!(!results[1]["success"].as_bool().unwrap());
    assert!(results[1]["error"].as_str().unwrap().contains("not found"));
    assert!(results[1]["task_id"].is_null());
    // index 2: mock-ok — enqueued
    assert!(results[2]["success"].as_bool().unwrap());
    assert!(results[2]["task_id"].is_string());

    // Collect enqueued task IDs
    let enqueued_ids: Vec<String> = results
        .iter()
        .filter(|r| r["success"].as_bool().unwrap_or(false))
        .map(|r| r["task_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(enqueued_ids.len(), 2);

    // Wait for terminal states
    let fail_task = wait_for_terminal_status(&client, &base, &enqueued_ids[0]).await;
    let ok_task = wait_for_terminal_status(&client, &base, &enqueued_ids[1]).await;

    assert_eq!(fail_task["status"].as_str().unwrap(), "failed");
    assert_eq!(ok_task["status"].as_str().unwrap(), "completed");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify callbacks: urgent (mock-fail) first, then normal (mock-ok)
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 2,
            "expected at least 2 callbacks, got {}",
            received.len()
        );
        assert_eq!(
            received[0]["metadata"]["order"].as_str().unwrap(),
            "fail-at-send"
        );
        assert_eq!(
            received[1]["metadata"]["order"].as_str().unwrap(),
            "succeeds"
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Concurrent batch async requests ─────────────────────

/// Fire multiple batch async requests concurrently and verify all are accepted
/// and all tasks eventually reach terminal states.
#[tokio::test]
async fn e2e_concurrent_batch_async_requests_all_accepted() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Fire 5 concurrent batch requests, each with 2 items
    let mut handles = Vec::new();
    for batch_idx in 0..5u32 {
        let client = client.clone();
        let base = base.clone();
        let callback_url = callback_url.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("batch-{batch_idx}-item-0"),
                            "priority": "normal",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "0"}
                        },
                        {
                            "provider": "mock-ok",
                            "text": format!("batch-{batch_idx}-item-1"),
                            "priority": "normal",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "1"}
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            let status = resp.status();
            let body: Value = resp.json().await.unwrap();
            (status, body)
        }));
    }

    // Collect all responses
    let mut all_task_ids = Vec::new();
    for handle in handles {
        let (status, body) = handle.await.unwrap();
        assert_eq!(
            status,
            StatusCode::ACCEPTED,
            "all batch requests should return 202"
        );
        assert_eq!(body["total"], 2);
        assert_eq!(body["enqueued"], 2);
        assert_eq!(body["failed"], 0);

        let results = body["results"].as_array().unwrap();
        for r in results {
            all_task_ids.push(r["task_id"].as_str().unwrap().to_string());
        }
    }

    assert_eq!(all_task_ids.len(), 10, "5 batches × 2 items = 10 tasks");

    // Wait for all tasks to reach terminal state
    for task_id in &all_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(
            task["status"].as_str().unwrap(),
            "completed",
            "task {task_id} should complete"
        );
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify all 10 callbacks arrived
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            10,
            "expected 10 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// Concurrent batch requests with a mix of mock-ok and mock-fail providers.
/// Verify all batches are accepted and tasks end in correct terminal states.
#[tokio::test]
async fn e2e_concurrent_batch_async_with_mixed_providers() {
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![]).await;

    let client = test_client();

    // Fire 3 concurrent batches: each has 1 mock-ok and 1 mock-fail item
    let mut handles = Vec::new();
    for batch_idx in 0..3u32 {
        let client = client.clone();
        let base = base.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("batch-{batch_idx}-ok"),
                            "metadata": {"batch": format!("{batch_idx}"), "provider": "ok"}
                        },
                        {
                            "provider": "mock-fail",
                            "text": format!("batch-{batch_idx}-fail"),
                            "metadata": {"batch": format!("{batch_idx}"), "provider": "fail"}
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            let status = resp.status();
            let body: Value = resp.json().await.unwrap();
            (status, body)
        }));
    }

    let mut ok_task_ids = Vec::new();
    let mut fail_task_ids = Vec::new();

    for handle in handles {
        let (status, body) = handle.await.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["enqueued"], 2, "both providers are registered");

        let results = body["results"].as_array().unwrap();
        // First item is mock-ok, second is mock-fail
        ok_task_ids.push(results[0]["task_id"].as_str().unwrap().to_string());
        fail_task_ids.push(results[1]["task_id"].as_str().unwrap().to_string());
    }

    // Wait for all tasks
    for task_id in &ok_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(task["status"].as_str().unwrap(), "completed");
    }
    for task_id in &fail_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        assert_eq!(task["status"].as_str().unwrap(), "failed");
    }

    worker_handle.shutdown_and_join().await;
}

/// Concurrent batch requests with SQLite backend — ensure no database contention issues.
#[tokio::test]
async fn e2e_sqlite_concurrent_batch_async_requests() {
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Fire 4 concurrent batch requests, each with 3 items
    let mut handles = Vec::new();
    for batch_idx in 0..4u32 {
        let client = client.clone();
        let base = base.clone();
        let callback_url = callback_url.clone();
        handles.push(tokio::spawn(async move {
            let resp = client
                .post(format!("{base}/api/v1/send/async/batch"))
                .json(&json!({
                    "items": [
                        {
                            "provider": "mock-ok",
                            "text": format!("sqlite-b{batch_idx}-0"),
                            "priority": "high",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "0"}
                        },
                        {
                            "provider": "mock-ok",
                            "text": format!("sqlite-b{batch_idx}-1"),
                            "priority": "normal",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "1"}
                        },
                        {
                            "provider": "mock-fail",
                            "text": format!("sqlite-b{batch_idx}-fail"),
                            "priority": "low",
                            "callback_url": &callback_url,
                            "metadata": {"batch": format!("{batch_idx}"), "item": "fail"}
                        }
                    ]
                }))
                .send()
                .await
                .unwrap();
            let status = resp.status();
            let body: Value = resp.json().await.unwrap();
            (status, body)
        }));
    }

    let mut all_task_ids = Vec::new();
    for handle in handles {
        let (status, body) = handle.await.unwrap();
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["total"], 3);
        assert_eq!(body["enqueued"], 3, "all providers are registered");
        assert_eq!(body["failed"], 0);

        let results = body["results"].as_array().unwrap();
        for r in results {
            all_task_ids.push(r["task_id"].as_str().unwrap().to_string());
        }
    }

    assert_eq!(all_task_ids.len(), 12, "4 batches × 3 items = 12 tasks");

    // Wait for all tasks to reach terminal state
    let mut completed = 0;
    let mut failed = 0;
    for task_id in &all_task_ids {
        let task = wait_for_terminal_status(&client, &base, task_id).await;
        match task["status"].as_str().unwrap() {
            "completed" => completed += 1,
            "failed" => failed += 1,
            other => panic!("unexpected status: {other}"),
        }
    }

    // 4 batches × 2 mock-ok = 8 completed, 4 batches × 1 mock-fail = 4 failed
    assert_eq!(completed, 8, "expected 8 completed tasks");
    assert_eq!(failed, 4, "expected 4 failed tasks");

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    // All 12 tasks should have generated callbacks
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            12,
            "SQLite: expected 12 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}
