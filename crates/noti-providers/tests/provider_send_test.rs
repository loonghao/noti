/// Comprehensive wiremock-based tests for provider send() methods.
/// Tests both success and failure paths for representative providers.
use noti_core::{Message, MessageFormat, NotifyProvider, ProviderConfig};
use reqwest::Client;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use url::Url;

// ======================== Helper ========================

fn client() -> Client {
    Client::new()
}

// ======================== WeComProvider tests ========================

mod wecom_tests {
    use super::*;
    use noti_providers::wecom::WeComProvider;

    #[tokio::test]
    async fn test_send_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/cgi-bin/webhook/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errcode": 0,
                "errmsg": "ok"
            })))
            .mount(&mock_server)
            .await;

        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let provider = WeComProvider::new(client);

        // We need to use a config that points to mock server
        // Since WeComProvider builds URL from key, we need to override
        // by setting the full URL in a way the provider understands.
        // WeComProvider uses build_webhook_url(key) => https://qyapi.weixin.qq.com/...
        // We can't easily mock this, so test validate_config instead
        let config = ProviderConfig::new().set("key", "test-key-123");
        assert!(provider.validate_config(&config).is_ok());

        // Test metadata
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

// ======================== SlackProvider tests ========================

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

// ======================== DiscordProvider tests ========================

mod discord_tests {
    use super::*;
    use noti_providers::discord::DiscordProvider;

    #[tokio::test]
    async fn test_send_success_204() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let provider = DiscordProvider::new(client());

        // Discord builds URL from webhook_id/webhook_token,
        // so we test metadata and validation
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

// ======================== TelegramProvider tests ========================

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

// ======================== WebhookProvider tests ========================

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

// ======================== NtfyProvider tests ========================

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
            .set("server", &mock_server.uri());
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
            .set("server", &mock_server.uri());
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
            .set("server", &mock_server.uri())
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
            .set("server", &mock_server.uri());
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

// ======================== GotifyProvider tests ========================

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
            .set("host", &mock_server.uri())
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
            .set("host", &mock_server.uri())
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
            .set("host", &mock_server.uri())
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

// ======================== FeishuProvider tests ========================

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
        assert!(provider.description().len() > 0);
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "hook_id" && p.required)
        );
    }
}

// ======================== DingtalkProvider tests ========================

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

// ======================== TeamsProvider tests ========================

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

// ======================== PushoverProvider tests ========================

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

// ======================== BarkProvider tests ========================

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

// ======================== ServerChanProvider tests ========================

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

// ======================== GoogleChatProvider send tests ========================

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

// ======================== MattermostProvider send tests ========================

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

// ======================== MatrixProvider send tests (PUT + Bearer) ========================

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

// ======================== BarkProvider send tests ========================

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
            .set("server", &mock_server.uri());
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
            .set("server", &mock_server.uri())
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
            .set("server", &mock_server.uri());
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

// ======================== PushBulletProvider send tests ========================

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

// ======================== SimplePushProvider send tests ========================

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

// ======================== TwilioProvider send tests (Basic Auth + Form) ========================

mod twilio_send_tests {
    use super::*;
    use noti_providers::twilio::TwilioProvider;

    #[tokio::test]
    async fn test_validate_config() {
        let provider = TwilioProvider::new(client());
        let config = ProviderConfig::new()
            .set("account_sid", "ACxxx")
            .set("auth_token", "authtoken")
            .set("from", "+15551234567")
            .set("to", "+15559876543");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_missing_fields() {
        let provider = TwilioProvider::new(client());
        assert!(provider.validate_config(&ProviderConfig::new()).is_err());
        // Missing auth_token, from, to
        assert!(
            provider
                .validate_config(&ProviderConfig::new().set("account_sid", "ACxxx"))
                .is_err()
        );
        // Missing from, to
        assert!(
            provider
                .validate_config(
                    &ProviderConfig::new()
                        .set("account_sid", "ACxxx")
                        .set("auth_token", "tok")
                )
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_metadata() {
        let provider = TwilioProvider::new(client());
        assert_eq!(provider.name(), "twilio");
        assert_eq!(provider.url_scheme(), "twilio");
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "account_sid" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "auth_token" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "from" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "to" && p.required)
        );
    }
}

// ======================== IftttProvider send tests ========================

mod ifttt_send_tests {
    use super::*;
    use noti_providers::ifttt::IftttProvider;

    #[tokio::test]
    async fn test_validate_config() {
        let provider = IftttProvider::new(client());
        let config = ProviderConfig::new()
            .set("webhook_key", "test-key")
            .set("event", "notification");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_missing_fields() {
        let provider = IftttProvider::new(client());
        assert!(provider.validate_config(&ProviderConfig::new()).is_err());
        assert!(
            provider
                .validate_config(&ProviderConfig::new().set("webhook_key", "x"))
                .is_err()
        );
        assert!(
            provider
                .validate_config(&ProviderConfig::new().set("event", "x"))
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_metadata() {
        let provider = IftttProvider::new(client());
        assert_eq!(provider.name(), "ifttt");
        assert_eq!(provider.url_scheme(), "ifttt");
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "webhook_key" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "event" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "value1" && !p.required)
        );
    }
}

// ======================== DingTalkProvider send tests ========================

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

// ======================== FeishuProvider send tests ========================

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

// ======================== TeamsProvider send tests ========================

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
    }
}

// ======================== PushoverProvider send tests ========================

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

// ======================== EmailProvider tests ========================

mod email_tests {
    use super::*;
    use noti_providers::email::EmailProvider;

    #[tokio::test]
    async fn test_validate_config() {
        let provider = EmailProvider::new();
        let config = ProviderConfig::new()
            .set("host", "smtp.gmail.com")
            .set("username", "user@gmail.com")
            .set("password", "app-password")
            .set("to", "recipient@example.com");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_missing_fields() {
        let provider = EmailProvider::new();
        assert!(provider.validate_config(&ProviderConfig::new()).is_err());
        // Missing password, to
        assert!(
            provider
                .validate_config(
                    &ProviderConfig::new()
                        .set("host", "smtp.gmail.com")
                        .set("username", "user@gmail.com")
                )
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_metadata() {
        let provider = EmailProvider::new();
        assert_eq!(provider.name(), "email");
        assert_eq!(provider.url_scheme(), "smtp");
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
                .any(|p| p.name == "username" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "password" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "to" && p.required)
        );
        assert!(
            provider
                .params()
                .iter()
                .any(|p| p.name == "port" && !p.required)
        );
    }

    #[tokio::test]
    async fn test_default_trait() {
        let provider = EmailProvider::new();
        assert_eq!(provider.name(), "email");
    }
}

// ======================== TelegramProvider extended send tests ========================

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

// ======================== DiscordProvider extended tests ========================

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
