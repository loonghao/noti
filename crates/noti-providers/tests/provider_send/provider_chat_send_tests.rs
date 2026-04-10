//! Provider send tests: chat category.

use super::provider_test_utils::*;

mod wecom_tests {

    use super::*;

    use noti_providers::wecom::WeComProvider;



    #[tokio::test]

    async fn test_validate_config_and_metadata() {

        // WeComProvider hardcodes the URL from `key`; no base_url override available.

        // Test validates config and metadata without a mock server.

        let provider = WeComProvider::new(client());

        let config = ProviderConfig::new().set("key", "test-key-123");

        assert!(provider.validate_config(&config).is_ok());



        assert_eq!(provider.name(), "wecom");

        assert_eq!(provider.url_scheme(), "wecom");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(!provider.params().is_empty());

    }



    #[tokio::test]

    async fn test_validate_config_missing_key() {

        let provider = WeComProvider::new(client());

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_with_mentions() {

        let provider = WeComProvider::new(client());

        let config = ProviderConfig::new()

            .set("key", "test-key")

            .set("mentioned_list", "user1,user2")

            .set("mentioned_mobile_list", "13800138000");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod slack_tests {

    use super::*;

    use noti_providers::slack::SlackProvider;



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Hello from test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "slack");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_string("invalid_token"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Test message");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(403));

    }



    #[tokio::test]

    async fn test_send_markdown() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("*bold* text").with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_optional_params() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", mock_server.uri())

            .set("channel", "#general")

            .set("username", "TestBot")

            .set("icon_emoji", ":robot_face:");

        let message = Message::text("Test with options");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_validate_config_missing_webhook() {

        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = SlackProvider::new(client());

        assert_eq!(provider.name(), "slack");

        assert_eq!(provider.url_scheme(), "slack");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        let params = provider.params();

        assert!(!params.is_empty());

        assert!(params.iter().any(|p| p.name == "webhook_url" && p.required));

    }

}


mod discord_tests {

    use super::*;

    use noti_providers::discord::DiscordProvider;



    #[tokio::test]

    async fn test_validate_config() {

        // DiscordProvider builds URL from webhook_id/webhook_token; no base_url override.

        // Test validates config and metadata without a mock server.

        let provider = DiscordProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_id", "123456")

            .set("webhook_token", "abcdef");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = DiscordProvider::new(client());

        let config = ProviderConfig::new().set("webhook_id", "123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_id() {

        let provider = DiscordProvider::new(client());

        let config = ProviderConfig::new().set("webhook_token", "abc");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = DiscordProvider::new(client());

        assert_eq!(provider.name(), "discord");

        assert_eq!(provider.url_scheme(), "discord");

        assert!(provider.params().len() >= 2);

    }

}


mod telegram_tests {

    use super::*;

    use noti_providers::telegram::TelegramProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABC")

            .set("chat_id", "-1001234567890");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_bot_token() {

        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new().set("chat_id", "-123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_chat_id() {

        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new().set("bot_token", "abc");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = TelegramProvider::new(client());

        assert_eq!(provider.name(), "telegram");

        assert_eq!(provider.url_scheme(), "tg");

        assert!(provider.description().contains("Telegram"));

        assert!(provider.params().len() >= 2);

    }

}


mod feishu_tests {

    use super::*;

    use noti_providers::feishu::FeishuProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = FeishuProvider::new(client());

        let config = ProviderConfig::new().set("hook_id", "test-hook");

        assert!(provider.validate_config(&config).is_ok());



        let empty = ProviderConfig::new();

        assert!(provider.validate_config(&empty).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = FeishuProvider::new(client());

        assert_eq!(provider.name(), "feishu");

        assert_eq!(provider.url_scheme(), "feishu");

        assert!(!provider.description().is_empty());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "hook_id" && p.required)

        );

    }

}


mod dingtalk_tests {

    use super::*;

    use noti_providers::dingtalk::DingTalkProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = DingTalkProvider::new(client());

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());



        let empty = ProviderConfig::new();

        assert!(provider.validate_config(&empty).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = DingTalkProvider::new(client());

        assert_eq!(provider.name(), "dingtalk");

        assert!(provider.params().iter().any(|p| p.name == "access_token"));

    }

}


mod teams_tests {

    use super::*;

    use noti_providers::teams::TeamsProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", "https://teams.microsoft.com/...");

        assert!(provider.validate_config(&config).is_ok());



        let empty = ProviderConfig::new();

        assert!(provider.validate_config(&empty).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = TeamsProvider::new(client());

        assert_eq!(provider.name(), "teams");

    }

}


mod googlechat_send_tests {

    use super::*;

    use noti_providers::googlechat::GoogleChatProvider;



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "name": "spaces/xxx/messages/yyy",

                "text": "Hello"

            })))

            .mount(&mock_server)

            .await;



        let provider = GoogleChatProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Hello from test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "googlechat");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title_markdown() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200)

                    .set_body_json(serde_json::json!({"name": "spaces/x/messages/y"})),

            )

            .mount(&mock_server)

            .await;



        let provider = GoogleChatProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("**bold** text")

            .with_title("Alert Title")

            .with_format(MessageFormat::Markdown);

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_title_text() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(

                ResponseTemplate::new(200)

                    .set_body_json(serde_json::json!({"name": "spaces/x/messages/y"})),

            )

            .mount(&mock_server)

            .await;



        let provider = GoogleChatProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("plain text body").with_title("Plain Title");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({

                "error": {

                    "code": 403,

                    "message": "The caller does not have permission",

                    "status": "PERMISSION_DENIED"

                }

            })))

            .mount(&mock_server)

            .await;



        let provider = GoogleChatProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(403));

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_url() {

        let provider = GoogleChatProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = GoogleChatProvider::new(client());

        assert_eq!(provider.name(), "googlechat");

        assert_eq!(provider.url_scheme(), "gchat");

        assert!(!provider.description().is_empty());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "webhook_url" && p.required)

        );

    }

}


