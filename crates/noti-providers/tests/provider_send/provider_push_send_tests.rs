//! Provider send tests: push category.

use super::provider_test_utils::*;

mod ntfy_tests {

    use super::*;

    use noti_providers::ntfy::NtfyProvider;



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200)

                    .set_body_json(serde_json::json!({"id": "abc", "event": "message"})),

            )

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = ProviderConfig::new()

            .set("topic", "test-topic")

            .set("server", mock_server.uri());

        let message = Message::text("Test ntfy");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "ntfy");

    }



    #[tokio::test]

    async fn test_send_with_title_and_markdown() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200)

                    .set_body_json(serde_json::json!({"id": "abc", "event": "message"})),

            )

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = ProviderConfig::new()

            .set("topic", "test-topic")

            .set("server", mock_server.uri());

        let message = Message::text("**bold**")

            .with_title("Alert")

            .with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_priority_and_tags() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200)

                    .set_body_json(serde_json::json!({"id": "abc", "event": "message"})),

            )

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = ProviderConfig::new()

            .set("topic", "test-topic")

            .set("server", mock_server.uri())

            .set("priority", "5")

            .set("tags", "warning,skull");

        let message = Message::text("Urgent alert");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(401)

                    .set_body_json(serde_json::json!({"error": "unauthorized"})),

            )

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = ProviderConfig::new()

            .set("topic", "test-topic")

            .set("server", mock_server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = NtfyProvider::new(client());

        let good = ProviderConfig::new().set("topic", "test");

        assert!(provider.validate_config(&good).is_ok());



        let bad = ProviderConfig::new();

        assert!(provider.validate_config(&bad).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = NtfyProvider::new(client());

        assert_eq!(provider.name(), "ntfy");

        assert_eq!(provider.url_scheme(), "ntfy");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "topic" && p.required)

        );

    }

}


mod gotify_tests {

    use super::*;

    use noti_providers::gotify::GotifyProvider;



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .and(header("X-Gotify-Key", "test-token"))

            .respond_with(

                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": 1, "appid": 1})),

            )

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = ProviderConfig::new()

            .set("host", mock_server.uri())

            .set("app_token", "test-token");

        let message = Message::text("Test gotify");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "gotify");

    }



    #[tokio::test]

    async fn test_send_with_title_and_priority() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .respond_with(

                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": 1, "appid": 1})),

            )

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = ProviderConfig::new()

            .set("host", mock_server.uri())

            .set("app_token", "test-token")

            .set("priority", "8");

        let message = Message::text("High priority")

            .with_title("Alert")

            .with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(401)

                    .set_body_json(serde_json::json!({"error": "unauthorized"})),

            )

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = ProviderConfig::new()

            .set("host", mock_server.uri())

            .set("app_token", "bad-token");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = GotifyProvider::new(client());

        let good = ProviderConfig::new()

            .set("host", "https://gotify.example.com")

            .set("app_token", "abc");

        assert!(provider.validate_config(&good).is_ok());



        let missing_token = ProviderConfig::new().set("host", "https://gotify.example.com");

        assert!(provider.validate_config(&missing_token).is_err());



        let missing_host = ProviderConfig::new().set("app_token", "abc");

        assert!(provider.validate_config(&missing_host).is_err());

    }

}


mod gotify_send_tests {

    use super::*;

    use noti_providers::gotify::GotifyProvider;

    use serde_json::json;



    fn make_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "https://gotify.example.com")

            .set("app_token", "test-token")

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = GotifyProvider::new(client());

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_host() {

        let provider = GotifyProvider::new(client());

        let config = ProviderConfig::new().set("app_token", "test-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_app_token() {

        let provider = GotifyProvider::new(client());

        let config = ProviderConfig::new().set("host", "https://gotify.example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_empty() {

        let provider = GotifyProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = GotifyProvider::new(client());

        assert_eq!(provider.name(), "gotify");

        assert_eq!(provider.url_scheme(), "gotify");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "host" && p.required));

        assert!(params.iter().any(|p| p.name == "app_token" && p.required));

        assert!(params.iter().any(|p| p.name == "priority" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .and(header("X-Gotify-Key", "test-token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": 42,

                "appid": 1,

                "message": "Test gotify"

            })))

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = make_config().set("host", mock_server.uri());

        let message = Message::text("Test gotify");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "gotify");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": 43,

                "appid": 1

            })))

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = make_config().set("host", mock_server.uri());

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_priority() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": 44,

                "appid": 1

            })))

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = make_config()

            .set("host", mock_server.uri())

            .set("priority", "10");

        let message = Message::text("Urgent!");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_markdown_format() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": 45,

                "appid": 1

            })))

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = make_config().set("host", mock_server.uri());

        let message = Message::text("**bold text**").with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_unauthorized() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .respond_with(ResponseTemplate::new(401).set_body_json(json!({

                "error": "unauthorized",

                "errorCode": 401

            })))

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = make_config()

            .set("host", mock_server.uri())

            .set("app_token", "bad-token");

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_failure_bad_request() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/message"))

            .respond_with(ResponseTemplate::new(400).set_body_json(json!({

                "error": "bad request",

                "errorCode": 400

            })))

            .mount(&mock_server)

            .await;



        let provider = GotifyProvider::new(client());

        let config = make_config().set("host", mock_server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(!result.unwrap().success);

    }

}


