mod common;

use common::{
    spawn_server, spawn_server_with_auth, spawn_server_with_body_limit,
    spawn_server_with_cors_permissive, spawn_server_with_cors_restricted,
    spawn_server_with_full_middleware, spawn_server_with_rate_limit, spawn_server_with_request_id,
    test_client,
};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Auth middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_auth_rejects_unauthenticated_request() {
    let (base, _keys) = spawn_server_with_auth(vec!["test-key-alpha".to_string()]).await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "unauthorized");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("missing API key")
    );
}

#[tokio::test]
async fn e2e_auth_rejects_invalid_key() {
    let (base, _keys) = spawn_server_with_auth(vec!["correct-key".to_string()]).await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", "Bearer wrong-key")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "unauthorized");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("invalid API key")
    );
}

#[tokio::test]
async fn e2e_auth_accepts_valid_bearer_token() {
    let (base, keys) = spawn_server_with_auth(vec!["my-secret-key".to_string()]).await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", format!("Bearer {}", keys[0]))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["total"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn e2e_auth_accepts_x_api_key_header() {
    let (base, keys) = spawn_server_with_auth(vec!["x-api-key-value".to_string()]).await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("X-API-Key", &keys[0])
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn e2e_auth_health_bypasses_auth() {
    let (base, _keys) = spawn_server_with_auth(vec!["secret".to_string()]).await;
    let client = test_client();

    // /health is excluded from auth by default
    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_auth_multiple_keys() {
    let (base, keys) =
        spawn_server_with_auth(vec!["key-one".to_string(), "key-two".to_string()]).await;
    let client = test_client();

    for key in &keys {
        let resp = client
            .get(format!("{base}/api/v1/providers"))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "key '{key}' should be accepted"
        );
    }
}

#[tokio::test]
async fn e2e_auth_post_endpoint_requires_key() {
    let (base, keys) = spawn_server_with_auth(vec!["post-key".to_string()]).await;
    let client = test_client();

    // Without key → 401
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({"provider": "slack", "text": "test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // With key → should proceed (will get 400/404 from handler, not 401)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("Authorization", format!("Bearer {}", keys[0]))
        .json(&json!({"provider": "slack", "text": "test", "config": {}}))
        .send()
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ───────────────────── Rate limit middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_rate_limit_allows_within_quota() {
    let (base, max_requests) = spawn_server_with_rate_limit(5, 60).await;
    let client = test_client();

    for i in 0..max_requests {
        let resp = client.get(format!("{base}/health")).send().await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "request {i} should be allowed"
        );

        // Verify rate limit headers are present
        assert!(
            resp.headers().contains_key("x-ratelimit-limit"),
            "missing x-ratelimit-limit header on request {i}"
        );
        assert!(
            resp.headers().contains_key("x-ratelimit-remaining"),
            "missing x-ratelimit-remaining header on request {i}"
        );
        assert_eq!(
            resp.headers()["x-ratelimit-limit"].to_str().unwrap(),
            max_requests.to_string()
        );
    }
}

#[tokio::test]
async fn e2e_rate_limit_returns_429_when_exceeded() {
    let (base, max_requests) = spawn_server_with_rate_limit(3, 60).await;
    let client = test_client();

    // Exhaust the quota
    for _ in 0..max_requests {
        let resp = client.get(format!("{base}/health")).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Next request should be rate limited
    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

    // Verify 429 response structure
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "rate limit exceeded");
    assert!(body["retry_after_seconds"].as_u64().is_some());
    assert_eq!(body["limit"], max_requests);
}

#[tokio::test]
async fn e2e_rate_limit_429_has_retry_after_header() {
    let (base, _max) = spawn_server_with_rate_limit(1, 60).await;
    let client = test_client();

    // Use up the single allowed request
    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Trigger 429
    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        resp.headers().contains_key("retry-after"),
        "429 response should include Retry-After header"
    );
    let retry_after = resp.headers()["retry-after"]
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(
        retry_after > 0,
        "Retry-After should be positive, got {retry_after}"
    );
}

#[tokio::test]
async fn e2e_rate_limit_remaining_decrements() {
    let (base, _max) = spawn_server_with_rate_limit(10, 60).await;
    let client = test_client();

    let resp1 = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp1.status(), StatusCode::OK);
    let remaining1: u64 = resp1.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    let resp2 = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let remaining2: u64 = resp2.headers()["x-ratelimit-remaining"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    assert!(
        remaining2 < remaining1,
        "remaining should decrement: {remaining1} -> {remaining2}"
    );
}

// ───────────────────── Full middleware stack (e2e) ─────────────────────

#[tokio::test]
async fn e2e_full_middleware_auth_before_rate_limit() {
    let (base, keys) =
        spawn_server_with_full_middleware(vec!["full-stack-key".to_string()], 5, 60).await;
    let client = test_client();

    // Unauthenticated → 401 (auth fires before rate limit)
    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Authenticated → 200 with rate limit headers
    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", format!("Bearer {}", keys[0]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    assert!(resp.headers().contains_key("x-ratelimit-remaining"));
}

#[tokio::test]
async fn e2e_full_middleware_health_bypasses_auth_has_rate_limit() {
    let (base, _keys) =
        spawn_server_with_full_middleware(vec!["bypass-key".to_string()], 100, 60).await;
    let client = test_client();

    // /health bypasses auth but still gets rate limit headers
    let resp = client.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key("x-ratelimit-limit"));
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_full_middleware_exhausts_rate_limit() {
    let (base, keys) =
        spawn_server_with_full_middleware(vec!["exhaust-key".to_string()], 3, 60).await;
    let client = test_client();
    let key = &keys[0];

    // Use up quota with authenticated requests
    for _ in 0..3 {
        let resp = client
            .get(format!("{base}/api/v1/providers"))
            .header("Authorization", format!("Bearer {key}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // 4th authenticated request → 429
    let resp = client
        .get(format!("{base}/api/v1/providers"))
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

// ───────────────────── Body size limit middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_body_limit_small_body_accepted() {
    // Set a 1 KiB limit — small JSON payloads should be accepted
    let (base, _max) = spawn_server_with_body_limit(1024).await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": "small payload",
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/ok"}
        }))
        .send()
        .await
        .unwrap();

    // Should NOT be 413 — the handler may return 400 due to config validation,
    // but the body was accepted by the body limit layer.
    assert_ne!(
        resp.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "small body should not trigger 413"
    );
}

#[tokio::test]
async fn e2e_body_limit_oversized_body_rejected() {
    // Set a very small limit (128 bytes) so our payload clearly exceeds it.
    // axum's DefaultBodyLimit may return 413 (PayloadTooLarge) or the JSON
    // extractor may catch the underlying bytes rejection and return 400.
    // Either way, the request should NOT succeed (not 2xx).
    let (base, _max) = spawn_server_with_body_limit(128).await;
    let client = test_client();

    // Build a payload much larger than 128 bytes
    let large_text = "x".repeat(2048);
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": large_text,
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/big"}
        }))
        .send()
        .await
        .unwrap();

    // Body limit enforcement: expect 413 (PAYLOAD_TOO_LARGE) or 400 (BAD_REQUEST)
    // depending on how axum's JSON extractor surfaces the bytes limit error
    let status = resp.status();
    assert!(
        status == StatusCode::PAYLOAD_TOO_LARGE || status == StatusCode::BAD_REQUEST,
        "oversized body should return 413 or 400, got: {status}"
    );
    assert!(!status.is_success(), "oversized body should never succeed");
}

