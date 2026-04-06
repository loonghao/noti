pub mod africas_talking;
pub mod apns;
pub mod apprise;
pub mod bark;
pub mod bluesky;
pub mod boxcar;
pub mod brevo;
pub mod bulksms;
pub mod bulkvs;
pub mod burstsms;
pub mod chanify;
pub mod clickatell;
pub mod clicksend;
pub mod d7networks;
pub mod dapnet;
pub mod dingtalk;
pub mod discord;
pub mod dot;
pub mod email;
pub mod emby;
pub mod enigma2;
pub mod fcm;
pub mod feishu;
pub mod flock;
pub mod fluxer;
pub mod form_webhook;
pub mod fortysixelks;
pub mod freemobile;
pub mod gitter;
pub mod googlechat;
pub mod gotify;
pub mod growl;
pub mod guilded;
pub mod homeassistant;
pub mod httpsms;
pub mod ifttt;
pub mod jellyfin;
pub mod jira;
pub mod join;
pub mod json_webhook;
pub mod kavenegar;
pub mod kodi;
pub mod kumulos;
pub mod lametric;
pub mod line;
pub mod lunasea;
pub mod mailgun;
pub mod mastodon;
pub mod matrix;
pub mod mattermost;
pub mod messagebird;
pub mod misskey;
pub mod mqtt;
pub mod msg91;
pub mod nctalk;
pub mod nextcloud;
pub mod notica;
pub mod notifiarr;
pub mod notification_api;
pub mod notifico;
pub mod ntfy;
pub mod o365;
pub mod onesignal;
pub mod opsgenie;
pub mod pagerduty;
pub mod pagertree;
pub mod parse;
pub mod plivo;
pub mod popcorn;
pub mod prowl;
pub mod pushbullet;
pub mod pushcut;
pub mod pushdeer;
pub mod pushed;
pub mod pushjet;
pub mod pushme;
pub mod pushover;
pub mod pushplus;
pub mod pushsafer;
pub mod pushy;
pub mod reddit;
pub mod resend;
pub mod revolt;
pub mod rocketchat;
pub mod rsyslog;
pub mod ryver;
pub mod sendgrid;
pub mod sendpulse;
pub mod serverchan;
pub mod ses;
pub mod seven;
pub mod sfr;
pub mod signal;
pub mod signl4;
pub mod simplepush;
pub mod sinch;
pub mod slack;
pub mod smseagle;
pub mod smsmanager;
pub mod smtp2go;
pub mod sns;
pub mod sparkpost;
pub mod spike;
pub mod spugpush;
pub mod statuspage;
pub mod streamlabs;
pub mod synology;
pub mod teams;
pub mod techulus;
pub mod telegram;
pub mod threema;
pub mod twilio;
pub mod twist;
pub mod twitter;
pub mod victorops;
pub mod voipms;
pub mod vonage;
pub mod webex;
pub mod webhook;
pub mod webpush;
pub mod wecom;
pub mod whatsapp;
pub mod workflows;
pub mod wxpusher;
pub mod xml_webhook;
pub mod zulip;

use noti_core::ProviderRegistry;
use reqwest::Client;
use std::sync::Arc;
use std::sync::LazyLock;

/// Shared HTTP client for all providers (created once, reused)
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

/// Register all built-in notification providers into the given registry.
pub fn register_all_providers(registry: &mut ProviderRegistry) {
    let client: &Client = &HTTP_CLIENT;
    register_chat_im_providers(registry, client);
    register_gaming_community_providers(registry, client);
    register_social_providers(registry, client);
    register_push_notification_providers(registry, client);
    register_automation_incident_providers(registry, client);
    register_sms_providers(registry, client);
    register_email_providers(registry, client);
    register_webhook_providers(registry, client);
    register_home_automation_providers(registry, client);
    register_media_cloud_providers(registry, client);
    register_secure_messaging_providers(registry, client);
    register_misc_providers(registry, client);
    register_iteration_7_providers(registry, client);
    register_iteration_8_providers(registry, client);
    register_iteration_9_providers(registry, client);
    register_iteration_10_providers(registry, client);
}

