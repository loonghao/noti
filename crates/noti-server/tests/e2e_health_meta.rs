mod common;

use common::{spawn_server, test_client};
use reqwest::StatusCode;
use serde_json::Value;

// ───────────────────── Health & Meta ─────────────────────

dual_backend_test!(
    basic,
    e2e_health_check,
    e2e_sqlite_health_check,
    |spawn_fn, _label| {
        let base = spawn_fn().await;
        let client = test_client();

        let resp = client
            .get(format!("{base}/health"))
            .send()
            .await
            .expect("request failed");

        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "ok");
        assert!(body["version"].is_string());
    }
);

#[tokio::test]
async fn e2e_api_versions_endpoint() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/versions"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();

    // Should have a versions array and a latest field
    let versions = body["versions"]
        .as_array()
        .expect("versions should be an array");
    assert!(
        !versions.is_empty(),
        "at least one version should be listed"
    );

    // v1 should be present and stable
    let v1 = versions.iter().find(|v| v["version"] == "v1");
    assert!(v1.is_some(), "v1 should be listed");
    let v1 = v1.unwrap();
    assert_eq!(v1["status"], "stable");
    assert_eq!(v1["deprecated"], false);

    // latest should be v1
    assert_eq!(body["latest"], "v1");
}

#[tokio::test]
async fn e2e_api_versions_in_openapi_spec() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();

    let paths = body["paths"].as_object().unwrap();
    assert!(
        paths.contains_key("/api/versions"),
        "OpenAPI spec should include /api/versions path"
    );

    // Verify the Meta tag exists
    let tags = body["tags"].as_array().unwrap();
    let meta_tag = tags.iter().find(|t| t["name"] == "Meta");
    assert!(
        meta_tag.is_some(),
        "Meta tag should be present in OpenAPI spec"
    );
}

#[tokio::test]
async fn e2e_metrics_endpoint() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/metrics"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["providers"]["total_registered"].as_u64().unwrap() > 100);
    assert!(body["version"].is_string());
    assert!(body["uptime_seconds"].as_u64().is_some());
}
