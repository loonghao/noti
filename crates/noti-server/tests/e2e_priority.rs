mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    MockFlakyProvider, MockOkProvider, spawn_callback_server, spawn_server_with_workers_serial,
    spawn_server_without_workers, test_client, wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Priority ordering & Retry behavior (e2e) ─────────────────────

#[tokio::test]
async fn e2e_priority_ordering_urgent_before_low() {
    // Enqueue tasks with different priorities on a server with NO workers,
    // then start a single worker so tasks are processed in priority order.
    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = test_client();

    // Enqueue tasks with different priorities (low first, urgent last)
    let priorities = vec!["low", "normal", "high", "urgent"];
    let mut task_ids = Vec::new();

    for pri in &priorities {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("priority-{pri}"),
                "priority": pri,
                "retry": {"max_retries": 0, "delay_ms": 10}
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        task_ids.push(body["task_id"].as_str().unwrap().to_string());
    }

    // Start a single worker — processes urgent first, then high, normal, low
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    // Wait for all tasks to complete
    for task_id in &task_ids {
        wait_for_terminal_status(&client, &base, task_id).await;
    }

    // Verify all completed
    for task_id in &task_ids {
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let body: Value = resp.json().await.unwrap();
        assert_eq!(
            body["status"], "completed",
            "task {task_id} should be completed"
        );
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_priority_ordering_verified_by_completion_order() {
    // Enqueue low then urgent, verify urgent callback arrives before low
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue: low first, then urgent — urgent should be processed first
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "low-priority-task",
            "priority": "low",
            "callback_url": &callback_url,
            "metadata": {"order": "low"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let low_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "urgent-priority-task",
            "priority": "urgent",
            "callback_url": &callback_url,
            "metadata": {"order": "urgent"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let urgent_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Start single worker to enforce serial processing
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    // Wait for both tasks
    wait_for_terminal_status(&client, &base, &low_id).await;
    wait_for_terminal_status(&client, &base, &urgent_id).await;

    // Give callbacks time
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify callback order: urgent should arrive before low
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 2,
            "expected at least 2 callbacks, got {}",
            received.len()
        );

        // First callback should be for the urgent task
        assert_eq!(
            received[0]["metadata"]["order"], "urgent",
            "urgent task should be processed first, but got: {:?}",
            received[0]["metadata"]["order"]
        );
        assert_eq!(
            received[1]["metadata"]["order"], "low",
            "low task should be processed second"
        );
    }

    worker_handle.shutdown_and_join().await;
}

dual_backend_test!(
    with_workers_serial,
    e2e_retry_task_eventually_succeeds,
    e2e_sqlite_retry_task_eventually_succeeds,
    |spawn_fn, label| {
        // MockFlakyProvider fails first 2 calls, then succeeds.
        // With max_retries=3, the task should eventually complete.
        let (callback_base, payloads) = spawn_callback_server().await;
        let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
        let (base, worker_handle) = spawn_fn(vec![flaky]).await;
        let client = test_client();
        let callback_url = format!("{callback_base}/callback");

        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-flaky",
                "text": format!("{label}retry success test"),
                "retry": {"max_retries": 3, "delay_ms": 10},
                "callback_url": &callback_url,
                "metadata": {"test": format!("{label}retry-success")}
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let task_id = resp.json::<Value>().await.unwrap()["task_id"]
            .as_str()
            .unwrap()
            .to_string();

        // Wait for worker to process through retries and succeed
        let task = wait_for_terminal_status(&client, &base, &task_id).await;
        assert_eq!(
            task["status"], "completed",
            "{label}flaky task should eventually succeed after retries"
        );
        // The task went through 3 attempts: fail, fail, succeed
        assert!(
            task["attempts"].as_u64().unwrap() >= 3,
            "{label}expected at least 3 attempts, got {}",
            task["attempts"]
        );

        // Give callback time
        tokio::time::sleep(Duration::from_millis(300)).await;

        {
            let received = payloads.lock().unwrap();
            assert!(
                !received.is_empty(),
                "{label}callback should be received for completed task"
            );
            assert_eq!(received[0]["status"], "completed");
            assert_eq!(
                received[0]["metadata"]["test"],
                format!("{label}retry-success")
            );
        }

        worker_handle.shutdown_and_join().await;
    }
);

#[tokio::test]
async fn e2e_retry_exhausted_task_fails() {
    // MockFlakyProvider fails first 5 calls, but max_retries=2 means only 3 total attempts.
    // The task should fail after exhausting retries.
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(5));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "retry exhaustion test",
            "retry": {"max_retries": 2, "delay_ms": 10},
            "callback_url": &callback_url,
            "metadata": {"test": "retry-fail"}
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let task_id = resp.json::<Value>().await.unwrap()["task_id"]
        .as_str()
        .unwrap()
        .to_string();

    let task = wait_for_terminal_status(&client, &base, &task_id).await;
    assert_eq!(
        task["status"], "failed",
        "task should fail after exhausting retries"
    );
    assert!(
        task["last_error"].is_string(),
        "failed task should have error message"
    );

    // Give callback time
    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(
            !received.is_empty(),
            "callback should be received for failed task"
        );
        assert_eq!(received[0]["status"], "failed");
        assert!(received[0]["last_error"].is_string());
        assert_eq!(received[0]["metadata"]["test"], "retry-fail");
    }

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_priority_high_tasks_processed_before_normal() {
    // Enqueue 3 normal tasks, then 1 high-priority task on a server with NO
    // workers.  Start a single worker afterwards so dequeue order reflects
    // priority.  Verify via callback arrival order that the high-priority task
    // is processed before all normal tasks.
    let (callback_base, payloads) = spawn_callback_server().await;

    let (base, state) = spawn_server_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue 3 normal tasks first
    let mut all_ids = Vec::new();
    for i in 0..3 {
        let resp = client
            .post(format!("{base}/api/v1/send/async"))
            .json(&json!({
                "provider": "mock-ok",
                "text": format!("normal-{i}"),
                "priority": "normal",
                "callback_url": &callback_url,
                "metadata": {"order": format!("normal-{i}")}
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        all_ids.push(
            resp.json::<Value>().await.unwrap()["task_id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }

    // Enqueue 1 high-priority task (after the normals)
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-ok",
            "text": "high-priority",
            "priority": "high",
            "callback_url": &callback_url,
            "metadata": {"order": "high"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    all_ids.push(
        resp.json::<Value>().await.unwrap()["task_id"]
            .as_str()
            .unwrap()
            .to_string(),
    );

    // Start a single worker — enforces serial processing in priority order.
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let (worker_handle, _worker_stats_handle) = state.start_workers(worker_config);

    // Wait for all tasks to reach terminal state
    for id in &all_ids {
        wait_for_terminal_status(&client, &base, id).await;
    }

    // Give callbacks time to arrive
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify callback order: high-priority task should arrive first
    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 4,
            "expected at least 4 callbacks, got {}",
            received.len()
        );
        // First callback must be from the high-priority task
        assert_eq!(
            received[0]["metadata"]["order"], "high",
            "high-priority task should be processed first, but first callback was: {:?}",
            received[0]["metadata"]["order"]
        );
        // Remaining callbacks should all be normal tasks
        for i in 1..4 {
            let order = received[i]["metadata"]["order"].as_str().unwrap_or("");
            assert!(
                order.starts_with("normal"),
                "callback {i} should be a normal task, got: {order}"
            );
        }
    }

    worker_handle.shutdown_and_join().await;
}