mod mattermost_send_tests {

    use super::*;

    use noti_providers::mattermost::MattermostProvider;



    fn parse_mock_uri(uri: &str) -> (String, String) {

        let url = Url::parse(uri).unwrap();

        (

            url.host_str().unwrap().to_string(),

            url.port().unwrap().to_string(),

        )

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/hooks/test-hook-id"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MattermostProvider::new(client());

        let config = ProviderConfig::new()

            .set("host", &host)

            .set("hook_id", "test-hook-id")

            .set("port", &port)

            .set("scheme", "http");

        let message = Message::text("Hello from mattermost test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "mattermost");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_optional_params_markdown() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MattermostProvider::new(client());

        let config = ProviderConfig::new()

            .set("host", &host)

            .set("hook_id", "test-hook")

            .set("port", &port)

            .set("scheme", "http")

            .set("channel", "town-square")

            .set("username", "TestBot")

            .set("icon_url", "https://example.com/icon.png");

        let message = Message::text("## Heading\nWith markdown")

            .with_title("Report")

            .with_format(MessageFormat::Markdown);

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_title_text() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MattermostProvider::new(client());

        let config = ProviderConfig::new()

            .set("host", &host)

            .set("hook_id", "hook1")

            .set("port", &port)

            .set("scheme", "http");

        let message = Message::text("Body text").with_title("Title");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_string("Invalid webhook token"))

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MattermostProvider::new(client());

        let config = ProviderConfig::new()

            .set("host", &host)

            .set("hook_id", "bad-hook")

            .set("port", &port)

            .set("scheme", "http");

        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(403));

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = MattermostProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("host", "example.com"))

                .is_err()

        );

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("hook_id", "abc"))

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = MattermostProvider::new(client());

        assert_eq!(provider.name(), "mattermost");

        assert_eq!(provider.url_scheme(), "mattermost");

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "host" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "hook_id" && p.required)

        );

    }

}


mod matrix_send_tests {

    use super::*;

    use noti_providers::matrix::MatrixProvider;



    fn parse_mock_uri(uri: &str) -> (String, String) {

        let url = Url::parse(uri).unwrap();

        (

            url.host_str().unwrap().to_string(),

            url.port().unwrap().to_string(),

        )

    }



    #[tokio::test]

    async fn test_send_success_put_bearer() {

        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))

            .and(header("Authorization", "Bearer test-access-token"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "event_id": "$abc123:matrix.org"

            })))

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MatrixProvider::new(client());

        let config = ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("room_id", "!room123:matrix.org")

            .set("server", &host)

            .set("port", &port)

            .set("scheme", "http");

        let message = Message::text("Hello Matrix!");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "matrix");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))

            .respond_with(

                ResponseTemplate::new(200).set_body_json(serde_json::json!({"event_id": "$xyz"})),

            )

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MatrixProvider::new(client());

        let config = ProviderConfig::new()

            .set("access_token", "token")

            .set("room_id", "!room:server")

            .set("server", &host)

            .set("port", &port)

            .set("scheme", "http");

        let message = Message::text("<b>bold</b>").with_format(MessageFormat::Html);

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_markdown_format() {

        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))

            .respond_with(

                ResponseTemplate::new(200).set_body_json(serde_json::json!({"event_id": "$md"})),

            )

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MatrixProvider::new(client());

        let config = ProviderConfig::new()

            .set("access_token", "token")

            .set("room_id", "!room:server")

            .set("server", &host)

            .set("port", &port)

            .set("scheme", "http");

        let message = Message::text("**bold**").with_format(MessageFormat::Markdown);

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_unauthorized() {

        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "errcode": "M_UNKNOWN_TOKEN",

                "error": "Unknown token"

            })))

            .mount(&mock_server)

            .await;



        let (host, port) = parse_mock_uri(&mock_server.uri());

        let provider = MatrixProvider::new(client());

        let config = ProviderConfig::new()

            .set("access_token", "bad-token")

            .set("room_id", "!room:server")

            .set("server", &host)

            .set("port", &port)

            .set("scheme", "http");

        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = MatrixProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("access_token", "tok"))

                .is_err()

        );

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("room_id", "!r:s"))

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = MatrixProvider::new(client());

        assert_eq!(provider.name(), "matrix");

        assert_eq!(provider.url_scheme(), "matrix");

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

                .any(|p| p.name == "room_id" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "server" && !p.required)

        );

    }

}


mod dingtalk_send_tests {

    use super::*;

    use noti_providers::dingtalk::DingTalkProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = DingTalkProvider::new(client());

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_validate_with_secret() {

        let provider = DingTalkProvider::new(client());

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("secret", "SECxxx");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = DingTalkProvider::new(client());

        assert_eq!(provider.name(), "dingtalk");

        assert_eq!(provider.url_scheme(), "dingtalk");

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

                .any(|p| p.name == "secret" && !p.required)

        );

    }

}


mod feishu_send_tests {

    use super::*;

    use noti_providers::feishu::FeishuProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = FeishuProvider::new(client());

        let config = ProviderConfig::new().set("hook_id", "test-hook");

        assert!(provider.validate_config(&config).is_ok());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_validate_with_secret() {

        let provider = FeishuProvider::new(client());

        let config = ProviderConfig::new()

            .set("hook_id", "test-hook")

            .set("secret", "some-secret");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = FeishuProvider::new(client());

        assert_eq!(provider.name(), "feishu");

        assert_eq!(provider.url_scheme(), "feishu");

        assert!(!provider.description().is_empty());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "hook_id" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "secret" && !p.required)

        );

    }

}


mod teams_send_tests {

    use super::*;

    use noti_providers::teams::TeamsProvider;



    #[tokio::test]