// Category: Chat / IM providers (20)
fn register_chat_im_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(wecom::WeComProvider::new(client.clone())));
    registry.register(Arc::new(feishu::FeishuProvider::new(client.clone())));
    registry.register(Arc::new(dingtalk::DingTalkProvider::new(client.clone())));
    registry.register(Arc::new(slack::SlackProvider::new(client.clone())));
    registry.register(Arc::new(telegram::TelegramProvider::new(client.clone())));
    registry.register(Arc::new(discord::DiscordProvider::new(client.clone())));
    registry.register(Arc::new(teams::TeamsProvider::new(client.clone())));
    registry.register(Arc::new(googlechat::GoogleChatProvider::new(client.clone())));
    registry.register(Arc::new(mattermost::MattermostProvider::new(client.clone())));
    registry.register(Arc::new(rocketchat::RocketChatProvider::new(client.clone())));
    registry.register(Arc::new(matrix::MatrixProvider::new(client.clone())));
    registry.register(Arc::new(zulip::ZulipProvider::new(client.clone())));
    registry.register(Arc::new(webex::WebexProvider::new(client.clone())));
    registry.register(Arc::new(line::LineProvider::new(client.clone())));
    registry.register(Arc::new(revolt::RevoltProvider::new(client.clone())));
    registry.register(Arc::new(mastodon::MastodonProvider::new(client.clone())));
    registry.register(Arc::new(ryver::RyverProvider::new(client.clone())));
    registry.register(Arc::new(twist::TwistProvider::new(client.clone())));
    registry.register(Arc::new(flock::FlockProvider::new(client.clone())));
    registry.register(Arc::new(gitter::GitterProvider::new(client.clone())));
}

// Category: Gaming / community chat (2)
fn register_gaming_community_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(guilded::GuildedProvider::new(client.clone())));
    registry.register(Arc::new(misskey::MisskeyProvider::new(client.clone())));
}

// Category: Social networks (1)
fn register_social_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(bluesky::BlueskyProvider::new(client.clone())));
}

// Category: Push notification providers (20)
fn register_push_notification_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(pushover::PushoverProvider::new(client.clone())));
    registry.register(Arc::new(ntfy::NtfyProvider::new(client.clone())));
    registry.register(Arc::new(gotify::GotifyProvider::new(client.clone())));
    registry.register(Arc::new(bark::BarkProvider::new(client.clone())));
    registry.register(Arc::new(pushdeer::PushDeerProvider::new(client.clone())));
    registry.register(Arc::new(serverchan::ServerChanProvider::new(client.clone())));
    registry.register(Arc::new(pushbullet::PushBulletProvider::new(client.clone())));
    registry.register(Arc::new(simplepush::SimplePushProvider::new(client.clone())));
    registry.register(Arc::new(notica::NoticaProvider::new(client.clone())));
    registry.register(Arc::new(prowl::ProwlProvider::new(client.clone())));
    registry.register(Arc::new(join::JoinProvider::new(client.clone())));
    registry.register(Arc::new(pushsafer::PushsaferProvider::new(client.clone())));
    registry.register(Arc::new(onesignal::OneSignalProvider::new(client.clone())));
    registry.register(Arc::new(techulus::TechulusProvider::new(client.clone())));
    registry.register(Arc::new(pushy::PushyProvider::new(client.clone())));
    registry.register(Arc::new(chanify::ChanifyProvider::new(client.clone())));
    registry.register(Arc::new(pushplus::PushplusProvider::new(client.clone())));
    registry.register(Arc::new(wxpusher::WxPusherProvider::new(client.clone())));
    registry.register(Arc::new(fcm::FcmProvider::new(client.clone())));
    registry.register(Arc::new(apns::ApnsProvider::new(client.clone())));
    registry.register(Arc::new(pushjet::PushjetProvider::new(client.clone())));
}

