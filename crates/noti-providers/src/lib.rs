pub mod africas_talking;
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
///
/// The table below is the single authoritative list of providers. To add a
/// new provider, append one line — `register!(ModulePath::TypeName)` for
/// client-bearing providers, or `register_no_client!(TypeName)` for the rare
/// cases where the constructor takes no arguments.
pub fn register_all_providers(registry: &mut ProviderRegistry) {
    let client: &Client = &HTTP_CLIENT;

    // Macro: register a provider whose `new(client)` takes a cloned client.
    macro_rules! register {
        ($ctor:expr) => {
            registry.register(Arc::new($ctor(client.clone())));
        };
    }

    // Macro: register a provider whose `new()` takes no arguments.
    macro_rules! register_no_client {
        ($ctor:expr) => {
            registry.register(Arc::new($ctor()));
        };
    }

    // ── Chat / IM ──────────────────────────────────────────────────────────
    register!(wecom::WeComProvider::new);
    register!(feishu::FeishuProvider::new);
    register!(dingtalk::DingTalkProvider::new);
    register!(slack::SlackProvider::new);
    register!(telegram::TelegramProvider::new);
    register!(discord::DiscordProvider::new);
    register!(teams::TeamsProvider::new);
    register!(googlechat::GoogleChatProvider::new);
    register!(mattermost::MattermostProvider::new);
    register!(rocketchat::RocketChatProvider::new);
    register!(matrix::MatrixProvider::new);
    register!(zulip::ZulipProvider::new);
    register!(webex::WebexProvider::new);
    register!(line::LineProvider::new);
    register!(revolt::RevoltProvider::new);
    register!(mastodon::MastodonProvider::new);
    register!(ryver::RyverProvider::new);
    register!(twist::TwistProvider::new);
    register!(flock::FlockProvider::new);
    register!(gitter::GitterProvider::new);

    // ── Gaming / community chat ────────────────────────────────────────────
    register!(guilded::GuildedProvider::new);
    register!(misskey::MisskeyProvider::new);

    // ── Social networks ────────────────────────────────────────────────────
    register!(bluesky::BlueskyProvider::new);

    // ── Push notifications ─────────────────────────────────────────────────
    register!(pushover::PushoverProvider::new);
    register!(ntfy::NtfyProvider::new);
    register!(gotify::GotifyProvider::new);
    register!(bark::BarkProvider::new);
    register!(pushdeer::PushDeerProvider::new);
    register!(serverchan::ServerChanProvider::new);
    register!(pushbullet::PushBulletProvider::new);
    register!(simplepush::SimplePushProvider::new);
    register!(notica::NoticaProvider::new);
    register!(prowl::ProwlProvider::new);
    register!(join::JoinProvider::new);
    register!(pushsafer::PushsaferProvider::new);
    register!(onesignal::OneSignalProvider::new);
    register!(techulus::TechulusProvider::new);
    register!(pushy::PushyProvider::new);
    register!(chanify::ChanifyProvider::new);
    register!(pushplus::PushplusProvider::new);
    register!(wxpusher::WxPusherProvider::new);
    register!(fcm::FcmProvider::new);
    register!(pushjet::PushjetProvider::new);

    // ── Automation & incident platforms ───────────────────────────────────
    register!(ifttt::IftttProvider::new);
    register!(pagerduty::PagerDutyProvider::new);
    register!(opsgenie::OpsgenieProvider::new);
    register!(pagertree::PagerTreeProvider::new);
    register!(signl4::Signl4Provider::new);
    register!(victorops::VictorOpsProvider::new);
    register!(spike::SpikeProvider::new);

    // ── SMS providers ──────────────────────────────────────────────────────
    register!(twilio::TwilioProvider::new);
    register!(vonage::VonageProvider::new);
    register!(d7networks::D7NetworksProvider::new);
    register!(sinch::SinchProvider::new);
    register!(clickatell::ClickatellProvider::new);
    register!(bulksms::BulkSmsProvider::new);
    register!(kavenegar::KavenegarProvider::new);
    register!(messagebird::MessageBirdProvider::new);
    register!(plivo::PlivoProvider::new);
    register!(burstsms::BurstSmsProvider::new);
    register!(popcorn::PopcornProvider::new);
    register!(clicksend::ClickSendProvider::new);
    register!(seven::SevenProvider::new);
    register!(smseagle::SmsEagleProvider::new);
    register!(httpsms::HttpSmsProvider::new);
    register!(msg91::Msg91Provider::new);
    register!(freemobile::FreeMobileProvider::new);

    // ── Email providers ────────────────────────────────────────────────────
    register_no_client!(email::EmailProvider::new);
    register!(mailgun::MailgunProvider::new);
    register!(sendgrid::SendGridProvider::new);
    register!(sparkpost::SparkPostProvider::new);
    register!(ses::SesProvider::new);
    register!(sns::SnsProvider::new);
    register!(resend::ResendProvider::new);
    register!(brevo::BrevoProvider::new);
    register!(smtp2go::Smtp2GoProvider::new);

    // ── Webhook providers ──────────────────────────────────────────────────
    register!(webhook::WebhookProvider::new);
    register!(json_webhook::JsonWebhookProvider::new);
    register!(form_webhook::FormWebhookProvider::new);
    register!(xml_webhook::XmlWebhookProvider::new);

    // ── Home automation & IoT ──────────────────────────────────────────────
    register!(homeassistant::HomeAssistantProvider::new);
    register!(lametric::LaMetricProvider::new);

    // ── Self-hosted media / cloud ──────────────────────────────────────────
    register!(lunasea::LunaseaProvider::new);
    register!(nextcloud::NextcloudProvider::new);

    // ── Secure messaging ───────────────────────────────────────────────────
    register!(threema::ThreemaProvider::new);
    register!(signal::SignalProvider::new);

    // ── Misc ───────────────────────────────────────────────────────────────
    register!(reddit::RedditProvider::new);
    register!(apprise::AppriseProvider::new);
    register!(webpush::WebPushProvider::new);

    // ── Community / multi-platform ─────────────────────────────────────────
    register!(whatsapp::WhatsAppProvider::new);
    register!(kodi::KodiProvider::new);
    register!(notifico::NotificoProvider::new);
    register!(fortysixelks::FortySixElksProvider::new);
    register!(bulkvs::BulkVsProvider::new);
    register!(jira::JiraProvider::new);
    register!(pushme::PushMeProvider::new);
    register!(sendpulse::SendPulseProvider::new);
    register!(streamlabs::StreamlabsProvider::new);
    register!(synology::SynologyProvider::new);
    register!(africas_talking::AfricasTalkingProvider::new);
    register!(o365::O365Provider::new);
    register!(nctalk::NcTalkProvider::new);
    register!(emby::EmbyProvider::new);
    register!(jellyfin::JellyfinProvider::new);
    register!(pushcut::PushcutProvider::new);
    register!(mqtt::MqttProvider::new);
    register!(voipms::VoipMsProvider::new);
    register!(sfr::SfrProvider::new);
    register!(pushed::PushedProvider::new);
    register!(growl::GrowlProvider::new);
    register!(kumulos::KumulosProvider::new);
    register!(parse::ParseProvider::new);
    register!(rsyslog::RsyslogProvider::new);
    register!(smsmanager::SmsManagerProvider::new);
    register!(twitter::TwitterProvider::new);
    register!(boxcar::BoxcarProvider::new);
    register!(dapnet::DapnetProvider::new);
    register!(enigma2::Enigma2Provider::new);
    register!(notifiarr::NotifiarrProvider::new);
    register!(statuspage::StatuspageProvider::new);
    register!(dot::DotProvider::new);
    register!(fluxer::FluxerProvider::new);
    register!(workflows::WorkflowsProvider::new);
    register!(notification_api::NotificationApiProvider::new);
    register!(spugpush::SpugPushProvider::new);
}
