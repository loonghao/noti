mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    MockFlakyProvider, spawn_callback_server, spawn_server_sqlite_with_workers_serial,
    spawn_server_with_workers_serial, test_client, wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Batch async with retry policies (mock-flaky + retry config) ─────────────────────

/// Batch-enqueue items where some use `mock-flaky` (fails first N calls then succeeds).
/// With retry policy configured, flaky tasks should eventually succeed.
#[tokio::test]
async fn e2e_batch_async_flaky_with_retry_succeeds() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "flaky retry batch item 1",
                    "retry": {"max_retries": 3, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "high"
                },
                {
                    "provider": "mock-ok",
                    "text": "reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "normal"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);
    assert_eq!(body["failed"], 0);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "completed",
        "flaky task should succeed after retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 3,
        "flaky task should have taken multiple attempts"
    );

    assert_eq!(task_ok["status"], "completed");
    assert_eq!(task_ok["attempts"].as_u64().unwrap(), 1);

    // Both callbacks should arrive
    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "expected 2 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// Batch-enqueue items where flaky provider has too few retries configured —
/// the task should fail after exhausting retries, while mock-ok still succeeds.
#[tokio::test]
async fn e2e_batch_async_flaky_retry_exhausted_fails() {
    let (callback_base, payloads) = spawn_callback_server().await;
    // MockFlakyProvider fails first 5 calls — with max_retries=1, only 2 total attempts → fails
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(5));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "flaky exhaustion batch",
                    "retry": {"max_retries": 1, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "urgent"
                },
                {
                    "provider": "mock-ok",
                    "text": "reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "low"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "failed",
        "flaky task should fail after exhausting retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 2,
        "flaky task should have attempted at least 2 times"
    );

    assert_eq!(task_ok["status"], "completed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "expected 2 callbacks, got {}",
            received.len()
        );

        // Verify the flaky callback reports failure
        let flaky_cb = received
            .iter()
            .find(|p| p["task_id"].as_str() == Some(task_id_flaky.as_str()))
            .expect("should find flaky task callback");
        assert_eq!(flaky_cb["status"], "failed");
    }

    worker_handle.shutdown_and_join().await;
}

