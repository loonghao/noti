use noti_core::{Message, MessageFormat, NotifyProvider, ProviderConfig};
use noti_providers::discord::DiscordProvider;
use noti_providers::slack::SlackProvider;
use noti_providers::wecom::WeComProvider;
use reqwest::Client;
use rstest::rstest;

#[tokio::test]
async fn test_wecom_validate_config() {
    let client = Client::new();
    let provider = WeComProvider::new(client);

    let config = ProviderConfig::new().set("key", "test-key");
    assert!(provider.validate_config(&config).is_ok());

    let bad_config = ProviderConfig::new();
    assert!(provider.validate_config(&bad_config).is_err());
}

#[tokio::test]
async fn test_slack_validate_config() {
    let client = Client::new();
    let provider = SlackProvider::new(client);

    let config = ProviderConfig::new().set("webhook_url", "https://hooks.slack.com/services/T/B/x");
    assert!(provider.validate_config(&config).is_ok());

    let bad_config = ProviderConfig::new();
    assert!(provider.validate_config(&bad_config).is_err());
}

#[tokio::test]
async fn test_discord_validate_config() {
    let client = Client::new();
    let provider = DiscordProvider::new(client);

    let config = ProviderConfig::new()
        .set("webhook_id", "123456")
        .set("webhook_token", "abcdef");
    assert!(provider.validate_config(&config).is_ok());

    let missing_token = ProviderConfig::new().set("webhook_id", "123456");
    assert!(provider.validate_config(&missing_token).is_err());
}

#[rstest]
fn test_message_builder() {
    let msg = Message::text("hello")
        .with_title("Test Title")
        .with_format(MessageFormat::Markdown);

    assert_eq!(msg.text, "hello");
    assert_eq!(msg.title, Some("Test Title".to_string()));
    assert_eq!(msg.format, MessageFormat::Markdown);
}

#[rstest]
fn test_provider_config_builder() {
    let config = ProviderConfig::new()
        .set("key", "value")
        .set("key2", "value2");

    assert_eq!(config.get("key"), Some("value"));
    assert_eq!(config.get("key2"), Some("value2"));
    assert_eq!(config.get("nonexistent"), None);
    assert!(config.require("key", "test").is_ok());
    assert!(config.require("nonexistent", "test").is_err());
}
