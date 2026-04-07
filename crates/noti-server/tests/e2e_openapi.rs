mod common;

use common::{spawn_server, test_client};
use reqwest::StatusCode;
use serde_json::Value;

// ───────────────────── OpenAPI / Swagger ─────────────────────

#[tokio::test]
async fn e2e_openapi_json_valid() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    // Basic OpenAPI 3.x structure validation
    assert!(
        body["openapi"].as_str().unwrap().starts_with("3."),
        "expected OpenAPI 3.x version"
    );
    assert!(body["info"]["title"].is_string());
    assert!(body["paths"].is_object());

    // Verify key paths exist
    let paths = body["paths"].as_object().unwrap();
    assert!(paths.contains_key("/health"), "missing /health path");
    assert!(
        paths.contains_key("/api/v1/send"),
        "missing /api/v1/send path"
    );
    assert!(
        paths.contains_key("/api/v1/providers"),
        "missing /api/v1/providers path"
    );
    assert!(
        paths.contains_key("/api/v1/templates"),
        "missing /api/v1/templates path"
    );
    assert!(
        paths.contains_key("/api/v1/queue/stats"),
        "missing /api/v1/queue/stats path"
    );
}

#[tokio::test]
async fn e2e_swagger_ui_accessible() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/swagger-ui/"))
        .send()
        .await
        .unwrap();

    // Swagger UI should return 200 (HTML)
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(
        content_type.contains("text/html"),
        "expected HTML content type, got: {content_type}"
    );
}

#[tokio::test]
async fn e2e_openapi_schema_retry_config_has_backoff_fields() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    // Navigate to RetryConfig schema
    let schemas = &body["components"]["schemas"];
    assert!(
        schemas["RetryConfig"].is_object(),
        "RetryConfig schema should exist in OpenAPI components"
    );

    let retry_props = &schemas["RetryConfig"]["properties"];
    assert!(
        retry_props.is_object(),
        "RetryConfig should have properties"
    );

    // Verify all four fields are present
    assert!(
        retry_props["max_retries"].is_object(),
        "RetryConfig should have max_retries field"
    );
    assert!(
        retry_props["delay_ms"].is_object(),
        "RetryConfig should have delay_ms field"
    );
    assert!(
        retry_props["backoff_multiplier"].is_object(),
        "RetryConfig should have backoff_multiplier field"
    );
    assert!(
        retry_props["max_delay_ms"].is_object(),
        "RetryConfig should have max_delay_ms field"
    );

    // Verify types: backoff_multiplier should be number, max_delay_ms should be integer
    let bm_type = retry_props["backoff_multiplier"]["type"]
        .as_str()
        .unwrap_or("");
    assert!(
        bm_type == "number" || retry_props["backoff_multiplier"]["format"].is_string(),
        "backoff_multiplier should be a number type, got: {bm_type}"
    );

    let md_type = retry_props["max_delay_ms"]["type"].as_str().unwrap_or("");
    assert!(
        md_type == "integer" || retry_props["max_delay_ms"]["format"].is_string(),
        "max_delay_ms should be an integer type, got: {md_type}"
    );
}

#[tokio::test]
async fn e2e_openapi_schema_all_key_components_exist() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    let schemas = body["components"]["schemas"].as_object().unwrap();

    // Verify all key schema components are present
    let expected_schemas = [
        "ApiError",
        "RetryConfig",
        "SendRequest",
        "HealthResponse",
        "EnqueueResponse",
        "StatsResponse",
        "TaskInfo",
        "TemplateResponse",
        "ProviderInfo",
        "MetricsResponse",
        // Storage schemas
        "UploadResponse",
        "FileMetadata",
        "DeleteFileResponse",
        // DLQ schemas
        "DlqEntryInfo",
        "DlqStatsResponse",
        "RequeueResponse",
        "DeleteDlqResponse",
    ];

    for name in &expected_schemas {
        assert!(
            schemas.contains_key(*name),
            "missing schema component: {name}"
        );
    }
}

