mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{
    MockOkProvider, spawn_callback_server, spawn_server_sqlite_without_workers, test_client,
    wait_for_terminal_status,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── SQLite queue backend (e2e) ─────────────────────

#[tokio::test]
async fn e2e_sqlite_priority_ordering_urgent_before_low() {
    let (callback_base, payloads) = spawn_callback_server().await;

    // Create AppState with SQLite queue but NO workers yet
    let (base, state) = spawn_server_sqlite_without_workers(vec![Arc::new(MockOkProvider)]).await;

    let client = test_client();
    let callback_url = format!("{callback_base}/callback");

    // Enqueue: low first, then urgent — SQLite should dequeue urgent first
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

    // NOW start a single worker so tasks are processed in priority order
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    // Wait for both tasks
    wait_for_terminal_status(&client, &base, &low_id).await;
    wait_for_terminal_status(&client, &base, &urgent_id).await;

    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let received = payloads.lock().unwrap();
        assert!(
            received.len() >= 2,
            "expected at least 2 callbacks (SQLite), got {}",
            received.len()
        );

        assert_eq!(
            received[0]["metadata"]["order"], "urgent",
            "SQLite: urgent task should be processed first, but got: {:?}",
            received[0]["metadata"]["order"]
        );
        assert_eq!(
            received[1]["metadata"]["order"], "low",
            "SQLite: low task should be processed second"
        );
    }

    worker_handle.shutdown_and_join().await;
}