// Category: Automation & incident platforms (7)
fn register_automation_incident_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(ifttt::IftttProvider::new(client.clone())));
    registry.register(Arc::new(pagerduty::PagerDutyProvider::new(client.clone())));
    registry.register(Arc::new(opsgenie::OpsgenieProvider::new(client.clone())));
    registry.register(Arc::new(pagertree::PagerTreeProvider::new(client.clone())));
    registry.register(Arc::new(signl4::Signl4Provider::new(client.clone())));
    registry.register(Arc::new(victorops::VictorOpsProvider::new(client.clone())));
    registry.register(Arc::new(spike::SpikeProvider::new(client.clone())));
}

// Category: SMS providers (17)
fn register_sms_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(twilio::TwilioProvider::new(client.clone())));
    registry.register(Arc::new(vonage::VonageProvider::new(client.clone())));
    registry.register(Arc::new(d7networks::D7NetworksProvider::new(client.clone())));
    registry.register(Arc::new(sinch::SinchProvider::new(client.clone())));
    registry.register(Arc::new(clickatell::ClickatellProvider::new(client.clone())));
    registry.register(Arc::new(bulksms::BulkSmsProvider::new(client.clone())));
    registry.register(Arc::new(kavenegar::KavenegarProvider::new(client.clone())));
    registry.register(Arc::new(messagebird::MessageBirdProvider::new(client.clone())));
    registry.register(Arc::new(plivo::PlivoProvider::new(client.clone())));
    registry.register(Arc::new(burstsms::BurstSmsProvider::new(client.clone())));
    registry.register(Arc::new(popcorn::PopcornProvider::new(client.clone())));
    registry.register(Arc::new(clicksend::ClickSendProvider::new(client.clone())));
    registry.register(Arc::new(seven::SevenProvider::new(client.clone())));
    registry.register(Arc::new(smseagle::SmsEagleProvider::new(client.clone())));
    registry.register(Arc::new(httpsms::HttpSmsProvider::new(client.clone())));
    registry.register(Arc::new(msg91::Msg91Provider::new(client.clone())));
    registry.register(Arc::new(freemobile::FreeMobileProvider::new(client.clone())));
}

// Category: Email providers (8)
fn register_email_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(email::EmailProvider::new()));
    registry.register(Arc::new(mailgun::MailgunProvider::new(client.clone())));
    registry.register(Arc::new(sendgrid::SendGridProvider::new(client.clone())));
    registry.register(Arc::new(sparkpost::SparkPostProvider::new(client.clone())));
    registry.register(Arc::new(ses::SesProvider::new(client.clone())));
    registry.register(Arc::new(sns::SnsProvider::new(client.clone())));
    registry.register(Arc::new(resend::ResendProvider::new(client.clone())));
    registry.register(Arc::new(brevo::BrevoProvider::new(client.clone())));
    registry.register(Arc::new(smtp2go::Smtp2GoProvider::new(client.clone())));
}

// Category: Webhook providers (4)
fn register_webhook_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(webhook::WebhookProvider::new(client.clone())));
    registry.register(Arc::new(json_webhook::JsonWebhookProvider::new(client.clone())));
    registry.register(Arc::new(form_webhook::FormWebhookProvider::new(client.clone())));
    registry.register(Arc::new(xml_webhook::XmlWebhookProvider::new(client.clone())));
}

// Category: Home automation & IoT (2)
fn register_home_automation_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(homeassistant::HomeAssistantProvider::new(client.clone())));
    registry.register(Arc::new(lametric::LaMetricProvider::new(client.clone())));
}

// Category: Self-hosted media / cloud (2)
fn register_media_cloud_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(lunasea::LunaseaProvider::new(client.clone())));
    registry.register(Arc::new(nextcloud::NextcloudProvider::new(client.clone())));
}

