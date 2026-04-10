//! Provider send tests: webhook category.

use super::provider_test_utils::*;

mod webhook_tests {

    use super::*;

    use noti_providers::webhook::WebhookProvider;



    #[tokio::test]

    async fn test_send_success_json() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(header("Content-Type", "application/json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new().set("url", mock_server.uri());

        let message = Message::text("Test webhook");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "webhook");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new().set("url", mock_server.uri());

        let message = Message::text("Body").with_title("Title");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_put_method() {

        let mock_server = MockServer::start().await;



        Mock::given(method("PUT"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new()

            .set("url", mock_server.uri())

            .set("method", "PUT");

        let message = Message::text("PUT test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_patch_method() {

        let mock_server = MockServer::start().await;



        Mock::given(method("PATCH"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new()

            .set("url", mock_server.uri())

            .set("method", "PATCH");

        let message = Message::text("PATCH test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_unsupported_method() {

        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new()

            .set("url", "http://localhost:1234")

            .set("method", "DELETE");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

    }



    #[tokio::test]

    async fn test_send_failure_status() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new().set("url", mock_server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(500));

    }



    #[tokio::test]

    async fn test_send_with_custom_headers() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(header("Authorization", "Bearer mytoken"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new()

            .set("url", mock_server.uri())

            .set("headers", "Authorization:Bearer mytoken");

        let message = Message::text("Authenticated request");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_body_template() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new().set("url", mock_server.uri()).set(

            "body_template",

            r#"{"text":"{message}","subject":"{title}"}"#,

        );

        let message = Message::text("Hello").with_title("Greeting");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_invalid_body_template() {

        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new()

            .set("url", "http://localhost:1234")

            .set("body_template", "not valid json {{{}");

        let message = Message::text("Hello");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_url() {

        let provider = WebhookProvider::new(client());

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = WebhookProvider::new(client());

        assert_eq!(provider.name(), "webhook");

        assert_eq!(provider.url_scheme(), "webhook");

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "url" && p.required));

        assert!(params.iter().any(|p| p.name == "method" && !p.required));

        assert!(params.iter().any(|p| p.name == "headers" && !p.required));

    }

}


mod webhook_send_tests {

    use super::*;

    use noti_providers::webhook::WebhookProvider;

    use serde_json::json;



    fn make_config(url: &str) -> ProviderConfig {

        ProviderConfig::new().set("url", url)

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = WebhookProvider::new(client());

        let config = make_config("https://example.com/webhook");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_url() {

        let provider = WebhookProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = WebhookProvider::new(client());

        assert_eq!(provider.name(), "webhook");

        assert_eq!(provider.url_scheme(), "webhook");

        assert!(!provider.description().is_empty());

        assert!(provider.example_url().starts_with("webhook://"));

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "url" && p.required));

        assert!(params.iter().any(|p| p.name == "method" && !p.required));

        assert!(params.iter().any(|p| p.name == "content_type" && !p.required));

        assert!(params.iter().any(|p| p.name == "headers" && !p.required));

        assert!(params.iter().any(|p| p.name == "body_template" && !p.required));

        assert!(params.iter().any(|p| p.name == "auth_type" && !p.required));

        assert!(params.iter().any(|p| p.name == "auth_token" && !p.required));

        assert!(params.iter().any(|p| p.name == "retry" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri());

        let message = Message::text("Hello webhook");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "webhook");

        assert_eq!(response.status_code, Some(200));

        assert!(response.raw_response.is_some());

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri());

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_body_template() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"received": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri()).set(

            "body_template",

            r#"{"content": "{message}", "heading": "{title}"}"#,

        );

        let message = Message::text("hello").with_title("Hi");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_put_method() {

        let mock_server = MockServer::start().await;



        Mock::given(method("PUT"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri()).set("method", "PUT");

        let message = Message::text("PUT body");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_patch_method() {

        let mock_server = MockServer::start().await;



        Mock::given(method("PATCH"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri()).set("method", "PATCH");

        let message = Message::text("PATCH body");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_bearer_auth() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(header("Authorization", "Bearer my-token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri())

            .set("auth_type", "bearer")

            .set("auth_token", "my-token");

        let message = Message::text("Authenticated request");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_api_key_auth() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(header("X-API-Key", "key-123"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri())

            .set("auth_type", "api_key")

            .set("auth_token", "key-123");

        let message = Message::text("API key request");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_custom_headers() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(header("X-Custom", "value"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri()).set("headers", "X-Custom:value");

        let message = Message::text("Custom headers");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_400() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(400).set_body_json(json!({"error": "bad request"})),

            )

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(400));

        assert!(response.raw_response.is_some());

    }



    #[tokio::test]

    async fn test_send_failure_500() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(500));

    }



    #[tokio::test]

    async fn test_send_unsupported_method() {

        let provider = WebhookProvider::new(client());

        let config = make_config("https://example.com/webhook").set("method", "DELETE");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        assert!(result.unwrap_err().to_string().contains("unsupported HTTP method"));

    }



    #[tokio::test]

    async fn test_send_retry_on_failure() {

        let mock_server = MockServer::start().await;



        // First request fails, second succeeds

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500))

            .up_to_n_times(1)

            .mount(&mock_server)

            .await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri()).set("retry", "2");

        let message = Message::text("Retry test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_custom_content_type() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(header("Content-Type", "text/plain"))

            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))

            .mount(&mock_server)

            .await;



        let provider = WebhookProvider::new(client());

        let config = make_config(&mock_server.uri()).set("content_type", "text/plain");

        let message = Message::text("Plain text body");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }

}


mod form_webhook_send_tests {

    use super::*;

    use noti_providers::form_webhook::FormWebhookProvider;



    fn provider() -> FormWebhookProvider {

        FormWebhookProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("url", "https://example.com/api/notify")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("url", "https://example.com/api/notify");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_url() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "form");

        assert_eq!(provider.url_scheme(), "form");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from FormWebhook");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "form");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_custom_type() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("type", "warning");

        let message = Message::text("Warning message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_put_method() {

        let server = MockServer::start().await;

        Mock::given(method("PUT"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("method", "PUT");

        let message = Message::text("PUT request");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Custom base URL");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new().set("url", "https://example.com/api/notify");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("url", "https://example.com/api/notify")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod json_webhook_send_tests {

    use super::*;

    use noti_providers::json_webhook::JsonWebhookProvider;



    fn provider() -> JsonWebhookProvider {

        JsonWebhookProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("url", "https://example.com/api/notify")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("url", "https://example.com/api/notify");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_url() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "json");

        assert_eq!(provider.url_scheme(), "json");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from JSON webhook");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "json");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_type() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("url", "https://example.com/api/notify")

            .set("type", "warning")

            .set("base_url", &server.uri());

        let message = Message::text("Warning message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_put_method() {

        let server = MockServer::start().await;

        Mock::given(method("PUT"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("url", "https://example.com/api/notify")

            .set("method", "PUT")

            .set("base_url", &server.uri());

        let message = Message::text("PUT request");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Custom base URL");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new().set("url", "https://example.com/api/notify");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("url", "https://example.com/api/notify")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}

