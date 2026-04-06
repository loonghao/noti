use crate::error::NotiError;
use crate::provider::ProviderConfig;

/// Parse query string from path, returning (path_without_query, params).
pub(super) fn parse_query(input: &str) -> (&str, Vec<(String, String)>) {
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

/// Parse SMTP URL path: `[user:pass@]host[:port]`
pub(super) fn parse_smtp_url(path: &str, config: &mut ProviderConfig) -> Result<(), NotiError> {
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

/// Parse `<user>:<password>@<host>[:<port>]` into config.
/// Used by kodi, enigma2 and similar host-based providers.
pub(super) fn parse_auth_at_host(path: &str, config: &mut ProviderConfig) {
    if let Some((auth, host_part)) = path.split_once('@') {
        if let Some((user, password)) = auth.split_once(':') {
            config.values.insert("user".into(), user.to_string());
            config
                .values
                .insert("password".into(), password.to_string());
        } else {
            config.values.insert("user".into(), auth.to_string());
        }
        insert_host_port(host_part, config);
    } else {
        insert_host_port(path, config);
    }
}

/// Insert `host` (and optionally `port`) from `<host>[:<port>]`.
pub(super) fn insert_host_port(host_part: &str, config: &mut ProviderConfig) {
    if let Some((host, port)) = host_part.split_once(':') {
        config.values.insert("host".into(), host.to_string());
        config.values.insert("port".into(), port.to_string());
    } else {
        config.values.insert("host".into(), host_part.to_string());
    }
}