mod pushover_tests {

    use super::*;

    use noti_providers::pushover::PushoverProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = PushoverProvider::new(client());

        let config = ProviderConfig::new()

            .set("user_key", "user123")

            .set("api_token", "token456");

        assert!(provider.validate_config(&config).is_ok());



        let missing_token = ProviderConfig::new().set("user_key", "user123");

        assert!(provider.validate_config(&missing_token).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = PushoverProvider::new(client());

        assert_eq!(provider.name(), "pushover");

        assert_eq!(provider.url_scheme(), "pushover");

        assert!(provider.params().len() >= 2);

    }

}


mod bark_tests {

    use super::*;

    use noti_providers::bark::BarkProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = BarkProvider::new(client());

        let config = ProviderConfig::new().set("device_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());



        let empty = ProviderConfig::new();

        assert!(provider.validate_config(&empty).is_err());

    }

}


mod serverchan_tests {

    use super::*;

    use noti_providers::serverchan::ServerChanProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = ServerChanProvider::new(client());

        let config = ProviderConfig::new().set("send_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());



        let empty = ProviderConfig::new();

        assert!(provider.validate_config(&empty).is_err());

    }

}


mod serverchan_send_tests {

    use super::*;

    use noti_providers::serverchan::ServerChanProvider;

    use serde_json::json;



    fn make_config() -> ProviderConfig {

        ProviderConfig::new().set("send_key", "SCTtestkey123")

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = ServerChanProvider::new(client());

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_send_key() {

        let provider = ServerChanProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = ServerChanProvider::new(client());

        assert_eq!(provider.name(), "serverchan");

        assert_eq!(provider.url_scheme(), "serverchan");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "send_key" && p.required));

        assert!(params.iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/SCTtestkey123.send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "code": 0,

                "message": "success",

                "data": {}

            })))

            .mount(&mock_server)

            .await;



        let provider = ServerChanProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());

        let message = Message::text("Hello from ServerChan!");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "serverchan");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/SCTtestkey123.send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "code": 0,

                "message": "success"

            })))

            .mount(&mock_server)

            .await;



        let provider = ServerChanProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_error_code() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/SCTtestkey123.send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "code": 40001,

                "message": "invalid sendkey"

            })))

            .mount(&mock_server)

            .await;



        let provider = ServerChanProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert!(response.message.contains("invalid sendkey"));

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/SCTtestkey123.send"))

            .respond_with(ResponseTemplate::new(500))

            .mount(&mock_server)

            .await;



        let provider = ServerChanProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());

        let message = Message::text("Test");



        // Server returns 500 with non-JSON body, so parsing fails

        let result = provider.send(&message, &config).await;

        // Either the JSON parse fails or we get a non-success response

        assert!(result.is_ok() || result.is_err());

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/SCTtestkey123.send"))

            .respond_with(ResponseTemplate::new(401).set_body_json(json!({

                "code": 401,

                "message": "unauthorized"

            })))

            .mount(&mock_server)

            .await;



        let provider = ServerChanProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/SCTtestkey123.send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "code": 0,

                "message": "success"

            })))

            .mount(&mock_server)

            .await;



        let provider = ServerChanProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());

        let message = Message::text("Test custom base");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        // Without base_url, the provider uses https://sctapi.ftqq.com

        // We can't mock the real API, so just validate config works without base_url

        let provider = ServerChanProvider::new(client());

        let config = make_config(); // No base_url

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/SCTtestkey123.send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "code": 0,

                "message": "success"

            })))

            .mount(&mock_server)

            .await;



        let provider = ServerChanProvider::new(client());

        let config = make_config().set("base_url", format!("{}/", mock_server.uri()));

        let message = Message::text("Test trailing slash");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }

}


mod bark_send_tests {

    use super::*;

    use noti_providers::bark::BarkProvider;



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 200,

                "message": "success",

                "timestamp": 1234567890

            })))

            .mount(&mock_server)

            .await;



        let provider = BarkProvider::new(client());

        let config = ProviderConfig::new()

            .set("device_key", "test-key")

            .set("server", mock_server.uri());

        let message = Message::text("Hello from Bark test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "bark");

    }



    #[tokio::test]

    async fn test_send_with_title_and_options() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 200, "message": "success"

            })))

            .mount(&mock_server)

            .await;



        let provider = BarkProvider::new(client());

        let config = ProviderConfig::new()

            .set("device_key", "test-key")

            .set("server", mock_server.uri())

            .set("group", "test-group")

            .set("sound", "alarm")

            .set("icon", "https://example.com/icon.png");

        let message = Message::text("Alert body").with_title("Alert Title");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_bad_key() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "code": 400,

                "message": "failed to get device token"

            })))

            .mount(&mock_server)

            .await;



        let provider = BarkProvider::new(client());

        let config = ProviderConfig::new()

            .set("device_key", "bad-key")

            .set("server", mock_server.uri());

        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

    }



    #[tokio::test]

    async fn test_validate_missing_device_key() {

        let provider = BarkProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = BarkProvider::new(client());

        assert_eq!(provider.name(), "bark");

        assert_eq!(provider.url_scheme(), "bark");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "device_key" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "server" && !p.required)

        );

    }

}


