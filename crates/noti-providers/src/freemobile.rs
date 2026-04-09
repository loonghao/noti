use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Free Mobile SMS provider.
///
/// Sends SMS to Free Mobile subscribers in France.
///
/// API reference: <https://www.freenews.fr/freenews-edition-nationale-299/free-mobile-170/nouvelle-option-notifications-par-sms-chez-free-mobile-14817>
pub struct FreeMobileProvider {
    client: Client,
}

impl FreeMobileProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FreeMobileProvider {
    fn name(&self) -> &str {
        "freemobile"
    }

    fn url_scheme(&self) -> &str {
        "freemobile"
    }

    fn description(&self) -> &str {
        "Free Mobile SMS (France)"
    }

    fn example_url(&self) -> &str {
        "freemobile://<user_id>/<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("user", "Free Mobile user login (phone number)")
                .with_example("12345678"),
            ParamDef::required("password", "Free Mobile API key (password)")
                .with_example("xxxxxxxx"),
            ParamDef::optional("base_url", "Override base URL for the Free Mobile API")
                .with_example("https://smsapi.free-mobile.fr"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let user = config.require("user", "freemobile")?;
        let password = config.require("password", "freemobile")?;

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let default_base = "https://smsapi.free-mobile.fr";
        let base = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| default_base.to_string());

        let url = format!(
            "{base}/sendmsg?user={user}&pass={password}&msg={}",
            urlencoding(&text)
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();

        if status == 200 {
            Ok(
                SendResponse::success("freemobile", "SMS sent via Free Mobile")
                    .with_status_code(status),
            )
        } else {
            let body = resp.text().await.unwrap_or_default();
            Ok(
                SendResponse::failure("freemobile", format!("API error ({status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"body": body})),
            )
        }
    }
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{b:02X}"));
            }
        }
    }
    result
}
