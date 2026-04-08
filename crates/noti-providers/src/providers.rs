//! Provider registration organized by category.
//!
//! This module contains the `register_all_providers()` function that registers
//! all notification providers into a `ProviderRegistry`. Providers are organized
//! by category for clarity:
//! - Chat / IM (22)
//! - Push notifications (22)
//! - SMS / telephony (29)
//! - Email (11)
//! - Webhooks (4)
//! - Automation / incident (7)
//! - Home automation (2)
//! - Media / cloud (6)
//! - Misc (23)
//!
//! Total: 126 providers

use noti_core::ProviderRegistry;
use reqwest::Client;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Category registration functions
// ─────────────────────────────────────────────────────────────────────────────

// Chat / IM providers
fn register_chat_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::wecom::WeComProvider::new(client.clone())));
    registry.register(Arc::new(crate::feishu::FeishuProvider::new(client.clone())));
    registry.register(Arc::new(crate::dingtalk::DingTalkProvider::new(client.clone())));
    registry.register(Arc::new(crate::slack::SlackProvider::new(client.clone())));
    registry.register(Arc::new(crate::telegram::TelegramProvider::new(client.clone())));
    registry.register(Arc::new(crate::discord::DiscordProvider::new(client.clone())));
    registry.register(Arc::new(crate::teams::TeamsProvider::new(client.clone())));
    registry.register(Arc::new(crate::googlechat::GoogleChatProvider::new(client.clone())));
    registry.register(Arc::new(crate::mattermost::MattermostProvider::new(client.clone())));
    registry.register(Arc::new(crate::rocketchat::RocketChatProvider::new(client.clone())));
    registry.register(Arc::new(crate::matrix::MatrixProvider::new(client.clone())));
    registry.register(Arc::new(crate::zulip::ZulipProvider::new(client.clone())));
    registry.register(Arc::new(crate::webex::WebexProvider::new(client.clone())));
    registry.register(Arc::new(crate::line::LineProvider::new(client.clone())));
    registry.register(Arc::new(crate::revolt::RevoltProvider::new(client.clone())));
    registry.register(Arc::new(crate::mastodon::MastodonProvider::new(client.clone())));
    registry.register(Arc::new(crate::ryver::RyverProvider::new(client.clone())));
    registry.register(Arc::new(crate::twist::TwistProvider::new(client.clone())));
    registry.register(Arc::new(crate::flock::FlockProvider::new(client.clone())));
    registry.register(Arc::new(crate::gitter::GitterProvider::new(client.clone())));
    // Gaming / community
    registry.register(Arc::new(crate::guilded::GuildedProvider::new(client.clone())));
    registry.register(Arc::new(crate::misskey::MisskeyProvider::new(client.clone())));
}

// Push notification providers
fn register_push_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::pushover::PushoverProvider::new(client.clone())));
    registry.register(Arc::new(crate::ntfy::NtfyProvider::new(client.clone())));
    registry.register(Arc::new(crate::gotify::GotifyProvider::new(client.clone())));
    registry.register(Arc::new(crate::bark::BarkProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushdeer::PushDeerProvider::new(client.clone())));
    registry.register(Arc::new(crate::serverchan::ServerChanProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushbullet::PushBulletProvider::new(client.clone())));
    registry.register(Arc::new(crate::simplepush::SimplePushProvider::new(client.clone())));
    registry.register(Arc::new(crate::notica::NoticaProvider::new(client.clone())));
    registry.register(Arc::new(crate::prowl::ProwlProvider::new(client.clone())));
    registry.register(Arc::new(crate::join::JoinProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushsafer::PushsaferProvider::new(client.clone())));
    registry.register(Arc::new(crate::onesignal::OneSignalProvider::new(client.clone())));
    registry.register(Arc::new(crate::techulus::TechulusProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushy::PushyProvider::new(client.clone())));
    registry.register(Arc::new(crate::chanify::ChanifyProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushplus::PushplusProvider::new(client.clone())));
    registry.register(Arc::new(crate::wxpusher::WxPusherProvider::new(client.clone())));
    registry.register(Arc::new(crate::fcm::FcmProvider::new(client.clone())));
    registry.register(Arc::new(crate::apns::ApnsProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushjet::PushjetProvider::new(client.clone())));
    // Social
    registry.register(Arc::new(crate::bluesky::BlueskyProvider::new(client.clone())));
}

