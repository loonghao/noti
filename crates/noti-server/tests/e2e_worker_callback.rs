mod common;

use std::time::Duration;

use common::{
    spawn_callback_server, spawn_server, spawn_server_with_workers, test_client,
    wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Worker processing & Webhook callback (e2e) ─────────────────────

dual_backend_test!(
    with_workers,
    e2e_worker_processes_task_to_completion,
    e2e_sqlite_worker_processes_task_to_completion,
    |spawn_with_workers, label| {
        let (base, worker_handle) = spawn_with_workers().await;
        let client = test_client();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": "worker e2e test"
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        let task_id = body["task_id"].as_str().unwrap().to_string();

        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        assert_eq!(
            task["status"], "completed",
            "{label}task should be completed by worker"
        );
        assert_eq!(task["provider"], "mock-ok");

        let resp = client
            .get(format!("{base}/api/v1/queue/stats"))
            .send()
            .await
            .unwrap();
        let stats: Value = resp.json().await.unwrap();
        assert!(stats["completed"].as_u64().unwrap() >= 1);

        worker_handle.shutdown_and_join().await;
    }
);

dual_backend_test!(
    with_workers,
    e2e_worker_handles_failed_task,
    e2e_sqlite_worker_handles_failed_task,
    |spawn_with_workers, label| {
        let (base, worker_handle) = spawn_with_workers().await;
        let client = test_client();

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-fail",
                "text": "worker failure test",
                "retry": {"max_retries": 0, "delay_ms": 10}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        let task_id = body["task_id"].as_str().unwrap().to_string();

        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        assert_eq!(
            task["status"], "failed",
            "{label}task should be failed by worker"
        );
        assert!(
            task["last_error"].is_string(),
            "{label}failed task should have an error message"
        );
        assert_eq!(
            task["attempts"].as_u64().unwrap(),
            1,
            "{label}with max_retries=0, should have exactly 1 attempt"
        );

        let resp = client
            .get(format!("{base}/api/v1/queue/stats"))
            .send()
            .await
            .unwrap();
        let stats: Value = resp.json().await.unwrap();
        assert!(stats["failed"].as_u64().unwrap() >= 1);

        worker_handle.shutdown_and_join().await;
    }
);

dual_backend_test!(
    with_workers,
    e2e_webhook_callback_on_success,
    e2e_sqlite_webhook_callback_on_success,
    |spawn_with_workers, label| {
        let (callback_base, payloads) = spawn_callback_server().await;
        let (base, worker_handle) = spawn_with_workers().await;
        let client = test_client();

        let callback_url = format!("{callback_base}/callback");

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": "callback success test",
                "callback_url": callback_url,
                "metadata": {"trace_id": "e2e-callback-ok"}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        let task_id = body["task_id"].as_str().unwrap().to_string();

        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        assert_eq!(task["status"], "completed");

        tokio::time::sleep(Duration::from_millis(200)).await;

        {
            let received = payloads.lock().unwrap();
            assert!(
                !received.is_empty(),
                "{label}callback server should have received at least one payload"
            );

            let cb = &received[0];
            assert_eq!(cb["task_id"], task_id);
            assert_eq!(cb["provider"], "mock-ok");
            assert_eq!(cb["status"], "completed");
            assert!(cb["attempts"].as_u64().unwrap() >= 1);
            assert_eq!(cb["metadata"]["trace_id"], "e2e-callback-ok");
        }

        worker_handle.shutdown_and_join().await;
    }
);

#[tokio::test]
async fn e2e_webhook_callback_on_failure() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let callback_url = format!("{callback_base}/callback");

    // Enqueue a task with callback_url that will fail
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "callback failure test",
            "callback_url": callback_url,
            "retry": {"max_retries": 0, "delay_ms": 10}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for task to fail
    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(task["status"], "failed");

    // Give callback a moment to fire
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify callback was received with failure info
    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback server should receive payload on failure"
        );

        let cb = &received[0];
        assert_eq!(cb["task_id"], task_id);
        assert_eq!(cb["provider"], "mock-fail");
        assert_eq!(cb["status"], "failed");
        assert!(
            cb["last_error"].is_string(),
            "failed callback should include last_error"
        );
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_no_callback_when_url_not_set() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    // Enqueue a task WITHOUT callback_url
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "no callback test"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Wait for task to complete
    let _task = wait_for_terminal_status(&client, &base, &task_id).await;

    // Give extra time
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Callback server should NOT have received anything
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.is_empty(),
            "callback server should NOT receive payload when no callback_url is set, got {} payloads: {callback_base}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

