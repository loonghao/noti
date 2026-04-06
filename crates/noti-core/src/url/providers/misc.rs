//! URL parsers for miscellaneous / special-purpose providers.

use crate::error::NotiError;
use crate::provider::ProviderConfig;
use crate::url::helpers::{insert_host_port, parse_auth_at_host};

pub fn parse_webhook(path: &str, scheme: &str, original_input: &str, config: &mut ProviderConfig) {
    let full_url = if scheme == "webhook" {
        format!("https://{path}")
    } else {
        original_input.to_string()
    };
    config.values.insert("url".into(), full_url);
}

pub fn parse_json_webhook(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("json:// requires a URL".into()));
    }
    config
        .values
        .insert("url".into(), format!("https://{path}"));
    Ok(())
}

pub fn parse_form_webhook(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("form:// requires a URL".into()));
    }
    config
        .values
        .insert("url".into(), format!("https://{path}"));
    Ok(())
}

pub fn parse_xml_webhook(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("xml:// requires a URL".into()));
    }
    config
        .values
        .insert("url".into(), format!("https://{path}"));
    Ok(())
}

pub fn parse_opsgenie(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "opsgenie:// requires an API key".into(),
        ));
    }
    config.values.insert("api_key".into(), path.to_string());
    Ok(())
}

pub fn parse_pagerduty(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "pagerduty:// requires an integration key".into(),
        ));
    }
    config
        .values
        .insert("integration_key".into(), path.to_string());
    Ok(())
}

pub fn parse_pagertree(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "pagertree:// requires an integration ID".into(),
        ));
    }
    config
        .values
        .insert("integration_id".into(), path.to_string());
    Ok(())
}

pub fn parse_signl4(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "signl4:// requires a team secret".into(),
        ));
    }
    config.values.insert("team_secret".into(), path.to_string());
    Ok(())
}

pub fn parse_victorops(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "victorops:// requires <api_key>/<routing_key>".into(),
        ));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    config
        .values
        .insert("routing_key".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_spike(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "spike:// requires a webhook URL".into(),
        ));
    }
    config
        .values
        .insert("webhook_url".into(), format!("https://{path}"));
    Ok(())
}

pub fn parse_ifttt(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "ifttt:// requires <webhook_key>/<event_name>".into(),
        ));
    }
    config
        .values
        .insert("webhook_key".into(), parts[0].to_string());
    config.values.insert("event".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_reddit(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_apprise(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("apprise:// requires a host".into()));
    }
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    config
        .values
        .insert("host".into(), format!("https://{}", parts[0]));
    if parts.len() > 1 && !parts[1].is_empty() {
        config
            .values
            .insert("config_key".into(), parts[1].to_string());
    }
    Ok(())
}

pub fn parse_webpush(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse(
            "webpush:// requires an endpoint URL".into(),
        ));
    }
    config
        .values
        .insert("endpoint".into(), format!("https://{path}"));
    Ok(())
}

pub fn parse_homeassistant(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, host)) = path.split_once('@') {
        config
            .values
            .insert("access_token".into(), token.to_string());
        config.values.insert("host".into(), host.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "hassio:// requires <access_token>@<host>".into(),
        ));
    }
    Ok(())
}

pub fn parse_kodi(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("kodi:// requires a host".into()));
    }
    parse_auth_at_host(path, config);
    Ok(())
}

pub fn parse_enigma2(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("enigma2:// requires a host".into()));
    }
    parse_auth_at_host(path, config);
    Ok(())
}

pub fn parse_emby(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_jellyfin(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_synology(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, host)) = path.split_once('@') {
        config.values.insert("token".into(), token.to_string());
        insert_host_port(host, config);
    } else {
        return Err(NotiError::UrlParse(
            "synology:// requires <token>@<host>".into(),
        ));
    }
    Ok(())
}

pub fn parse_nextcloud(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_growl(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if path.is_empty() {
        return Err(NotiError::UrlParse("growl:// requires a host".into()));
    }
    if let Some((password, host_part)) = path.split_once('@') {
        if !password.is_empty() {
            config
                .values
                .insert("password".into(), password.to_string());
        }
        insert_host_port(host_part, config);
    } else {
        insert_host_port(path, config);
    }
    Ok(())
}

pub fn parse_kumulos(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, server_key)) = path.split_once(':') {
        config.values.insert("api_key".into(), api_key.to_string());
        config
            .values
            .insert("server_key".into(), server_key.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "kumulos:// requires <api_key>:<server_key>".into(),
        ));
    }
    Ok(())
}

pub fn parse_parse(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, host)) = path.split_once('@') {
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
    } else if let Some((app_id, rest_key)) = path.split_once(':') {
        config.values.insert("app_id".into(), app_id.to_string());
        config
            .values
            .insert("rest_api_key".into(), rest_key.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "parse:// requires <app_id>:<rest_api_key>".into(),
        ));
    }
    Ok(())
}

pub fn parse_rsyslog(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("rsyslog:// requires a host".into()));
    }
    config.values.insert("host".into(), parts[0].to_string());
    if parts.len() > 1 && !parts[1].is_empty() {
        config.values.insert("token".into(), parts[1].to_string());
    }
    Ok(())
}

pub fn parse_pushed(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((app_key, app_secret)) = path.split_once(':') {
        config.values.insert("app_key".into(), app_key.to_string());
        config
            .values
            .insert("app_secret".into(), app_secret.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "pushed:// requires <app_key>:<app_secret>".into(),
        ));
    }
    Ok(())
}