/// Batch with mixed retry policies: flaky with sufficient retries, mock-fail with zero retries,
/// mock-ok with no retries, mock-fail with retries. Verifies each task gets its own retry behavior.
#[tokio::test]
async fn e2e_batch_async_mixed_retry_policies() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "flaky with enough retries",
                    "retry": {"max_retries": 5, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "always fails with zero retries",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-ok",
                    "text": "always succeeds",
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "always fails with retries",
                    "retry": {"max_retries": 2, "delay_ms": 10},
                    "callback_url": &callback_url
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 4);
    assert_eq!(body["failed"], 0);

    let task_ids: Vec<String> = (0..4)
        .map(|i| body["results"][i]["task_id"].as_str().unwrap().to_string())
        .collect();

    // Wait for all tasks to reach terminal state
    let mut terminal_statuses = Vec::new();
    for tid in &task_ids {
        let task = wait_for_terminal_status(&client, &base, tid).await;
        terminal_statuses.push(task);
    }

    // Item 0: mock-flaky with 5 retries → should succeed (fails 2, succeeds on 3rd)
    assert_eq!(
        terminal_statuses[0]["status"], "completed",
        "flaky with enough retries should complete"
    );

    // Item 1: mock-fail with 0 retries → should fail immediately (1 attempt only)
    assert_eq!(
        terminal_statuses[1]["status"], "failed",
        "mock-fail with zero retries should fail"
    );
    assert_eq!(
        terminal_statuses[1]["attempts"].as_u64().unwrap(),
        1,
        "zero-retry mock-fail should only try once"
    );

    // Item 2: mock-ok → always succeeds on first try
    assert_eq!(terminal_statuses[2]["status"], "completed");
    assert_eq!(terminal_statuses[2]["attempts"].as_u64().unwrap(), 1);

    // Item 3: mock-fail with 2 retries → should fail after 3 attempts (1 + 2 retries)
    assert_eq!(
        terminal_statuses[3]["status"], "failed",
        "mock-fail with retries should still fail"
    );
    assert!(
        terminal_statuses[3]["attempts"].as_u64().unwrap() >= 3,
        "mock-fail should exhaust all retries"
    );

    // All 4 callbacks should arrive
    tokio::time::sleep(Duration::from_millis(300)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            4,
            "expected 4 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Concurrent batch async with rate limiting ─────────────────────

// Send multiple concurrent batch requests with rate limiting enabled.
// Some requests should be accepted and some rejected per rate limit quota.
dual_backend_test!(
    with_workers_and_rate_limit,
    e2e_concurrent_batch_async_with_rate_limit_partial_reject,
    e2e_sqlite_concurrent_batch_async_with_rate_limit_partial_reject,
    |spawn_fn, label| {
        let (callback_base, payloads) = spawn_callback_server().await;
        // Rate limit: 3 requests per 60s window. We'll send 5 concurrent batch requests.
        let (base, worker_handle, _max_requests) = spawn_fn(vec![], 3, 60).await;
        let client = test_client();
        let callback_url = format!("{callback_base}/callback");

        let mut handles = Vec::new();
        for i in 0..5 {
            let client = client.clone();
            let base = base.clone();
            let cb_url = callback_url.clone();
            handles.push(tokio::spawn(async move {
                let resp = client
                    .post(format!("{base}/api/v1/send/async/batch"))
                    .json(&json!({
                        "items": [
                            {
                                "provider": "mock-ok",
                                "text": format!("rate-limited batch item {i}"),
                                "callback_url": &cb_url,
                            }
                        ]
                    }))
                    .send()
                    .await
                    .unwrap();
                resp.status()
            }));
        }

        let mut accepted = 0u32;
        let mut rate_limited = 0u32;
        for handle in handles {
            let status = handle.await.unwrap();
            match status {
                StatusCode::ACCEPTED => accepted += 1,
                StatusCode::TOO_MANY_REQUESTS => rate_limited += 1,
                other => panic!("{label}unexpected status: {other}"),
            }
        }

        // At most 3 should be accepted (rate limit), at least 2 should be rejected
        assert!(
            accepted <= 3,
            "{label}at most 3 requests should pass rate limit, got {accepted}"
        );
        assert!(
            rate_limited >= 2,
            "{label}at least 2 requests should be rate limited, got {rate_limited}"
        );

        // Wait for accepted tasks to complete
        tokio::time::sleep(Duration::from_millis(500)).await;
        {
            let received = payloads.lock().unwrap();
            assert_eq!(
                received.len() as u32,
                accepted,
                "{label}callbacks should match accepted count: expected {accepted}, got {}",
                received.len()
            );
        }

        worker_handle.shutdown_and_join().await;
    }
);

// Rate limited server: a single batch request within quota should succeed normally.
dual_backend_test!(
    with_workers_and_rate_limit,
    e2e_batch_async_within_rate_limit_succeeds,
    e2e_sqlite_batch_async_within_rate_limit_succeeds,
    |spawn_fn, label| {
        let (callback_base, payloads) = spawn_callback_server().await;
        let (base, worker_handle, _max) = spawn_fn(vec![], 10, 60).await;
        let client = test_client();
        let callback_url = format!("{callback_base}/callback");

        let resp = client
            .post(format!("{base}/api/v1/send/async/batch"))
            .json(&json!({
                "items": [
                    {
                        "provider": "mock-ok",
                        "text": "rate limited batch 1",
                        "callback_url": &callback_url,
                    },
                    {
                        "provider": "mock-ok",
                        "text": "rate limited batch 2",
                        "callback_url": &callback_url,
                    }
                ]
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::ACCEPTED);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["enqueued"], 2);
        assert_eq!(body["failed"], 0);

        // Verify rate limit headers are present on a separate request
        let health_resp = client.get(format!("{base}/health")).send().await.unwrap();
        assert!(
            health_resp.headers().contains_key("x-ratelimit-limit"),
            "{label}expected rate limit headers on health response"
        );

        let task_ids: Vec<String> = (0..2)
            .map(|i| body["results"][i]["task_id"].as_str().unwrap().to_string())
            .collect();

        for tid in &task_ids {
            let task = wait_for_terminal_status(&client, &base, tid).await;
            assert_eq!(task["status"], "completed");
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
        {
            let received = payloads.lock().unwrap();
            assert_eq!(
                received.len(),
                2,
                "{label}expected two callbacks for accepted batch items"
            );
        }

        worker_handle.shutdown_and_join().await;
    }
);

// Rate limit exhausted mid-sequence: first batch goes through, second batch gets 429.
dual_backend_test!(
    with_workers_and_rate_limit,
    e2e_sequential_batch_async_rate_limit_exhaustion,
    e2e_sqlite_sequential_batch_async_rate_limit_exhaustion,
    |spawn_fn, label| {
        let (callback_base, payloads) = spawn_callback_server().await;
        // Only 2 requests allowed per 60s
        let (base, worker_handle, _max) = spawn_fn(vec![], 2, 60).await;
        let client = test_client();
        let callback_url = format!("{callback_base}/callback");

        // First batch — should succeed (request 1)
        let resp1 = client
            .post(format!("{base}/api/v1/send/async/batch"))
            .json(&json!({
                "items": [
                    {
                        "provider": "mock-ok",
                        "text": "first batch",
                        "callback_url": &callback_url,
                    }
                ]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp1.status(), StatusCode::ACCEPTED);

        // Second batch — should succeed (request 2)
        let resp2 = client
            .post(format!("{base}/api/v1/send/async/batch"))
            .json(&json!({
                "items": [
                    {
                        "provider": "mock-ok",
                        "text": "second batch",
                        "callback_url": &callback_url,
                    }
                ]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp2.status(), StatusCode::ACCEPTED);

        // Third batch — should be rate limited (request 3 > quota 2)
        let resp3 = client
            .post(format!("{base}/api/v1/send/async/batch"))
            .json(&json!({
                "items": [
                    {
                        "provider": "mock-ok",
                        "text": "third batch - should be rejected",
                        "callback_url": &callback_url,
                    }
                ]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp3.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "{label}third request should be rate limited"
        );
        let body_429: Value = resp3.json().await.unwrap();
        assert_eq!(body_429["error"], "rate limit exceeded");

        // Wait for the 2 accepted tasks to complete
        tokio::time::sleep(Duration::from_millis(500)).await;
        {
            let received = payloads.lock().unwrap();
            assert_eq!(
                received.len(),
                2,
                "{label}only 2 accepted tasks should produce callbacks"
            );
        }

        worker_handle.shutdown_and_join().await;
    }
);

// ───────────────────── SQLite batch async retry policy tests ─────────────────────

/// SQLite mirror of `e2e_batch_async_flaky_with_retry_succeeds`.
/// Batch-enqueue items where some use `mock-flaky` on SQLite queue backend.
/// With retry policy configured, flaky tasks should eventually succeed.
#[tokio::test]
async fn e2e_sqlite_batch_async_flaky_with_retry_succeeds() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "sqlite flaky retry batch item 1",
                    "retry": {"max_retries": 3, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "high"
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "normal"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);
    assert_eq!(body["failed"], 0);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "completed",
        "SQLite: flaky task should succeed after retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 3,
        "SQLite: flaky task should have taken multiple attempts"
    );

    assert_eq!(task_ok["status"], "completed");
    assert_eq!(task_ok["attempts"].as_u64().unwrap(), 1);

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "SQLite: expected 2 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}

/// SQLite mirror of `e2e_batch_async_flaky_retry_exhausted_fails`.
/// Batch-enqueue items where flaky provider has too few retries configured on SQLite backend —
/// the task should fail after exhausting retries, while mock-ok still succeeds.
#[tokio::test]
async fn e2e_sqlite_batch_async_flaky_retry_exhausted_fails() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(5));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "sqlite flaky exhaustion batch",
                    "retry": {"max_retries": 1, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "urgent"
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite reliable batch item",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url,
                    "priority": "low"
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 2);

    let task_id_flaky = body["results"][0]["task_id"].as_str().unwrap().to_string();
    let task_id_ok = body["results"][1]["task_id"].as_str().unwrap().to_string();

    let task_flaky = wait_for_terminal_status(&client, &base, &task_id_flaky).await;
    let task_ok = wait_for_terminal_status(&client, &base, &task_id_ok).await;

    assert_eq!(
        task_flaky["status"], "failed",
        "SQLite: flaky task should fail after exhausting retries"
    );
    assert!(
        task_flaky["attempts"].as_u64().unwrap() >= 2,
        "SQLite: flaky task should have attempted at least 2 times"
    );

    assert_eq!(task_ok["status"], "completed");

    tokio::time::sleep(Duration::from_millis(200)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            2,
            "SQLite: expected 2 callbacks, got {}",
            received.len()
        );

        let flaky_cb = received
            .iter()
            .find(|p| p["task_id"].as_str() == Some(task_id_flaky.as_str()))
            .expect("SQLite: should find flaky task callback");
        assert_eq!(flaky_cb["status"], "failed");
    }

    worker_handle.shutdown_and_join().await;
}

/// SQLite mirror of `e2e_batch_async_mixed_retry_policies`.
/// Batch with mixed retry policies on SQLite backend: flaky with sufficient retries,
/// mock-fail with zero retries, mock-ok with no retries, mock-fail with retries.
#[tokio::test]
async fn e2e_sqlite_batch_async_mixed_retry_policies() {
    let (callback_base, payloads) = spawn_callback_server().await;
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    let resp = client
        .post(format!("{base}/api/v1/send/async/batch"))
        .json(&json!({
            "items": [
                {
                    "provider": "mock-flaky",
                    "text": "sqlite flaky with enough retries",
                    "retry": {"max_retries": 5, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "sqlite always fails with zero retries",
                    "retry": {"max_retries": 0, "delay_ms": 10},
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-ok",
                    "text": "sqlite always succeeds",
                    "callback_url": &callback_url
                },
                {
                    "provider": "mock-fail",
                    "text": "sqlite always fails with retries",
                    "retry": {"max_retries": 2, "delay_ms": 10},
                    "callback_url": &callback_url
                }
            ]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["enqueued"], 4);
    assert_eq!(body["failed"], 0);

    let task_ids: Vec<String> = (0..4)
        .map(|i| body["results"][i]["task_id"].as_str().unwrap().to_string())
        .collect();

    let mut terminal_statuses = Vec::new();
    for tid in &task_ids {
        let task = wait_for_terminal_status(&client, &base, tid).await;
        terminal_statuses.push(task);
    }

    assert_eq!(
        terminal_statuses[0]["status"], "completed",
        "SQLite: flaky with enough retries should complete"
    );

    assert_eq!(
        terminal_statuses[1]["status"], "failed",
        "SQLite: mock-fail with zero retries should fail"
    );
    assert_eq!(
        terminal_statuses[1]["attempts"].as_u64().unwrap(),
        1,
        "SQLite: zero-retry mock-fail should only try once"
    );

    assert_eq!(terminal_statuses[2]["status"], "completed");
    assert_eq!(terminal_statuses[2]["attempts"].as_u64().unwrap(), 1);

    assert_eq!(
        terminal_statuses[3]["status"], "failed",
        "SQLite: mock-fail with retries should still fail"
    );
    assert!(
        terminal_statuses[3]["attempts"].as_u64().unwrap() >= 3,
        "SQLite: mock-fail should exhaust all retries"
    );

    tokio::time::sleep(Duration::from_millis(300)).await;
    {
        let received = payloads.lock().unwrap();
        assert_eq!(
            received.len(),
            4,
            "SQLite: expected 4 callbacks, got {}",
            received.len()
        );
    }

    worker_handle.shutdown_and_join().await;
}
