use crate::error::NotiError;
use crate::provider::ProviderConfig;
use std::collections::HashMap;

/// Parsed result from a notification URL scheme.
#[derive(Debug, Clone)]
pub struct ParsedUrl {
    /// The provider scheme (e.g. "wecom", "slack", "tg").
    pub scheme: String,
    /// Extracted provider configuration from the URL.
    pub config: ProviderConfig,
}

/// Normalize a scheme alias to its canonical name.
fn normalize_scheme(scheme: &str) -> String {
    match scheme {
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
    }
}

/// Return an error if path_part is empty.
fn require_non_empty(path: &str, scheme: &str, desc: &str) -> Result<(), NotiError> {
    if path.is_empty() {
        Err(NotiError::UrlParse(format!("{scheme}:// requires {desc}")))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Scheme parsers — one function per scheme family
// ---------------------------------------------------------------------------

/// Parse `wecom://<key>`
fn parse_wecom(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "wecom", "a webhook key")?;
    config.values.insert("key".into(), path_part.to_string());
    Ok(())
}

/// Parse `feishu://<hook_id>?secret=<optional>`
fn parse_feishu(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "feishu", "a hook ID")?;
    config.values.insert("hook_id".into(), path_part.to_string());
    Ok(())
}

/// Parse `slack://<token_a>/<token_b>/<token_c>` or `slack://<full_webhook_url>`
fn parse_slack(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.len() == 3 {
        let webhook_url = format!(
            "https://hooks.slack.com/services/{}/{}/{}",
            parts[0], parts[1], parts[2]
        );
        config.values.insert("webhook_url".into(), webhook_url);
        Ok(())
    } else if parts.len() == 1 && !parts[0].is_empty() {
        config.values.insert("webhook_url".into(), path_part.to_string());
        Ok(())
    } else {
        Err(NotiError::UrlParse(
            "slack:// requires <token_a>/<token_b>/<token_c>".into(),
        ))
    }
}

/// Parse `tg://<bot_token>/<chat_id>` (alias: telegram)
fn parse_telegram(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "tg:// requires <bot_token>/<chat_id>".into(),
        ));
    }
    config.values.insert("bot_token".into(), parts[0].to_string());
    config.values.insert("chat_id".into(), parts[1].to_string());
    Ok(())
}

/// Parse `discord://<webhook_id>/<webhook_token>`
fn parse_discord(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "discord:// requires <webhook_id>/<webhook_token>".into(),
        ));
    }
    config.values.insert("webhook_id".into(), parts[0].to_string());
    config.values.insert("webhook_token".into(), parts[1].to_string());
    Ok(())
}

/// Parse `smtp://<user>:<pass>@<host>:<port>?to=<recipient>` (alias: email)
fn parse_smtp(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    parse_smtp_url(path_part, config)
}

/// Parse `dingtalk://<access_token>?secret=<optional>`
fn parse_dingtalk(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "dingtalk", "an access token")?;
    config.values.insert("access_token".into(), path_part.to_string());
    Ok(())
}

/// Parse `pushover://<user_key>/<api_token>`
fn parse_pushover(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "pushover:// requires <user_key>/<api_token>".into(),
        ));
    }
    config.values.insert("user_key".into(), parts[0].to_string());
    config.values.insert("api_token".into(), parts[1].to_string());
    Ok(())
}

/// Parse `ntfy://<topic>?server=<optional>`
fn parse_ntfy(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "ntfy", "a topic name")?;
    config.values.insert("topic".into(), path_part.to_string());
    Ok(())
}

/// Parse `gotify://<host>/<app_token>`
fn parse_gotify(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "gotify:// requires <host>/<app_token>".into(),
        ));
    }
    config.values.insert("host".into(), format!("https://{}", parts[0]));
    config.values.insert("app_token".into(), parts[1].to_string());
    Ok(())
}

/// Parse `bark://<device_key>?server=<optional>`
fn parse_bark(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "bark", "a device key")?;
    config.values.insert("device_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `pushdeer://<push_key>?server=<optional>`
fn parse_pushdeer(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pushdeer", "a push key")?;
    config.values.insert("push_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `serverchan://<send_key>`
fn parse_serverchan(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "serverchan", "a send key")?;
    config.values.insert("send_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `teams://<webhook_url_host>/<path...>`
fn parse_teams(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "teams", "a webhook URL")?;
    let webhook_url = format!("https://{path_part}");
    config.values.insert("webhook_url".into(), webhook_url);
    Ok(())
}

/// Parse `gchat://<space>/<key>/<token>` (alias: googlechat)
fn parse_gchat(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.len() == 3 {
        let webhook_url = format!(
            "https://chat.googleapis.com/v1/spaces/{}/messages?key={}&token={}",
            parts[0], parts[1], parts[2]
        );
        config.values.insert("webhook_url".into(), webhook_url);
        Ok(())
    } else if parts.len() == 1 && !parts[0].is_empty() {
        let webhook_url = format!("https://{path_part}");
        config.values.insert("webhook_url".into(), webhook_url);
        Ok(())
    } else {
        Err(NotiError::UrlParse(
            "gchat:// requires <space>/<key>/<token>".into(),
        ))
    }
}

/// Parse `mattermost://<host>/<hook_id>`
fn parse_mattermost(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "mattermost:// requires <host>/<hook_id>".into(),
        ));
    }
    config.values.insert("host".into(), parts[0].to_string());
    config.values.insert("hook_id".into(), parts[1].to_string());
    Ok(())
}

/// Parse `rocketchat://<host>/<token_a>/<token_b>`
fn parse_rocketchat(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.len() != 3 {
        return Err(NotiError::UrlParse(
            "rocketchat:// requires <host>/<token_a>/<token_b>".into(),
        ));
    }
    config.values.insert("host".into(), parts[0].to_string());
    config.values.insert("token_a".into(), parts[1].to_string());
    config.values.insert("token_b".into(), parts[2].to_string());
    Ok(())
}

/// Parse `matrix://<access_token>/<room_id>?server=<optional>`
fn parse_matrix(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "matrix:// requires <access_token>/<room_id>".into(),
        ));
    }
    config.values.insert("access_token".into(), parts[0].to_string());
    config.values.insert("room_id".into(), parts[1].to_string());
    Ok(())
}

/// Parse `twilio://<account_sid>:<auth_token>@<from>/<to>`
fn parse_twilio(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part
        .split_once('@')
        .ok_or_else(|| NotiError::UrlParse("twilio:// requires <account_sid>:<auth_token>@...".into()))?;
    let (sid, token) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("twilio:// requires <account_sid>:<auth_token>@...".into())
    })?;
    config.values.insert("account_sid".into(), sid.to_string());
    config.values.insert("auth_token".into(), token.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "twilio:// requires <from_number>/<to_number> after @".into(),
        ));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `zulip://<bot_email>:<api_key>@<domain>/<stream>/<topic>`
