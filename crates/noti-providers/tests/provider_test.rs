use noti_core::{
    Attachment, AttachmentKind, Message, MessageFormat, NotifyProvider, ProviderConfig,
};
use noti_providers::apprise::AppriseProvider;
use noti_providers::bluesky::BlueskyProvider;
use noti_providers::boxcar::BoxcarProvider;
use noti_providers::discord::DiscordProvider;
use noti_providers::growl::GrowlProvider;
use noti_providers::guilded::GuildedProvider;
use noti_providers::ifttt::IftttProvider;
use noti_providers::kodi::KodiProvider;
use noti_providers::kumulos::KumulosProvider;
use noti_providers::lunasea::LunaseaProvider;
use noti_providers::misskey::MisskeyProvider;
use noti_providers::nctalk::NcTalkProvider;
use noti_providers::nextcloud::NextcloudProvider;
use noti_providers::pagertree::PagerTreeProvider;
use noti_providers::parse::ParseProvider;
use noti_providers::prowl::ProwlProvider;
use noti_providers::pushed::PushedProvider;
use noti_providers::reddit::RedditProvider;
use noti_providers::revolt::RevoltProvider;
use noti_providers::rocketchat::RocketChatProvider;
use noti_providers::signal::SignalProvider;
use noti_providers::slack::SlackProvider;
use noti_providers::smseagle::SmsEagleProvider;
use noti_providers::spike::SpikeProvider;
use noti_providers::spugpush::SpugPushProvider;
use noti_providers::techulus::TechulusProvider;
use noti_providers::telegram::TelegramProvider;
use noti_providers::twitter::TwitterProvider;
use noti_providers::victorops::VictorOpsProvider;
use noti_providers::voipms::VoipMsProvider;
use noti_providers::webpush::WebPushProvider;
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

    // Newly added providers with attachment support
    let twitter = TwitterProvider::new(client.clone());
    assert!(twitter.supports_attachments());

    let webpush = WebPushProvider::new(client.clone());
    assert!(webpush.supports_attachments());

    let ifttt = IftttProvider::new(client.clone());
    assert!(ifttt.supports_attachments());

    let boxcar = BoxcarProvider::new(client.clone());
    assert!(boxcar.supports_attachments());

    let kumulos = KumulosProvider::new(client.clone());
    assert!(kumulos.supports_attachments());

    let kodi = KodiProvider::new(client.clone());
    assert!(kodi.supports_attachments());

    let lunasea = LunaseaProvider::new(client.clone());
    assert!(lunasea.supports_attachments());

    let pushed = PushedProvider::new(client.clone());
    assert!(pushed.supports_attachments());

    let victorops = VictorOpsProvider::new(client.clone());
    assert!(victorops.supports_attachments());

    let pagertree = PagerTreeProvider::new(client.clone());
    assert!(pagertree.supports_attachments());

    let spike = SpikeProvider::new(client.clone());
    assert!(spike.supports_attachments());

    let reddit = RedditProvider::new(client.clone());
    assert!(reddit.supports_attachments());

    let parse = ParseProvider::new(client.clone());
    assert!(parse.supports_attachments());

    let nextcloud = NextcloudProvider::new(client.clone());
    assert!(nextcloud.supports_attachments());

    let prowl = ProwlProvider::new(client.clone());
    assert!(prowl.supports_attachments());

    let spugpush = SpugPushProvider::new(client.clone());
    assert!(spugpush.supports_attachments());

    // SMSEagle MMS, VoIP.ms MMS, Techulus image_url, Growl icon
    let smseagle = SmsEagleProvider::new(client.clone());
    assert!(smseagle.supports_attachments());

    let voipms = VoipMsProvider::new(client.clone());
    assert!(voipms.supports_attachments());

    let techulus = TechulusProvider::new(client.clone());
    assert!(techulus.supports_attachments());

    let growl = GrowlProvider::new(client.clone());
    assert!(growl.supports_attachments());
}