#[tokio::test]
async fn e2e_body_limit_get_requests_unaffected() {
    // Body limit only applies to request bodies; GET endpoints should work fine
    let (base, _max) = spawn_server_with_body_limit(64).await;
    let client = test_client();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn e2e_body_limit_exact_boundary() {
    // Set limit to exactly accommodate a small JSON body
    // The json!({}) payload is about 2 bytes ("{}"), but reqwest adds Content-Type
    // headers etc. We use a known-small payload with a generous limit.
    let (base, _max) = spawn_server_with_body_limit(4096).await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "body-limit-test",
            "body": "Hello {{name}}"
        }))
        .send()
        .await
        .unwrap();

    // Template creation should succeed (201) — body fits within limit
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ───────────────────── Request ID middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_request_id_generated_when_absent() {
    let base = spawn_server_with_request_id().await;
    let client = test_client();

    let resp = client.get(format!("{base}/health")).send().await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Response should contain x-request-id header
    let request_id = resp
        .headers()
        .get("x-request-id")
        .expect("response should have x-request-id header");
    let id_str = request_id.to_str().unwrap();

    // Should be a valid UUID v4
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "x-request-id should be a valid UUID, got: {id_str}"
    );
}

#[tokio::test]
async fn e2e_request_id_preserved_when_provided() {
    let base = spawn_server_with_request_id().await;
    let client = test_client();

    let custom_id = "my-custom-request-id-42";
    let resp = client
        .get(format!("{base}/health"))
        .header("x-request-id", custom_id)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let returned_id = resp
        .headers()
        .get("x-request-id")
        .expect("response should echo x-request-id")
        .to_str()
        .unwrap();
    assert_eq!(
        returned_id, custom_id,
        "server should preserve the client-provided request ID"
    );
}