fn parse_zulip(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("zulip:// requires <bot_email>:<api_key>@<domain>".into())
    })?;
    let (email, key) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("zulip:// requires <bot_email>:<api_key>@...".into())
    })?;
    config.values.insert("bot_email".into(), email.to_string());
    config.values.insert("api_key".into(), key.to_string());
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("zulip:// requires a domain after @".into()));
    }
    config.values.insert("domain".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("stream".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        config.values.insert("topic".into(), parts[2].to_string());
    }
    Ok(())
}

/// Parse `pushbullet://<access_token>`
fn parse_pushbullet(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pushbullet", "an access token")?;
    config.values.insert("access_token".into(), path_part.to_string());
    Ok(())
}

/// Parse `ifttt://<webhook_key>/<event_name>`
fn parse_ifttt(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "ifttt:// requires <webhook_key>/<event_name>".into(),
        ));
    }
    config.values.insert("webhook_key".into(), parts[0].to_string());
    config.values.insert("event".into(), parts[1].to_string());
    Ok(())
}

/// Parse `simplepush://<key>`
fn parse_simplepush(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "simplepush", "a key")?;
    config.values.insert("key".into(), path_part.to_string());
    Ok(())
}

/// Parse `mailgun://<api_key>@<domain>/<to>`
fn parse_mailgun(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("mailgun:// requires <api_key>@<domain>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("mailgun:// requires a domain after @".into()));
    }
    config.values.insert("domain".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `webex://<access_token>/<room_id>`
#[allow(dead_code)]
fn parse_webex(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "webex:// requires <access_token>/<room_id>".into(),
        ));
    }
    config.values.insert("access_token".into(), parts[0].to_string());
    config.values.insert("room_id".into(), parts[1].to_string());
    Ok(())
}

/// Parse `line://<access_token>`
fn parse_line(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "line", "an access token")?;
    config.values.insert("access_token".into(), path_part.to_string());
    Ok(())
}

/// Parse `vonage://<api_key>:<api_secret>@<from>/<to>` (alias: nexmo)
fn parse_vonage(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("vonage:// requires <api_key>:<api_secret>@...".into())
    })?;
    let (key, secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("vonage:// requires <api_key>:<api_secret>@...".into())
    })?;
    config.values.insert("api_key".into(), key.to_string());
    config.values.insert("api_secret".into(), secret.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse("vonage:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `pagerduty://<integration_key>`
fn parse_pagerduty(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pagerduty", "an integration key")?;
    config.values.insert("integration_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `revolt://<bot_token>/<channel_id>`
fn parse_revolt(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "revolt:// requires <bot_token>/<channel_id>".into(),
        ));
    }
    config.values.insert("bot_token".into(), parts[0].to_string());
    config.values.insert("channel_id".into(), parts[1].to_string());
    Ok(())
}

/// Parse `opsgenie://<api_key>?region=<optional>`
fn parse_opsgenie(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "opsgenie", "an API key")?;
    config.values.insert("api_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `sendgrid://<api_key>@<from_email>/<to_email>`
fn parse_sendgrid(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("sendgrid:// requires <api_key>@<from>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("sendgrid:// requires a from email after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `notica://<token>`
fn parse_notica(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "notica", "a token")?;
    config.values.insert("token".into(), path_part.to_string());
    Ok(())
}

/// Parse `mastodon://<access_token>@<instance>` (alias: toot)
fn parse_mastodon(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (token, instance) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("mastodon:// requires <access_token>@<instance>".into())
    })?;
    config.values.insert("access_token".into(), token.to_string());
    config.values.insert("instance".into(), instance.to_string());
    Ok(())
}

/// Parse `json://<host>/<path>` (also `form://`, `xml://`)
fn parse_generic_url(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "generic", "a URL")?;
    let full_url = format!("https://{path_part}");
    config.values.insert("url".into(), full_url);
    Ok(())
}

/// Parse `prowl://<api_key>`
fn parse_prowl(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "prowl", "an API key")?;
    config.values.insert("api_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `join://<api_key>/<device_id>`
fn parse_join(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("join:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("device_id".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `pushsafer://<private_key>`
fn parse_pushsafer(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pushsafer", "a private key")?;
    config.values.insert("private_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `hassio://<access_token>@<host>` (alias: homeassistant)
fn parse_hassio(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (token, host) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("hassio:// requires <access_token>@<host>".into())
    })?;
    config.values.insert("access_token".into(), token.to_string());
    config.values.insert("host".into(), host.to_string());
    Ok(())
}

/// Parse `signal://<from>/<to>?server=<optional>`
fn parse_signal(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "signal:// requires <from_number>/<to_number>".into(),
        ));
    }
    config.values.insert("from".into(), parts[0].to_string());
    config.values.insert("to".into(), parts[1].to_string());
    Ok(())
}

/// Parse `sparkpost://<api_key>@<from_email>/<to_email>`
fn parse_sparkpost(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("sparkpost:// requires <api_key>@<from>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("sparkpost:// requires a from email after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `sns://<access_key>:<secret_key>@<region>/<topic_arn>`
fn parse_sns(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("sns:// requires <access_key>:<secret_key>@<region>/<topic_arn>".into())
    })?;
    let (key, secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("sns:// requires <access_key>:<secret_key>@...".into())
    })?;
    config.values.insert("access_key".into(), key.to_string());
    config.values.insert("secret_key".into(), secret.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("sns:// requires a region after @".into()));
    }
    config.values.insert("region".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("topic_arn".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `ses://<access_key>:<secret_key>@<region>/<from>/<to>`
fn parse_ses(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("ses:// requires <access_key>:<secret_key>@<region>/<from>/<to>".into())
    })?;
    let (key, secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("ses:// requires <access_key>:<secret_key>@...".into())
    })?;
    config.values.insert("access_key".into(), key.to_string());
    config.values.insert("secret_key".into(), secret.to_string());
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("ses:// requires a region after @".into()));
    }
    config.values.insert("region".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("from".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        config.values.insert("to".into(), parts[2].to_string());
    }
    Ok(())
}

/// Parse `d7sms://<api_token>@<from>/<to>`
fn parse_d7sms(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, rest)) = path_part.split_once('@') {
        config.values.insert("api_token".into(), token.to_string());
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            config.values.insert("from".into(), parts[0].to_string());
        }
        if parts.len() > 1 {
            config.values.insert("to".into(), parts[1].to_string());
        }
    } else {
        require_non_empty(path_part, "d7sms", "an API token")?;
        config.values.insert("api_token".into(), path_part.to_string());
    }
    Ok(())
}

