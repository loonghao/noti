use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// BulkSMS provider.
///
/// Sends SMS messages via the BulkSMS REST API v2.
/// Requires a token ID and secret, plus sender and recipient numbers.
pub struct BulkSmsProvider {
    client: Client,
}

impl BulkSmsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BulkSmsProvider {
    fn name(&self) -> &str {
        "bulksms"
    }

    fn url_scheme(&self) -> &str {
        "bulksms"
    }

    fn description(&self) -> &str {
        "BulkSMS gateway via REST API v2"
    }

    fn example_url(&self) -> &str {
        "bulksms://<token_id>:<token_secret>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token_id", "BulkSMS token ID").with_example("your-token-id"),
            ParamDef::required("token_secret", "BulkSMS token secret")
                .with_example("your-token-secret"),
            ParamDef::required("to", "Recipient phone number (international format)")
                .with_example("+15559876543"),
            ParamDef::optional("from", "Sender phone number or name").with_example("+15551234567"),
            ParamDef::optional("base_url", "API base URL override (default: https://api.bulksms.com)")
                .with_example("https://api.bulksms.com"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token_id = config.require("token_id", "bulksms")?;
        let token_secret = config.require("token_secret", "bulksms")?;
        let to = config.require("to", "bulksms")?;

        let base_url = config
            .get("base_url")
            .unwrap_or("https://api.bulksms.com")
            .trim_end_matches('/');
        let url = format!("{base_url}/v1/messages");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "to": to,
            "body": body_text
        });

        if let Some(from) = config.get("from") {
            payload["from"] = json!(from);
        }

        let resp = self
            .client
            .post(url)
            .basic_auth(token_id, Some(token_secret))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("bulksms", "SMS sent via BulkSMS")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("bulksms", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