// Category: Secure messaging (2)
fn register_secure_messaging_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(threema::ThreemaProvider::new(client.clone())));
    registry.register(Arc::new(signal::SignalProvider::new(client.clone())));
}

// Category: Misc (3)
fn register_misc_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(reddit::RedditProvider::new(client.clone())));
    registry.register(Arc::new(apprise::AppriseProvider::new(client.clone())));
    registry.register(Arc::new(webpush::WebPushProvider::new(client.clone())));
}

// Category: New providers — Iteration 7 (13)
fn register_iteration_7_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(whatsapp::WhatsAppProvider::new(client.clone())));
    registry.register(Arc::new(kodi::KodiProvider::new(client.clone())));
    registry.register(Arc::new(notifico::NotificoProvider::new(client.clone())));
    registry.register(Arc::new(fortysixelks::FortySixElksProvider::new(client.clone())));
    registry.register(Arc::new(bulkvs::BulkVsProvider::new(client.clone())));
    registry.register(Arc::new(jira::JiraProvider::new(client.clone())));
    registry.register(Arc::new(pushme::PushMeProvider::new(client.clone())));
    registry.register(Arc::new(sendpulse::SendPulseProvider::new(client.clone())));
    registry.register(Arc::new(streamlabs::StreamlabsProvider::new(client.clone())));
    registry.register(Arc::new(synology::SynologyProvider::new(client.clone())));
    registry.register(Arc::new(africas_talking::AfricasTalkingProvider::new(client.clone())));
    registry.register(Arc::new(o365::O365Provider::new(client.clone())));
    registry.register(Arc::new(nctalk::NcTalkProvider::new(client.clone())));
}

// Category: New providers — Iteration 8 (13)
fn register_iteration_8_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(emby::EmbyProvider::new(client.clone())));
    registry.register(Arc::new(jellyfin::JellyfinProvider::new(client.clone())));
    registry.register(Arc::new(pushcut::PushcutProvider::new(client.clone())));
    registry.register(Arc::new(mqtt::MqttProvider::new(client.clone())));
    registry.register(Arc::new(voipms::VoipMsProvider::new(client.clone())));
    registry.register(Arc::new(sfr::SfrProvider::new(client.clone())));
    registry.register(Arc::new(pushed::PushedProvider::new(client.clone())));
    registry.register(Arc::new(growl::GrowlProvider::new(client.clone())));
    registry.register(Arc::new(kumulos::KumulosProvider::new(client.clone())));
    registry.register(Arc::new(parse::ParseProvider::new(client.clone())));
    registry.register(Arc::new(rsyslog::RsyslogProvider::new(client.clone())));
    registry.register(Arc::new(smsmanager::SmsManagerProvider::new(client.clone())));
    registry.register(Arc::new(twitter::TwitterProvider::new(client.clone())));
}

// Category: New providers — Iteration 9 (5)
fn register_iteration_9_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(boxcar::BoxcarProvider::new(client.clone())));
    registry.register(Arc::new(dapnet::DapnetProvider::new(client.clone())));
    registry.register(Arc::new(enigma2::Enigma2Provider::new(client.clone())));
    registry.register(Arc::new(notifiarr::NotifiarrProvider::new(client.clone())));
    registry.register(Arc::new(statuspage::StatuspageProvider::new(client.clone())));
}

// Category: New providers — Iteration 10 (5)
fn register_iteration_10_providers(registry: &mut ProviderRegistry, client: &Client) {
    registry.register(Arc::new(dot::DotProvider::new(client.clone())));
    registry.register(Arc::new(fluxer::FluxerProvider::new(client.clone())));
    registry.register(Arc::new(workflows::WorkflowsProvider::new(client.clone())));
    registry.register(Arc::new(notification_api::NotificationApiProvider::new(client.clone())));
    registry.register(Arc::new(spugpush::SpugPushProvider::new(client.clone())));
}