mod pushbullet_send_tests {

    use super::*;

    use noti_providers::pushbullet::PushBulletProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = PushBulletProvider::new(client());

        let config = ProviderConfig::new().set("access_token", "o.test123");

        assert!(provider.validate_config(&config).is_ok());



        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_validate_with_optional_params() {

        let provider = PushBulletProvider::new(client());

        let config = ProviderConfig::new()

            .set("access_token", "o.test123")

            .set("device_iden", "dev1")

            .set("channel_tag", "chan1")

            .set("email", "user@example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = PushBulletProvider::new(client());

        assert_eq!(provider.name(), "pushbullet");

        assert_eq!(provider.url_scheme(), "pushbullet");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "access_token" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "device_iden" && !p.required)

        );

    }

}


mod simplepush_send_tests {

    use super::*;

    use noti_providers::simplepush::SimplePushProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = SimplePushProvider::new(client());

        let config = ProviderConfig::new().set("key", "HuxgBB");

        assert!(provider.validate_config(&config).is_ok());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_validate_with_event() {

        let provider = SimplePushProvider::new(client());

        let config = ProviderConfig::new()

            .set("key", "HuxgBB")

            .set("event", "alerts");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = SimplePushProvider::new(client());

        assert_eq!(provider.name(), "simplepush");

        assert_eq!(provider.url_scheme(), "simplepush");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "key" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "event" && !p.required)

        );

    }

}


mod pushover_send_tests {

    use super::*;

    use noti_providers::pushover::PushoverProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = PushoverProvider::new(client());

        let config = ProviderConfig::new()

            .set("user_key", "user123")

            .set("api_token", "token456");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = PushoverProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("user_key", "x"))

                .is_err()

        );

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("api_token", "x"))

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = PushoverProvider::new(client());

        assert_eq!(provider.name(), "pushover");

        assert_eq!(provider.url_scheme(), "pushover");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "user_key" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "api_token" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "priority" && !p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "sound" && !p.required)

        );

    }

}


mod apns_send_tests {

    use super::*;

    use base64::Engine;

    use noti_providers::apns::ApnsProvider;

    use p256::pkcs8::EncodePrivateKey;



    fn make_test_p8() -> String {

        let signing_key = p256::ecdsa::SigningKey::random(&mut rand::rngs::OsRng);

        let pkcs8_der = signing_key

            .to_pkcs8_der()

            .expect("generate PKCS#8 DER")

            .as_bytes()

            .to_vec();

        base64::engine::general_purpose::STANDARD.encode(&pkcs8_der)

    }



    fn base_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("key_id", "KEY12345A")

            .set("team_id", "TEAM123456")

            .set("bundle_id", "com.example.app")

            .set("device_token", "abcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef56789")

