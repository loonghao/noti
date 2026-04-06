//! URL parsers for email / cloud messaging providers.

use crate::error::NotiError;
use crate::provider::ProviderConfig;
use crate::url::helpers::parse_smtp_url;

pub fn parse_smtp(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    parse_smtp_url(path, config)
}

pub fn parse_mailgun(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_sendgrid(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    parse_api_key_at_from_to("sendgrid", path, config)
}

pub fn parse_sparkpost(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    parse_api_key_at_from_to("sparkpost", path, config)
}

pub fn parse_resend(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    parse_api_key_at_from_to("resend", path, config)
}

pub fn parse_brevo(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    parse_api_key_at_from_to("brevo", path, config)
}

pub fn parse_smtp2go(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    parse_api_key_at_from_to("smtp2go", path, config)
}

pub fn parse_sendpulse(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_ses(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_sns(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_o365(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

/// Shared helper for providers using pattern: `<api_key>@<from>/<to>`
fn parse_api_key_at_from_to(
    scheme: &str,
    path: &str,
    config: &mut ProviderConfig,
) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path.split_once('@') {
        config.values.insert("api_key".into(), api_key.to_string());
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err(NotiError::UrlParse(format!(
                "{scheme}:// requires a from email after @"
            )));
        }
        config.values.insert("from".into(), parts[0].to_string());
        if parts.len() > 1 {
            config.values.insert("to".into(), parts[1].to_string());
        }
    } else {
        return Err(NotiError::UrlParse(format!(
            "{scheme}:// requires <api_key>@<from>/<to>"
        )));
    }
    Ok(())
}
