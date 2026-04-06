mod common;

use common::{spawn_server, test_client};
use reqwest::StatusCode;
use serde_json::{Value, json};

// ───────────────────── Template CRUD depth tests ─────────────────────

/// Multiple templates can be created and listed; list returns sorted names.
#[tokio::test]
async fn e2e_template_list_multiple_sorted() {
    let base = spawn_server().await;
    let client = test_client();

    for name in ["zulu-tpl", "alpha-tpl", "mike-tpl"] {
        let resp = client
            .post(format!("{base}/api/v1/templates"))
            .json(&json!({
                "name": name,
                "body": format!("Hello from {name}")
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = client
        .get(format!("{base}/api/v1/templates"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"].as_u64().unwrap(), 3);
    let names: Vec<&str> = body["templates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["alpha-tpl", "mike-tpl", "zulu-tpl"]);
}

/// Update preserves existing defaults that are not overridden.
#[tokio::test]
async fn e2e_template_update_preserves_defaults() {
    let base = spawn_server().await;
    let client = test_client();

    // Create with two defaults
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "defaults-tpl",
            "body": "{{greeting}}, {{name}}!",
            "defaults": {"greeting": "Hi", "name": "World"}
        }))
        .send()
        .await
        .unwrap();

    // Update only the greeting default
    let resp = client
        .put(format!("{base}/api/v1/templates/defaults-tpl"))
        .json(&json!({
            "defaults": {"greeting": "Hello"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["defaults"]["greeting"], "Hello");
    assert_eq!(body["defaults"]["name"], "World");
}

/// Rendering with missing required variables (no defaults) returns 400.
#[tokio::test]
async fn e2e_template_render_missing_required_var_returns_400() {
    let base = spawn_server().await;
    let client = test_client();

    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "required-vars",
            "body": "{{a}} and {{b}}"
        }))
        .send()
        .await
        .unwrap();

    // Provide only one of two required variables
    let resp = client
        .post(format!("{base}/api/v1/templates/required-vars/render"))
        .json(&json!({
            "variables": {"a": "hello"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    let msg = body["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("b"),
        "error should mention missing variable 'b'"
    );
}

/// Rendering with defaults supplying the missing variable succeeds.
#[tokio::test]
async fn e2e_template_render_defaults_fill_missing_vars() {
    let base = spawn_server().await;
    let client = test_client();

    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "with-default",
            "body": "{{greeting}}, {{name}}!",
            "title": "{{greeting}} title",
            "defaults": {"greeting": "Hey"}
        }))
        .send()
        .await
        .unwrap();

    let resp = client
        .post(format!("{base}/api/v1/templates/with-default/render"))
        .json(&json!({
            "variables": {"name": "Alice"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["text"], "Hey, Alice!");
    assert_eq!(body["title"], "Hey title");
}

/// Deleting a non-existent template returns 404.
#[tokio::test]
async fn e2e_template_delete_nonexistent_returns_404() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .delete(format!("{base}/api/v1/templates/no-such-template"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Getting a non-existent template returns 404.
#[tokio::test]
async fn e2e_template_get_nonexistent_returns_404() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/templates/nonexistent"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Rendering a non-existent template returns 404.
#[tokio::test]
async fn e2e_template_render_nonexistent_returns_404() {
    let base = spawn_server().await;
    let client = test_client();

    let resp = client
        .post(format!("{base}/api/v1/templates/ghost/render"))
        .json(&json!({"variables": {}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Creating a template with the same name overwrites the previous one.
#[tokio::test]
async fn e2e_template_create_same_name_overwrites() {
    let base = spawn_server().await;
    let client = test_client();

    // Create v1
    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "overwrite-tpl",
            "body": "version 1"
        }))
        .send()
        .await
        .unwrap();

    // Create v2 with same name
    let resp = client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "overwrite-tpl",
            "body": "version 2 with {{var}}"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Get should show v2
    let resp = client
        .get(format!("{base}/api/v1/templates/overwrite-tpl"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert!(body["body"].as_str().unwrap().contains("version 2"));
    assert_eq!(body["variables"].as_array().unwrap().len(), 1);

    // List should still show 1 total
    let resp = client
        .get(format!("{base}/api/v1/templates"))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"].as_u64().unwrap(), 1);
}

/// Update body only; title and defaults remain from original.
#[tokio::test]
async fn e2e_template_update_body_only_preserves_title() {
    let base = spawn_server().await;
    let client = test_client();

    client
        .post(format!("{base}/api/v1/templates"))
        .json(&json!({
            "name": "title-keep",
            "body": "original body {{x}}",
            "title": "Original Title"
        }))
        .send()
        .await
        .unwrap();

    let resp = client
        .put(format!("{base}/api/v1/templates/title-keep"))
        .json(&json!({
            "body": "updated body {{y}}"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["body"], "updated body {{y}}");
    assert_eq!(body["title"], "Original Title");
}