/// Parse `sinch://<service_plan_id>:<api_token>@<from>/<to>`
fn parse_sinch(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("sinch:// requires <service_plan_id>:<api_token>@<from>/<to>".into())
    })?;
    let (plan_id, token) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("sinch:// requires <service_plan_id>:<api_token>@...".into())
    })?;
    config.values.insert("service_plan_id".into(), plan_id.to_string());
    config.values.insert("api_token".into(), token.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse("sinch:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `clickatell://<api_key>/<to>`
fn parse_clickatell(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("clickatell:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `bulksms://<token_id>:<token_secret>@<from>/<to>`
fn parse_bulksms(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("bulksms:// requires <token_id>:<token_secret>@...".into())
    })?;
    let (id, secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("bulksms:// requires <token_id>:<token_secret>@...".into())
    })?;
    config.values.insert("token_id".into(), id.to_string());
    config.values.insert("token_secret".into(), secret.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse("bulksms:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `kavenegar://<api_key>/<from>/<to>`
fn parse_kavenegar(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("kavenegar:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("from".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        config.values.insert("to".into(), parts[2].to_string());
    }
    Ok(())
}

/// Parse `lametric://<api_key>@<host>`
fn parse_lametric(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, host) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("lametric:// requires <api_key>@<host>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    config.values.insert("host".into(), host.to_string());
    Ok(())
}

/// Parse `lunasea://<user_token>`
fn parse_lunasea(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "lunasea", "a user token")?;
    config.values.insert("user_token".into(), path_part.to_string());
    Ok(())
}

/// Parse `onesignal://<app_id>:<api_key>`
fn parse_onesignal(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (app_id, api_key) = path_part.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("onesignal:// requires <app_id>:<api_key>".into())
    })?;
    config.values.insert("app_id".into(), app_id.to_string());
    config.values.insert("api_key".into(), api_key.to_string());
    Ok(())
}

/// Parse `push://<api_key>` (Techulus Push)
fn parse_techulus_push(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "push", "an API key")?;
    config.values.insert("api_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `pushy://<api_key>/<device_token>`
fn parse_pushy(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("pushy:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("device_token".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `reddit://<client_id>:<client_secret>@<user>:<password>/<to>`
fn parse_reddit(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("reddit:// requires <client_id>:<client_secret>@...".into())
    })?;
    let (client_id, client_secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("reddit:// requires <client_id>:<client_secret>@...".into())
    })?;
    config.values.insert("client_id".into(), client_id.to_string());
    config.values.insert("client_secret".into(), client_secret.to_string());
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("reddit:// requires user:password after @".into()));
    }
    if let Some((user, password)) = parts[0].split_once(':') {
        config.values.insert("user".into(), user.to_string());
        config.values.insert("password".into(), password.to_string());
    } else {
        config.values.insert("user".into(), parts[0].to_string());
    }
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `ryver://<organization>/<token>`
fn parse_ryver(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "ryver:// requires <organization>/<token>".into(),
        ));
    }
    config.values.insert("organization".into(), parts[0].to_string());
    config.values.insert("token".into(), parts[1].to_string());
    Ok(())
}

/// Parse `twist://<token_a>/<token_b>/<token_c>`
fn parse_twist(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "twist", "webhook URL components")?;
    let webhook_url = format!("https://twist.com/api/v3/integration_incoming/post_data?{path_part}");
    config.values.insert("webhook_url".into(), webhook_url);
    Ok(())
}

/// Parse `flock://<token>`
fn parse_flock(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "flock", "a webhook token")?;
    config.values.insert("token".into(), path_part.to_string());
    Ok(())
}

/// Parse `guilded://<webhook_id>/<webhook_token>`
fn parse_guilded(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "guilded:// requires <webhook_id>/<webhook_token>".into(),
        ));
    }
    config.values.insert("webhook_id".into(), parts[0].to_string());
    config.values.insert("webhook_token".into(), parts[1].to_string());
    Ok(())
}

/// Parse `misskey://<access_token>@<instance>`
fn parse_misskey(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (token, instance) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("misskey:// requires <access_token>@<instance>".into())
    })?;
    config.values.insert("access_token".into(), token.to_string());
    config.values.insert("instance".into(), instance.to_string());
    Ok(())
}

/// Parse `chanify://<token>` or `chanify://<token>@<host>`
fn parse_chanify(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, host)) = path_part.split_once('@') {
        config.values.insert("token".into(), token.to_string());
        config.values.insert("server".into(), format!("https://{host}"));
    } else {
        require_non_empty(path_part, "chanify", "a device token")?;
        config.values.insert("token".into(), path_part.to_string());
    }
    Ok(())
}

/// Parse `pushplus://<token>`
fn parse_pushplus(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pushplus", "a user token")?;
    config.values.insert("token".into(), path_part.to_string());
    Ok(())
}

/// Parse `wxpusher://<app_token>/<uid>`
fn parse_wxpusher(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "wxpusher:// requires <app_token>/<uid>".into(),
        ));
    }
    config.values.insert("app_token".into(), parts[0].to_string());
    config.values.insert("uid".into(), parts[1].to_string());
    Ok(())
}

/// Parse `resend://<api_key>@<from>/<to>`
fn parse_resend(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("resend:// requires <api_key>@<from>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("resend:// requires a from email after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `bluesky://<handle>:<app_password>`
fn parse_bluesky(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (handle, app_password) = path_part.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("bluesky:// requires <handle>:<app_password>".into())
    })?;
    config.values.insert("handle".into(), handle.to_string());
    config.values.insert("app_password".into(), app_password.to_string());
    Ok(())
}

/// Parse `msgbird://<access_key>@<from>/<to>`
fn parse_msgbird(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (access_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("msgbird:// requires <access_key>@<from>/<to>".into())
    })?;
    config.values.insert("access_key".into(), access_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("msgbird:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `plivo://<auth_id>:<auth_token>@<from>/<to>`
fn parse_plivo(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("plivo:// requires <auth_id>:<auth_token>@<from>/<to>".into())
    })?;
    let (auth_id, auth_token) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("plivo:// requires <auth_id>:<auth_token>@...".into())
    })?;
    config.values.insert("auth_id".into(), auth_id.to_string());
    config.values.insert("auth_token".into(), auth_token.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse("plivo:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `burstsms://<api_key>:<api_secret>@<from>/<to>`
fn parse_burstsms(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("burstsms:// requires <api_key>:<api_secret>@...".into())
    })?;
    let (api_key, api_secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("burstsms:// requires <api_key>:<api_secret>@...".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    config.values.insert("api_secret".into(), api_secret.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse("burstsms:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `gitter://<token>/<room_id>`
fn parse_gitter(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "gitter:// requires <token>/<room_id>".into(),
        ));
    }
    config.values.insert("token".into(), parts[0].to_string());
    config.values.insert("room_id".into(), parts[1].to_string());
    Ok(())
}

/// Parse `ncloud://<user>:<password>@<host>/<target_user>` (alias: nextcloud)
fn parse_ncloud(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("ncloud:// requires <user>:<password>@<host>".into())
    })?;
    let (user, password) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("ncloud:// requires <user>:<password>@...".into())
    })?;
    config.values.insert("user".into(), user.to_string());
    config.values.insert("password".into(), password.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("ncloud:// requires a host after @".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("target_user".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `pagertree://<integration_id>`
fn parse_pagertree(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pagertree", "an integration ID")?;
    config.values.insert("integration_id".into(), path_part.to_string());
    Ok(())
}

/// Parse `signl4://<team_secret>`
fn parse_signl4(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "signl4", "a team secret")?;
    config.values.insert("team_secret".into(), path_part.to_string());
    Ok(())
}

/// Parse `victorops://<api_key>/<routing_key>` (alias: splunk)
fn parse_victorops(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "victorops:// requires <api_key>/<routing_key>".into(),
        ));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    config.values.insert("routing_key".into(), parts[1].to_string());
    Ok(())
}

