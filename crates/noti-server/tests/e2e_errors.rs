mod common;

use common::{spawn_server, test_client};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Error response structure consistency (e2e) ─────────────────────

/// Helper: assert that a JSON error response has the standard {error, message} shape.
fn assert_error_shape(body: &Value, expected_error: &str, context: &str) {
    assert!(
        body["error"].is_string(),
        "{context}: response should have a string 'error' field, got: {body}"
    );
    assert!(
        body["message"].is_string(),
        "{context}: response should have a string 'message' field, got: {body}"
    );
    assert_eq!(
        body["error"].as_str().unwrap(),
        expected_error,
        "{context}: error code mismatch"
    );
}

/// All 404 error responses share the same {error: "not_found", message: "..."} shape.
#[tokio::test]
async fn e2e_error_structure_not_found_responses() {
    let base = spawn_server().await;
    let client = test_client();

    // Provider not found
    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "provider not found");

    // Status not found
    let resp = client
        .get(format!("{base}/api/v1/status/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "status not found");

    // Queue task not found
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "queue task not found");

    // Template not found
    let resp = client
        .get(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "template not found");

    // Template delete not found
    let resp = client
        .delete(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "template delete not found");

    // Send with nonexistent provider
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "nonexistent", "text": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "send provider not found");

    // Async send with nonexistent provider
    let resp = client
        .post(format!("{base}/api/v1/send/async"))
        .json(&json!({"provider": "nonexistent", "text": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "not_found", "async send provider not found");
}

/// 400 Bad Request errors share the same {error: "bad_request", message: "..."} shape.
#[tokio::test]
async fn e2e_error_structure_bad_request_responses() {
    let base = spawn_server().await;
    let client = test_client();

    // Send with missing config (provider validation failure)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "slack", "text": "hello", "config": {}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "bad_request", "send missing config");

    // Invalid status filter
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=bogus"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "bad_request", "invalid status filter");

    // Template render with missing required var
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({"name": "err-struct-tpl", "body": "{{a}} and {{b}}"}))
        .send()
        .await
        .unwrap();
    let resp = client
        .post(format!("{base}/api/v1/templates/err-struct-tpl/render"))
        .json(&json!({"variables": {"a": "hello"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "bad_request", "template render missing var");
}

/// 422 Unprocessable Entity (validation) errors include {error, message, fields}.
#[tokio::test]
async fn e2e_error_structure_validation_responses() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "", "text": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["message"].is_string(),
        "validation error should have message"
    );
    assert!(
        body["fields"].is_object(),
        "validation error should have fields object"
    );
}

/// Invalid JSON body returns {error: "invalid_json", message: "..."}.
#[tokio::test]
async fn e2e_error_structure_invalid_json_response() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body("not valid json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_error_shape(&body, "invalid_json", "invalid json body");
}

// ───────────────────── Granular error codes (e2e) ─────────────────────

/// Verify that 404 error responses include the granular `code` field.
#[tokio::test]
async fn e2e_error_codes_not_found_responses_have_code() {
    let base = spawn_server().await;
    let client = test_client();

    // Provider not found → PROVIDER_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "PROVIDER_NOT_FOUND");

    // Template not found → TEMPLATE_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "TEMPLATE_NOT_FOUND");

    // Notification status not found → NOTIFICATION_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/status/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "NOTIFICATION_NOT_FOUND");

    // Queue task not found → TASK_NOT_FOUND
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks/nonexistent-id"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");
    assert_eq!(body["code"], "TASK_NOT_FOUND");
}

/// Verify that 400 error responses include the granular `code` field.
#[tokio::test]
async fn e2e_error_codes_bad_request_responses_have_code() {
    let base = spawn_server().await;
    let client = test_client();

    // Config validation failure → CONFIG_VALIDATION_FAILED
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "slack", "text": "hello", "config": {}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["code"], "CONFIG_VALIDATION_FAILED");

    // Invalid status filter → INVALID_PARAMETER
    let resp = client
        .get(format!("{base}/api/v1/queue/tasks?status=bogus"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["code"], "INVALID_PARAMETER");

    // Template variable missing → TEMPLATE_VARIABLE_MISSING
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({"name": "code-test-tpl", "body": "{{a}} and {{b}}"}))
        .send()
        .await
        .unwrap();
    let resp = client
        .post(format!("{base}/api/v1/templates/code-test-tpl/render"))
        .json(&json!({"variables": {"a": "hello"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "bad_request");
    assert_eq!(body["code"], "TEMPLATE_VARIABLE_MISSING");
}

/// Verify that error responses without a code omit the field entirely (backward compat).
#[tokio::test]
async fn e2e_error_codes_absent_when_not_applicable() {
    let base = spawn_server().await;
    let client = test_client();

    // Invalid JSON body → no code field (handled by ValidatedJsonRejection, not ApiError)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body("not valid json")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "invalid_json");
    assert!(
        body.get("code").is_none() || body["code"].is_null(),
        "invalid_json error should not have a code field"
    );
}