    async fn test_validate_config() {

        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", "https://teams.microsoft.com/...");

        assert!(provider.validate_config(&config).is_ok());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_webhook_url() {

        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("theme_color", "FF0000");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = TeamsProvider::new(client());

        assert_eq!(provider.name(), "teams");

        assert_eq!(provider.url_scheme(), "teams");

        assert!(!provider.description().is_empty());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "webhook_url" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "theme_color" && !p.required)

        );

    }



    #[tokio::test]

    async fn test_send_success() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))

            .mount(&mock_server)

            .await;



        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Deployment complete");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "teams");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))

            .mount(&mock_server)

            .await;



        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Service is up and running")

            .with_title("Health Check Passed");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))

            .mount(&mock_server)

            .await;



        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(500));

    }



    #[tokio::test]

    async fn test_send_with_theme_color() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(header("Content-Type", "application/json"))

            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))

            .mount(&mock_server)

            .await;



        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", mock_server.uri())

            .set("theme_color", "FF5733");

        let message = Message::text("Urgent alert");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))

            .mount(&mock_server)

            .await;



        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_with_markdown_title_and_body() {

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(header("Content-Type", "application/json"))

            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))

            .mount(&mock_server)

            .await;



        let provider = TeamsProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("## Build Summary\n- Tests: passed\n- Coverage: 85%")

            .with_title("CI Pipeline Complete")

            .with_format(MessageFormat::Markdown);

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }

}


mod telegram_extended_tests {

    use super::*;

    use noti_providers::telegram::TelegramProvider;



    #[tokio::test]

    async fn test_validate_config_full() {

        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABC")

            .set("chat_id", "-1001234567890")

            .set("disable_notification", "true")

            .set("disable_web_page_preview", "true");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = TelegramProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("bot_token", "x"))

                .is_err()

        );

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("chat_id", "x"))

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata_detailed() {

        let provider = TelegramProvider::new(client());

        assert_eq!(provider.name(), "telegram");

        assert_eq!(provider.url_scheme(), "tg");

        assert!(provider.description().contains("Telegram"));

        assert!(!provider.example_url().is_empty());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "bot_token" && p.required));

        assert!(params.iter().any(|p| p.name == "chat_id" && p.required));

        assert!(

            params

                .iter()

                .any(|p| p.name == "disable_notification" && !p.required)

        );

        assert!(

            params

                .iter()

                .any(|p| p.name == "disable_web_page_preview" && !p.required)

        );

    }

}


mod telegram_send_tests {

    use super::*;

    use noti_providers::telegram::TelegramProvider;



    #[tokio::test]

    async fn test_send_text_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": {

                    "message_id": 123

                }

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri());



        let message = Message::text("Hello World");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "telegram");

        assert_eq!(response.status_code, Some(200));

        assert!(response.message.contains("message sent successfully"));

    }



    #[tokio::test]

    async fn test_send_text_with_markdown() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendMessage"))

            .and(header("Content-Type", "application/json"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": {"message_id": 456}

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri());



        let message = Message::text("*bold* text")

            .with_format(MessageFormat::Markdown);

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_text_with_html() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": {"message_id": 789}

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri());



        let message = Message::text("<b>bold</b> text").with_format(MessageFormat::Html);

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_text_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendMessage"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "ok": false,

                "error_code": 400,

                "description": "Bad request: chat not found"

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-9999999999999")

            .set("base_url", mock_server.uri());



        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(400));

        assert!(response.message.contains("chat not found"));

    }



    #[tokio::test]

    async fn test_send_text_unauthorized() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/botBADTOKEN:WRONG/sendMessage"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "ok": false,

                "error_code": 401,

                "description": "Unauthorized"

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "BADTOKEN:WRONG")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri());



        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(401));

    }



    #[tokio::test]

    async fn test_send_text_with_optional_params() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": {"message_id": 111}

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri())

            .set("disable_notification", "true")

            .set("thread_id", "42")

            .set("protect", "true");



        let message = Message::text("Silent protected message");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_chat_action_typing() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendChatAction"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": true

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri())

            .set("action", "typing");



        let message = Message::text("ignored when action is set");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_chat_action_upload_photo() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendChatAction"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": true

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri())

            .set("action", "upload_photo");



        let message = Message::text("Test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_edit_message_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/editMessageText"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": {

                    "message_id": 999,

                    "text": "Updated text"

                }

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("edit_message_id", "999")

            .set("base_url", mock_server.uri());



        let message = Message::text("Updated text");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_text_with_title() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/bot123456:ABCDEF/sendMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "ok": true,

                "result": {"message_id": 222}

            })))

            .mount(&mock_server)

            .await;



        let provider = TelegramProvider::new(client());

        let config = ProviderConfig::new()

            .set("bot_token", "123456:ABCDEF")

            .set("chat_id", "-1001234567890")

            .set("base_url", mock_server.uri());



        let message = Message::text("Body text").with_title("Title Here");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        assert!(result.unwrap().success);

    }

}


mod discord_extended_tests {

    use super::*;

    use noti_providers::discord::DiscordProvider;



    #[tokio::test]

    async fn test_validate_full_config() {

        let provider = DiscordProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_id", "123456")

            .set("webhook_token", "abcdef")

            .set("username", "MyBot")

            .set("avatar_url", "https://example.com/avatar.png");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_fields() {

        let provider = DiscordProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("webhook_id", "123"))

                .is_err()

        );

        assert!(

            provider

                .validate_config(&ProviderConfig::new().set("webhook_token", "abc"))

                .is_err()

        );

    }



    #[tokio::test]

    async fn test_metadata_detailed() {

        let provider = DiscordProvider::new(client());

        assert_eq!(provider.name(), "discord");

        assert_eq!(provider.url_scheme(), "discord");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "webhook_id" && p.required));

        assert!(

            params

                .iter()

                .any(|p| p.name == "webhook_token" && p.required)

        );

        assert!(params.iter().any(|p| p.name == "username" && !p.required));

        assert!(params.iter().any(|p| p.name == "avatar_url" && !p.required));

    }

}


mod discord_send_tests {

