use noti_core::{
    Attachment, AttachmentKind, Message, MessageFormat, NotifyProvider, ProviderConfig,
};
use noti_providers::apprise::AppriseProvider;
use noti_providers::bluesky::BlueskyProvider;
use noti_providers::dingtalk::DingTalkProvider;
use noti_providers::discord::DiscordProvider;
use noti_providers::feishu::FeishuProvider;
use noti_providers::flock::FlockProvider;
use noti_providers::gitter::GitterProvider;
use noti_providers::googlechat::GoogleChatProvider;
use noti_providers::guilded::GuildedProvider;
use noti_providers::httpsms::HttpSmsProvider;
use noti_providers::misskey::MisskeyProvider;
use noti_providers::nctalk::NcTalkProvider;
use noti_providers::pushsafer::PushsaferProvider;
use noti_providers::revolt::RevoltProvider;
use noti_providers::rocketchat::RocketChatProvider;
use noti_providers::seven::SevenProvider;
use noti_providers::signal::SignalProvider;
use noti_providers::slack::SlackProvider;
use noti_providers::telegram::TelegramProvider;
use noti_providers::wecom::WeComProvider;
use noti_providers::zulip::ZulipProvider;
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

// --- Attachment tests ---

#[rstest]
fn test_attachment_from_path_image() {
    let attachment = Attachment::from_path("photo.png");
    assert_eq!(attachment.kind, AttachmentKind::Image);
    assert_eq!(attachment.effective_mime(), "image/png");
    assert_eq!(attachment.effective_file_name(), "photo.png");
}

#[rstest]
fn test_attachment_from_path_pdf() {
    let attachment = Attachment::from_path("report.pdf");
    assert_eq!(attachment.kind, AttachmentKind::File);
    assert_eq!(attachment.effective_mime(), "application/pdf");
}

#[rstest]
fn test_attachment_from_path_audio() {
    let attachment = Attachment::from_path("song.mp3");
    assert_eq!(attachment.kind, AttachmentKind::Audio);
    assert!(attachment.effective_mime().starts_with("audio/"));
}

#[rstest]
fn test_attachment_from_path_video() {
    let attachment = Attachment::from_path("clip.mp4");
    assert_eq!(attachment.kind, AttachmentKind::Video);
    assert!(attachment.effective_mime().starts_with("video/"));
}

#[rstest]
fn test_attachment_unknown_extension() {
    let attachment = Attachment::from_path("data.xyz123");
    assert_eq!(attachment.kind, AttachmentKind::File);
    assert_eq!(attachment.effective_mime(), "application/octet-stream");
}

#[rstest]
fn test_attachment_overrides() {
    let attachment = Attachment::from_path("photo.png")
        .with_mime("image/webp")
        .with_file_name("custom_name.webp");

    assert_eq!(attachment.effective_mime(), "image/webp");
    assert_eq!(attachment.effective_file_name(), "custom_name.webp");
    assert_eq!(attachment.kind, AttachmentKind::Image);
}

#[rstest]
fn test_message_with_attachment() {
    let msg = Message::text("check this out").with_file("image.jpg");

    assert!(msg.has_attachments());
    assert_eq!(msg.attachments.len(), 1);
    assert!(msg.first_image().is_some());
}

#[rstest]
fn test_message_without_attachment() {
    let msg = Message::text("just text");
    assert!(!msg.has_attachments());
    assert!(msg.first_image().is_none());
}

#[rstest]
fn test_message_multiple_attachments() {
    let msg = Message::text("files")
        .with_file("doc.pdf")
        .with_file("photo.png");

    assert_eq!(msg.attachments.len(), 2);
    assert!(msg.first_image().is_some());
    assert_eq!(msg.first_image().unwrap().kind, AttachmentKind::Image);
}

// --- Supports attachments tests ---

#[rstest]
fn test_providers_supports_attachments() {
    let client = Client::new();

    let telegram = TelegramProvider::new(client.clone());
    assert!(telegram.supports_attachments());

    let discord = DiscordProvider::new(client.clone());
    assert!(discord.supports_attachments());

    let slack = SlackProvider::new(client.clone());
    assert!(slack.supports_attachments());

    let wecom = WeComProvider::new(client.clone());
    assert!(wecom.supports_attachments());

    // New providers with attachment support
    let zulip = ZulipProvider::new(client.clone());
    assert!(zulip.supports_attachments());

    let revolt = RevoltProvider::new(client.clone());
    assert!(revolt.supports_attachments());

    let guilded = GuildedProvider::new(client.clone());
    assert!(guilded.supports_attachments());

    let misskey = MisskeyProvider::new(client.clone());
    assert!(misskey.supports_attachments());

    let signal = SignalProvider::new(client.clone());
    assert!(signal.supports_attachments());

    let bluesky = BlueskyProvider::new(client.clone());
    assert!(bluesky.supports_attachments());

    let apprise = AppriseProvider::new(client.clone());
    assert!(apprise.supports_attachments());

    let nctalk = NcTalkProvider::new(client.clone());
    assert!(nctalk.supports_attachments());

    let rocketchat = RocketChatProvider::new(client.clone());
    assert!(rocketchat.supports_attachments());

    // Providers with real attachment handling code
    let pushsafer = PushsaferProvider::new(client.clone());
    assert!(pushsafer.supports_attachments());

    let feishu = FeishuProvider::new(client.clone());
    assert!(feishu.supports_attachments());

    let dingtalk = DingTalkProvider::new(client.clone());
    assert!(dingtalk.supports_attachments());

    // Providers with newly added attachment support
    let googlechat = GoogleChatProvider::new(client.clone());
    assert!(googlechat.supports_attachments());

    let flock = FlockProvider::new(client.clone());
    assert!(flock.supports_attachments());

    let gitter = GitterProvider::new(client.clone());
    assert!(gitter.supports_attachments());

    let seven = SevenProvider::new(client.clone());
    assert!(seven.supports_attachments());

    let httpsms = HttpSmsProvider::new(client.clone());
    assert!(httpsms.supports_attachments());
}
