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

// Provider registration module - all registration logic organized by category
pub mod providers;

use std::sync::LazyLock;
use std::time::Duration;

use noti_core::ProviderRegistry;
use reqwest::Client;

/// Shared HTTP client for all providers (created once, reused).
///
/// Configured with connection pooling, keepalive, and timeouts for high-throughput production use:
/// - `pool_max_idle_per_host(8)`: limit idle connections per host to avoid resource exhaustion
/// - `tcp_keepalive(Duration::from_secs(30))`: detect half-open connections
/// - `tcp_nodelay(true)`: disable Nagle's algorithm for lower latency
/// - `connect_timeout(10s)`: fail fast when a provider host is unreachable
/// - `timeout(30s)`: total request timeout prevents indefinite hangs on slow providers
static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .pool_max_idle_per_host(8)
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("reqwest Client::builder should always succeed")
});

/// Register all built-in notification providers into the given registry.
///
/// Delegates to `providers::register_all_providers()` which organizes
/// all 120+ providers by category for maintainability.
pub fn register_all_providers(registry: &mut ProviderRegistry) {
    providers::register_all_providers(registry);
}
