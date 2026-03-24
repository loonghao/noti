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
        "http" | "https" => "webhook".to_string(),
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