/// Parse `spike://<webhook_url_path>`
fn parse_spike(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "spike", "a webhook URL")?;
    let webhook_url = format!("https://{path_part}");
    config.values.insert("webhook_url".into(), webhook_url);
    Ok(())
}

/// Parse `popcorn://<api_key>@<from>/<to>`
fn parse_popcorn(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("popcorn:// requires <api_key>@<from>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("popcorn:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `fcm://<server_key>/<device_token>`
fn parse_fcm(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("fcm:// requires a server key".into()));
    }
    config.values.insert("server_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("device_token".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `threema://<gateway_id>:<api_secret>@<recipient_id>`
fn parse_threema(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, recipient) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("threema:// requires <gateway_id>:<api_secret>@<recipient_id>".into())
    })?;
    let (gw_id, secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("threema:// requires <gateway_id>:<api_secret>@...".into())
    })?;
    config.values.insert("gateway_id".into(), gw_id.to_string());
    config.values.insert("api_secret".into(), secret.to_string());
    if !recipient.is_empty() {
        config.values.insert("to".into(), recipient.to_string());
    }
    Ok(())
}

/// Parse `clicksend://<username>:<api_key>@<from>/<to>`
fn parse_clicksend(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("clicksend:// requires <username>:<api_key>@<from>/<to>".into())
    })?;
    let (user, key) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("clicksend:// requires <username>:<api_key>@...".into())
    })?;
    config.values.insert("username".into(), user.to_string());
    config.values.insert("api_key".into(), key.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if !phone_parts.is_empty() && !phone_parts[0].is_empty() {
        config.values.insert("from".into(), phone_parts[0].to_string());
    }
    if phone_parts.len() > 1 {
        config.values.insert("to".into(), phone_parts[1].to_string());
    }
    Ok(())
}

/// Parse `brevo://<api_key>@<from_email>/<to_email>` (alias: sendinblue)
fn parse_brevo(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("brevo:// requires <api_key>@<from>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("brevo:// requires a from email after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `smtp2go://<api_key>@<from_email>/<to_email>`
fn parse_smtp2go(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("smtp2go:// requires <api_key>@<from>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("smtp2go:// requires a from email after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `apprise://<host>/<config_key>`
fn parse_apprise(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "apprise", "a host")?;
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    config.values.insert("host".into(), format!("https://{}", parts[0]));
    if parts.len() > 1 && !parts[1].is_empty() {
        config.values.insert("config_key".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `freemobile://<user_id>/<api_key>`
fn parse_freemobile(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "freemobile:// requires <user_id>/<api_key>".into(),
        ));
    }
    config.values.insert("user".into(), parts[0].to_string());
    config.values.insert("password".into(), parts[1].to_string());
    Ok(())
}

/// Parse `httpsms://<api_key>@<from>/<to>`
fn parse_httpsms(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("httpsms:// requires <api_key>@<from>/<to>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("httpsms:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `msg91://<authkey>/<sender_id>/<to>`
fn parse_msg91(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("msg91:// requires an authkey".into()));
    }
    config.values.insert("authkey".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("sender".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        config.values.insert("to".into(), parts[2].to_string());
    }
    Ok(())
}

/// Parse `pushjet://<secret_key>`
fn parse_pushjet(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pushjet", "a secret key")?;
    config.values.insert("secret".into(), path_part.to_string());
    Ok(())
}

/// Parse `smseagle://<access_token>@<host>/<to>`
fn parse_smseagle(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (token, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("smseagle:// requires <access_token>@<host>/<to>".into())
    })?;
    config.values.insert("access_token".into(), token.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("smseagle:// requires a host after @".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `seven://<api_key>/<to>` (alias: sms77)
fn parse_seven(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("seven:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `webpush://<endpoint_encoded>`
fn parse_webpush(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "webpush", "an endpoint URL")?;
    let endpoint = format!("https://{path_part}");
    config.values.insert("endpoint".into(), endpoint);
    Ok(())
}

/// Parse `whatsapp://<access_token>@<phone_number_id>/<to>`
fn parse_whatsapp(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (token, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("whatsapp:// requires <access_token>@<phone_number_id>/<to>".into())
    })?;
    config.values.insert("access_token".into(), token.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse(
            "whatsapp:// requires a phone_number_id after @".into(),
        ))
    }
    config.values.insert("phone_number_id".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `kodi://<host>` or `kodi://<user>:<password>@<host>:<port>` (alias: xbmc)
fn parse_kodi(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, host_part_local)) = path_part.split_once('@') {
        if let Some((user, password)) = auth.split_once(':') {
            config.values.insert("user".into(), user.to_string());
            config.values.insert("password".into(), password.to_string());
        } else {
            config.values.insert("user".into(), auth.to_string());
        }
        if let Some((host, port)) = host_part_local.split_once(':') {
            config.values.insert("host".into(), host.to_string());
            config.values.insert("port".into(), port.to_string());
        } else {
            config.values.insert("host".into(), host_part_local.to_string());
        }
        Ok(())
    } else if path_part.is_empty() {
        Err(NotiError::UrlParse("kodi:// requires a host".into()))
    } else if let Some((host, port)) = path_part.split_once(':') {
        config.values.insert("host".into(), host.to_string());
        config.values.insert("port".into(), port.to_string());
        Ok(())
    } else {
        config.values.insert("host".into(), path_part.to_string());
        Ok(())
    }
}

/// Parse `notifico://<project_id>/<msghook>`
fn parse_notifico(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "notifico:// requires <project_id>/<msghook>".into(),
        ));
    }
    config.values.insert("project_id".into(), parts[0].to_string());
    config.values.insert("msghook".into(), parts[1].to_string());
    Ok(())
}

/// Parse `46elks://<api_username>:<api_password>@<from>/<to>`
fn parse_46elks(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("46elks:// requires <api_username>:<api_password>@<from>/<to>".into())
    })?;
    let (user, pass) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("46elks:// requires <api_username>:<api_password>@...".into())
    })?;
    config.values.insert("api_username".into(), user.to_string());
    config.values.insert("api_password".into(), pass.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse("46elks:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `bulkvs://<username>:<password>@<from>/<to>`
fn parse_bulkvs(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, phone_part) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("bulkvs:// requires <username>:<password>@<from>/<to>".into())
    })?;
    let (user, pass) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("bulkvs:// requires <username>:<password>@...".into())
    })?;
    config.values.insert("username".into(), user.to_string());
    config.values.insert("password".into(), pass.to_string());
    let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
    if phone_parts.len() != 2 {
        return Err(NotiError::UrlParse("bulkvs:// requires <from>/<to> after @".into()));
    }
    config.values.insert("from".into(), phone_parts[0].to_string());
    config.values.insert("to".into(), phone_parts[1].to_string());
    Ok(())
}