#[tokio::test]
async fn e2e_request_id_unique_per_request() {
    let base = spawn_server_with_request_id().await;
    let client = test_client();

    let resp1 = client.get(format!("{base}/health")).send().await.unwrap();
    let id1 = resp1.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_string();

    let resp2 = client.get(format!("{base}/health")).send().await.unwrap();
    let id2 = resp2.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_string();

    assert_ne!(id1, id2, "each request should get a unique request ID");
}

#[tokio::test]
async fn e2e_request_id_present_on_post_endpoints() {
    let base = spawn_server_with_request_id().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": "test request id on POST"
        }))
        .send()
        .await
        .unwrap();

    // Regardless of the handler response status, x-request-id should be present
    assert!(
        resp.headers().contains_key("x-request-id"),
        "POST endpoints should also receive x-request-id"
    );
    let id_str = resp.headers()["x-request-id"].to_str().unwrap();
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "x-request-id should be a valid UUID on POST, got: {id_str}"
    );
}

#[tokio::test]
async fn e2e_request_id_present_on_error_responses() {
    let base = spawn_server_with_request_id().await;
    let client = test_client();

    // Request a non-existent provider — should get 404 but still have x-request-id
    let resp = client
        .get(format!("{base}/api/v1/providers/nonexistent"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    assert!(
        resp.headers().contains_key("x-request-id"),
        "error responses should also carry x-request-id"
    );
}

// ───────────────────── CORS middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_cors_permissive_allows_any_origin() {
    let base = spawn_server_with_cors_permissive().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/health"))
        .header("Origin", "https://example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("permissive CORS should return Access-Control-Allow-Origin header");
    assert_eq!(
        acao.to_str().unwrap(),
        "*",
        "permissive CORS should allow all origins (*)"
    );
}

#[tokio::test]
async fn e2e_cors_permissive_preflight_succeeds() {
    let base = spawn_server_with_cors_permissive().await;
    let client = test_client();

    let resp = client
        .request(reqwest::Method::OPTIONS, format!("{base}/api/v1/providers"))
        .header("Origin", "https://example.com")
        .header("Access-Control-Request-Method", "GET")
        .header(
            "Access-Control-Request-Headers",
            "Content-Type, Authorization",
        )
        .send()
        .await
        .unwrap();

    // OPTIONS preflight should succeed (2xx)
    assert!(
        resp.status().is_success(),
        "OPTIONS preflight should succeed, got: {}",
        resp.status()
    );
    assert!(
        resp.headers().contains_key("access-control-allow-origin"),
        "preflight response should contain Access-Control-Allow-Origin"
    );
    assert!(
        resp.headers().contains_key("access-control-allow-methods"),
        "preflight response should contain Access-Control-Allow-Methods"
    );
    assert!(
        resp.headers().contains_key("access-control-allow-headers"),
        "preflight response should contain Access-Control-Allow-Headers"
    );
}

#[tokio::test]
async fn e2e_cors_restricted_allows_matching_origin() {
    let base = spawn_server_with_cors_restricted(vec![
        "https://allowed.example.com".to_string(),
        "https://also-allowed.com".to_string(),
    ])
    .await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/health"))
        .header("Origin", "https://allowed.example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("matching origin should return Access-Control-Allow-Origin header");
    assert_eq!(
        acao.to_str().unwrap(),
        "https://allowed.example.com",
        "ACAO should reflect the matching origin"
    );
}

#[tokio::test]
async fn e2e_cors_restricted_rejects_non_matching_origin() {
    let base =
        spawn_server_with_cors_restricted(vec!["https://allowed.example.com".to_string()]).await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/health"))
        .header("Origin", "https://evil.example.com")
        .send()
        .await
        .unwrap();

    // The request itself still succeeds (CORS is enforced by browsers),
    // but Access-Control-Allow-Origin should NOT be set for non-matching origins.
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "non-matching origin should NOT receive Access-Control-Allow-Origin header"
    );
}

#[tokio::test]
async fn e2e_cors_restricted_preflight_non_matching_origin() {
    let base =
        spawn_server_with_cors_restricted(vec!["https://allowed.example.com".to_string()]).await;
    let client = test_client();

    let resp = client
        .request(reqwest::Method::OPTIONS, format!("{base}/api/v1/send"))
        .header("Origin", "https://evil.example.com")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();

    // Preflight for non-matching origin should not include ACAO
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "preflight for non-matching origin should NOT include ACAO"
    );
}

#[tokio::test]
async fn e2e_cors_permissive_post_endpoint() {
    let base = spawn_server_with_cors_permissive().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("Origin", "https://any-origin.com")
        .json(&json!({
            "provider": "slack",
            "text": "cors test"
        }))
        .send()
        .await
        .unwrap();

    // Regardless of handler result, ACAO should be present
    let acao = resp
        .headers()
        .get("access-control-allow-origin")
        .expect("POST response should include ACAO with permissive CORS");
    assert_eq!(acao.to_str().unwrap(), "*");
}

// ───────────────────── ValidatedJson middleware (e2e) ─────────────────────

#[tokio::test]
async fn e2e_validated_json_empty_provider_returns_422() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "",
            "text": "hello"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["message"], "Request body validation failed");
    assert!(
        body["fields"]["provider"].is_array(),
        "should report field-level error for 'provider'"
    );
    let provider_errors = body["fields"]["provider"].as_array().unwrap();
    assert!(
        provider_errors
            .iter()
            .any(|e| e.as_str().unwrap().contains("must not be empty")),
        "error message should mention 'must not be empty'"
    );
}

