mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    MockFlakyProvider, spawn_server_sqlite_with_workers, spawn_server_sqlite_with_workers_serial,
    spawn_server_with_workers, spawn_server_with_workers_serial, test_client,
    wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── SQLite concurrent batch async with rate limiting ─────────────────────

// ───────────────────── Backoff delay timing (e2e) ─────────────────────

#[tokio::test]
async fn e2e_backoff_delay_timing_flaky_task() {
    // MockFlakyProvider fails first 2 calls, then succeeds on the 3rd.
    // With delay_ms=200 (fixed), the queue should hold the task for ~200ms per retry.
    // Total expected wall-clock time >= 200ms * 2 retries = 400ms.
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "backoff timing test",
            "retry": {"max_retries": 3, "delay_ms": 200}
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "flaky task should eventually succeed after retries"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "expected at least 3 attempts, got {}",
        task["attempts"]
    );
    // 2 retries × 200ms delay = 400ms minimum (allow some slack for poll interval)
    assert!(
        elapsed >= Duration::from_millis(350),
        "backoff delay should enforce at least ~400ms total delay, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_backoff_delay_timing_exhausted_retries() {
    // MockFailProvider always fails. With max_retries=2, delay_ms=150,
    // the task should fail after 3 attempts with >= 300ms total delay.
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "backoff exhaustion timing test",
            "retry": {"max_retries": 2, "delay_ms": 150}
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "failed");
    // 2 retries × 150ms = 300ms minimum
    assert!(
        elapsed >= Duration::from_millis(250),
        "backoff delay should enforce at least ~300ms before final failure, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_backoff_delay_zero_delay_is_fast() {
    // With delay_ms=0, retries should happen immediately (no backoff delay).
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "zero delay test",
            "retry": {"max_retries": 3, "delay_ms": 0}
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "expected at least 3 attempts"
    );
    // With zero delay, should complete well under 2 seconds (just poll intervals)
    assert!(
        elapsed < Duration::from_secs(2),
        "zero delay retries should be fast, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_backoff_delay_timing_flaky_task() {
    // Same as e2e_backoff_delay_timing_flaky_task but with SQLite queue backend.
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = test_client();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "sqlite backoff timing test",
            "retry": {"max_retries": 3, "delay_ms": 200}
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "SQLite: flaky task should eventually succeed after retries"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "SQLite: expected at least 3 attempts, got {}",
        task["attempts"]
    );
    assert!(
        elapsed >= Duration::from_millis(350),
        "SQLite: backoff delay should enforce at least ~400ms total, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_backoff_delay_timing_exhausted_retries() {
    // Same as e2e_backoff_delay_timing_exhausted_retries but with SQLite queue backend.
    let (base, worker_handle) = spawn_server_sqlite_with_workers().await;
    let client = test_client();

    let start = std::time::Instant::now();

    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "sqlite backoff exhaustion timing test",
            "retry": {"max_retries": 2, "delay_ms": 150}
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "failed");
    assert!(
        elapsed >= Duration::from_millis(250),
        "SQLite: backoff delay should enforce at least ~300ms before final failure, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

// ───────────────────── Exponential backoff via API (e2e) ─────────────────────

#[tokio::test]
async fn e2e_exponential_backoff_api_flaky_task() {
    // Test that backoff_multiplier in the API request produces exponential delays.
    // MockFlakyProvider fails first 2 calls, succeeds on 3rd.
    // With delay_ms=100 and backoff_multiplier=2.0:
    //   attempt 1 fails → wait 100ms
    //   attempt 2 fails → wait 200ms
    //   attempt 3 succeeds
    // Total backoff ≥ 250ms (100 + 200 = 300, minus timing slack)
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "exponential backoff test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 100,
                "backoff_multiplier": 2.0,
                "max_delay_ms": 5000
            }
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "flaky task with exponential backoff should eventually succeed"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "expected at least 3 attempts, got {}",
        task["attempts"]
    );
    // 100ms + 200ms = 300ms minimum backoff
    assert!(
        elapsed >= Duration::from_millis(250),
        "exponential backoff should take at least ~300ms, but elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_exponential_backoff_api_exhausted() {
    // Test exponential backoff with max_retries=2, always-fail provider.
    // delay_ms=100, backoff_multiplier=2.0 → waits 100ms + 200ms = 300ms total.
    let (base, worker_handle) = spawn_server_with_workers().await;
    let client = test_client();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-fail",
            "text": "exponential exhaustion test",
            "retry": {
                "max_retries": 2,
                "delay_ms": 100,
                "backoff_multiplier": 2.0
            }
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "failed");
    assert!(
        elapsed >= Duration::from_millis(250),
        "exponential backoff exhaustion should take at least ~300ms, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_exponential_backoff_api_max_delay_caps() {
    // Test that max_delay_ms caps the exponential growth.
    // delay_ms=200, backoff_multiplier=10.0, max_delay_ms=300
    // attempt 1 fails → wait 200ms
    // attempt 2 fails → would be 2000ms but capped at 300ms → wait 300ms
    // Total ≥ 450ms (200 + 300 = 500, minus timing slack)
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "max delay cap test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 200,
                "backoff_multiplier": 10.0,
                "max_delay_ms": 300
            }
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    // 200ms + 300ms (capped) = 500ms minimum
    assert!(
        elapsed >= Duration::from_millis(450),
        "max_delay_ms should cap growth, expected ≥450ms, elapsed was {elapsed:?}"
    );
    // Without the cap it would be 200+2000=2200ms, so verify it was fast enough
    assert!(
        elapsed < Duration::from_millis(2000),
        "max_delay_ms cap should prevent 2s+ delays, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_sqlite_exponential_backoff_api_flaky_task() {
    // Same as e2e_exponential_backoff_api_flaky_task but with SQLite queue backend.
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_sqlite_with_workers_serial(vec![flaky]).await;
    let client = test_client();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "sqlite exponential backoff test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 100,
                "backoff_multiplier": 2.0,
                "max_delay_ms": 5000
            }
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
    let elapsed = start.elapsed();

    assert_eq!(
        task["status"], "completed",
        "SQLite: flaky task with exponential backoff should succeed"
    );
    assert!(
        task["attempts"].as_u64().unwrap() >= 3,
        "SQLite: expected at least 3 attempts, got {}",
        task["attempts"]
    );
    assert!(
        elapsed >= Duration::from_millis(250),
        "SQLite: exponential backoff should take at least ~300ms, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}

#[tokio::test]
async fn e2e_backoff_multiplier_1_is_fixed() {
    // Verify that backoff_multiplier=1.0 behaves the same as fixed delay.
    // MockFlakyProvider fails first 2 calls, succeeds on 3rd.
    // With delay_ms=100 and backoff_multiplier=1.0 → fixed 100ms each retry.
    // Total backoff = 100ms + 100ms = 200ms
    let flaky: Arc<dyn noti_core::NotifyProvider> = Arc::new(MockFlakyProvider::new(2));
    let (base, worker_handle) = spawn_server_with_workers_serial(vec![flaky]).await;
    let client = test_client();

    let start = std::time::Instant::now();
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({
            "provider": "mock-flaky",
            "text": "fixed delay via multiplier=1 test",
            "retry": {
                "max_retries": 3,
                "delay_ms": 100,
                "backoff_multiplier": 1.0
            }
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
    let elapsed = start.elapsed();

    assert_eq!(task["status"], "completed");
    // Fixed 100ms × 2 retries = 200ms minimum
    assert!(
        elapsed >= Duration::from_millis(150),
        "fixed delay (multiplier=1) should take at least ~200ms, elapsed was {elapsed:?}"
    );
    // Should NOT grow beyond 200ms + overhead (not exponential)
    assert!(
        elapsed < Duration::from_millis(1000),
        "fixed delay should not grow exponentially, elapsed was {elapsed:?}"
    );

    worker_handle.shutdown_and_join().await;
}