/// Parse `jira://<user>:<api_token>@<host>/<issue_key>`
fn parse_jira(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("jira:// requires <user>:<api_token>@<host>/<issue_key>".into())
    })?;
    let (user, token) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("jira:// requires <user>:<api_token>@...".into())
    })?;
    config.values.insert("user".into(), user.to_string());
    config.values.insert("api_token".into(), token.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("jira:// requires a host after @".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("issue_key".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `pushme://<push_key>`
fn parse_pushme(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "pushme", "a push key")?;
    config.values.insert("push_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `sendpulse://<client_id>:<client_secret>@<from>/<to>`
fn parse_sendpulse(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("sendpulse:// requires <client_id>:<client_secret>@<from>/<to>".into())
    })?;
    let (id, secret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("sendpulse:// requires <client_id>:<client_secret>@...".into())
    })?;
    config.values.insert("client_id".into(), id.to_string());
    config.values.insert("client_secret".into(), secret.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("sendpulse:// requires a from email after @".into()));
    }
    config.values.insert("from".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `streamlabs://<access_token>`
fn parse_streamlabs(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "streamlabs", "an access token")?;
    config.values.insert("access_token".into(), path_part.to_string());
    Ok(())
}

/// Parse `synology://<token>@<host>`
fn parse_synology(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (token, host) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("synology:// requires <token>@<host>".into())
    })?;
    config.values.insert("token".into(), token.to_string());
    if let Some((h, port)) = host.split_once(':') {
        config.values.insert("host".into(), h.to_string());
        config.values.insert("port".into(), port.to_string());
    } else {
        config.values.insert("host".into(), host.to_string());
    }
    Ok(())
}

/// Parse `africastalking://<username>:<api_key>@<to>`
fn parse_africastalking(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, to) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("africastalking:// requires <username>:<api_key>@<to>".into())
    })?;
    let (user, key) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("africastalking:// requires <username>:<api_key>@...".into())
    })?;
    config.values.insert("username".into(), user.to_string());
    config.values.insert("api_key".into(), key.to_string());
    if !to.is_empty() {
        config.values.insert("to".into(), to.to_string());
    }
    Ok(())
}

/// Parse `o365://<client_id>:<client_secret>@<tenant_id>/<from>/<to>` (alias: outlook)
fn parse_o365(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("o365:// requires <client_id>:<client_secret>@<tenant_id>/<from>/<to>".into())
    })?;
    let (cid, csecret) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("o365:// requires <client_id>:<client_secret>@...".into())
    })?;
    config.values.insert("client_id".into(), cid.to_string());
    config.values.insert("client_secret".into(), csecret.to_string());
    let parts: Vec<&str> = rest.splitn(3, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("o365:// requires a tenant_id after @".into()));
    }
    config.values.insert("tenant_id".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("from".into(), parts[1].to_string());
    }
    if parts.len() > 2 {
        config.values.insert("to".into(), parts[2].to_string());
    }
    Ok(())
}

/// Parse `nctalk://<user>:<password>@<host>/<room_token>`
fn parse_nctalk(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("nctalk:// requires <user>:<password>@<host>/<room_token>".into())
    })?;
    let (user, password) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("nctalk:// requires <user>:<password>@...".into())
    })?;
    config.values.insert("user".into(), user.to_string());
    config.values.insert("password".into(), password.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("nctalk:// requires a host after @".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("room_token".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `emby://<api_key>@<host>/<user_id>`
fn parse_emby(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("emby:// requires <api_key>@<host>/<user_id>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("emby:// requires a host after @".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("user_id".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `jellyfin://<api_key>@<host>/<user_id>`
fn parse_jellyfin(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("jellyfin:// requires <api_key>@<host>/<user_id>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("jellyfin:// requires a host after @".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("user_id".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `pushcut://<api_key>/<notification_name>`
fn parse_pushcut(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "pushcut:// requires <api_key>/<notification_name>".into(),
        ));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    config.values.insert("notification_name".into(), parts[1].to_string());
    Ok(())
}

/// Parse `mqtt://<user>:<password>@<host>/<topic>` or `mqtt://<host>/<topic>`
fn parse_mqtt(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path_part.split_once('@') {
        if let Some((user, password)) = auth.split_once(':') {
            config.values.insert("user".into(), user.to_string());
            config.values.insert("password".into(), password.to_string());
        } else {
            config.values.insert("user".into(), auth.to_string());
        }
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err(NotiError::UrlParse("mqtt:// requires a host".into()));
        }
        config.values.insert("host".into(), parts[0].to_string());
        if parts.len() > 1 {
            config.values.insert("topic".into(), parts[1].to_string());
        }
    } else {
        let parts: Vec<&str> = path_part.splitn(2, '/').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err(NotiError::UrlParse("mqtt:// requires a host".into()));
        }
        config.values.insert("host".into(), parts[0].to_string());
        if parts.len() > 1 {
            config.values.insert("topic".into(), parts[1].to_string());
        }
    }
    Ok(())
}

/// Parse `voipms://<email>:<password>@<did>/<to>`
fn parse_voipms(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, rest) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("voipms:// requires <email>:<password>@<did>/<to>".into())
    })?;
    let (email, password) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("voipms:// requires <email>:<password>@...".into())
    })?;
    config.values.insert("email".into(), email.to_string());
    config.values.insert("password".into(), password.to_string());
    let parts: Vec<&str> = rest.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("voipms:// requires a DID after @".into()));
    }
    config.values.insert("did".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `sfr://<phone>:<password>`
fn parse_sfr(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (phone, password) = path_part.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("sfr:// requires <phone>:<password>".into())
    })?;
    config.values.insert("phone".into(), phone.to_string());
    config.values.insert("password".into(), password.to_string());
    Ok(())
}

/// Parse `pushed://<app_key>:<app_secret>`
fn parse_pushed(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (app_key, app_secret) = path_part.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("pushed:// requires <app_key>:<app_secret>".into())
    })?;
    config.values.insert("app_key".into(), app_key.to_string());
    config.values.insert("app_secret".into(), app_secret.to_string());
    Ok(())
}