#[tokio::test]
async fn e2e_openapi_schema_all_api_paths_exist() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    let paths = body["paths"].as_object().unwrap();

    // All API routes should be documented
    let expected_paths = [
        "/health",
        "/api/versions",
        "/api/v1/send",
        "/api/v1/send/batch",
        "/api/v1/send/async",
        "/api/v1/send/async/batch",
        "/api/v1/status/{notification_id}",
        "/api/v1/status",
        "/api/v1/status/purge",
        "/api/v1/templates",
        "/api/v1/templates/{name}",
        "/api/v1/templates/{name}/render",
        "/api/v1/providers",
        "/api/v1/providers/{name}",
        "/api/v1/queue/stats",
        "/api/v1/queue/tasks",
        "/api/v1/queue/tasks/{task_id}",
        "/api/v1/queue/tasks/{task_id}/cancel",
        "/api/v1/queue/purge",
        "/api/v1/queue/dlq",
        "/api/v1/queue/dlq/stats",
        "/api/v1/queue/dlq/{task_id}/requeue",
        "/api/v1/queue/dlq/{task_id}",
        "/api/v1/storage/upload",
        "/api/v1/storage/{file_id}",
        "/api/v1/storage/{file_id}/thumbnail",
        "/api/v1/metrics",
    ];

    for path in &expected_paths {
        assert!(
            paths.contains_key(*path),
            "missing API path in OpenAPI spec: {path}"
        );
    }
}

#[tokio::test]
async fn e2e_openapi_schema_storage_schemas_have_required_fields() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    let schemas = &body["components"]["schemas"];

    // UploadResponse should have required fields
    let upload = &schemas["UploadResponse"]["properties"];
    assert!(upload["file_id"].is_object(), "UploadResponse should have file_id");
    assert!(upload["file_name"].is_object(), "UploadResponse should have file_name");
    assert!(upload["mime_type"].is_object(), "UploadResponse should have mime_type");
    assert!(upload["size_bytes"].is_object(), "UploadResponse should have size_bytes");
    assert!(
        upload["download_url"].is_object(),
        "UploadResponse should have download_url"
    );
    // thumbnail_url is optional
    assert!(
        upload["thumbnail_url"].is_object(),
        "UploadResponse should have thumbnail_url (even if optional)"
    );

    // FileMetadata should have required fields
    let meta = &schemas["FileMetadata"]["properties"];
    assert!(meta["file_id"].is_object(), "FileMetadata should have file_id");
    assert!(meta["is_image"].is_object(), "FileMetadata should have is_image");
    assert!(
        meta["has_thumbnail"].is_object(),
        "FileMetadata should have has_thumbnail"
    );

    // DeleteFileResponse should have required fields
    let del = &schemas["DeleteFileResponse"]["properties"];
    assert!(del["file_id"].is_object(), "DeleteFileResponse should have file_id");
    assert!(del["deleted"].is_object(), "DeleteFileResponse should have deleted");
    assert!(del["message"].is_object(), "DeleteFileResponse should have message");
}

#[tokio::test]
async fn e2e_openapi_storage_endpoints_have_get_post_delete_methods() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    let paths = body["paths"].as_object().unwrap();

    // POST /api/v1/storage/upload
    let upload_path = &paths["/api/v1/storage/upload"];
    assert!(
        upload_path["post"].is_object(),
        "/api/v1/storage/upload should have POST method"
    );

    // GET /api/v1/storage/{file_id}
    let download_path = &paths["/api/v1/storage/{file_id}"];
    assert!(
        download_path["get"].is_object(),
        "/api/v1/storage/{{file_id}} should have GET method"
    );
    assert!(
        download_path["delete"].is_object(),
        "/api/v1/storage/{{file_id}} should have DELETE method"
    );

    // GET /api/v1/storage/{file_id}/thumbnail
    let thumb_path = &paths["/api/v1/storage/{file_id}/thumbnail"];
    assert!(
        thumb_path["get"].is_object(),
        "/api/v1/storage/{{file_id}}/thumbnail should have GET method"
    );
}

#[tokio::test]
async fn e2e_openapi_has_storage_tag() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    let tags = body["tags"].as_array().unwrap();
    let tag_names: Vec<&str> = tags.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(
        tag_names.contains(&"Storage"),
        "OpenAPI spec should have 'Storage' tag. Found: {tag_names:?}"
    );
}
