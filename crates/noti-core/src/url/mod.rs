//! Notification URL parsing.
//!
//! The entry point is [`parse_notification_url`]. Each provider family's
//! parsing logic lives in a dedicated submodule under `providers/`, keeping
//! this file as a thin dispatch table rather than a single monolithic function.

mod helpers;
pub mod providers;

use crate::error::NotiError;
use crate::provider::ProviderConfig;
use helpers::parse_query;
use providers::{chat, email as email_providers, misc, push, sms};

/// Parsed result from a notification URL scheme.
#[derive(Debug, Clone)]
pub struct ParsedUrl {
    /// The provider scheme (e.g. "wecom", "slack", "tg").
    pub scheme: String,
    /// Extracted provider configuration from the URL.
    pub config: ProviderConfig,
}

/// Parse a notification URL scheme string into a provider name and config.
///
/// Supported formats:
/// - `wecom://<key>`
/// - `feishu://<hook_id>` or `feishu://<hook_id>?secret=<secret>`
/// - `slack://<token_a>/<token_b>/<token_c>`
/// - `tg://<bot_token>/<chat_id>`
/// - `discord://<webhook_id>/<webhook_token>`
/// - `smtp://<user>:<pass>@<host>:<port>?to=<recipient>`
/// - `webhook://<url>`
///
/// For a full list of schemes, see the `providers/` submodules.
pub fn parse_notification_url(input: &str) -> Result<ParsedUrl, NotiError> {
    let (scheme, rest) = input
        .split_once("://")
        .ok_or_else(|| NotiError::UrlParse(format!("missing '://' in URL: {input}")))?;

    let scheme = scheme.to_lowercase();

    let (path_part, query_params) = parse_query(rest);

    let mut config = ProviderConfig::new();

    // Apply query params first so per-scheme logic can override if needed.
    for (k, v) in &query_params {
        config.values.insert(k.clone(), v.clone());
    }

    // ── Dispatch ──────────────────────────────────────────────────────────
    match scheme.as_str() {
        // Chat / IM
        "wecom" => chat::parse_wecom(path_part, &mut config)?,
        "feishu" | "lark" => chat::parse_feishu(path_part, &mut config)?,
        "slack" => chat::parse_slack(path_part, &mut config)?,
        "tg" | "telegram" => chat::parse_telegram(path_part, &mut config)?,
        "discord" => chat::parse_discord(path_part, &mut config)?,
        "dingtalk" => chat::parse_dingtalk(path_part, &mut config)?,
        "teams" => chat::parse_teams(path_part, &mut config)?,
        "gchat" | "googlechat" => chat::parse_gchat(path_part, &mut config)?,
        "mattermost" => chat::parse_mattermost(path_part, &mut config)?,
        "rocketchat" => chat::parse_rocketchat(path_part, &mut config)?,
        "matrix" => chat::parse_matrix(path_part, &mut config)?,
        "zulip" => chat::parse_zulip(path_part, &mut config)?,
        "webex" => chat::parse_webex(path_part, &mut config)?,
        "line" => chat::parse_line(path_part, &mut config)?,
        "mastodon" | "toot" => chat::parse_mastodon(path_part, &mut config)?,
        "revolt" => chat::parse_revolt(path_part, &mut config)?,
        "ryver" => chat::parse_ryver(path_part, &mut config)?,
        "twist" => chat::parse_twist(path_part, &mut config)?,
        "flock" => chat::parse_flock(path_part, &mut config)?,
        "gitter" => chat::parse_gitter(path_part, &mut config)?,
        "guilded" => chat::parse_guilded(path_part, &mut config)?,
        "misskey" => chat::parse_misskey(path_part, &mut config)?,
        "nctalk" => chat::parse_nctalk(path_part, &mut config)?,
        "jira" => chat::parse_jira(path_part, &mut config)?,
        "workflows" | "workflow" | "powerautomate" => {
            chat::parse_workflows(path_part, &mut config)?
        }

        // Push notifications
        "pushover" => push::parse_pushover(path_part, &mut config)?,
        "ntfy" => push::parse_ntfy(path_part, &mut config)?,
        "gotify" => push::parse_gotify(path_part, &mut config)?,
        "bark" => push::parse_bark(path_part, &mut config)?,
        "pushdeer" => push::parse_pushdeer(path_part, &mut config)?,
        "serverchan" => push::parse_serverchan(path_part, &mut config)?,
        "pushbullet" => push::parse_pushbullet(path_part, &mut config)?,
        "simplepush" => push::parse_simplepush(path_part, &mut config)?,
        "notica" => push::parse_notica(path_part, &mut config)?,
        "prowl" => push::parse_prowl(path_part, &mut config)?,
        "join" => push::parse_join(path_part, &mut config)?,
        "pushsafer" => push::parse_pushsafer(path_part, &mut config)?,
        "onesignal" => push::parse_onesignal(path_part, &mut config)?,
        "push" => push::parse_techulus(path_part, &mut config)?,
        "pushy" => push::parse_pushy(path_part, &mut config)?,
        "chanify" => push::parse_chanify(path_part, &mut config)?,
        "pushplus" => push::parse_pushplus(path_part, &mut config)?,
        "wxpusher" => push::parse_wxpusher(path_part, &mut config)?,
        "fcm" => push::parse_fcm(path_part, &mut config)?,
        "pushjet" => push::parse_pushjet(path_part, &mut config)?,
        "pushme" => push::parse_pushme(path_part, &mut config)?,
        "pushcut" => push::parse_pushcut(path_part, &mut config)?,
        "spugpush" => push::parse_spugpush(path_part, &mut config)?,
        "bluesky" => push::parse_bluesky(path_part, &mut config)?,
        "boxcar" => push::parse_boxcar(path_part, &mut config)?,
        "streamlabs" => push::parse_streamlabs(path_part, &mut config)?,
        "lametric" => push::parse_lametric(path_part, &mut config)?,
        "lunasea" => push::parse_lunasea(path_part, &mut config)?,
        "notifiarr" => push::parse_notifiarr(path_part, &mut config)?,
        "twitter" | "x" => push::parse_twitter(path_part, &mut config)?,
        "statuspage" => push::parse_statuspage(path_part, &mut config)?,
        "dot" => push::parse_dot(path_part, &mut config)?,
        "fluxer" => push::parse_fluxer(path_part, &mut config)?,
        "napi" | "notificationapi" => push::parse_napi(path_part, &mut config)?,

        // SMS / telephony
        "twilio" => sms::parse_twilio(path_part, &mut config)?,
        "vonage" | "nexmo" => sms::parse_vonage(path_part, &mut config)?,
        "d7sms" => sms::parse_d7sms(path_part, &mut config)?,
        "sinch" => sms::parse_sinch(path_part, &mut config)?,
        "clickatell" => sms::parse_clickatell(path_part, &mut config)?,
        "bulksms" => sms::parse_bulksms(path_part, &mut config)?,
        "kavenegar" => sms::parse_kavenegar(path_part, &mut config)?,
        "msgbird" => sms::parse_messagebird(path_part, &mut config)?,
        "plivo" => sms::parse_plivo(path_part, &mut config)?,
        "burstsms" => sms::parse_burstsms(path_part, &mut config)?,
        "popcorn" => sms::parse_popcorn(path_part, &mut config)?,
        "clicksend" => sms::parse_clicksend(path_part, &mut config)?,
        "seven" | "sms77" => sms::parse_seven(path_part, &mut config)?,
        "smseagle" => sms::parse_smseagle(path_part, &mut config)?,
        "httpsms" => sms::parse_httpsms(path_part, &mut config)?,
        "msg91" => sms::parse_msg91(path_part, &mut config)?,
        "freemobile" => sms::parse_freemobile(path_part, &mut config)?,
        "46elks" => sms::parse_fortysixelks(path_part, &mut config)?,
        "bulkvs" => sms::parse_bulkvs(path_part, &mut config)?,
        "africastalking" => sms::parse_africas_talking(path_part, &mut config)?,
        "dapnet" => sms::parse_dapnet(path_part, &mut config)?,
        "sfr" => sms::parse_sfr(path_part, &mut config)?,
        "voipms" => sms::parse_voipms(path_part, &mut config)?,
        "smsmanager" => sms::parse_smsmanager(path_part, &mut config)?,
        "signal" => sms::parse_signal(path_part, &mut config)?,
        "whatsapp" => sms::parse_whatsapp(path_part, &mut config)?,
        "threema" => sms::parse_threema(path_part, &mut config)?,
        "mqtt" => sms::parse_mqtt(path_part, &mut config)?,
        "notifico" => sms::parse_notifico(path_part, &mut config)?,

        // Email
        "smtp" | "email" => email_providers::parse_smtp(path_part, &mut config)?,
        "mailgun" => email_providers::parse_mailgun(path_part, &mut config)?,
        "sendgrid" => email_providers::parse_sendgrid(path_part, &mut config)?,
        "sparkpost" => email_providers::parse_sparkpost(path_part, &mut config)?,
        "resend" => email_providers::parse_resend(path_part, &mut config)?,
        "brevo" | "sendinblue" => email_providers::parse_brevo(path_part, &mut config)?,
        "smtp2go" => email_providers::parse_smtp2go(path_part, &mut config)?,
        "sendpulse" => email_providers::parse_sendpulse(path_part, &mut config)?,
        "ses" => email_providers::parse_ses(path_part, &mut config)?,
        "sns" => email_providers::parse_sns(path_part, &mut config)?,
        "o365" | "outlook" => email_providers::parse_o365(path_part, &mut config)?,

        // Webhooks
        "webhook" | "http" | "https" => misc::parse_webhook(path_part, &scheme, input, &mut config),
        "json" => misc::parse_json_webhook(path_part, &mut config)?,
        "form" => misc::parse_form_webhook(path_part, &mut config)?,
        "xml" => misc::parse_xml_webhook(path_part, &mut config)?,

        // Automation & incident
        "opsgenie" => misc::parse_opsgenie(path_part, &mut config)?,
        "pagerduty" => misc::parse_pagerduty(path_part, &mut config)?,
        "pagertree" => misc::parse_pagertree(path_part, &mut config)?,
        "signl4" => misc::parse_signl4(path_part, &mut config)?,
        "victorops" | "splunk" => misc::parse_victorops(path_part, &mut config)?,
        "spike" => misc::parse_spike(path_part, &mut config)?,
        "ifttt" => misc::parse_ifttt(path_part, &mut config)?,

        // Misc / multi-platform
        "reddit" => misc::parse_reddit(path_part, &mut config)?,
        "apprise" => misc::parse_apprise(path_part, &mut config)?,
        "webpush" => misc::parse_webpush(path_part, &mut config)?,
        "hassio" | "homeassistant" => misc::parse_homeassistant(path_part, &mut config)?,
        "kodi" | "xbmc" => misc::parse_kodi(path_part, &mut config)?,
        "enigma2" | "e2" => misc::parse_enigma2(path_part, &mut config)?,
        "emby" => misc::parse_emby(path_part, &mut config)?,
        "jellyfin" => misc::parse_jellyfin(path_part, &mut config)?,
        "synology" => misc::parse_synology(path_part, &mut config)?,
        "ncloud" | "nextcloud" => misc::parse_nextcloud(path_part, &mut config)?,
        "growl" => misc::parse_growl(path_part, &mut config)?,
        "kumulos" => misc::parse_kumulos(path_part, &mut config)?,
        "parse" => misc::parse_parse(path_part, &mut config)?,
        "rsyslog" | "syslog" => misc::parse_rsyslog(path_part, &mut config)?,
        "pushed" => misc::parse_pushed(path_part, &mut config)?,

        _ => {
            return Err(NotiError::UrlParse(format!("unknown URL scheme: {scheme}")));
        }
    }

    // Normalize scheme aliases
    let normalized_scheme = match scheme.as_str() {
        "lark" => "feishu".to_string(),
        "telegram" => "tg".to_string(),
        "email" => "smtp".to_string(),
        "googlechat" => "gchat".to_string(),
        "nexmo" => "vonage".to_string(),
        "toot" => "mastodon".to_string(),
        "homeassistant" => "hassio".to_string(),
        "http" | "https" => "webhook".to_string(),
        "splunk" => "victorops".to_string(),
        "nextcloud" => "ncloud".to_string(),
        "sendinblue" => "brevo".to_string(),
        "sms77" => "seven".to_string(),
        "xbmc" => "kodi".to_string(),
        "outlook" => "o365".to_string(),
        "syslog" => "rsyslog".to_string(),
        "x" => "twitter".to_string(),
        "e2" => "enigma2".to_string(),
        "workflow" | "powerautomate" => "workflows".to_string(),
        "notificationapi" => "napi".to_string(),
        other => other.to_string(),
    };

    Ok(ParsedUrl {
        scheme: normalized_scheme,
        config,
    })
}

// end of file