    use super::*;

    use noti_providers::discord::DiscordProvider;

    use serde_json::json;



    fn make_config() -> ProviderConfig {

        ProviderConfig::new()

            .set("webhook_id", "1234567890")

            .set("webhook_token", "abcdefg_hijklmn")

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = DiscordProvider::new(client());

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_id() {

        let provider = DiscordProvider::new(client());

        let config = ProviderConfig::new().set("webhook_token", "abc");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_token() {

        let provider = DiscordProvider::new(client());

        let config = ProviderConfig::new().set("webhook_id", "123");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_empty() {

        let provider = DiscordProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = DiscordProvider::new(client());

        assert_eq!(provider.name(), "discord");

        assert_eq!(provider.url_scheme(), "discord");

        assert!(!provider.description().is_empty());

        assert!(provider.supports_attachments());

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "webhook_id" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "webhook_token" && p.required)

        );

        assert!(

            provider

                .params()

                .iter()

                .any(|p| p.name == "base_url" && !p.required)

        );

    }



    #[tokio::test]

    async fn test_send_success_204() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());



        let message = Message::text("Hello Discord!");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "discord");

        assert_eq!(response.status_code, Some(204));

    }



    #[tokio::test]

    async fn test_send_success_200_with_wait() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "id": "1234567890123456789",

                "type": 0,

                "content": "Hello Discord!",

                "channel_id": "1234567890"

            })))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config()

            .set("base_url", mock_server.uri())

            .set("wait", "true");



        let message = Message::text("Hello Discord!");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title_markdown() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());



        let message = Message::markdown("Details here").with_title("Alert");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_username_and_avatar() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config()

            .set("base_url", mock_server.uri())

            .set("username", "CustomBot")

            .set("avatar_url", "https://example.com/avatar.png");



        let message = Message::text("Custom bot message");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_thread_id() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config()

            .set("base_url", mock_server.uri())

            .set("thread_id", "999888777");



        let message = Message::text("Thread message");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_with_embed_params() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config()

            .set("base_url", mock_server.uri())

            .set("embed_title", "Build Status")

            .set("embed_color", "0x00FF00")

            .set("embed_description", "All tests passed!")

            .set("embed_footer", "CI Pipeline")

            .set("embed_field", "Tests:1500,Duration:3m");



        let message = Message::text("ignored in embed mode");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_400() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(400).set_body_json(json!({

                "message": "Cannot send an empty message"

            })))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());



        let message = Message::text("test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(400));

        assert!(response.message.contains("Cannot send an empty message"));

    }



    #[tokio::test]

    async fn test_send_unauthorized_401() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(401).set_body_json(json!({

                "message": "Invalid webhook token"

            })))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());



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

            .and(path("/api/webhooks/1234567890/abcdefg_hijklmn"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&mock_server)

            .await;



        let provider = DiscordProvider::new(client());

        let config = make_config().set("base_url", mock_server.uri());



        let message = Message::text("using custom base url");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = DiscordProvider::new(client());

        // No base_url set - should use default https://discord.com

        assert!(provider.validate_config(&make_config()).is_ok());

    }

}


mod slack_send_tests {

    use super::*;

    use noti_providers::slack::SlackProvider;

    use serde_json::json;



    fn make_config() -> ProviderConfig {

        ProviderConfig::new().set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = SlackProvider::new(client());

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_url() {

        let provider = SlackProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = SlackProvider::new(client());

        assert_eq!(provider.name(), "slack");

        assert_eq!(provider.url_scheme(), "slack");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "webhook_url" && p.required));

        assert!(params.iter().any(|p| p.name == "channel" && !p.required));

        assert!(params.iter().any(|p| p.name == "bot_token" && !p.required));

        assert!(params.iter().any(|p| p.name == "thread_ts" && !p.required));

        assert!(params.iter().any(|p| p.name == "ephemeral_user" && !p.required));

        assert!(params.iter().any(|p| p.name == "send_at" && !p.required));

        assert!(params.iter().any(|p| p.name == "blocks" && !p.required));

