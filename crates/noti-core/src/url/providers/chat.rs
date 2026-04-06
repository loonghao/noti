//! URL parsers for chat / IM providers.

use crate::error::NotiError;
use crate::provider::ProviderConfig;

pub fn parse_wecom(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "wecom:// requires a webhook key".into(),
        ));
    }
    config.values.insert("key".into(), path.to_string());
    Ok(())
}

pub fn parse_feishu(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("feishu:// requires a hook ID".into()));
    }
    config.values.insert("hook_id".into(), path.to_string());
    Ok(())
}

pub fn parse_slack(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() == 3 {
        let webhook_url = format!(
            "https://hooks.slack.com/services/{}/{}/{}",
            parts[0], parts[1], parts[2]
        );
        config.values.insert("webhook_url".into(), webhook_url);
    } else if parts.len() == 1 && !parts[0].is_empty() {
        config.values.insert("webhook_url".into(), path.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "slack:// requires <token_a>/<token_b>/<token_c>".into(),
        ));
    }
    Ok(())
}

pub fn parse_telegram(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "tg:// requires <bot_token>/<chat_id>".into(),
        ));
    }
    config
        .values
        .insert("bot_token".into(), parts[0].to_string());
    config.values.insert("chat_id".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_discord(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
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
    Ok(())
}

pub fn parse_dingtalk(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "dingtalk:// requires an access token".into(),
        ));
    }
    config
        .values
        .insert("access_token".into(), path.to_string());
    Ok(())
}

pub fn parse_teams(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "teams:// requires a webhook URL".into(),
        ));
    }
    config
        .values
        .insert("webhook_url".into(), format!("https://{path}"));
    Ok(())
}

pub fn parse_gchat(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() == 3 {
        let webhook_url = format!(
            "https://chat.googleapis.com/v1/spaces/{}/messages?key={}&token={}",
            parts[0], parts[1], parts[2]
        );
        config.values.insert("webhook_url".into(), webhook_url);
    } else if parts.len() == 1 && !parts[0].is_empty() {
        config
            .values
            .insert("webhook_url".into(), format!("https://{path}"));
    } else {
        return Err(NotiError::UrlParse(
            "gchat:// requires <space>/<key>/<token>".into(),
        ));
    }
    Ok(())
}

pub fn parse_mattermost(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "mattermost:// requires <host>/<hook_id>".into(),
        ));
    }
    config.values.insert("host".into(), parts[0].to_string());
    config.values.insert("hook_id".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_rocketchat(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
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

pub fn parse_matrix(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "matrix:// requires <access_token>/<room_id>".into(),
        ));
    }
    config
        .values
        .insert("access_token".into(), parts[0].to_string());
    config.values.insert("room_id".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_zulip(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_webex(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "webex:// requires <access_token>/<room_id>".into(),
        ));
    }
    config
        .values
        .insert("access_token".into(), parts[0].to_string());
    config.values.insert("room_id".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_line(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "line:// requires an access token".into(),
        ));
    }
    config
        .values
        .insert("access_token".into(), path.to_string());
    Ok(())
}

pub fn parse_mastodon(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, instance)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_revolt(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
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
    Ok(())
}

pub fn parse_ryver(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "ryver:// requires <organization>/<token>".into(),
        ));
    }
    config
        .values
        .insert("organization".into(), parts[0].to_string());
    config.values.insert("token".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_twist(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "twist:// requires webhook URL components".into(),
        ));
    }
    let webhook_url = format!("https://twist.com/api/v3/integration_incoming/post_data?{path}");
    config.values.insert("webhook_url".into(), webhook_url);
    Ok(())
}

pub fn parse_flock(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "flock:// requires a webhook token".into(),
        ));
    }
    config.values.insert("token".into(), path.to_string());
    Ok(())
}

pub fn parse_gitter(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "gitter:// requires <token>/<room_id>".into(),
        ));
    }
    config.values.insert("token".into(), parts[0].to_string());
    config.values.insert("room_id".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_guilded(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
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
    Ok(())
}

pub fn parse_misskey(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, instance)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_nctalk(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_jira(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_workflows(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
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
    config
        .values
        .insert("workflow".into(), parts[1].to_string());
    if parts.len() > 2 {
        config
            .values
            .insert("signature".into(), parts[2].to_string());
    }
    Ok(())
}