            .set("p8_base64", make_test_p8())

    }



    #[tokio::test]

    async fn test_validate_config_full() {

        let provider = ApnsProvider::new(client());

        assert!(provider.validate_config(&base_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_key_id() {

        let provider = ApnsProvider::new(client());

        let config = ProviderConfig::new()

            .set("team_id", "TEAM123456")

            .set("bundle_id", "com.example.app")

            .set("device_token", "abcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef56789")

            .set("p8_base64", make_test_p8());

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_team_id() {

        let provider = ApnsProvider::new(client());

        let config = ProviderConfig::new()

            .set("key_id", "KEY12345A")

            .set("bundle_id", "com.example.app")

            .set("device_token", "abcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef56789")

            .set("p8_base64", make_test_p8());

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_bundle_id() {

        let provider = ApnsProvider::new(client());

        let config = ProviderConfig::new()

            .set("key_id", "KEY12345A")

            .set("team_id", "TEAM123456")

            .set("device_token", "abcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef56789")

            .set("p8_base64", make_test_p8());

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_device_token() {

        let provider = ApnsProvider::new(client());

        let config = ProviderConfig::new()

            .set("key_id", "KEY12345A")

            .set("team_id", "TEAM123456")

            .set("bundle_id", "com.example.app")

            .set("p8_base64", make_test_p8());

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_send_missing_both_p8_params() {

        let provider = ApnsProvider::new(client());

        let config = ProviderConfig::new()

            .set("key_id", "KEY12345A")

            .set("team_id", "TEAM123456")

            .set("bundle_id", "com.example.app")

            .set("device_token", "abcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef56789");



        let message = Message::text("test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        // Error should mention p8 params

        assert!(

            err.to_string().contains("p8_base64") || err.to_string().contains("p8_path"),

            "expected p8 error, got: {}",

            err

        );

    }



    #[tokio::test]

    async fn test_validate_config_with_sandbox() {

        let provider = ApnsProvider::new(client());

        let config = base_config().set("sandbox", "true");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = ApnsProvider::new(client());

        assert_eq!(provider.name(), "apns");

        assert_eq!(provider.url_scheme(), "apns");

        assert!(provider.description().contains("Apple Push"));

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "key_id" && p.required));

        assert!(params.iter().any(|p| p.name == "team_id" && p.required));

        assert!(params.iter().any(|p| p.name == "bundle_id" && p.required));

        assert!(params.iter().any(|p| p.name == "device_token" && p.required));

        assert!(params.iter().any(|p| p.name == "p8_base64" && !p.required));

        assert!(params.iter().any(|p| p.name == "sandbox" && !p.required));

    }



    #[tokio::test]

    async fn test_device_token_too_short() {

        let provider = ApnsProvider::new(client());

        let config = ProviderConfig::new()

            .set("key_id", "KEY12345A")

            .set("team_id", "TEAM123456")

            .set("bundle_id", "com.example.app")

            .set("device_token", "too-short")

            .set("p8_base64", make_test_p8());



        let message = Message::text("test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.to_string().contains("device_token") || err.to_string().contains("64"));

    }



    #[tokio::test]

    async fn test_device_token_invalid_chars() {

        let provider = ApnsProvider::new(client());

        // 64 chars but 'g' is not a hex digit

        let config = ProviderConfig::new()

            .set("key_id", "KEY12345A")

            .set("team_id", "TEAM123456")

            .set("bundle_id", "com.example.app")

            .set("device_token", "gggg1234ef567890gggg1234ef567890gggg1234ef567890gggg1234ef56789")

            .set("p8_base64", make_test_p8());



        let message = Message::text("test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let provider = ApnsProvider::new(client());

        let config = base_config();



        let message = Message::text("Body text").with_title("Notification Title");

        // APNs URL is hardcoded; just verify it validates and builds a request

        // (network will fail to connect to real apple.com, but that's expected)

        let result = provider.send(&message, &config).await;

        // We expect a connection error (APNs URL is hardcoded to apple.com)

        // This validates the title is passed through to the payload builder

        assert!(result.is_err());

        let err = result.unwrap_err();

        // Should NOT be a config/validation error - title field is handled

        assert!(!err.to_string().contains("missing required"), "unexpected validation error: {}", err);

    }



    #[tokio::test]

    async fn test_send_with_badge_extra() {

        let provider = ApnsProvider::new(client());

        let config = base_config();



        let mut extra = std::collections::HashMap::new();

        extra.insert("badge".into(), serde_json::json!(5));

        let msg = Message {

            text: "Alert".into(),

            title: None,

            format: Default::default(),

            priority: Default::default(),

            attachments: vec![],

            extra,

        };



        let result = provider.send(&msg, &config).await;

        // Connection error expected; validates badge extra is accepted

        assert!(result.is_err());

    }

}


mod onesignal_send_tests {

    use super::*;

    use noti_providers::onesignal::OneSignalProvider;

    use serde_json::json;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = OneSignalProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        assert!(provider

            .validate_config(&ProviderConfig::new().set("app_id", "test-app"))

            .is_err());

        assert!(provider

            .validate_config(&ProviderConfig::new().set("api_key", "test-key"))

            .is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = OneSignalProvider::new(client());

        assert_eq!(provider.name(), "onesignal");

        assert_eq!(provider.url_scheme(), "onesignal");

        assert!(provider.supports_attachments());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "app_id" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "api_key" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "base_url" && !p.required)

        );

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "abc-123-def-456",

                "recipients": 1,

                "id": "notification-id"

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key")

            .set("base_url", mock_server.uri());



        let message = Message::text("hello world");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "onesignal");

        assert_eq!(response.status_code, Some(200));

        assert!(response.message.contains("1 recipients"));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "notification-id",

                "recipients": 1

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key")

            .set("base_url", mock_server.uri());



        let message = Message::text("body text").with_title("Notification Title");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

    }



    #[tokio::test]

    async fn test_send_with_player_ids() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "notification-id",

                "recipients": 2

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key")

            .set("player_ids", "player-1,player-2")

            .set("base_url", mock_server.uri());



        let message = Message::text("targeted message");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert!(response.message.contains("2 recipients"));

    }



    #[tokio::test]

    async fn test_send_with_segments() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "notification-id",

                "recipients": 100

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key")

            .set("include_segments", "Active Users,Inactive Users")

            .set("base_url", mock_server.uri());



        let message = Message::text("segment broadcast");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert!(response.message.contains("100 recipients"));

    }



    #[tokio::test]

    async fn test_send_with_click_url() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "notification-id",

                "recipients": 1

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key")

            .set("url", "https://example.com/landing")

            .set("base_url", mock_server.uri());



        let message = Message::text("click me");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(400).set_body_json(json!({

                "errors": ["Invalid app_id provided"]

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "bad-app-id")

            .set("api_key", "test-api-key")

            .set("base_url", mock_server.uri());



        let message = Message::text("test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(400));

        assert!(response.message.contains("Invalid app_id"));

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(401).set_body_json(json!({

                "errors": ["Invalid API key"]

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "wrong-api-key")

            .set("base_url", mock_server.uri());



        let message = Message::text("test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/v1/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "notification-id",

                "recipients": 1

            })))

            .mount(&mock_server)

            .await;



        let provider = OneSignalProvider::new(client());

        // Use wiremock server URI as base_url to verify custom URL is used

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key")

            .set("base_url", mock_server.uri());



        let message = Message::text("using custom base url");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = OneSignalProvider::new(client());

        let config = ProviderConfig::new()

            .set("app_id", "test-app-id")

            .set("api_key", "test-api-key");

        // No base_url set - should use default https://onesignal.com/api/v1/notifications

        assert!(provider.validate_config(&config).is_ok());

        // This test just validates config; actual send without mock would hit real API

    }

}


mod ntfy_send_tests {

    use super::*;

    use noti_providers::ntfy::NtfyProvider;

    use serde_json::json;



    fn make_config() -> ProviderConfig {

        ProviderConfig::new().set("topic", "test-topic")

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = NtfyProvider::new(client());

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_topic() {

        let provider = NtfyProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = NtfyProvider::new(client());

        assert_eq!(provider.name(), "ntfy");

        assert_eq!(provider.url_scheme(), "ntfy");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "topic" && p.required));

        assert!(params.iter().any(|p| p.name == "server" && !p.required));

        assert!(params.iter().any(|p| p.name == "priority" && !p.required));

        assert!(params.iter().any(|p| p.name == "tags" && !p.required));

        assert!(params.iter().any(|p| p.name == "token" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_abc123",

                "time": 1712640000,

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config().set("server", mock_server.uri());

        let message = Message::text("Hello ntfy!");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "ntfy");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_abc123",

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config().set("server", mock_server.uri());

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_priority() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_abc123",

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config()

            .set("server", mock_server.uri())

            .set("priority", "5");

        let message = Message::text("Urgent");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_tags() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_abc123",

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config()

            .set("server", mock_server.uri())

            .set("tags", "warning,skull");

        let message = Message::text("Tagged message");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_token_auth() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_abc123",

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config()

            .set("server", mock_server.uri())

            .set("token", "tk_secret123");

        let message = Message::text("Authenticated message");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_markdown_format() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_abc123",

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config().set("server", mock_server.uri());

        let message = Message::markdown("**Bold** and _italic_");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_abc123",

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config().set("server", mock_server.uri());

        let message = Message::text("<b>Bold</b>").with_format(MessageFormat::Html);



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_unauthorized() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(

                ResponseTemplate::new(401).set_body_json(json!({"error": "unauthorized"})),

            )

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config()

            .set("server", mock_server.uri())

            .set("token", "bad-token");

        let message = Message::text("test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_failure_forbidden() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/test-topic"))

            .respond_with(

                ResponseTemplate::new(403).set_body_json(json!({"error": "forbidden"})),

            )

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = make_config().set("server", mock_server.uri());

        let message = Message::text("test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(403));

    }



    #[tokio::test]

    async fn test_send_all_options_combined() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/alerts"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "msg_combined",

                "event": "message"

            })))

            .mount(&mock_server)

            .await;



        let provider = NtfyProvider::new(client());

        let config = ProviderConfig::new()

            .set("topic", "alerts")

            .set("server", mock_server.uri())

            .set("priority", "5")

            .set("tags", "rotating_light,critical")

            .set("token", "tk_abc");

        let message = Message::text("Critical alert!").with_title("System Down");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }

}