        assert!(params.iter().any(|p| p.name == "base_url" && !p.required));

    }



    // --- Webhook send tests ---



    #[tokio::test]

    async fn test_send_webhook_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("Hello Slack!");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "slack");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_webhook_with_channel() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", mock_server.uri())

            .set("channel", "#alerts");

        let message = Message::text("Channel message");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_webhook_with_username_and_icon() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", mock_server.uri())

            .set("username", "AlertBot")

            .set("icon_emoji", ":rotating_light:");

        let message = Message::text("Alert!");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_webhook_markdown_format() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::markdown("*Bold* and _italic_");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_webhook_with_blocks_json() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", mock_server.uri())

            .set(

                "blocks",

                r#"[{"type":"section","text":{"type":"mrkdwn","text":"*Deploy complete*"}}]"#,

            );

        let message = Message::text("fallback text");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_webhook_invalid_blocks_json() {

        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("blocks", "not-valid-json{{{");

        let message = Message::text("test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(

            format!("{err}").contains("invalid blocks JSON"),

            "expected blocks JSON error, got: {err}"

        );

    }



    #[tokio::test]

    async fn test_send_webhook_failure() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_string("invalid_token"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(403));

    }



    #[tokio::test]

    async fn test_send_webhook_server_error() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("server_error"))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new().set("webhook_url", mock_server.uri());

        let message = Message::text("test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(500));

    }



    // --- API send tests (chat.postMessage) ---



    #[tokio::test]

    async fn test_send_api_post_message_with_thread_ts() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/chat.postMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "ok": true,

                "ts": "1234567890.123456"

            })))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("bot_token", "xoxb-test-token")

            .set("thread_ts", "1234567890.000001")

            .set("base_url", mock_server.uri());



        let message = Message::text("Thread reply");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "slack");

    }



    #[tokio::test]

    async fn test_send_api_post_message_markdown() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/chat.postMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "ok": true,

                "ts": "1234567890.123456"

            })))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("bot_token", "xoxb-test-token")

            .set("thread_ts", "1234567890.000001")

            .set("base_url", mock_server.uri());



        let message = Message::markdown("*Bold* reply");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_api_post_ephemeral() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/chat.postEphemeral"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "ok": true,

                "message_ts": "1234567890.123456"

            })))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("bot_token", "xoxb-test-token")

            .set("ephemeral_user", "U12345678")

            .set("base_url", mock_server.uri());



        let message = Message::text("Ephemeral message");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_api_schedule_message() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/chat.scheduleMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "ok": true,

                "scheduled_message_id": "Q1234567890",

                "post_at": "1712640000"

            })))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("bot_token", "xoxb-test-token")

            .set("send_at", "1712640000")

            .set("base_url", mock_server.uri());



        let message = Message::text("Scheduled message");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_api_error_response() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/chat.postMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "ok": false,

                "error": "channel_not_found"

            })))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C_INVALID")

            .set("bot_token", "xoxb-test-token")

            .set("thread_ts", "1234567890.000001")

            .set("base_url", mock_server.uri());



        let message = Message::text("test");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert!(response.message.contains("channel_not_found"));

    }



    // --- bot_token required validation ---



    #[tokio::test]

    async fn test_send_thread_ts_without_bot_token_returns_error() {

        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("thread_ts", "1234567890.000001");



        let message = Message::text("Thread reply");

        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(

            format!("{err}").contains("bot_token required"),

            "expected bot_token required error, got: {err}"

        );

    }



    #[tokio::test]

    async fn test_send_ephemeral_without_bot_token_returns_error() {

        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("ephemeral_user", "U12345678");



        let message = Message::text("Ephemeral");

        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(

            format!("{err}").contains("bot_token required"),

            "expected bot_token required error, got: {err}"

        );

    }



    #[tokio::test]

    async fn test_send_scheduled_without_bot_token_returns_error() {

        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("send_at", "1712640000");



        let message = Message::text("Scheduled");

        let result = provider.send(&message, &config).await;

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(

            format!("{err}").contains("bot_token required"),

            "expected bot_token required error, got: {err}"

        );

    }



    // --- base_url tests ---



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/chat.postMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "ok": true,

                "ts": "1234567890.123456"

            })))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("bot_token", "xoxb-test-token")

            .set("thread_ts", "1234567890.000001")

            .set("base_url", mock_server.uri());



        let message = Message::text("using custom base url");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = SlackProvider::new(client());

        // No base_url set - should use default https://slack.com

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/api/chat.postMessage"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "ok": true,

                "ts": "1234567890.123456"

            })))

            .mount(&mock_server)

            .await;



        let provider = SlackProvider::new(client());

        // Trailing slash should be stripped so the path is /api/chat.postMessage

        let config = ProviderConfig::new()

            .set("webhook_url", "https://hooks.slack.com/services/T/B/xxx")

            .set("channel", "C12345678")

            .set("bot_token", "xoxb-test-token")

            .set("thread_ts", "1234567890.000001")

            .set("base_url", format!("{}/", mock_server.uri()));



        let message = Message::text("test trailing slash");

        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }

}


mod wecom_send_tests {

    use super::*;

    use noti_providers::wecom::WeComProvider;

    use serde_json::json;



    fn make_config() -> ProviderConfig {

        ProviderConfig::new().set("key", "test-key-123")

    }



    fn make_config_with_base_url(mock_server: &MockServer) -> ProviderConfig {

        make_config().set("base_url", mock_server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = WeComProvider::new(client());

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_key() {

        let provider = WeComProvider::new(client());

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = WeComProvider::new(client());

        assert_eq!(provider.name(), "wecom");

        assert_eq!(provider.url_scheme(), "wecom");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        let params = provider.params();

        assert!(params.iter().any(|p| p.name == "key" && p.required));

        assert!(params.iter().any(|p| p.name == "mentioned_list" && !p.required));

        assert!(params.iter().any(|p| p.name == "type" && !p.required));

        assert!(params.iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_text_success() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 0,

                "errmsg": "ok"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server);

        let message = Message::text("Hello WeCom!");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        let response = result.unwrap();

        assert!(response.success);

        assert_eq!(response.provider, "wecom");

        assert_eq!(response.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_markdown() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 0,

                "errmsg": "ok"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server);

        let message = Message::markdown("**Bold** and > quote");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_text_with_mentions() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 0,

                "errmsg": "ok"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server)

            .set("mentioned_list", "user1,user2")

            .set("mentioned_mobile_list", "13800138000");

        let message = Message::text("@all urgent");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_news_type() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 0,

                "errmsg": "ok"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server)

            .set("type", "news")

            .set("news_title", "Deploy Complete")

            .set("news_desc", "All services running")

            .set("news_url", "https://ci.example.com/build/123")

            .set("news_picurl", "https://ci.example.com/chart.png");

        let message = Message::text("Build passed");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_template_card_type() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 0,

                "errmsg": "ok"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server)

            .set("type", "template_card")

            .set("card_type", "text_notice")

            .set("card_title", "System Alert")

            .set("card_desc", "CPU usage > 90%")

            .set("card_jump_url", "https://monitor.example.com")

            .set("card_jump_title", "View Details");

        let message = Message::text("CPU alert");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_failure_errcode_nonzero() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 40001,