/// Parse `growl://<password>@<host>:<port>` or `growl://<host>:<port>` or `growl://<host>`
fn parse_growl(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((password, host_part_local)) = path_part.split_once('@') {
        if !password.is_empty() {
            config.values.insert("password".into(), password.to_string());
        }
        if let Some((host, port)) = host_part_local.split_once(':') {
            config.values.insert("host".into(), host.to_string());
            config.values.insert("port".into(), port.to_string());
        } else {
            config.values.insert("host".into(), host_part_local.to_string());
        }
        Ok(())
    } else if path_part.is_empty() {
        Err(NotiError::UrlParse("growl:// requires a host".into()))
    } else if let Some((host, port)) = path_part.split_once(':') {
        config.values.insert("host".into(), host.to_string());
        config.values.insert("port".into(), port.to_string());
        Ok(())
    } else {
        config.values.insert("host".into(), path_part.to_string());
        Ok(())
    }
}

/// Parse `kumulos://<api_key>:<server_key>`
fn parse_kumulos(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, server_key) = path_part.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("kumulos:// requires <api_key>:<server_key>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    config.values.insert("server_key".into(), server_key.to_string());
    Ok(())
}

/// Parse `parse://<app_id>:<rest_api_key>@<host>` or `parse://<app_id>:<rest_api_key>`
fn parse_parse(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, host)) = path_part.split_once('@') {
        let (app_id, rest_key) = auth.split_once(':').ok_or_else(|| {
            NotiError::UrlParse("parse:// requires <app_id>:<rest_api_key>@...".into())
        })?;
        config.values.insert("app_id".into(), app_id.to_string());
        config.values.insert("rest_api_key".into(), rest_key.to_string());
        config.values.insert("host".into(), host.to_string());
        Ok(())
    } else {
        let (app_id, rest_key) = path_part.split_once(':').ok_or_else(|| {
            NotiError::UrlParse("parse:// requires <app_id>:<rest_api_key>".into())
        })?;
        config.values.insert("app_id".into(), app_id.to_string());
        config.values.insert("rest_api_key".into(), rest_key.to_string());
        Ok(())
    }
}

/// Parse `rsyslog://<host>/<token>` (alias: syslog)
fn parse_rsyslog(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("rsyslog:// requires a host".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 && !parts[1].is_empty() {
        config.values.insert("token".into(), parts[1].to_string());
    }
    Ok(())
}

/// Parse `smsmanager://<api_key>@<from>/<to>` or `smsmanager://<api_key>`
fn parse_smsmanager(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path_part.split_once('@') {
        config.values.insert("api_key".into(), api_key.to_string());
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            config.values.insert("from".into(), parts[0].to_string());
        }
        if parts.len() > 1 {
            config.values.insert("to".into(), parts[1].to_string());
        }
    } else {
        require_non_empty(path_part, "smsmanager", "an API key")?;
        config.values.insert("api_key".into(), path_part.to_string());
    }
    Ok(())
}

/// Parse `twitter://<bearer_token>` (alias: x)
fn parse_twitter(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "twitter", "a bearer token")?;
    config.values.insert("bearer_token".into(), path_part.to_string());
    Ok(())
}

/// Parse `boxcar://<access_token>`
fn parse_boxcar(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "boxcar", "an access token")?;
    config.values.insert("access_token".into(), path_part.to_string());
    Ok(())
}

/// Parse `dapnet://<callsign>:<password>@<to>`
fn parse_dapnet(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth, to) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("dapnet:// requires <callsign>:<password>@<to_callsign>".into())
    })?;
    let (callsign, password) = auth.split_once(':').ok_or_else(|| {
        NotiError::UrlParse("dapnet:// requires <callsign>:<password>@...".into())
    })?;
    config.values.insert("callsign".into(), callsign.to_string());
    config.values.insert("password".into(), password.to_string());
    if !to.is_empty() {
        config.values.insert("to".into(), to.to_string());
    }
    Ok(())
}

/// Parse `enigma2://<host>` or `enigma2://<user>:<password>@<host>:<port>` (alias: e2)
fn parse_enigma2(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, host_part_local)) = path_part.split_once('@') {
        if let Some((user, password)) = auth.split_once(':') {
            config.values.insert("user".into(), user.to_string());
            config.values.insert("password".into(), password.to_string());
        } else {
            config.values.insert("user".into(), auth.to_string());
        }
        if let Some((host, port)) = host_part_local.split_once(':') {
            config.values.insert("host".into(), host.to_string());
            config.values.insert("port".into(), port.to_string());
        } else {
            config.values.insert("host".into(), host_part_local.to_string());
        }
        Ok(())
    } else if path_part.is_empty() {
        Err(NotiError::UrlParse("enigma2:// requires a host".into()))
    } else if let Some((host, port)) = path_part.split_once(':') {
        config.values.insert("host".into(), host.to_string());
        config.values.insert("port".into(), port.to_string());
        Ok(())
    } else {
        config.values.insert("host".into(), path_part.to_string());
        Ok(())
    }
}

/// Parse `notifiarr://<api_key>`
fn parse_notifiarr(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "notifiarr", "an API key")?;
    config.values.insert("api_key".into(), path_part.to_string());
    Ok(())
}

/// Parse `statuspage://<api_key>@<page_id>`
fn parse_statuspage(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (api_key, page_id) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("statuspage:// requires <api_key>@<page_id>".into())
    })?;
    config.values.insert("api_key".into(), api_key.to_string());
    config.values.insert("page_id".into(), page_id.to_string());
    Ok(())
}

/// Parse `dot://<token>@<device_id>`
fn parse_dot(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (token, device_id) = path_part.split_once('@').ok_or_else(|| {
        NotiError::UrlParse("dot:// requires <token>@<device_id>".into())
    })?;
    config.values.insert("token".into(), token.to_string());
    config.values.insert("device_id".into(), device_id.to_string());
    Ok(())
}

/// Parse `fluxer://<webhook_id>/<webhook_token>`
fn parse_fluxer(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "fluxer:// requires <webhook_id>/<webhook_token>".into(),
        ));
    }
    config.values.insert("webhook_id".into(), parts[0].to_string());
    config.values.insert("webhook_token".into(), parts[1].to_string());
    Ok(())
}

/// Parse `workflows://<host>:<port>/<workflow>/<signature>` (aliases: workflow, powerautomate)
fn parse_workflows(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.len() < 2 {
        return Err(NotiError::UrlParse(
            "workflows:// requires <host>/<workflow>/<signature>".into(),
        ));
    }
    if let Some((host, port)) = parts[0].split_once(':') {
        config.values.insert("host".into(), host.to_string());
        config.values.insert("port".into(), port.to_string());
    } else {
        config.values.insert("host".into(), parts[0].to_string());
    }
    config.values.insert("workflow".into(), parts[1].to_string());
    if parts.len() > 2 {
        config.values.insert("signature".into(), parts[2].to_string());
    }
    Ok(())
}

