//! URL parsers for push notification providers.

use crate::error::NotiError;
use crate::provider::ProviderConfig;

pub fn parse_pushover(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
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
    Ok(())
}

pub fn parse_ntfy(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("ntfy:// requires a topic name".into()));
    }
    config.values.insert("topic".into(), path.to_string());
    Ok(())
}

pub fn parse_gotify(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
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
    Ok(())
}

pub fn parse_bark(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("bark:// requires a device key".into()));
    }
    config.values.insert("device_key".into(), path.to_string());
    Ok(())
}

pub fn parse_pushdeer(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "pushdeer:// requires a push key".into(),
        ));
    }
    config.values.insert("push_key".into(), path.to_string());
    Ok(())
}

pub fn parse_serverchan(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "serverchan:// requires a send key".into(),
        ));
    }
    config.values.insert("send_key".into(), path.to_string());
    Ok(())
}

pub fn parse_pushbullet(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "pushbullet:// requires an access token".into(),
        ));
    }
    config
        .values
        .insert("access_token".into(), path.to_string());
    Ok(())
}

pub fn parse_simplepush(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("simplepush:// requires a key".into()));
    }
    config.values.insert("key".into(), path.to_string());
    Ok(())
}

pub fn parse_notica(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("notica:// requires a token".into()));
    }
    config.values.insert("token".into(), path.to_string());
    Ok(())
}

pub fn parse_prowl(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("prowl:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), path.to_string());
    Ok(())
}

pub fn parse_join(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("join:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config
            .values
            .insert("device_id".into(), parts[1].to_string());
    }
    Ok(())
}

pub fn parse_pushsafer(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "pushsafer:// requires a private key".into(),
        ));
    }
    config.values.insert("private_key".into(), path.to_string());
    Ok(())
}

pub fn parse_onesignal(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((app_id, api_key)) = path.split_once(':') {
        config.values.insert("app_id".into(), app_id.to_string());
        config.values.insert("api_key".into(), api_key.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "onesignal:// requires <app_id>:<api_key>".into(),
        ));
    }
    Ok(())
}

pub fn parse_techulus(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("push:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), path.to_string());
    Ok(())
}

pub fn parse_pushy(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("pushy:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config
            .values
            .insert("device_token".into(), parts[1].to_string());
    }
    Ok(())
}

pub fn parse_chanify(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, host)) = path.split_once('@') {
        config.values.insert("token".into(), token.to_string());
        config
            .values
            .insert("server".into(), format!("https://{host}"));
    } else {
        if path.is_empty() {
            return Err(NotiError::UrlParse(
                "chanify:// requires a device token".into(),
            ));
        }
        config.values.insert("token".into(), path.to_string());
    }
    Ok(())
}

pub fn parse_pushplus(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "pushplus:// requires a user token".into(),
        ));
    }
    config.values.insert("token".into(), path.to_string());
    Ok(())
}

pub fn parse_wxpusher(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "wxpusher:// requires <app_token>/<uid>".into(),
        ));
    }
    config
        .values
        .insert("app_token".into(), parts[0].to_string());
    config.values.insert("uid".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_fcm(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
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
    Ok(())
}

pub fn parse_pushjet(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "pushjet:// requires a secret key".into(),
        ));
    }
    config.values.insert("secret".into(), path.to_string());
    Ok(())
}

pub fn parse_pushme(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("pushme:// requires a push key".into()));
    }
    config.values.insert("push_key".into(), path.to_string());
    Ok(())
}

pub fn parse_pushcut(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "pushcut:// requires <api_key>/<notification_name>".into(),
        ));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    config
        .values
        .insert("notification_name".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_spugpush(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("spugpush:// requires a token".into()));
    }
    config.values.insert("token".into(), path.to_string());
    Ok(())
}

pub fn parse_bluesky(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((handle, app_password)) = path.split_once(':') {
        config.values.insert("handle".into(), handle.to_string());
        config
            .values
            .insert("app_password".into(), app_password.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "bluesky:// requires <handle>:<app_password>".into(),
        ));
    }
    Ok(())
}

pub fn parse_boxcar(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "boxcar:// requires an access token".into(),
        ));
    }
    config
        .values
        .insert("access_token".into(), path.to_string());
    Ok(())
}

pub fn parse_streamlabs(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "streamlabs:// requires an access token".into(),
        ));
    }
    config
        .values
        .insert("access_token".into(), path.to_string());
    Ok(())
}

pub fn parse_lametric(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, host)) = path.split_once('@') {
        config.values.insert("api_key".into(), api_key.to_string());
        config.values.insert("host".into(), host.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "lametric:// requires <api_key>@<host>".into(),
        ));
    }
    Ok(())
}

pub fn parse_lunasea(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "lunasea:// requires a user token".into(),
        ));
    }
    config.values.insert("user_token".into(), path.to_string());
    Ok(())
}

pub fn parse_notifiarr(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "notifiarr:// requires an API key".into(),
        ));
    }
    config.values.insert("api_key".into(), path.to_string());
    Ok(())
}

pub fn parse_twitter(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "twitter:// requires a bearer token".into(),
        ));
    }
    config
        .values
        .insert("bearer_token".into(), path.to_string());
    Ok(())
}

pub fn parse_statuspage(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, page_id)) = path.split_once('@') {
        config.values.insert("api_key".into(), api_key.to_string());
        config.values.insert("page_id".into(), page_id.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "statuspage:// requires <api_key>@<page_id>".into(),
        ));
    }
    Ok(())
}

pub fn parse_dot(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, device_id)) = path.split_once('@') {
        config.values.insert("token".into(), token.to_string());
        config
            .values
            .insert("device_id".into(), device_id.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "dot:// requires <token>@<device_id>".into(),
        ));
    }
    Ok(())
}

pub fn parse_fluxer(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
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
    Ok(())
}

pub fn parse_napi(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
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
    Ok(())
}