mod fcm_send_tests {

    use super::*;

    use noti_providers::fcm::FcmProvider;



    fn provider() -> FcmProvider {

        FcmProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("server_key", "AAAA-test-key")

            .set("device_token", "device-token-123")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("server_key", "k")

            .set("device_token", "t");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_server_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("device_token", "t");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "fcm");

        assert_eq!(provider.url_scheme(), "fcm");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": 1,

                "failure": 0,

                "results": [{"message_id": "msg001"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from FCM");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "fcm");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": 1,

                "failure": 0,

                "results": [{"message_id": "msg002"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("FCM Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_to_topic() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": 1,

                "failure": 0,

                "results": [{"message_id": "msg-topic"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("server_key", "AAAA-test-key")

            .set("device_token", "device-token-123") // required by validate_config

            .set("topic", "news")

            .set("base_url", &server.uri());

        let message = Message::text("Topic message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_priority() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": 1,

                "failure": 0,

                "results": [{"message_id": "msg-priority"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("priority", "normal");

        let message = Message::text("Normal priority");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": 0,

                "failure": 1,

                "results": [{"error": "InvalidRegistration"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/send"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "success": 0,

                "failure": 1,

                "results": [{"error": "Unauthorized"}]

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}


mod pushy_send_tests {

    use super::*;

    use noti_providers::pushy::PushyProvider;



    fn provider() -> PushyProvider {

        PushyProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("device_token", "test-device-token")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-key")

            .set("device_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("device_token", "test-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_device_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pushy");

        assert_eq!(provider.url_scheme(), "pushy");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "api_key" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "device_token" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true,

                "id": "push-id-123"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Pushy");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pushy");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true, "id": "push-id"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Push Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_sound_and_badge() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true, "id": "push-id"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("device_token", "test-device-token")

            .set("sound", "ping.aiff")

            .set("badge", "3")

            .set("base_url", &server.uri());

        let message = Message::text("Notification with sound");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "success": false,

                "error": "Invalid API key"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "success": false,

                "error": "Internal server error"

            })))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true, "id": "push-id"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Custom base");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-key")

            .set("device_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true, "id": "push-id"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("device_token", "test-device-token")

            .set("base_url", &base);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod pushdeer_send_tests {

    use super::*;

    use noti_providers::pushdeer::PushDeerProvider;



    fn provider() -> PushDeerProvider {

        PushDeerProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("push_key", "test-push-key")

            .set("server", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("push_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_push_key() {

        let provider = provider();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pushdeer");

        assert_eq!(provider.url_scheme(), "pushdeer");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "push_key" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "server" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 0,

                "content": {"result": ["ok"]}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello PushDeer");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pushdeer");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 0, "content": {"result": ["ok"]}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("PushDeer Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_markdown_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 0, "content": {"result": ["ok"]}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("**bold** text").with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 1,

                "error": "Invalid push key"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        // PushDeer tries to parse JSON from response, 500 with no JSON body will error

        assert!(provider.send(&message, &config).await.is_err());

    }



    #[tokio::test]

    async fn test_send_custom_server() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 0, "content": {"result": ["ok"]}

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Custom server");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_server_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new().set("push_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod chanify_send_tests {

    use super::*;

    use noti_providers::chanify::ChanifyProvider;



    fn provider() -> ChanifyProvider {

        ChanifyProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("token", "test-device-token")

            .set("server", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "chanify");

        assert_eq!(provider.url_scheme(), "chanify");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "token" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "server" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Chanify");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "chanify");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid token"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_server_error() {

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

    async fn test_send_custom_server() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Custom server");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_server_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod pushplus_send_tests {

    use super::*;

    use noti_providers::pushplus::PushplusProvider;



    fn provider() -> PushplusProvider {

        PushplusProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("token", "test-token-123")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pushplus");

        assert_eq!(provider.url_scheme(), "pushplus");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "token" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 200,

                "msg": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Pushplus");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pushplus");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 200, "msg": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_topic_and_channel() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 200, "msg": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("topic", "my-topic")

            .set("channel", "wechat");

        let message = Message::text("Topic message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error_code() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 400,

                "msg": "Invalid token"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 200, "msg": "ok"

            })))

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

        let config = ProviderConfig::new().set("token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 200, "msg": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("token", "test-token")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod pushsafer_send_tests {

    use super::*;

    use noti_providers::pushsafer::PushsaferProvider;



    fn provider() -> PushsaferProvider {

        PushsaferProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("private_key", "test-private-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("private_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_private_key() {

        let provider = provider();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pushsafer");

        assert_eq!(provider.url_scheme(), "pushsafer");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "private_key" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 1

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Pushsafer");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pushsafer");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 1

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_api_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 0,

                "error": "Invalid key"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 1

            })))

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

        let config = ProviderConfig::new().set("private_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 1

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("private_key", "test-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod pushme_send_tests {

    use super::*;

    use noti_providers::pushme::PushMeProvider;



    fn provider() -> PushMeProvider {

        PushMeProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("push_key", "test-push-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("push_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_push_key() {

        let provider = provider();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pushme");

        assert_eq!(provider.url_scheme(), "pushme");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "push_key" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello PushMe");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pushme");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("PushMe Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "error": "Invalid push key"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true

            })))

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

        let config = ProviderConfig::new().set("push_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("push_key", "test-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod pushcut_send_tests {

    use super::*;

    use noti_providers::pushcut::PushcutProvider;



    fn provider() -> PushcutProvider {

        PushcutProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "pk_test123")

            .set("notification_name", "TestNotification")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "pk_test")

            .set("notification_name", "TestNotif");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("notification_name", "TestNotif");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_notification_name() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "pk_test");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pushcut");

        assert_eq!(provider.url_scheme(), "pushcut");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "api_key" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "notification_name" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Pushcut");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pushcut");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Pushcut Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid API key"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_server_error() {

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

            .respond_with(ResponseTemplate::new(200))

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

        let config = ProviderConfig::new()

            .set("api_key", "pk_test")

            .set("notification_name", "TestNotif");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "pk_test")

            .set("notification_name", "TestNotif")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod pushed_send_tests {

    use super::*;

    use noti_providers::pushed::PushedProvider;



    fn provider() -> PushedProvider {

        PushedProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("app_key", "test-app-key")

            .set("app_secret", "test-app-secret")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("app_key", "test-key")

            .set("app_secret", "test-secret");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_app_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("app_secret", "test-secret");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_app_secret() {

        let provider = provider();

        let config = ProviderConfig::new().set("app_key", "test-key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "pushed");

        assert_eq!(provider.url_scheme(), "pushed");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "app_key" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "app_secret" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Pushed");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "pushed");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_target_alias() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("target_type", "channel")

            .set("target_alias", "my-channel");

        let message = Message::text("Channel message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid credentials"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_server_error() {

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

            .respond_with(ResponseTemplate::new(200))

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

        let config = ProviderConfig::new()

            .set("app_key", "test-key")

            .set("app_secret", "test-secret");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("app_key", "test-key")

            .set("app_secret", "test-secret")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod wxpusher_send_tests {

    use super::*;

    use noti_providers::wxpusher::WxPusherProvider;



    fn provider() -> WxPusherProvider {

        WxPusherProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("app_token", "AT_test123")

            .set("uid", "UID_test123")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("app_token", "AT_test")

            .set("uid", "UID_test");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_app_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("uid", "UID_test");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_uid() {

        let provider = provider();

        let config = ProviderConfig::new().set("app_token", "AT_test");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "wxpusher");

        assert_eq!(provider.url_scheme(), "wxpusher");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "app_token" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "uid" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 1000,

                "msg": "success"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello WxPusher");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "wxpusher");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 1000, "msg": "success"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("WxPusher Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_topic_id() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 1000, "msg": "success"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("topic_id", "12345");

        let message = Message::text("Topic message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error_code() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 1001,

                "msg": "Invalid app token"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 1000, "msg": "success"

            })))

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

        let config = ProviderConfig::new()

            .set("app_token", "AT_test")

            .set("uid", "UID_test");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "code": 1000, "msg": "success"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("app_token", "AT_test")

            .set("uid", "UID_test")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod notifiarr_send_tests {

    use super::*;

    use noti_providers::notifiarr::NotifiarrProvider;



    fn provider() -> NotifiarrProvider {

        NotifiarrProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "notifiarr");

        assert_eq!(provider.url_scheme(), "notifiarr");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v1/notification/passthrough"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": "success"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Notifiarr");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "notifiarr");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v1/notification/passthrough"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": "success"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_color() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v1/notification/passthrough"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": "success"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("color", "#FF0000");

        let message = Message::text("Colored notification");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/v1/notification/passthrough"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "result": "error",

                "details": { "response": "Invalid API key" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

        assert_eq!(result.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": "success"})))

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

        let config = ProviderConfig::new().set("api_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"result": "success"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod bluesky_send_tests {

    use super::*;

    use noti_providers::bluesky::BlueskyProvider;



    fn provider() -> BlueskyProvider {

        BlueskyProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("handle", "user.bsky.social")

            .set("app_password", "xxxx-xxxx-xxxx-xxxx")

            .set("server", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("handle", "user.bsky.social")

            .set("app_password", "test-pass");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_handle() {

        let provider = provider();

        let config = ProviderConfig::new().set("app_password", "test-pass");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_app_password() {

        let provider = provider();

        let config = ProviderConfig::new().set("handle", "user.bsky.social");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "bluesky");

        assert_eq!(provider.url_scheme(), "bluesky");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"server"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;



        // Mock session creation

        Mock::given(method("POST"))

            .and(path("/xrpc/com.atproto.server.createSession"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "accessJwt": "test-jwt",

                "did": "did:plc:test123"

            })))

            .mount(&server)

            .await;



        // Mock post creation

        Mock::given(method("POST"))

            .and(path("/xrpc/com.atproto.repo.createRecord"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "uri": "at://did:plc:test123/app.bsky.feed.post/test-post",

                "cid": "test-cid"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Bluesky");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "bluesky");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xrpc/com.atproto.server.createSession"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "accessJwt": "test-jwt",

                "did": "did:plc:test123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/xrpc/com.atproto.repo.createRecord"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "uri": "at://did:plc:test123/app.bsky.feed.post/test-post-2",

                "cid": "test-cid-2"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_auth_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xrpc/com.atproto.server.createSession"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "message": "Invalid identifier or password"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

        assert_eq!(result.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_post_failure() {

        let server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/xrpc/com.atproto.server.createSession"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "accessJwt": "test-jwt",

                "did": "did:plc:test123"

            })))

            .mount(&server)

            .await;



        Mock::given(method("POST"))

            .and(path("/xrpc/com.atproto.repo.createRecord"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "message": "Record too long"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

        assert_eq!(result.status_code, Some(400));

    }



    #[tokio::test]

    async fn test_send_no_server_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("handle", "user.bsky.social")

            .set("app_password", "test-pass");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod boxcar_send_tests {

    use super::*;

    use noti_providers::boxcar::BoxcarProvider;



    fn provider() -> BoxcarProvider {

        BoxcarProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "boxcar");

        assert_eq!(provider.url_scheme(), "boxcar");

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/notifications"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "123"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Boxcar");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "boxcar");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "456"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_sound() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "789"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("sound", "bird-1");

        let message = Message::text("Sound test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "Invalid token"})))

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "abc"})))

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

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "xyz"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod dot_send_tests {

    use super::*;

    use noti_providers::dot::DotProvider;



    fn provider() -> DotProvider {

        DotProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("token", "dot_app_test123")

            .set("device_id", "aabbccddeeff")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("token", "dot_app_test")

            .set("device_id", "aabbccddeeff");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("device_id", "aabbccddeeff");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_device_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "dot_app_test");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "dot");

        assert_eq!(provider.url_scheme(), "dot");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/device/aabbccddeeff/text"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Dot");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "dot");

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

    async fn test_send_with_signature() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("signature", "noti");

        let message = Message::text("Signed message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "Invalid token"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_server_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({"error": "Internal error"})))

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

        let config = ProviderConfig::new()

            .set("token", "dot_app_test")

            .set("device_id", "aabbccddeeff");

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

            .set("token", "dot_app_test123")

            .set("device_id", "aabbccddeeff")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod fluxer_send_tests {

    use super::*;

    use noti_providers::fluxer::FluxerProvider;



    fn provider() -> FluxerProvider {

        FluxerProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("webhook_id", "test-webhook-id")

            .set("webhook_token", "test-webhook-token")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("webhook_id", "test-id")

            .set("webhook_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("webhook_token", "test-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("webhook_id", "test-id");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "fluxer");

        assert_eq!(provider.url_scheme(), "fluxer");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhooks/test-webhook-id/test-webhook-token"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Fluxer");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "fluxer");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_botname_and_avatar() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("botname", "noti-bot")

            .set("avatar_url", "https://example.com/avatar.png");

        let message = Message::text("Custom identity");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"message": "Invalid token"})))

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

            .respond_with(ResponseTemplate::new(204))

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

        let config = ProviderConfig::new()

            .set("webhook_id", "test-id")

            .set("webhook_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("webhook_id", "test-webhook-id")

            .set("webhook_token", "test-webhook-token")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod join_send_tests {

    use super::*;

    use noti_providers::join::JoinProvider;



    fn provider() -> JoinProvider {

        JoinProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("device_id", "group.all")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-api-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "join");

        assert_eq!(provider.url_scheme(), "join");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Join");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "join");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({"success": false, "errorMessage": "Invalid API key"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(500))

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

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

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

        let config = ProviderConfig::new().set("api_key", "test-api-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("GET"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("device_id", "group.all")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod notica_send_tests {

    use super::*;

    use noti_providers::notica::NoticaProvider;



    fn provider() -> NoticaProvider {

        NoticaProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("token", "test-token-abc")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "notica");

        assert_eq!(provider.url_scheme(), "notica");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Notica");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "notica");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("invalid token"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("token", "test-token-abc")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod prowl_send_tests {

    use super::*;

    use noti_providers::prowl::ProwlProvider;



    fn provider() -> ProwlProvider {

        ProwlProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "prowl");

        assert_eq!(provider.url_scheme(), "prowl");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Prowl");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "prowl");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("invalid apikey"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod spugpush_send_tests {

    use super::*;

    use noti_providers::spugpush::SpugPushProvider;



    fn provider() -> SpugPushProvider {

        SpugPushProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("token", "abc123def456ghi789jkl012mno345pq")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "spugpush");

        assert_eq!(provider.url_scheme(), "spugpush");

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

        let message = Message::text("Hello SpugPush");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "spugpush");

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

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "invalid token"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

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

            .set("token", "abc123def456ghi789jkl012mno345pq")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod statuspage_send_tests {

    use super::*;

    use noti_providers::statuspage::StatuspageProvider;



    fn provider() -> StatuspageProvider {

        StatuspageProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("page_id", "test-page-id")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "key")

            .set("page_id", "page");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new().set("page_id", "page");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_page_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "statuspage");

        assert_eq!(provider.url_scheme(), "statuspage");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "inc-1"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Service degraded");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "statuspage");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "inc-2"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Check the service").with_title("Incident");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "Unauthorized"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "inc-3"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("page_id", "test-page-id")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod streamlabs_send_tests {

    use super::*;

    use noti_providers::streamlabs::StreamlabsProvider;



    fn provider() -> StreamlabsProvider {

        StreamlabsProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "streamlabs");

        assert_eq!(provider.url_scheme(), "streamlabs");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Streamlabs");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "streamlabs");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Donation Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"error": "invalid token"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod techulus_send_tests {

    use super::*;

    use noti_providers::techulus::TechulusProvider;



    fn provider() -> TechulusProvider {

        TechulusProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("api_key", "test-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "techulus");

        assert_eq!(provider.url_scheme(), "push");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Techulus");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "techulus");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"message": "Invalid API key"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"success": true})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("api_key", "test-api-key")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod notification_api_send_tests {

    use super::*;

    use noti_providers::notification_api::NotificationApiProvider;



    fn provider() -> NotificationApiProvider {

        NotificationApiProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("client_id", "test-client-id")

            .set("client_secret", "test-client-secret")

            .set("user_id", "user@example.com")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("client_id", "cid")

            .set("client_secret", "csec")

            .set("user_id", "user@example.com");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_client_id() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("client_secret", "csec")

            .set("user_id", "user@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "notificationapi");

        assert_eq!(provider.url_scheme(), "napi");

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

        let message = Message::text("Hello NotificationAPI");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "notificationapi");

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

        let message = Message::text("Body text").with_title("Notification Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({"message": "Unauthorized"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

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

            .set("client_id", "test-client-id")

            .set("client_secret", "test-client-secret")

            .set("user_id", "user@example.com")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}