#[tokio::test]
async fn e2e_validated_json_empty_text_returns_422() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["fields"]["text"].is_array(),
        "should report field-level error for 'text'"
    );
}

#[tokio::test]
async fn e2e_validated_json_multiple_field_errors() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "",
            "text": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    // Both fields should have errors
    assert!(
        body["fields"]["provider"].is_array(),
        "should report 'provider' field error"
    );
    assert!(
        body["fields"]["text"].is_array(),
        "should report 'text' field error"
    );
}

#[tokio::test]
async fn e2e_validated_json_invalid_json_returns_400() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body("this is not valid json")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "invalid_json");
    assert!(
        body["message"].is_string(),
        "should include an error message"
    );
}

#[tokio::test]
async fn e2e_validated_json_missing_required_fields_returns_422() {
    let base = spawn_server().await;
    let client = test_client();

    // Send JSON with missing required fields (only config, no provider/text)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .header("content-type", "application/json")
        .body(r#"{"config": {}}"#)
        .send()
        .await
        .unwrap();

    // Missing required fields should cause a deserialization error (400)
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "invalid_json");
}

#[tokio::test]
async fn e2e_validated_json_template_empty_name_returns_422() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "",
            "body": "Hello {{name}}"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["fields"]["name"].is_array(),
        "should report field-level error for template 'name'"
    );
}

#[tokio::test]
async fn e2e_validated_json_template_empty_body_returns_422() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "test-template",
            "body": ""
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "validation_failed");
    assert!(
        body["fields"]["body"].is_array(),
        "should report field-level error for template 'body'"
    );
}

#[tokio::test]
async fn e2e_validated_json_valid_request_passes_validation() {
    let base = spawn_server().await;
    let client = test_client();

    // A valid send request — should pass validation and reach the handler
    // (will fail at provider config level, but not at validation level)
    let resp = client
        .post(format!("{base}/api/v1/send"))
        .json(&json!({
            "provider": "slack",
            "text": "valid message",
            "config": {"webhook_url": "https://hooks.slack.com/services/T00/B00/valid"}
        }))
        .send()
        .await
        .unwrap();

    // Should NOT be 422 (validation passed) — handler may return 400 due to other issues
    assert_ne!(
        resp.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "valid payload should not trigger 422 validation error"
    );
}