// SMS / telephony providers
fn register_sms_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::twilio::TwilioProvider::new(client.clone())));
    registry.register(Arc::new(crate::vonage::VonageProvider::new(client.clone())));
    registry.register(Arc::new(crate::d7networks::D7NetworksProvider::new(client.clone())));
    registry.register(Arc::new(crate::sinch::SinchProvider::new(client.clone())));
    registry.register(Arc::new(crate::clickatell::ClickatellProvider::new(client.clone())));
    registry.register(Arc::new(crate::bulksms::BulkSmsProvider::new(client.clone())));
    registry.register(Arc::new(crate::kavenegar::KavenegarProvider::new(client.clone())));
    registry.register(Arc::new(crate::messagebird::MessageBirdProvider::new(client.clone())));
    registry.register(Arc::new(crate::plivo::PlivoProvider::new(client.clone())));
    registry.register(Arc::new(crate::burstsms::BurstSmsProvider::new(client.clone())));
    registry.register(Arc::new(crate::popcorn::PopcornProvider::new(client.clone())));
    registry.register(Arc::new(crate::clicksend::ClickSendProvider::new(client.clone())));
    registry.register(Arc::new(crate::seven::SevenProvider::new(client.clone())));
    registry.register(Arc::new(crate::smseagle::SmsEagleProvider::new(client.clone())));
    registry.register(Arc::new(crate::httpsms::HttpSmsProvider::new(client.clone())));
    registry.register(Arc::new(crate::msg91::Msg91Provider::new(client.clone())));
    registry.register(Arc::new(crate::freemobile::FreeMobileProvider::new(client.clone())));
    registry.register(Arc::new(crate::fortysixelks::FortySixElksProvider::new(client.clone())));
    registry.register(Arc::new(crate::bulkvs::BulkVsProvider::new(client.clone())));
    registry.register(Arc::new(crate::africas_talking::AfricasTalkingProvider::new(client.clone())));
    registry.register(Arc::new(crate::dapnet::DapnetProvider::new(client.clone())));
    registry.register(Arc::new(crate::sfr::SfrProvider::new(client.clone())));
    registry.register(Arc::new(crate::voipms::VoipMsProvider::new(client.clone())));
    registry.register(Arc::new(crate::smsmanager::SmsManagerProvider::new(client.clone())));
    registry.register(Arc::new(crate::signal::SignalProvider::new(client.clone())));
    registry.register(Arc::new(crate::whatsapp::WhatsAppProvider::new(client.clone())));
    registry.register(Arc::new(crate::threema::ThreemaProvider::new(client.clone())));
    registry.register(Arc::new(crate::mqtt::MqttProvider::new(client.clone())));
    registry.register(Arc::new(crate::notifico::NotificoProvider::new(client.clone())));
}

// Email providers
fn register_email_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::email::EmailProvider::new()));
    registry.register(Arc::new(crate::mailgun::MailgunProvider::new(client.clone())));
    registry.register(Arc::new(crate::sendgrid::SendGridProvider::new(client.clone())));
    registry.register(Arc::new(crate::sparkpost::SparkPostProvider::new(client.clone())));
    registry.register(Arc::new(crate::ses::SesProvider::new(client.clone())));
    registry.register(Arc::new(crate::sns::SnsProvider::new(client.clone())));
    registry.register(Arc::new(crate::resend::ResendProvider::new(client.clone())));
    registry.register(Arc::new(crate::brevo::BrevoProvider::new(client.clone())));
    registry.register(Arc::new(crate::smtp2go::Smtp2GoProvider::new(client.clone())));
    registry.register(Arc::new(crate::sendpulse::SendPulseProvider::new(client.clone())));
    registry.register(Arc::new(crate::o365::O365Provider::new(client.clone())));
}

// Webhook providers
fn register_webhook_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::webhook::WebhookProvider::new(client.clone())));
    registry.register(Arc::new(crate::json_webhook::JsonWebhookProvider::new(client.clone())));
    registry.register(Arc::new(crate::form_webhook::FormWebhookProvider::new(client.clone())));
    registry.register(Arc::new(crate::xml_webhook::XmlWebhookProvider::new(client.clone())));
}

// Automation & incident platforms
fn register_automation_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::ifttt::IftttProvider::new(client.clone())));
    registry.register(Arc::new(crate::pagerduty::PagerDutyProvider::new(client.clone())));
    registry.register(Arc::new(crate::opsgenie::OpsgenieProvider::new(client.clone())));
    registry.register(Arc::new(crate::pagertree::PagerTreeProvider::new(client.clone())));
    registry.register(Arc::new(crate::signl4::Signl4Provider::new(client.clone())));
    registry.register(Arc::new(crate::victorops::VictorOpsProvider::new(client.clone())));
    registry.register(Arc::new(crate::spike::SpikeProvider::new(client.clone())));
}

