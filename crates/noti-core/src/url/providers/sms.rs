//! URL parsers for SMS / telephony providers.

use crate::error::NotiError;
use crate::provider::ProviderConfig;

pub fn parse_twilio(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
        if let Some((sid, token)) = auth.split_once(':') {
            config.values.insert("account_sid".into(), sid.to_string());
            config.values.insert("auth_token".into(), token.to_string());
        } else {
            return Err(NotiError::UrlParse(
                "twilio:// requires <account_sid>:<auth_token>@...".into(),
            ));
        }
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "twilio:// requires <from_number>/<to_number> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "twilio:// requires <account_sid>:<auth_token>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_vonage(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
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
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "vonage:// requires <from>/<to> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "vonage:// requires <api_key>:<api_secret>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_d7sms(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, rest)) = path.split_once('@') {
        config.values.insert("api_token".into(), token.to_string());
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            config.values.insert("from".into(), parts[0].to_string());
        }
        if parts.len() > 1 {
            config.values.insert("to".into(), parts[1].to_string());
        }
    } else {
        if path.is_empty() {
            return Err(NotiError::UrlParse("d7sms:// requires an API token".into()));
        }
        config.values.insert("api_token".into(), path.to_string());
    }
    Ok(())
}

pub fn parse_sinch(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
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
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "sinch:// requires <from>/<to> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "sinch:// requires <service_plan_id>:<api_token>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_clickatell(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse(
            "clickatell:// requires an API key".into(),
        ));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

pub fn parse_bulksms(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
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
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "bulksms:// requires <from>/<to> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "bulksms:// requires <token_id>:<token_secret>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_kavenegar(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
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
    Ok(())
}

pub fn parse_messagebird(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((access_key, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_plivo(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
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
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "plivo:// requires <from>/<to> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "plivo:// requires <auth_id>:<auth_token>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_burstsms(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
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
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "burstsms:// requires <from>/<to> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "burstsms:// requires <api_key>:<api_secret>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_popcorn(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_clicksend(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
        if let Some((user, key)) = auth.split_once(':') {
            config.values.insert("username".into(), user.to_string());
            config.values.insert("api_key".into(), key.to_string());
        } else {
            return Err(NotiError::UrlParse(
                "clicksend:// requires <username>:<api_key>@...".into(),
            ));
        }
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if !pp.is_empty() && !pp[0].is_empty() {
            config.values.insert("from".into(), pp[0].to_string());
        }
        if pp.len() > 1 {
            config.values.insert("to".into(), pp[1].to_string());
        }
    } else {
        return Err(NotiError::UrlParse(
            "clicksend:// requires <username>:<api_key>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_seven(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(NotiError::UrlParse("seven:// requires an API key".into()));
    }
    config.values.insert("api_key".into(), parts[0].to_string());
    if parts.len() > 1 {
        config.values.insert("to".into(), parts[1].to_string());
    }
    Ok(())
}

pub fn parse_smseagle(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_httpsms(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_msg91(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(3, '/').collect();
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

pub fn parse_freemobile(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "freemobile:// requires <user_id>/<api_key>".into(),
        ));
    }
    config.values.insert("user".into(), parts[0].to_string());
    config
        .values
        .insert("password".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_fortysixelks(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
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
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "46elks:// requires <from>/<to> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "46elks:// requires <api_username>:<api_password>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_bulkvs(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, phone_part)) = path.split_once('@') {
        if let Some((user, pass)) = auth.split_once(':') {
            config.values.insert("username".into(), user.to_string());
            config.values.insert("password".into(), pass.to_string());
        } else {
            return Err(NotiError::UrlParse(
                "bulkvs:// requires <username>:<password>@...".into(),
            ));
        }
        let pp: Vec<&str> = phone_part.splitn(2, '/').collect();
        if pp.len() != 2 {
            return Err(NotiError::UrlParse(
                "bulkvs:// requires <from>/<to> after @".into(),
            ));
        }
        config.values.insert("from".into(), pp[0].to_string());
        config.values.insert("to".into(), pp[1].to_string());
    } else {
        return Err(NotiError::UrlParse(
            "bulkvs:// requires <username>:<password>@<from>/<to>".into(),
        ));
    }
    Ok(())
}

pub fn parse_africas_talking(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, to)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_dapnet(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, to)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_sfr(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((phone, password)) = path.split_once(':') {
        config.values.insert("phone".into(), phone.to_string());
        config
            .values
            .insert("password".into(), password.to_string());
    } else {
        return Err(NotiError::UrlParse(
            "sfr:// requires <phone>:<password>".into(),
        ));
    }
    Ok(())
}

pub fn parse_voipms(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_smsmanager(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((api_key, rest)) = path.split_once('@') {
        config.values.insert("api_key".into(), api_key.to_string());
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            config.values.insert("from".into(), parts[0].to_string());
        }
        if parts.len() > 1 {
            config.values.insert("to".into(), parts[1].to_string());
        }
    } else {
        if path.is_empty() {
            return Err(NotiError::UrlParse(
                "smsmanager:// requires an API key".into(),
            ));
        }
        config.values.insert("api_key".into(), path.to_string());
    }
    Ok(())
}

pub fn parse_signal(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "signal:// requires <from_number>/<to_number>".into(),
        ));
    }
    config.values.insert("from".into(), parts[0].to_string());
    config.values.insert("to".into(), parts[1].to_string());
    Ok(())
}

pub fn parse_whatsapp(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((token, rest)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_threema(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, recipient)) = path.split_once('@') {
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
    Ok(())
}

pub fn parse_mqtt(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    if let Some((auth, rest)) = path.split_once('@') {
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
        let parts: Vec<&str> = path.splitn(2, '/').collect();
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

pub fn parse_notifico(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return Err(NotiError::UrlParse(
            "notifico:// requires <project_id>/<msghook>".into(),
        ));
    }
    config
        .values
        .insert("project_id".into(), parts[0].to_string());
    config.values.insert("msghook".into(), parts[1].to_string());
    Ok(())
}