/// Parse `napi://<client_id>/<client_secret>/<user_id>` (alias: notificationapi)
fn parse_napi(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path_part.splitn(3, '/').collect();
    if parts.len() != 3 {
        return Err(NotiError::UrlParse(
            "napi:// requires <client_id>/<client_secret>/<user_id>".into(),
        ));
    }
    config.values.insert("client_id".into(), parts[0].to_string());
    config.values.insert("client_secret".into(), parts[1].to_string());
    config.values.insert("user_id".into(), parts[2].to_string());
    Ok(())
}

/// Parse `spugpush://<token>`
fn parse_spugpush(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    require_non_empty(path_part, "spugpush", "a token")?;
    config.values.insert("token".into(), path_part.to_string());
    Ok(())
}

/// Parse `webhook://<full_url>` or pass-through for `http://` / `https://`
fn parse_webhook(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let full_url = format!("https://{path_part}");
    config.values.insert("url".into(), full_url);
    Ok(())
}

// ---------------------------------------------------------------------------
// Dispatch table
// ---------------------------------------------------------------------------

type SchemeParser = fn(path_part: &str, config: &mut ProviderConfig) -> Result<(), NotiError>;

/// Returns the static scheme → parser dispatch map.
fn scheme_parsers() -> HashMap<&'static str, SchemeParser> {
    let mut m: HashMap<&str, SchemeParser> = HashMap::new();
    // Chat / IM
    m.insert("wecom", parse_wecom as SchemeParser);
    m.insert("feishu", parse_feishu as SchemeParser);
    m.insert("lark", parse_feishu as SchemeParser); // alias
    m.insert("slack", parse_slack as SchemeParser);
    m.insert("tg", parse_telegram as SchemeParser);
    m.insert("telegram", parse_telegram as SchemeParser); // alias
    m.insert("discord", parse_discord as SchemeParser);
    m.insert("dingtalk", parse_dingtalk as SchemeParser);
    m.insert("pushover", parse_pushover as SchemeParser);
    m.insert("mattermost", parse_mattermost as SchemeParser);
    m.insert("rocketchat", parse_rocketchat as SchemeParser);
    m.insert("matrix", parse_matrix as SchemeParser);
    m.insert("zulip", parse_zulip as SchemeParser);
    m.insert("gchat", parse_gchat as SchemeParser);
    m.insert("googlechat", parse_gchat as SchemeParser); // alias
    m.insert("teams", parse_teams as SchemeParser);
    m.insert("ryver", parse_ryver as SchemeParser);
    m.insert("twist", parse_twist as SchemeParser);
    m.insert("flock", parse_flock as SchemeParser);
    m.insert("guilded", parse_guilded as SchemeParser);
    m.insert("misskey", parse_misskey as SchemeParser);
    m.insert("gitter", parse_gitter as SchemeParser);
    m.insert("streamlabs", parse_streamlabs as SchemeParser);
    m.insert("webex", parse_webex as SchemeParser);
    m.insert("line", parse_line as SchemeParser);
    m.insert("mastodon", parse_mastodon as SchemeParser);
    m.insert("toot", parse_mastodon as SchemeParser); // alias
    m.insert("prowl", parse_prowl as SchemeParser);
    m.insert("wxpusher", parse_wxpusher as SchemeParser);
    m.insert("pagertree", parse_pagertree as SchemeParser);
    m.insert("signl4", parse_signl4 as SchemeParser);

    // Push / Simple
    m.insert("ntfy", parse_ntfy as SchemeParser);
    m.insert("gotify", parse_gotify as SchemeParser);
    m.insert("bark", parse_bark as SchemeParser);
    m.insert("pushdeer", parse_pushdeer as SchemeParser);
    m.insert("serverchan", parse_serverchan as SchemeParser);
    m.insert("pushbullet", parse_pushbullet as SchemeParser);
    m.insert("ifttt", parse_ifttt as SchemeParser);
    m.insert("simplepush", parse_simplepush as SchemeParser);
    m.insert("join", parse_join as SchemeParser);
    m.insert("pushsafer", parse_pushsafer as SchemeParser);
    m.insert("chanify", parse_chanify as SchemeParser);
    m.insert("pushplus", parse_pushplus as SchemeParser);
    m.insert("pushme", parse_pushme as SchemeParser);
    m.insert("pushcut", parse_pushcut as SchemeParser);
    m.insert("pushjet", parse_pushjet as SchemeParser);
    m.insert("pushy", parse_pushy as SchemeParser);
    m.insert("push", parse_techulus_push as SchemeParser);
    m.insert("notica", parse_notica as SchemeParser);
    m.insert("boxcar", parse_boxcar as SchemeParser);
    m.insert("fluxer", parse_fluxer as SchemeParser);
    m.insert("spugpush", parse_spugpush as SchemeParser);

    // SMS
    m.insert("twilio", parse_twilio as SchemeParser);
    m.insert("vonage", parse_vonage as SchemeParser);
    m.insert("nexmo", parse_vonage as SchemeParser); // alias
    m.insert("pagerduty", parse_pagerduty as SchemeParser);
    m.insert("opsgenie", parse_opsgenie as SchemeParser);
    m.insert("signal", parse_signal as SchemeParser);
    m.insert("d7sms", parse_d7sms as SchemeParser);
    m.insert("sinch", parse_sinch as SchemeParser);
    m.insert("clickatell", parse_clickatell as SchemeParser);
    m.insert("bulksms", parse_bulksms as SchemeParser);
    m.insert("kavenegar", parse_kavenegar as SchemeParser);
    m.insert("lametric", parse_lametric as SchemeParser);
    m.insert("lunasea", parse_lunasea as SchemeParser);
    m.insert("onesignal", parse_onesignal as SchemeParser);
    m.insert("reddit", parse_reddit as SchemeParser);
    m.insert("clicksend", parse_clicksend as SchemeParser);
    m.insert("plivo", parse_plivo as SchemeParser);
    m.insert("burstsms", parse_burstsms as SchemeParser);
    m.insert("msg91", parse_msg91 as SchemeParser);
    m.insert("smseagle", parse_smseagle as SchemeParser);
    m.insert("seven", parse_seven as SchemeParser);
    m.insert("sms77", parse_seven as SchemeParser); // alias
    m.insert("smtp", parse_smtp as SchemeParser);
    m.insert("email", parse_smtp as SchemeParser); // alias
    m.insert("mailgun", parse_mailgun as SchemeParser);
    m.insert("sendgrid", parse_sendgrid as SchemeParser);
    m.insert("sparkpost", parse_sparkpost as SchemeParser);
    m.insert("resend", parse_resend as SchemeParser);
    m.insert("brevo", parse_brevo as SchemeParser);
    m.insert("sendinblue", parse_brevo as SchemeParser); // alias
    m.insert("smtp2go", parse_smtp2go as SchemeParser);
    m.insert("freemobile", parse_freemobile as SchemeParser);
    m.insert("httpsms", parse_httpsms as SchemeParser);
    m.insert("africastalking", parse_africastalking as SchemeParser);
    m.insert("msgbird", parse_msgbird as SchemeParser);
    m.insert("o365", parse_o365 as SchemeParser);
    m.insert("outlook", parse_o365 as SchemeParser); // alias
    m.insert("sendpulse", parse_sendpulse as SchemeParser);
    m.insert("nctalk", parse_nctalk as SchemeParser);

    // Email / AWS
    m.insert("ses", parse_ses as SchemeParser);
    m.insert("sns", parse_sns as SchemeParser);

    // Home / Media
    m.insert("hassio", parse_hassio as SchemeParser);
    m.insert("homeassistant", parse_hassio as SchemeParser); // alias
    m.insert("kodi", parse_kodi as SchemeParser);
    m.insert("xbmc", parse_kodi as SchemeParser); // alias
    m.insert("emby", parse_emby as SchemeParser);
    m.insert("jellyfin", parse_jellyfin as SchemeParser);
    m.insert("synology", parse_synology as SchemeParser);
    m.insert("apprise", parse_apprise as SchemeParser);

    // Misc
    m.insert("ncloud", parse_ncloud as SchemeParser);
    m.insert("nextcloud", parse_ncloud as SchemeParser); // alias
    m.insert("notifico", parse_notifico as SchemeParser);
    m.insert("46elks", parse_46elks as SchemeParser);
    m.insert("bulkvs", parse_bulkvs as SchemeParser);
    m.insert("jira", parse_jira as SchemeParser);
    m.insert("victorops", parse_victorops as SchemeParser);
    m.insert("splunk", parse_victorops as SchemeParser); // alias
    m.insert("spike", parse_spike as SchemeParser);
    m.insert("popcorn", parse_popcorn as SchemeParser);
    m.insert("fcm", parse_fcm as SchemeParser);
    m.insert("threema", parse_threema as SchemeParser);
    m.insert("bluesky", parse_bluesky as SchemeParser);
    m.insert("revolt", parse_revolt as SchemeParser);
    m.insert("webpush", parse_webpush as SchemeParser);
    m.insert("whatsapp", parse_whatsapp as SchemeParser);
    m.insert("pushed", parse_pushed as SchemeParser);
    m.insert("growl", parse_growl as SchemeParser);
    m.insert("kumulos", parse_kumulos as SchemeParser);
    m.insert("parse", parse_parse as SchemeParser);
    m.insert("rsyslog", parse_rsyslog as SchemeParser);
    m.insert("syslog", parse_rsyslog as SchemeParser); // alias
    m.insert("smsmanager", parse_smsmanager as SchemeParser);
    m.insert("twitter", parse_twitter as SchemeParser);
    m.insert("x", parse_twitter as SchemeParser); // alias
    m.insert("dapnet", parse_dapnet as SchemeParser);
    m.insert("enigma2", parse_enigma2 as SchemeParser);
    m.insert("e2", parse_enigma2 as SchemeParser); // alias
    m.insert("notifiarr", parse_notifiarr as SchemeParser);
    m.insert("statuspage", parse_statuspage as SchemeParser);
    m.insert("dot", parse_dot as SchemeParser);
    m.insert("workflows", parse_workflows as SchemeParser);
    m.insert("workflow", parse_workflows as SchemeParser); // alias
    m.insert("powerautomate", parse_workflows as SchemeParser); // alias
    m.insert("napi", parse_napi as SchemeParser);
    m.insert("notificationapi", parse_napi as SchemeParser); // alias
    m.insert("mqtt", parse_mqtt as SchemeParser);
    m.insert("voipms", parse_voipms as SchemeParser);
    m.insert("sfr", parse_sfr as SchemeParser);

    // Generic URL schemes (registered by canonical name only)
    m.insert("json", parse_generic_url as SchemeParser);
    m.insert("form", parse_generic_url as SchemeParser);
    m.insert("xml", parse_generic_url as SchemeParser);
    m.insert("webhook", parse_webhook as SchemeParser);
    m
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

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
pub fn parse_notification_url(input: &str) -> Result<ParsedUrl, NotiError> {
    // Split scheme from the rest
    let (scheme, rest) = input
        .split_once("://")
        .ok_or_else(|| NotiError::UrlParse(format!("missing '://' in URL: {input}")))?;

    let scheme = scheme.to_lowercase();

    // Parse query parameters if present
    let (path_part, query_params) = parse_query(rest);

    let mut config = ProviderConfig::new();

    // Add query params to config
    for (k, v) in &query_params {
        config.values.insert(k.clone(), v.clone());
    }

    // Dispatch to scheme-specific parser
    let parsers = scheme_parsers();
    let canonical = normalize_scheme(&scheme);

    // Special case: http/https pass through the original URL as-is
    if scheme == "http" || scheme == "https" {
        config.values.insert("url".into(), input.to_string());
        return Ok(ParsedUrl {
            scheme: "webhook".to_string(),
            config,
        });
    }

    let lookup_key = if parsers.contains_key(&canonical.as_str()) {
        canonical.as_str()
    } else {
        scheme.as_str()
    };

    if let Some(parser) = parsers.get(lookup_key) {
        parser(path_part, &mut config)?;
    } else {
        return Err(NotiError::UrlParse(format!("unknown URL scheme: {canonical}")));
    }

    Ok(ParsedUrl {
        scheme: canonical,
        config,
    })
}

/// Parse query string from path.
fn parse_query(input: &str) -> (&str, Vec<(String, String)>) {
    if let Some((path, query)) = input.split_once('?') {
        let params: Vec<(String, String)> = query
            .split('&')
            .filter_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect();
        (path, params)
    } else {
        (input, vec![])
    }
}

/// Parse SMTP URL: `user:pass@host:port`
fn parse_smtp_url(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let (auth_part, host_part) = if let Some((auth, host)) = path.split_once('@') {
        (Some(auth), host)
    } else {
        (None, path)
    };

    if let Some(auth) = auth_part {
        if let Some((user, pass)) = auth.split_once(':') {
            config.values.insert("username".into(), user.to_string());
            config.values.insert("password".into(), pass.to_string());
        } else {
            config.values.insert("username".into(), auth.to_string());
        }
    }

    if let Some((host, port)) = host_part.split_once(':') {
        config.values.insert("host".into(), host.to_string());
        config.values.insert("port".into(), port.to_string());
    } else if !host_part.is_empty() {
        config.values.insert("host".into(), host_part.to_string());
    }

    Ok(())
}