dual_backend_test!(
    with_workers,
    e2e_worker_multiple_tasks_processed,
    e2e_sqlite_multiple_tasks_processed,
    |spawn_fn, label| {
        let (base, worker_handle) = spawn_fn().await;
        let client = test_client();

        let mut task_ids = Vec::new();

        // Enqueue 5 tasks
        for i in 0..5 {
            let resp = client
                .post(format!("{base}/api/v1/send/async"))
                .json(&json!({
                    "provider": "mock-ok",
                    "text": format!("{label}batch worker test {i}")
                }))
                .send()
                .await
                .unwrap();

            assert_eq!(resp.status(), StatusCode::ACCEPTED);
            let body: Value = resp.json().await.unwrap();
            task_ids.push(body["task_id"].as_str().unwrap().to_string());
        }

        // Wait for all tasks to complete
        for task_id in &task_ids {
            let task = wait_for_terminal_status(&client, &base, task_id).await;
            assert_eq!(
                task["status"], "completed",
                "{label}task {task_id} should be completed"
            );
        }

        // Verify stats
        let resp = client
            .get(format!("{base}/api/v1/queue/stats"))
            .send()
            .await
            .unwrap();
        let stats: Value = resp.json().await.unwrap();
        assert!(stats["completed"].as_u64().unwrap() >= 5);

        worker_handle.shutdown_and_join().await;
    }
);

#[tokio::test]
async fn e2e_webhook_callback_not_fired_for_cancelled_before_processing() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let callback_url = format!("{callback_base}/callback");

    // Use a server WITHOUT workers so the task stays queued
    let base = spawn_server().await;
    let client = test_client();

    // Enqueue a task with callback_url (but no workers to process it)
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "slack",
            "text": "cancel before processing",
            "callback_url": callback_url,
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/test"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    let task_id = body["task_id"].as_str().unwrap().to_string();

    // Cancel the task while it's still queued
    let resp = client
        .post(format!("{base}/api/v1/queue/tasks/{task_id}/cancel"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cancel_body: Value = resp.json().await.unwrap();
    assert!(cancel_body["cancelled"].as_bool().unwrap());

    // Give time for any spurious callback
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Callback should NOT be fired (cancellation via API doesn't trigger worker callback path)
    let received = payloads.lock().unwrap();
    assert!(
        received.is_empty(),
        "callback should not fire for API-cancelled tasks (not worker-triggered)"
    );
}

dual_backend_test!(
    with_workers,
    e2e_worker_task_with_metadata_preserved,
    e2e_sqlite_task_metadata_preserved,
    |spawn_fn, label| {
        let (callback_base, payloads) = spawn_callback_server().await;
        let (base, worker_handle) = spawn_fn().await;
        let client = test_client();

        let callback_url = format!("{callback_base}/callback");

        // Enqueue with metadata
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("{label}metadata test"),
                "callback_url": callback_url,
                "metadata": {
                    "request_id": format!("{label}req-abc-123"),
                    "source": format!("{label}e2e-test"),
                    "env": "test"
                }
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        let task_id = body["task_id"].as_str().unwrap().to_string();

        // Wait for completion
        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        assert_eq!(task["status"], "completed");

        // Verify metadata is preserved in task info (SQLite roundtrip)
        assert_eq!(
            task["metadata"]["request_id"],
            format!("{label}req-abc-123")
        );
        assert_eq!(task["metadata"]["source"], format!("{label}e2e-test"));
        assert_eq!(task["metadata"]["env"], "test");

        // Give callback time
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify metadata in callback payload
        {
            let received = payloads.lock().unwrap();
            assert!(!received.is_empty());
            let cb = &received[0];
            assert_eq!(cb["metadata"]["request_id"], format!("{label}req-abc-123"));
            assert_eq!(cb["metadata"]["source"], format!("{label}e2e-test"));
            assert_eq!(cb["metadata"]["env"], "test");
        }

        worker_handle.shutdown_and_join().await;
    }
);