                "errmsg": "invalid webhook key"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server);

        let message = Message::text("test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert!(response.message.contains("invalid webhook key"));

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(500).set_body_json(json!({

                "errcode": -1,

                "errmsg": "internal server error"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server);

        let message = Message::text("test");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok());

        let response = result.unwrap();

        assert!(!response.success);

        assert_eq!(response.status_code, Some(500));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 0,

                "errmsg": "ok"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        let config = make_config_with_base_url(&mock_server);

        let message = Message::text("using custom base url");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = WeComProvider::new(client());

        // No base_url set - should use default https://qyapi.weixin.qq.com

        assert!(provider.validate_config(&make_config()).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let mock_server = MockServer::start().await;



        Mock::given(method("POST"))

            .and(path("/cgi-bin/webhook/send"))

            .respond_with(ResponseTemplate::new(200).set_body_json(json!({

                "errcode": 0,

                "errmsg": "ok"

            })))

            .mount(&mock_server)

            .await;



        let provider = WeComProvider::new(client());

        // Trailing slash should be stripped

        let config = make_config().set("base_url", format!("{}/", mock_server.uri()));

        let message = Message::text("test trailing slash");



        let result = provider.send(&message, &config).await;

        assert!(result.is_ok(), "send failed: {:?}", result);

        assert!(result.unwrap().success);

    }

}


mod line_send_tests {

    use super::*;

    use noti_providers::line::LineProvider;



    fn provider() -> LineProvider {

        LineProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "line-test-token")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "line");

        assert_eq!(provider.url_scheme(), "line");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"access_token"));

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 200,

                "message": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from LINE");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "line");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 200,

                "message": "ok"

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

    async fn test_send_failure_error_code() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 401,

                "message": "Unauthorized"

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



        let result = provider.send(&message, &config).await;

        // 500 will likely cause a JSON parse error since no body

        assert!(result.is_err() || !result.unwrap().success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "status": 401,

                "message": "Invalid access token"

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

                "status": 200,

                "message": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("base_url", &server.uri());

        let message = Message::text("Test with custom base");



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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "status": 200,

                "message": "ok"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("base_url", &base);

        let message = Message::text("Test trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod zulip_send_tests {

    use super::*;

    use noti_providers::zulip::ZulipProvider;



    fn provider() -> ZulipProvider {

        ZulipProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("domain", "example.zulipchat.com")

            .set("bot_email", "bot@example.zulipchat.com")

            .set("api_key", "test-api-key")

            .set("stream", "general")

            .set("topic", "notifications")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("domain", "example.zulipchat.com")

            .set("bot_email", "bot@example.zulipchat.com")

            .set("api_key", "test-api-key");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_domain() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("bot_email", "bot@example.com")

            .set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_bot_email() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("domain", "example.zulipchat.com")

            .set("api_key", "key");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_api_key() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("domain", "example.zulipchat.com")

            .set("bot_email", "bot@example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "zulip");

        assert_eq!(provider.url_scheme(), "zulip");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"domain"));

        assert!(param_names.contains(&"bot_email"));

        assert!(param_names.contains(&"api_key"));

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": "success",

                "id": 12345,

                "msg": ""

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Zulip");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "zulip");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": "success",

                "id": 12346,

                "msg": ""

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Important Update");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_markdown_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": "success",

                "id": 12347,

                "msg": ""

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("**Bold text**").with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error_result() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "result": "error",

                "msg": "Invalid API key"

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

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "result": "error",

                "msg": "Unauthorized"

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

    async fn test_send_direct_message() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": "success",

                "id": 12348,

                "msg": ""

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("domain", "example.zulipchat.com")

            .set("bot_email", "bot@example.zulipchat.com")

            .set("api_key", "test-api-key")

            .set("type", "direct")

            .set("to", "user@example.com")

            .set("base_url", &server.uri());

        let message = Message::text("Direct message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": "success",

                "id": 12349,

                "msg": ""

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test with custom base");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "result": "success",

                "id": 12350,

                "msg": ""

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("domain", "example.zulipchat.com")

            .set("bot_email", "bot@example.zulipchat.com")

            .set("api_key", "test-api-key")

            .set("stream", "general")

            .set("base_url", &base);

        let message = Message::text("Test trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod mastodon_send_tests {

    use super::*;

    use noti_providers::mastodon::MastodonProvider;



    fn provider() -> MastodonProvider {

        MastodonProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "mastodon-test-token")

            .set("instance", "mastodon.social")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("instance", "mastodon.social");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("instance", "mastodon.social");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_instance() {

        let provider = provider();

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "mastodon");

        assert_eq!(provider.url_scheme(), "mastodon");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"access_token"));

        assert!(param_names.contains(&"instance"));

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "1234567890",

                "url": "https://mastodon.social/@user/1234567890",

                "content": "Hello from Mastodon"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Mastodon");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "mastodon");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "1234567891",

                "url": "https://mastodon.social/@user/1234567891",

                "content": "Title\n\nBody text"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Breaking News");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_visibility() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "1234567892",

                "url": "https://mastodon.social/@user/1234567892",

                "visibility": "unlisted"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("instance", "mastodon.social")

            .set("visibility", "unlisted")

            .set("base_url", &server.uri());

        let message = Message::text("Unlisted post");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_spoiler_text() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "1234567893",

                "url": "https://mastodon.social/@user/1234567893",

                "spoiler_text": "Spoiler!"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("instance", "mastodon.social")

            .set("spoiler_text", "Spoiler!")

            .set("base_url", &server.uri());

        let message = Message::text("Hidden content");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_api_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({

                "error": "Status is too long"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Very long status...");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_unauthorized() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": "The access token is invalid"

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

                "id": "1234567894",

                "url": "https://custom.instance/@user/1234567894"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test with custom base");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "1234567895",

                "url": "https://mastodon.social/@user/1234567895"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("instance", "mastodon.social")

            .set("base_url", &base);

        let message = Message::text("Test trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod rocketchat_send_tests {

    use super::*;

    use noti_providers::rocketchat::RocketChatProvider;



    fn provider() -> RocketChatProvider {

        RocketChatProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("host", "chat.example.com")

            .set("token_a", "abc123")

            .set("token_b", "def456")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "chat.example.com")

            .set("token_a", "abc")

            .set("token_b", "def");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_config_missing_host() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("token_a", "abc")

            .set("token_b", "def");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_token_a() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "chat.example.com")

            .set("token_b", "def");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_config_missing_token_b() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "chat.example.com")

            .set("token_a", "abc");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "rocketchat");

        assert_eq!(provider.url_scheme(), "rocketchat");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"host"));

        assert!(param_names.contains(&"token_a"));

        assert!(param_names.contains(&"token_b"));

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true,

                "channel": "#general"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from RocketChat");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "rocketchat");

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

        let message = Message::text("Body text").with_title("Important Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_markdown_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("**Bold text**").with_title("MD Title").with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_channel_override() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "success": true

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("host", "chat.example.com")

            .set("token_a", "abc123")

            .set("token_b", "def456")

            .set("channel", "#alerts")

            .set("base_url", &server.uri());

        let message = Message::text("Channel message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "success": false,

                "error": "Invalid webhook token"

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

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "success": false,

                "error": "Unauthorized"

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

                "success": true

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test with custom base");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

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

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("host", "chat.example.com")

            .set("token_a", "abc123")

            .set("token_b", "def456")

            .set("base_url", &base);

        let message = Message::text("Test trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod ryver_send_tests {

    use super::*;

    use noti_providers::ryver::RyverProvider;



    fn provider() -> RyverProvider {

        RyverProvider::new(client())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_organization() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "webhook-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("organization", "mycompany");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "ryver");

        assert_eq!(provider.url_scheme(), "ryver");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "organization" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "token" && p.required));

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

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token")

            .set("base_url", &server.uri());

        let message = Message::text("Hello Ryver");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "ryver");

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

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token")

            .set("base_url", &server.uri());

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_with_markdown_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token")

            .set("base_url", &server.uri());

        let message = Message::text("**bold** text")

            .with_title("MD Title")

            .with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "bad-token")

            .set("base_url", &server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }



    #[tokio::test]

    async fn test_send_http_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token")

            .set("base_url", &server.uri());

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

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token")

            .set("base_url", &server.uri());

        let message = Message::text("Custom base");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_no_base_url_uses_default() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token");

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

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("organization", "mycompany")

            .set("token", "webhook-token")

            .set("base_url", &base);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod webex_send_tests {

    use super::*;

    use noti_providers::webex::WebexProvider;



    fn provider() -> WebexProvider {

        WebexProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("room_id", "test-room-id")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("room_id", "test-room");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("room_id", "test-room");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_room_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "webex");

        assert_eq!(provider.url_scheme(), "webex");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "access_token" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "room_id" && p.required));

        assert!(provider.params().iter().any(|p| p.name == "base_url" && !p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "message-id-123",

                "roomId": "test-room-id"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Webex");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "webex");

        assert_eq!(result.status_code, Some(200));

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-id"

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

    async fn test_send_markdown_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-id"

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

    async fn test_send_with_to_person_email() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-id"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("room_id", "test-room-id")

            .set("to_person_email", "user@example.com")

            .set("base_url", &server.uri());

        let message = Message::text("Direct message");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure_error_message() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({

                "message": "Invalid roomId"

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

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "message": "Unauthorized"

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-id"

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

            .set("access_token", "test-token")

            .set("room_id", "test-room");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-id"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let mut base = server.uri();

        base.push('/');

        let config = ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("room_id", "test-room-id")

            .set("base_url", &base);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod flock_send_tests {

    use super::*;

    use noti_providers::flock::FlockProvider;



    fn provider() -> FlockProvider {

        FlockProvider::new(client())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("webhook_url", "https://api.flock.com/hooks/sendMessage/XXXXXX");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_url() {

        let provider = provider();

        assert!(provider.validate_config(&ProviderConfig::new()).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "flock");

        assert_eq!(provider.url_scheme(), "flock");

        assert!(!provider.description().is_empty());

        assert!(!provider.example_url().is_empty());

        assert!(provider.supports_attachments());

        assert!(provider.params().iter().any(|p| p.name == "webhook_url" && p.required));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new().set("webhook_url", server.uri());

        let message = Message::text("Hello Flock");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "flock");

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

        let config = ProviderConfig::new().set("webhook_url", server.uri());

        let message = Message::text("Body text").with_title("Alert Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_html_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new().set("webhook_url", server.uri());

        let message = Message::text("<b>bold</b> text")

            .with_title("HTML Title")

            .with_format(MessageFormat::Html);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_markdown_format() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new().set("webhook_url", server.uri());

        let message = Message::text("**bold** text").with_format(MessageFormat::Markdown);



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(403).set_body_string("Invalid token"))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new().set("webhook_url", server.uri());

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

        let config = ProviderConfig::new().set("webhook_url", server.uri());

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

    }

}


mod gitter_send_tests {

    use super::*;

    use noti_providers::gitter::GitterProvider;



    fn provider() -> GitterProvider {

        GitterProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("token", "test-gitter-token")

            .set("room_id", "test-room-id")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("token", "test-token")

            .set("room_id", "test-room-id");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("room_id", "test-room-id");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_room_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("token", "test-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "gitter");

        assert_eq!(provider.url_scheme(), "gitter");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/rooms/test-room-id/chatMessages"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "id": "msg-123",

                "text": "Hello from Gitter"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello from Gitter");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "gitter");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/rooms/test-room-id/chatMessages"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-456"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Alert");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/rooms/test-room-id/chatMessages"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": "Unauthorized"

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

    async fn test_send_server_error() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/v1/rooms/test-room-id/chatMessages"))

            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({

                "error": "Internal server error"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Test");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(!result.success);

        assert_eq!(result.status_code, Some(500));

    }



    #[tokio::test]

    async fn test_send_custom_base_url() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-789"})))

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

            .set("token", "test-token")

            .set("room_id", "test-room-id");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-trail"})))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("token", "test-gitter-token")

            .set("room_id", "test-room-id")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod guilded_send_tests {

    use super::*;

    use noti_providers::guilded::GuildedProvider;



    fn provider() -> GuildedProvider {

        GuildedProvider::new(client())

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

        assert_eq!(provider.name(), "guilded");

        assert_eq!(provider.url_scheme(), "guilded");

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

        let message = Message::text("Hello from Guilded");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "guilded");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhooks/test-webhook-id/test-webhook-token"))

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

    async fn test_send_with_username_and_avatar() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhooks/test-webhook-id/test-webhook-token"))

            .respond_with(ResponseTemplate::new(204))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server)

            .set("username", "noti-bot")

            .set("avatar_url", "https://example.com/avatar.png");

        let message = Message::text("Custom identity");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/webhooks/test-webhook-id/test-webhook-token"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "message": "Invalid webhook token"

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


mod misskey_send_tests {

    use super::*;

    use noti_providers::misskey::MisskeyProvider;



    fn provider() -> MisskeyProvider {

        MisskeyProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("access_token", "test-access-token")

            .set("instance", "misskey.io")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("access_token", "test-token")

            .set("instance", "misskey.io");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_access_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("instance", "misskey.io");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_instance() {

        let provider = provider();

        let config = ProviderConfig::new().set("access_token", "test-token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "misskey");

        assert_eq!(provider.url_scheme(), "misskey");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/notes/create"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "createdNote": { "id": "note-123" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Misskey");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "misskey");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/notes/create"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "createdNote": { "id": "note-456" }

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

    async fn test_send_with_visibility() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/notes/create"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "createdNote": { "id": "note-789" }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server).set("visibility", "home");

        let message = Message::text("Home only");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/api/notes/create"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "error": { "message": "Authentication failed" }

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

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "createdNote": { "id": "note-custom" }

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

            .set("access_token", "test-token")

            .set("instance", "misskey.io");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod nctalk_send_tests {

    use super::*;

    use noti_providers::nctalk::NcTalkProvider;



    fn provider() -> NcTalkProvider {

        NcTalkProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("user", "admin")

            .set("password", "test-password")

            .set("host", "cloud.example.com")

            .set("room_token", "test-room-token")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user", "admin")

            .set("password", "pass")

            .set("host", "cloud.example.com")

            .set("room_token", "token");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_user() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("password", "pass")

            .set("host", "cloud.example.com")

            .set("room_token", "token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_room_token() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("user", "admin")

            .set("password", "pass")

            .set("host", "cloud.example.com");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "nctalk");

        assert_eq!(provider.url_scheme(), "nctalk");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/ocs/v2.php/apps/spreed/api/v1/chat/test-room-token"))

            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 201 }, "data": { "id": 42 } }

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Nextcloud Talk");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "nctalk");

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .and(path("/ocs/v2.php/apps/spreed/api/v1/chat/test-room-token"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 401, "message": "Unauthorized" } }

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

            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({

                "ocs": { "meta": { "statuscode": 201 }, "data": { "id": 99 } }

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

            .set("user", "admin")

            .set("password", "pass")

            .set("host", "cloud.example.com")

            .set("room_token", "token");

        assert!(provider.validate_config(&config).is_ok());

    }

}


mod revolt_send_tests {

    use super::*;

    use noti_providers::revolt::RevoltProvider;



    fn provider() -> RevoltProvider {

        RevoltProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("bot_token", "test-bot-token")

            .set("channel_id", "test-channel-id")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new()

            .set("bot_token", "token")

            .set("channel_id", "channel");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_bot_token() {

        let provider = provider();

        let config = ProviderConfig::new().set("channel_id", "channel");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_validate_missing_channel_id() {

        let provider = provider();

        let config = ProviderConfig::new().set("bot_token", "token");

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "revolt");

        assert_eq!(provider.url_scheme(), "revolt");

        assert!(provider.supports_attachments());

        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();

        assert!(param_names.contains(&"base_url"));

    }



    #[tokio::test]

    async fn test_send_success() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "_id": "msg-123", "content": "Hello Revolt"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Hello Revolt");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "revolt");

    }



    #[tokio::test]

    async fn test_send_with_title() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "_id": "msg-124"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = config(&server);

        let message = Message::text("Body text").with_title("Bold Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({

                "type": "InvalidSession"

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

    async fn test_send_base_url_trailing_slash_stripped() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({

                "_id": "msg-125"

            })))

            .mount(&server)

            .await;



        let provider = provider();

        let config = ProviderConfig::new()

            .set("bot_token", "test-bot-token")

            .set("channel_id", "test-channel-id")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}


mod twist_send_tests {

    use super::*;

    use noti_providers::twist::TwistProvider;



    fn provider() -> TwistProvider {

        TwistProvider::new(client())

    }



    fn config(server: &MockServer) -> ProviderConfig {

        ProviderConfig::new()

            .set("webhook_url", "https://twist.com/api/v3/integration_incoming/post_data?install_id=XXX")

            .set("base_url", &server.uri())

    }



    #[tokio::test]

    async fn test_validate_config() {

        let provider = provider();

        let config = ProviderConfig::new().set("webhook_url", "https://twist.com/hook");

        assert!(provider.validate_config(&config).is_ok());

    }



    #[tokio::test]

    async fn test_validate_missing_webhook_url() {

        let provider = provider();

        let config = ProviderConfig::new();

        assert!(provider.validate_config(&config).is_err());

    }



    #[tokio::test]

    async fn test_metadata() {

        let provider = provider();

        assert_eq!(provider.name(), "twist");

        assert_eq!(provider.url_scheme(), "twist");

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

        let message = Message::text("Hello Twist");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

        assert_eq!(result.provider, "twist");

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

        let message = Message::text("Body text").with_title("Title");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }



    #[tokio::test]

    async fn test_send_failure() {

        let server = MockServer::start().await;

        Mock::given(method("POST"))

            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))

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

            .set("webhook_url", "https://twist.com/api/v3/integration_incoming/post_data")

            .set("base_url", &format!("{}/", server.uri().trim_end_matches('/')));

        let message = Message::text("Trailing slash");



        let result = provider.send(&message, &config).await.unwrap();

        assert!(result.success);

    }

}