// Home automation & IoT
fn register_home_automation_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::homeassistant::HomeAssistantProvider::new(client.clone())));
    registry.register(Arc::new(crate::lametric::LaMetricProvider::new(client.clone())));
}

// Media / cloud services
fn register_media_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::lunasea::LunaseaProvider::new(client.clone())));
    registry.register(Arc::new(crate::nextcloud::NextcloudProvider::new(client.clone())));
    registry.register(Arc::new(crate::emby::EmbyProvider::new(client.clone())));
    registry.register(Arc::new(crate::jellyfin::JellyfinProvider::new(client.clone())));
    registry.register(Arc::new(crate::synology::SynologyProvider::new(client.clone())));
    registry.register(Arc::new(crate::streamlabs::StreamlabsProvider::new(client.clone())));
}

// Misc / multi-platform
fn register_misc_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(crate::reddit::RedditProvider::new(client.clone())));
    registry.register(Arc::new(crate::apprise::AppriseProvider::new(client.clone())));
    registry.register(Arc::new(crate::webpush::WebPushProvider::new(client.clone())));
    registry.register(Arc::new(crate::kodi::KodiProvider::new(client.clone())));
    registry.register(Arc::new(crate::jira::JiraProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushcut::PushcutProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushme::PushMeProvider::new(client.clone())));
    registry.register(Arc::new(crate::pushed::PushedProvider::new(client.clone())));
    registry.register(Arc::new(crate::growl::GrowlProvider::new(client.clone())));
    registry.register(Arc::new(crate::kumulos::KumulosProvider::new(client.clone())));
    registry.register(Arc::new(crate::parse::ParseProvider::new(client.clone())));
    registry.register(Arc::new(crate::rsyslog::RsyslogProvider::new(client.clone())));
    registry.register(Arc::new(crate::twitter::TwitterProvider::new(client.clone())));
    registry.register(Arc::new(crate::boxcar::BoxcarProvider::new(client.clone())));
    registry.register(Arc::new(crate::enigma2::Enigma2Provider::new(client.clone())));
    registry.register(Arc::new(crate::notifiarr::NotifiarrProvider::new(client.clone())));
    registry.register(Arc::new(crate::statuspage::StatuspageProvider::new(client.clone())));
    registry.register(Arc::new(crate::dot::DotProvider::new(client.clone())));
    registry.register(Arc::new(crate::fluxer::FluxerProvider::new(client.clone())));
    registry.register(Arc::new(crate::workflows::WorkflowsProvider::new(client.clone())));
    registry.register(Arc::new(crate::notification_api::NotificationApiProvider::new(client.clone())));
    registry.register(Arc::new(crate::spugpush::SpugPushProvider::new(client.clone())));
    registry.register(Arc::new(crate::nctalk::NcTalkProvider::new(client.clone())));
}

// ─────────────────────────────────────────────────────────────────────────────
// Main registration function
// ─────────────────────────────────────────────────────────────────────────────

/// Register all built-in notification providers into the given registry.
///
/// All providers use the shared HTTP client for connection pooling and timeouts.
pub fn register_all_providers(registry: &mut ProviderRegistry) {
    let client: &Client = &crate::HTTP_CLIENT;
    register_chat_providers(registry, client);
    register_push_providers(registry, client);
    register_sms_providers(registry, client);
    register_email_providers(registry, client);
    register_webhook_providers(registry, client);
    register_automation_providers(registry, client);
    register_home_automation_providers(registry, client);
    register_media_providers(registry, client);
    register_misc_providers(registry, client);
}

#[cfg(test)]
mod tests {
    use super::*;
    use noti_core::ProviderRegistry;

    #[test]
    fn test_all_providers_registered() {
        let mut registry = ProviderRegistry::new();
        register_all_providers(&mut registry);

        let providers = registry.all_providers();

        // Should have all 120+ providers registered
        assert!(
            providers.len() > 120,
            "Expected >120 providers, got {}",
            providers.len()
        );

        // Verify some well-known providers are registered
        let provider_names: Vec<_> = providers.iter().map(|p| p.name()).collect();
        assert!(provider_names.contains(&"slack"), "slack should be registered");
        assert!(provider_names.contains(&"wecom"), "wecom should be registered");
        assert!(provider_names.contains(&"twilio"), "twilio should be registered");
        assert!(provider_names.contains(&"email"), "email should be registered");
        assert!(provider_names.contains(&"webhook"), "webhook should be registered");
    }
}
