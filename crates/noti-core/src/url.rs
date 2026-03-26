use crate::error::NotiError;
use crate::provider::ProviderConfig;

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

    match scheme.as_str() {
        "wecom" => {
            // wecom://<key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "wecom:// requires a webhook key".into(),
                ));
            }
            config.values.insert("key".into(), path_part.to_string());
        }
        "feishu" | "lark" => {
            // feishu://<hook_id>?secret=<optional>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("feishu:// requires a hook ID".into()));
            }
            config
                .values
                .insert("hook_id".into(), path_part.to_string());
        }
        "slack" => {
            // slack://<token_a>/<token_b>/<token_c>
            let parts: Vec<&str> = path_part.splitn(3, '/').collect();
            if parts.len() == 3 {
                let webhook_url = format!(
                    "https://hooks.slack.com/services/{}/{}/{}",
                    parts[0], parts[1], parts[2]
                );
                config.values.insert("webhook_url".into(), webhook_url);
            } else if parts.len() == 1 && !parts[0].is_empty() {
                // Allow full webhook URL as single token
                config
                    .values
                    .insert("webhook_url".into(), path_part.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "slack:// requires <token_a>/<token_b>/<token_c>".into(),
                ));
            }
        }
        "tg" | "telegram" => {
            // tg://<bot_token>/<chat_id>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "tg:// requires <bot_token>/<chat_id>".into(),
                ));
            }
            config
                .values
                .insert("bot_token".into(), parts[0].to_string());
            config.values.insert("chat_id".into(), parts[1].to_string());
        }
        "discord" => {
            // discord://<webhook_id>/<webhook_token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "discord:// requires <webhook_id>/<webhook_token>".into(),
                ));
            }
            config
                .values
                .insert("webhook_id".into(), parts[0].to_string());
            config
                .values
                .insert("webhook_token".into(), parts[1].to_string());
        }
        "smtp" | "email" => {
            // smtp://<user>:<pass>@<host>:<port>?to=<recipient>
            parse_smtp_url(path_part, &mut config)?;
        }
        "dingtalk" => {
            // dingtalk://<access_token>?secret=<optional>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "dingtalk:// requires an access token".into(),
                ));
            }
            config
                .values
                .insert("access_token".into(), path_part.to_string());
        }
        "pushover" => {
            // pushover://<user_key>/<api_token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "pushover:// requires <user_key>/<api_token>".into(),
                ));
            }
            config
                .values
                .insert("user_key".into(), parts[0].to_string());
            config
                .values
                .insert("api_token".into(), parts[1].to_string());
        }
        "ntfy" => {
            // ntfy://<topic>?server=<optional>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("ntfy:// requires a topic name".into()));
            }
            config.values.insert("topic".into(), path_part.to_string());
        }
        "gotify" => {
            // gotify://<host>/<app_token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "gotify:// requires <host>/<app_token>".into(),
                ));
            }
            config
                .values
                .insert("host".into(), format!("https://{}", parts[0]));
            config
                .values
                .insert("app_token".into(), parts[1].to_string());
        }
        "bark" => {
            // bark://<device_key>?server=<optional>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("bark:// requires a device key".into()));
            }
            config
                .values
                .insert("device_key".into(), path_part.to_string());
        }
        "pushdeer" => {
            // pushdeer://<push_key>?server=<optional>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "pushdeer:// requires a push key".into(),
                ));
            }
            config
                .values
                .insert("push_key".into(), path_part.to_string());
        }
        "serverchan" => {
            // serverchan://<send_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "serverchan:// requires a send key".into(),
                ));
            }
            config
                .values
                .insert("send_key".into(), path_part.to_string());
        }
        "teams" => {
            // teams://<webhook_url_host>/<path...>
            // The full webhook URL is reconstructed as https://<path_part>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "teams:// requires a webhook URL".into(),
                ));
            }
            // Reconstruct the full webhook URL from the path
            let webhook_url = format!("https://{path_part}");
            config.values.insert("webhook_url".into(), webhook_url);
        }
        "gchat" | "googlechat" => {
            // gchat://<space>/<key>/<token>
            let parts: Vec<&str> = path_part.splitn(3, '/').collect();
            if parts.len() == 3 {
                let webhook_url = format!(
                    "https://chat.googleapis.com/v1/spaces/{}/messages?key={}&token={}",
                    parts[0], parts[1], parts[2]
                );
                config.values.insert("webhook_url".into(), webhook_url);
            } else if parts.len() == 1 && !parts[0].is_empty() {
                // Allow full webhook URL as single value
                let webhook_url = format!("https://{path_part}");
                config.values.insert("webhook_url".into(), webhook_url);
            } else {
                return Err(NotiError::UrlParse(
                    "gchat:// requires <space>/<key>/<token>".into(),
                ));
            }
        }
        "mattermost" => {
            // mattermost://<host>/<hook_id>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "mattermost:// requires <host>/<hook_id>".into(),
                ));
            }
            config.values.insert("host".into(), parts[0].to_string());
            config.values.insert("hook_id".into(), parts[1].to_string());
        }
        "rocketchat" => {
            // rocketchat://<host>/<token_a>/<token_b>
            let parts: Vec<&str> = path_part.splitn(3, '/').collect();
            if parts.len() != 3 {
                return Err(NotiError::UrlParse(
                    "rocketchat:// requires <host>/<token_a>/<token_b>".into(),
                ));
            }
            config.values.insert("host".into(), parts[0].to_string());
            config.values.insert("token_a".into(), parts[1].to_string());
            config.values.insert("token_b".into(), parts[2].to_string());
        }
        "matrix" => {
            // matrix://<access_token>/<room_id>?server=<optional>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "matrix:// requires <access_token>/<room_id>".into(),
                ));
            }
            config
                .values
                .insert("access_token".into(), parts[0].to_string());
            config.values.insert("room_id".into(), parts[1].to_string());
        }
        "twilio" => {
            // twilio://<account_sid>:<auth_token>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((sid, token)) = auth.split_once(':') {
                    config.values.insert("account_sid".into(), sid.to_string());
                    config.values.insert("auth_token".into(), token.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "twilio:// requires <account_sid>:<auth_token>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "twilio:// requires <from_number>/<to_number> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "twilio:// requires <account_sid>:<auth_token>@<from>/<to>".into(),
                ));
            }
        }
        "zulip" => {
            // zulip://<bot_email>:<api_key>@<domain>/<stream>/<topic>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((email, key)) = auth.split_once(':') {
                    config.values.insert("bot_email".into(), email.to_string());
                    config.values.insert("api_key".into(), key.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "zulip:// requires <bot_email>:<api_key>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(3, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "zulip:// requires a domain after @".into(),
                    ));
                }
                config.values.insert("domain".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("stream".into(), parts[1].to_string());
                }
                if parts.len() > 2 {
                    config.values.insert("topic".into(), parts[2].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "zulip:// requires <bot_email>:<api_key>@<domain>".into(),
                ));
            }
        }
        "pushbullet" => {
            // pushbullet://<access_token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "pushbullet:// requires an access token".into(),
                ));
            }
            config
                .values
                .insert("access_token".into(), path_part.to_string());
        }
        "ifttt" => {
            // ifttt://<webhook_key>/<event_name>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "ifttt:// requires <webhook_key>/<event_name>".into(),
                ));
            }
            config
                .values
                .insert("webhook_key".into(), parts[0].to_string());
            config.values.insert("event".into(), parts[1].to_string());
        }
        "simplepush" => {
            // simplepush://<key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("simplepush:// requires a key".into()));
            }
            config.values.insert("key".into(), path_part.to_string());
        }
        "mailgun" => {
            // mailgun://<api_key>@<domain>/<to>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "mailgun:// requires a domain after @".into(),
                    ));
                }
                config.values.insert("domain".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "mailgun:// requires <api_key>@<domain>/<to>".into(),
                ));
            }
        }
        "webex" => {
            // webex://<access_token>/<room_id>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "webex:// requires <access_token>/<room_id>".into(),
                ));
            }
            config
                .values
                .insert("access_token".into(), parts[0].to_string());
            config.values.insert("room_id".into(), parts[1].to_string());
        }
        "line" => {
            // line://<access_token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "line:// requires an access token".into(),
                ));
            }
            config
                .values
                .insert("access_token".into(), path_part.to_string());
        }
        "vonage" | "nexmo" => {
            // vonage://<api_key>:<api_secret>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((key, secret)) = auth.split_once(':') {
                    config.values.insert("api_key".into(), key.to_string());
                    config
                        .values
                        .insert("api_secret".into(), secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "vonage:// requires <api_key>:<api_secret>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "vonage:// requires <from>/<to> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "vonage:// requires <api_key>:<api_secret>@<from>/<to>".into(),
                ));
            }
        }
        "pagerduty" => {
            // pagerduty://<integration_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "pagerduty:// requires an integration key".into(),
                ));
            }
            config
                .values
                .insert("integration_key".into(), path_part.to_string());
        }
        "revolt" => {
            // revolt://<bot_token>/<channel_id>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "revolt:// requires <bot_token>/<channel_id>".into(),
                ));
            }
            config
                .values
                .insert("bot_token".into(), parts[0].to_string());
            config
                .values
                .insert("channel_id".into(), parts[1].to_string());
        }
        "opsgenie" => {
            // opsgenie://<api_key>?region=<optional>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "opsgenie:// requires an API key".into(),
                ));
            }
            config
                .values
                .insert("api_key".into(), path_part.to_string());
        }
        "sendgrid" => {
            // sendgrid://<api_key>@<from_email>/<to_email>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "sendgrid:// requires a from email after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "sendgrid:// requires <api_key>@<from>/<to>".into(),
                ));
            }
        }
        "notica" => {
            // notica://<token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("notica:// requires a token".into()));
            }
            config.values.insert("token".into(), path_part.to_string());
        }
        "mastodon" | "toot" => {
            // mastodon://<access_token>@<instance>
            if let Some((token, instance)) = path_part.split_once('@') {
                config
                    .values
                    .insert("access_token".into(), token.to_string());
                config
                    .values
                    .insert("instance".into(), instance.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "mastodon:// requires <access_token>@<instance>".into(),
                ));
            }
        }
        "json" => {
            // json://<host>/<path>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("json:// requires a URL".into()));
            }
            let full_url = format!("https://{path_part}");
            config.values.insert("url".into(), full_url);
        }
        "form" => {
            // form://<host>/<path>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("form:// requires a URL".into()));
            }
            let full_url = format!("https://{path_part}");
            config.values.insert("url".into(), full_url);
        }
        "prowl" => {
            // prowl://<api_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("prowl:// requires an API key".into()));
            }
            config
                .values
                .insert("api_key".into(), path_part.to_string());
        }
        "join" => {
            // join://<api_key>/<device_id>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(NotiError::UrlParse("join:// requires an API key".into()));
            }
            config.values.insert("api_key".into(), parts[0].to_string());
            if parts.len() > 1 {
                config
                    .values
                    .insert("device_id".into(), parts[1].to_string());
            }
        }
        "pushsafer" => {
            // pushsafer://<private_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "pushsafer:// requires a private key".into(),
                ));
            }
            config
                .values
                .insert("private_key".into(), path_part.to_string());
        }
        "hassio" | "homeassistant" => {
            // hassio://<access_token>@<host>
            if let Some((token, host)) = path_part.split_once('@') {
                config
                    .values
                    .insert("access_token".into(), token.to_string());
                config.values.insert("host".into(), host.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "hassio:// requires <access_token>@<host>".into(),
                ));
            }
        }
        "signal" => {
            // signal://<from>/<to>?server=<optional>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "signal:// requires <from_number>/<to_number>".into(),
                ));
            }
            config.values.insert("from".into(), parts[0].to_string());
            config.values.insert("to".into(), parts[1].to_string());
        }
        "sparkpost" => {
            // sparkpost://<api_key>@<from_email>/<to_email>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "sparkpost:// requires a from email after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "sparkpost:// requires <api_key>@<from>/<to>".into(),
                ));
            }
        }
        "xml" => {
            // xml://<host>/<path>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("xml:// requires a URL".into()));
            }
            let full_url = format!("https://{path_part}");
            config.values.insert("url".into(), full_url);
        }
        "sns" => {
            // sns://<access_key>:<secret_key>@<region>/<topic_arn>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((key, secret)) = auth.split_once(':') {
                    config.values.insert("access_key".into(), key.to_string());
                    config
                        .values
                        .insert("secret_key".into(), secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "sns:// requires <access_key>:<secret_key>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "sns:// requires a region after @".into(),
                    ));
                }
                config.values.insert("region".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config
                        .values
                        .insert("topic_arn".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "sns:// requires <access_key>:<secret_key>@<region>/<topic_arn>".into(),
                ));
            }
        }
        "ses" => {
            // ses://<access_key>:<secret_key>@<region>/<from>/<to>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((key, secret)) = auth.split_once(':') {
                    config.values.insert("access_key".into(), key.to_string());
                    config
                        .values
                        .insert("secret_key".into(), secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "ses:// requires <access_key>:<secret_key>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(3, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "ses:// requires a region after @".into(),
                    ));
                }
                config.values.insert("region".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("from".into(), parts[1].to_string());
                }
                if parts.len() > 2 {
                    config.values.insert("to".into(), parts[2].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "ses:// requires <access_key>:<secret_key>@<region>/<from>/<to>".into(),
                ));
            }
        }
        "d7sms" => {
            // d7sms://<api_token>@<from>/<to>
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
                // d7sms://<api_token>
                if path_part.is_empty() {
                    return Err(NotiError::UrlParse("d7sms:// requires an API token".into()));
                }
                config
                    .values
                    .insert("api_token".into(), path_part.to_string());
            }
        }
        "sinch" => {
            // sinch://<service_plan_id>:<api_token>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((plan_id, token)) = auth.split_once(':') {
                    config
                        .values
                        .insert("service_plan_id".into(), plan_id.to_string());
                    config.values.insert("api_token".into(), token.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "sinch:// requires <service_plan_id>:<api_token>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "sinch:// requires <from>/<to> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "sinch:// requires <service_plan_id>:<api_token>@<from>/<to>".into(),
                ));
            }
        }
        "clickatell" => {
            // clickatell://<api_key>/<to>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(NotiError::UrlParse(
                    "clickatell:// requires an API key".into(),
                ));
            }
            config.values.insert("api_key".into(), parts[0].to_string());
            if parts.len() > 1 {
                config.values.insert("to".into(), parts[1].to_string());
            }
        }
        "bulksms" => {
            // bulksms://<token_id>:<token_secret>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((id, secret)) = auth.split_once(':') {
                    config.values.insert("token_id".into(), id.to_string());
                    config
                        .values
                        .insert("token_secret".into(), secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "bulksms:// requires <token_id>:<token_secret>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "bulksms:// requires <from>/<to> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "bulksms:// requires <token_id>:<token_secret>@<from>/<to>".into(),
                ));
            }
        }
        "kavenegar" => {
            // kavenegar://<api_key>/<from>/<to>
            let parts: Vec<&str> = path_part.splitn(3, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(NotiError::UrlParse(
                    "kavenegar:// requires an API key".into(),
                ));
            }
            config.values.insert("api_key".into(), parts[0].to_string());
            if parts.len() > 1 {
                config.values.insert("from".into(), parts[1].to_string());
            }
            if parts.len() > 2 {
                config.values.insert("to".into(), parts[2].to_string());
            }
        }
        "lametric" => {
            // lametric://<api_key>@<host>
            if let Some((api_key, host)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                config.values.insert("host".into(), host.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "lametric:// requires <api_key>@<host>".into(),
                ));
            }
        }
        "lunasea" => {
            // lunasea://<user_token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "lunasea:// requires a user token".into(),
                ));
            }
            config
                .values
                .insert("user_token".into(), path_part.to_string());
        }
        "onesignal" => {
            // onesignal://<app_id>:<api_key>
            if let Some((app_id, api_key)) = path_part.split_once(':') {
                config.values.insert("app_id".into(), app_id.to_string());
                config.values.insert("api_key".into(), api_key.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "onesignal:// requires <app_id>:<api_key>".into(),
                ));
            }
        }
        "push" => {
            // push://<api_key> (Techulus Push)
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("push:// requires an API key".into()));
            }
            config
                .values
                .insert("api_key".into(), path_part.to_string());
        }
        "pushy" => {
            // pushy://<api_key>/<device_token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(NotiError::UrlParse("pushy:// requires an API key".into()));
            }
            config.values.insert("api_key".into(), parts[0].to_string());
            if parts.len() > 1 {
                config
                    .values
                    .insert("device_token".into(), parts[1].to_string());
            }
        }
        "reddit" => {
            // reddit://<client_id>:<client_secret>@<user>:<password>/<to>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((client_id, client_secret)) = auth.split_once(':') {
                    config
                        .values
                        .insert("client_id".into(), client_id.to_string());
                    config
                        .values
                        .insert("client_secret".into(), client_secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "reddit:// requires <client_id>:<client_secret>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(3, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "reddit:// requires user:password after @".into(),
                    ));
                }
                // user:password/to or just user:password
                if let Some((user, password)) = parts[0].split_once(':') {
                    config.values.insert("user".into(), user.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
                } else {
                    config.values.insert("user".into(), parts[0].to_string());
                }
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "reddit:// requires <client_id>:<client_secret>@<user>:<password>/<to>".into(),
                ));
            }
        }
        "ryver" => {
            // ryver://<organization>/<token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "ryver:// requires <organization>/<token>".into(),
                ));
            }
            config
                .values
                .insert("organization".into(), parts[0].to_string());
            config.values.insert("token".into(), parts[1].to_string());
        }
        "twist" => {
            // twist://<token_a>/<token_b>/<token_c>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "twist:// requires webhook URL components".into(),
                ));
            }
            // Reconstruct the webhook URL
            let webhook_url =
                format!("https://twist.com/api/v3/integration_incoming/post_data?{path_part}");
            config.values.insert("webhook_url".into(), webhook_url);
        }
        "flock" => {
            // flock://<token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "flock:// requires a webhook token".into(),
                ));
            }
            config.values.insert("token".into(), path_part.to_string());
        }
        "guilded" => {
            // guilded://<webhook_id>/<webhook_token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "guilded:// requires <webhook_id>/<webhook_token>".into(),
                ));
            }
            config
                .values
                .insert("webhook_id".into(), parts[0].to_string());
            config
                .values
                .insert("webhook_token".into(), parts[1].to_string());
        }
        "misskey" => {
            // misskey://<access_token>@<instance>
            if let Some((token, instance)) = path_part.split_once('@') {
                config
                    .values
                    .insert("access_token".into(), token.to_string());
                config
                    .values
                    .insert("instance".into(), instance.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "misskey:// requires <access_token>@<instance>".into(),
                ));
            }
        }
        "chanify" => {
            // chanify://<token> or chanify://<token>@<host>
            if let Some((token, host)) = path_part.split_once('@') {
                config.values.insert("token".into(), token.to_string());
                config
                    .values
                    .insert("server".into(), format!("https://{host}"));
            } else {
                if path_part.is_empty() {
                    return Err(NotiError::UrlParse(
                        "chanify:// requires a device token".into(),
                    ));
                }
                config.values.insert("token".into(), path_part.to_string());
            }
        }
        "pushplus" => {
            // pushplus://<token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "pushplus:// requires a user token".into(),
                ));
            }
            config.values.insert("token".into(), path_part.to_string());
        }
        "wxpusher" => {
            // wxpusher://<app_token>/<uid>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "wxpusher:// requires <app_token>/<uid>".into(),
                ));
            }
            config
                .values
                .insert("app_token".into(), parts[0].to_string());
            config.values.insert("uid".into(), parts[1].to_string());
        }
        "resend" => {
            // resend://<api_key>@<from>/<to>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "resend:// requires a from email after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "resend:// requires <api_key>@<from>/<to>".into(),
                ));
            }
        }
        "bluesky" => {
            // bluesky://<handle>:<app_password>
            if let Some((handle, app_password)) = path_part.split_once(':') {
                config.values.insert("handle".into(), handle.to_string());
                config
                    .values
                    .insert("app_password".into(), app_password.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "bluesky:// requires <handle>:<app_password>".into(),
                ));
            }
        }
        "msgbird" => {
            // msgbird://<access_key>@<from>/<to>
            if let Some((access_key, rest)) = path_part.split_once('@') {
                config
                    .values
                    .insert("access_key".into(), access_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "msgbird:// requires <from>/<to> after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "msgbird:// requires <access_key>@<from>/<to>".into(),
                ));
            }
        }
        "plivo" => {
            // plivo://<auth_id>:<auth_token>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((auth_id, auth_token)) = auth.split_once(':') {
                    config.values.insert("auth_id".into(), auth_id.to_string());
                    config
                        .values
                        .insert("auth_token".into(), auth_token.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "plivo:// requires <auth_id>:<auth_token>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "plivo:// requires <from>/<to> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "plivo:// requires <auth_id>:<auth_token>@<from>/<to>".into(),
                ));
            }
        }
        "burstsms" => {
            // burstsms://<api_key>:<api_secret>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((api_key, api_secret)) = auth.split_once(':') {
                    config.values.insert("api_key".into(), api_key.to_string());
                    config
                        .values
                        .insert("api_secret".into(), api_secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "burstsms:// requires <api_key>:<api_secret>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "burstsms:// requires <from>/<to> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "burstsms:// requires <api_key>:<api_secret>@<from>/<to>".into(),
                ));
            }
        }
        "gitter" => {
            // gitter://<token>/<room_id>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "gitter:// requires <token>/<room_id>".into(),
                ));
            }
            config.values.insert("token".into(), parts[0].to_string());
            config.values.insert("room_id".into(), parts[1].to_string());
        }
        "ncloud" | "nextcloud" => {
            // ncloud://<user>:<password>@<host>/<target_user>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((user, password)) = auth.split_once(':') {
                    config.values.insert("user".into(), user.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "ncloud:// requires <user>:<password>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "ncloud:// requires a host after @".into(),
                    ));
                }
                config.values.insert("host".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config
                        .values
                        .insert("target_user".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "ncloud:// requires <user>:<password>@<host>".into(),
                ));
            }
        }
        "pagertree" => {
            // pagertree://<integration_id>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "pagertree:// requires an integration ID".into(),
                ));
            }
            config
                .values
                .insert("integration_id".into(), path_part.to_string());
        }
        "signl4" => {
            // signl4://<team_secret>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "signl4:// requires a team secret".into(),
                ));
            }
            config
                .values
                .insert("team_secret".into(), path_part.to_string());
        }
        "victorops" | "splunk" => {
            // victorops://<api_key>/<routing_key>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "victorops:// requires <api_key>/<routing_key>".into(),
                ));
            }
            config.values.insert("api_key".into(), parts[0].to_string());
            config
                .values
                .insert("routing_key".into(), parts[1].to_string());
        }
        "spike" => {
            // spike://<webhook_url_path>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "spike:// requires a webhook URL".into(),
                ));
            }
            let webhook_url = format!("https://{path_part}");
            config.values.insert("webhook_url".into(), webhook_url);
        }
        "popcorn" => {
            // popcorn://<api_key>@<from>/<to>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "popcorn:// requires <from>/<to> after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "popcorn:// requires <api_key>@<from>/<to>".into(),
                ));
            }
        }
        "fcm" => {
            // fcm://<server_key>/<device_token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(NotiError::UrlParse("fcm:// requires a server key".into()));
            }
            config
                .values
                .insert("server_key".into(), parts[0].to_string());
            if parts.len() > 1 {
                config
                    .values
                    .insert("device_token".into(), parts[1].to_string());
            }
        }
        "threema" => {
            // threema://<gateway_id>:<api_secret>@<recipient_id>
            if let Some((auth, recipient)) = path_part.split_once('@') {
                if let Some((gw_id, secret)) = auth.split_once(':') {
                    config.values.insert("gateway_id".into(), gw_id.to_string());
                    config
                        .values
                        .insert("api_secret".into(), secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "threema:// requires <gateway_id>:<api_secret>@...".into(),
                    ));
                }
                if !recipient.is_empty() {
                    config.values.insert("to".into(), recipient.to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "threema:// requires <gateway_id>:<api_secret>@<recipient_id>".into(),
                ));
            }
        }
        "clicksend" => {
            // clicksend://<username>:<api_key>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((user, key)) = auth.split_once(':') {
                    config.values.insert("username".into(), user.to_string());
                    config.values.insert("api_key".into(), key.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "clicksend:// requires <username>:<api_key>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if !phone_parts.is_empty() && !phone_parts[0].is_empty() {
                    config
                        .values
                        .insert("from".into(), phone_parts[0].to_string());
                }
                if phone_parts.len() > 1 {
                    config
                        .values
                        .insert("to".into(), phone_parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "clicksend:// requires <username>:<api_key>@<from>/<to>".into(),
                ));
            }
        }
        "brevo" | "sendinblue" => {
            // brevo://<api_key>@<from_email>/<to_email>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "brevo:// requires a from email after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "brevo:// requires <api_key>@<from>/<to>".into(),
                ));
            }
        }
        "smtp2go" => {
            // smtp2go://<api_key>@<from_email>/<to_email>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "smtp2go:// requires a from email after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "smtp2go:// requires <api_key>@<from>/<to>".into(),
                ));
            }
        }
        "apprise" => {
            // apprise://<host>/<config_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("apprise:// requires a host".into()));
            }
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            config
                .values
                .insert("host".into(), format!("https://{}", parts[0]));
            if parts.len() > 1 && !parts[1].is_empty() {
                config
                    .values
                    .insert("config_key".into(), parts[1].to_string());
            }
        }
        "freemobile" => {
            // freemobile://<user_id>/<api_key>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "freemobile:// requires <user_id>/<api_key>".into(),
                ));
            }
            config.values.insert("user".into(), parts[0].to_string());
            config
                .values
                .insert("password".into(), parts[1].to_string());
        }
        "httpsms" => {
            // httpsms://<api_key>@<from>/<to>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "httpsms:// requires <from>/<to> after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "httpsms:// requires <api_key>@<from>/<to>".into(),
                ));
            }
        }
        "msg91" => {
            // msg91://<authkey>/<sender_id>/<to>
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
        }
        "pushjet" => {
            // pushjet://<secret_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "pushjet:// requires a secret key".into(),
                ));
            }
            config.values.insert("secret".into(), path_part.to_string());
        }
        "smseagle" => {
            // smseagle://<access_token>@<host>/<to>
            if let Some((token, rest)) = path_part.split_once('@') {
                config
                    .values
                    .insert("access_token".into(), token.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "smseagle:// requires a host after @".into(),
                    ));
                }
                config.values.insert("host".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "smseagle:// requires <access_token>@<host>/<to>".into(),
                ));
            }
        }
        "seven" | "sms77" => {
            // seven://<api_key>/<to>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(NotiError::UrlParse("seven:// requires an API key".into()));
            }
            config.values.insert("api_key".into(), parts[0].to_string());
            if parts.len() > 1 {
                config.values.insert("to".into(), parts[1].to_string());
            }
        }
        "webpush" => {
            // webpush://<endpoint_encoded>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "webpush:// requires an endpoint URL".into(),
                ));
            }
            let endpoint = format!("https://{path_part}");
            config.values.insert("endpoint".into(), endpoint);
        }
        "whatsapp" => {
            // whatsapp://<access_token>@<phone_number_id>/<to>
            if let Some((token, rest)) = path_part.split_once('@') {
                config
                    .values
                    .insert("access_token".into(), token.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "whatsapp:// requires a phone_number_id after @".into(),
                    ));
                }
                config
                    .values
                    .insert("phone_number_id".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "whatsapp:// requires <access_token>@<phone_number_id>/<to>".into(),
                ));
            }
        }
        "kodi" | "xbmc" => {
            // kodi://<host> or kodi://<user>:<password>@<host>:<port>
            if let Some((auth, host_part_local)) = path_part.split_once('@') {
                if let Some((user, password)) = auth.split_once(':') {
                    config.values.insert("user".into(), user.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
                } else {
                    config.values.insert("user".into(), auth.to_string());
                }
                if let Some((host, port)) = host_part_local.split_once(':') {
                    config.values.insert("host".into(), host.to_string());
                    config.values.insert("port".into(), port.to_string());
                } else {
                    config
                        .values
                        .insert("host".into(), host_part_local.to_string());
                }
            } else if path_part.is_empty() {
                return Err(NotiError::UrlParse("kodi:// requires a host".into()));
            } else if let Some((host, port)) = path_part.split_once(':') {
                config.values.insert("host".into(), host.to_string());
                config.values.insert("port".into(), port.to_string());
            } else {
                config.values.insert("host".into(), path_part.to_string());
            }
        }
        "notifico" => {
            // notifico://<project_id>/<msghook>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "notifico:// requires <project_id>/<msghook>".into(),
                ));
            }
            config
                .values
                .insert("project_id".into(), parts[0].to_string());
            config.values.insert("msghook".into(), parts[1].to_string());
        }
        "46elks" => {
            // 46elks://<api_username>:<api_password>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((user, pass)) = auth.split_once(':') {
                    config
                        .values
                        .insert("api_username".into(), user.to_string());
                    config
                        .values
                        .insert("api_password".into(), pass.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "46elks:// requires <api_username>:<api_password>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "46elks:// requires <from>/<to> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "46elks:// requires <api_username>:<api_password>@<from>/<to>".into(),
                ));
            }
        }
        "bulkvs" => {
            // bulkvs://<username>:<password>@<from>/<to>
            if let Some((auth, phone_part)) = path_part.split_once('@') {
                if let Some((user, pass)) = auth.split_once(':') {
                    config.values.insert("username".into(), user.to_string());
                    config.values.insert("password".into(), pass.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "bulkvs:// requires <username>:<password>@...".into(),
                    ));
                }
                let phone_parts: Vec<&str> = phone_part.splitn(2, '/').collect();
                if phone_parts.len() != 2 {
                    return Err(NotiError::UrlParse(
                        "bulkvs:// requires <from>/<to> after @".into(),
                    ));
                }
                config
                    .values
                    .insert("from".into(), phone_parts[0].to_string());
                config
                    .values
                    .insert("to".into(), phone_parts[1].to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "bulkvs:// requires <username>:<password>@<from>/<to>".into(),
                ));
            }
        }
        "jira" => {
            // jira://<user>:<api_token>@<host>/<issue_key>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((user, token)) = auth.split_once(':') {
                    config.values.insert("user".into(), user.to_string());
                    config.values.insert("api_token".into(), token.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "jira:// requires <user>:<api_token>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "jira:// requires a host after @".into(),
                    ));
                }
                config.values.insert("host".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config
                        .values
                        .insert("issue_key".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "jira:// requires <user>:<api_token>@<host>/<issue_key>".into(),
                ));
            }
        }
        "pushme" => {
            // pushme://<push_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("pushme:// requires a push key".into()));
            }
            config
                .values
                .insert("push_key".into(), path_part.to_string());
        }
        "sendpulse" => {
            // sendpulse://<client_id>:<client_secret>@<from>/<to>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((id, secret)) = auth.split_once(':') {
                    config.values.insert("client_id".into(), id.to_string());
                    config
                        .values
                        .insert("client_secret".into(), secret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "sendpulse:// requires <client_id>:<client_secret>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "sendpulse:// requires a from email after @".into(),
                    ));
                }
                config.values.insert("from".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "sendpulse:// requires <client_id>:<client_secret>@<from>/<to>".into(),
                ));
            }
        }
        "streamlabs" => {
            // streamlabs://<access_token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "streamlabs:// requires an access token".into(),
                ));
            }
            config
                .values
                .insert("access_token".into(), path_part.to_string());
        }
        "synology" => {
            // synology://<token>@<host>
            if let Some((token, host)) = path_part.split_once('@') {
                config.values.insert("token".into(), token.to_string());
                if let Some((h, port)) = host.split_once(':') {
                    config.values.insert("host".into(), h.to_string());
                    config.values.insert("port".into(), port.to_string());
                } else {
                    config.values.insert("host".into(), host.to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "synology:// requires <token>@<host>".into(),
                ));
            }
        }
        "africastalking" => {
            // africastalking://<username>:<api_key>@<to>
            if let Some((auth, to)) = path_part.split_once('@') {
                if let Some((user, key)) = auth.split_once(':') {
                    config.values.insert("username".into(), user.to_string());
                    config.values.insert("api_key".into(), key.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "africastalking:// requires <username>:<api_key>@...".into(),
                    ));
                }
                if !to.is_empty() {
                    config.values.insert("to".into(), to.to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "africastalking:// requires <username>:<api_key>@<to>".into(),
                ));
            }
        }
        "o365" | "outlook" => {
            // o365://<client_id>:<client_secret>@<tenant_id>/<from>/<to>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((cid, csecret)) = auth.split_once(':') {
                    config.values.insert("client_id".into(), cid.to_string());
                    config
                        .values
                        .insert("client_secret".into(), csecret.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "o365:// requires <client_id>:<client_secret>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(3, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "o365:// requires a tenant_id after @".into(),
                    ));
                }
                config
                    .values
                    .insert("tenant_id".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("from".into(), parts[1].to_string());
                }
                if parts.len() > 2 {
                    config.values.insert("to".into(), parts[2].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "o365:// requires <client_id>:<client_secret>@<tenant_id>/<from>/<to>".into(),
                ));
            }
        }
        "nctalk" => {
            // nctalk://<user>:<password>@<host>/<room_token>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((user, password)) = auth.split_once(':') {
                    config.values.insert("user".into(), user.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "nctalk:// requires <user>:<password>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "nctalk:// requires a host after @".into(),
                    ));
                }
                config.values.insert("host".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config
                        .values
                        .insert("room_token".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "nctalk:// requires <user>:<password>@<host>/<room_token>".into(),
                ));
            }
        }
        "emby" => {
            // emby://<api_key>@<host>/<user_id>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "emby:// requires a host after @".into(),
                    ));
                }
                config.values.insert("host".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("user_id".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "emby:// requires <api_key>@<host>".into(),
                ));
            }
        }
        "jellyfin" => {
            // jellyfin://<api_key>@<host>/<user_id>
            if let Some((api_key, rest)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "jellyfin:// requires a host after @".into(),
                    ));
                }
                config.values.insert("host".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("user_id".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "jellyfin:// requires <api_key>@<host>".into(),
                ));
            }
        }
        "pushcut" => {
            // pushcut://<api_key>/<notification_name>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "pushcut:// requires <api_key>/<notification_name>".into(),
                ));
            }
            config.values.insert("api_key".into(), parts[0].to_string());
            config
                .values
                .insert("notification_name".into(), parts[1].to_string());
        }
        "mqtt" => {
            // mqtt://<user>:<password>@<host>/<topic> or mqtt://<host>/<topic>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((user, password)) = auth.split_once(':') {
                    config.values.insert("user".into(), user.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
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
        }
        "voipms" => {
            // voipms://<email>:<password>@<did>/<to>
            if let Some((auth, rest)) = path_part.split_once('@') {
                if let Some((email, password)) = auth.split_once(':') {
                    config.values.insert("email".into(), email.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "voipms:// requires <email>:<password>@...".into(),
                    ));
                }
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err(NotiError::UrlParse(
                        "voipms:// requires a DID after @".into(),
                    ));
                }
                config.values.insert("did".into(), parts[0].to_string());
                if parts.len() > 1 {
                    config.values.insert("to".into(), parts[1].to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "voipms:// requires <email>:<password>@<did>/<to>".into(),
                ));
            }
        }
        "sfr" => {
            // sfr://<phone>:<password>
            if let Some((phone, password)) = path_part.split_once(':') {
                config.values.insert("phone".into(), phone.to_string());
                config
                    .values
                    .insert("password".into(), password.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "sfr:// requires <phone>:<password>".into(),
                ));
            }
        }
        "pushed" => {
            // pushed://<app_key>:<app_secret>
            if let Some((app_key, app_secret)) = path_part.split_once(':') {
                config.values.insert("app_key".into(), app_key.to_string());
                config
                    .values
                    .insert("app_secret".into(), app_secret.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "pushed:// requires <app_key>:<app_secret>".into(),
                ));
            }
        }
        "growl" => {
            // growl://<password>@<host>:<port> or growl://<host>:<port> or growl://<host>
            if let Some((password, host_part_local)) = path_part.split_once('@') {
                if !password.is_empty() {
                    config
                        .values
                        .insert("password".into(), password.to_string());
                }
                if let Some((host, port)) = host_part_local.split_once(':') {
                    config.values.insert("host".into(), host.to_string());
                    config.values.insert("port".into(), port.to_string());
                } else {
                    config
                        .values
                        .insert("host".into(), host_part_local.to_string());
                }
            } else if path_part.is_empty() {
                return Err(NotiError::UrlParse("growl:// requires a host".into()));
            } else if let Some((host, port)) = path_part.split_once(':') {
                config.values.insert("host".into(), host.to_string());
                config.values.insert("port".into(), port.to_string());
            } else {
                config.values.insert("host".into(), path_part.to_string());
            }
        }
        "kumulos" => {
            // kumulos://<api_key>:<server_key>
            if let Some((api_key, server_key)) = path_part.split_once(':') {
                config.values.insert("api_key".into(), api_key.to_string());
                config
                    .values
                    .insert("server_key".into(), server_key.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "kumulos:// requires <api_key>:<server_key>".into(),
                ));
            }
        }
        "parse" => {
            // parse://<app_id>:<rest_api_key>@<host>
            if let Some((auth, host)) = path_part.split_once('@') {
                if let Some((app_id, rest_key)) = auth.split_once(':') {
                    config.values.insert("app_id".into(), app_id.to_string());
                    config
                        .values
                        .insert("rest_api_key".into(), rest_key.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "parse:// requires <app_id>:<rest_api_key>@...".into(),
                    ));
                }
                config.values.insert("host".into(), host.to_string());
            } else {
                // parse://<app_id>:<rest_api_key> (use default host)
                if let Some((app_id, rest_key)) = path_part.split_once(':') {
                    config.values.insert("app_id".into(), app_id.to_string());
                    config
                        .values
                        .insert("rest_api_key".into(), rest_key.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "parse:// requires <app_id>:<rest_api_key>".into(),
                    ));
                }
            }
        }
        "rsyslog" | "syslog" => {
            // rsyslog://<host>/<token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.is_empty() || parts[0].is_empty() {
                return Err(NotiError::UrlParse("rsyslog:// requires a host".into()));
            }
            config.values.insert("host".into(), parts[0].to_string());
            if parts.len() > 1 && !parts[1].is_empty() {
                config.values.insert("token".into(), parts[1].to_string());
            }
        }
        "smsmanager" => {
            // smsmanager://<api_key>@<from>/<to>
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
                // smsmanager://<api_key>
                if path_part.is_empty() {
                    return Err(NotiError::UrlParse(
                        "smsmanager:// requires an API key".into(),
                    ));
                }
                config
                    .values
                    .insert("api_key".into(), path_part.to_string());
            }
        }
        "twitter" | "x" => {
            // twitter://<bearer_token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "twitter:// requires a bearer token".into(),
                ));
            }
            config
                .values
                .insert("bearer_token".into(), path_part.to_string());
        }
        "boxcar" => {
            // boxcar://<access_token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "boxcar:// requires an access token".into(),
                ));
            }
            config
                .values
                .insert("access_token".into(), path_part.to_string());
        }
        "dapnet" => {
            // dapnet://<callsign>:<password>@<to>
            if let Some((auth, to)) = path_part.split_once('@') {
                if let Some((callsign, password)) = auth.split_once(':') {
                    config
                        .values
                        .insert("callsign".into(), callsign.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
                } else {
                    return Err(NotiError::UrlParse(
                        "dapnet:// requires <callsign>:<password>@...".into(),
                    ));
                }
                if !to.is_empty() {
                    config.values.insert("to".into(), to.to_string());
                }
            } else {
                return Err(NotiError::UrlParse(
                    "dapnet:// requires <callsign>:<password>@<to_callsign>".into(),
                ));
            }
        }
        "enigma2" | "e2" => {
            // enigma2://<host> or enigma2://<user>:<password>@<host>:<port>
            if let Some((auth, host_part_local)) = path_part.split_once('@') {
                if let Some((user, password)) = auth.split_once(':') {
                    config.values.insert("user".into(), user.to_string());
                    config
                        .values
                        .insert("password".into(), password.to_string());
                } else {
                    config.values.insert("user".into(), auth.to_string());
                }
                if let Some((host, port)) = host_part_local.split_once(':') {
                    config.values.insert("host".into(), host.to_string());
                    config.values.insert("port".into(), port.to_string());
                } else {
                    config
                        .values
                        .insert("host".into(), host_part_local.to_string());
                }
            } else if path_part.is_empty() {
                return Err(NotiError::UrlParse("enigma2:// requires a host".into()));
            } else if let Some((host, port)) = path_part.split_once(':') {
                config.values.insert("host".into(), host.to_string());
                config.values.insert("port".into(), port.to_string());
            } else {
                config.values.insert("host".into(), path_part.to_string());
            }
        }
        "notifiarr" => {
            // notifiarr://<api_key>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse(
                    "notifiarr:// requires an API key".into(),
                ));
            }
            config
                .values
                .insert("api_key".into(), path_part.to_string());
        }
        "statuspage" => {
            // statuspage://<api_key>@<page_id>
            if let Some((api_key, page_id)) = path_part.split_once('@') {
                config.values.insert("api_key".into(), api_key.to_string());
                config.values.insert("page_id".into(), page_id.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "statuspage:// requires <api_key>@<page_id>".into(),
                ));
            }
        }
        "dot" => {
            // dot://<token>@<device_id>
            if let Some((token, device_id)) = path_part.split_once('@') {
                config.values.insert("token".into(), token.to_string());
                config
                    .values
                    .insert("device_id".into(), device_id.to_string());
            } else {
                return Err(NotiError::UrlParse(
                    "dot:// requires <token>@<device_id>".into(),
                ));
            }
        }
        "fluxer" => {
            // fluxer://<webhook_id>/<webhook_token>
            let parts: Vec<&str> = path_part.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(NotiError::UrlParse(
                    "fluxer:// requires <webhook_id>/<webhook_token>".into(),
                ));
            }
            config
                .values
                .insert("webhook_id".into(), parts[0].to_string());
            config
                .values
                .insert("webhook_token".into(), parts[1].to_string());
        }
        "workflows" | "workflow" | "powerautomate" => {
            // workflows://<host>:<port>/<workflow>/<signature>
            // or workflows://<host>/<workflow>/<signature>
            let parts: Vec<&str> = path_part.splitn(3, '/').collect();
            if parts.len() < 2 {
                return Err(NotiError::UrlParse(
                    "workflows:// requires <host>/<workflow>/<signature>".into(),
                ));
            }
            // Parse host:port from first segment
            if let Some((host, port)) = parts[0].split_once(':') {
                config.values.insert("host".into(), host.to_string());
                config.values.insert("port".into(), port.to_string());
            } else {
                config.values.insert("host".into(), parts[0].to_string());
            }
            config
                .values
                .insert("workflow".into(), parts[1].to_string());
            if parts.len() > 2 {
                config
                    .values
                    .insert("signature".into(), parts[2].to_string());
            }
        }
        "napi" | "notificationapi" => {
            // napi://<client_id>/<client_secret>/<user_id>
            let parts: Vec<&str> = path_part.splitn(3, '/').collect();
            if parts.len() != 3 {
                return Err(NotiError::UrlParse(
                    "napi:// requires <client_id>/<client_secret>/<user_id>".into(),
                ));
            }
            config
                .values
                .insert("client_id".into(), parts[0].to_string());
            config
                .values
                .insert("client_secret".into(), parts[1].to_string());
            config.values.insert("user_id".into(), parts[2].to_string());
        }
        "spugpush" => {
            // spugpush://<token>
            if path_part.is_empty() {
                return Err(NotiError::UrlParse("spugpush:// requires a token".into()));
            }
            config.values.insert("token".into(), path_part.to_string());
        }
        "webhook" | "http" | "https" => {
            // webhook://<full_url> or just pass through http(s)://
            let full_url = if scheme == "webhook" {
                // The rest after webhook:// is the actual URL
                format!("https://{path_part}")
            } else {
                input.to_string()
            };
            config.values.insert("url".into(), full_url);
        }
        _ => {
            return Err(NotiError::UrlParse(format!("unknown URL scheme: {scheme}")));
        }
    }

    // Normalize scheme name
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
