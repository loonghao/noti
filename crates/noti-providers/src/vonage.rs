use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Vonage (formerly Nexmo) SMS provider.
///
/// Sends SMS messages via the Vonage SMS API.
pub struct VonageProvider {
    client: Client,
}

impl VonageProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for VonageProvider {
    fn name(&self) -> &str {
        "vonage"
    }

    fn url_scheme(&self) -> &str {
        "vonage"
    }

    fn description(&self) -> &str {
        "Vonage (Nexmo) SMS via REST API"
    }

    fn example_url(&self) -> &str {
        "vonage://<api_key>:<api_secret>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Vonage API key").with_example("abc12345"),
            ParamDef::required("api_secret", "Vonage API secret").with_example("xyz9876543"),
            ParamDef::required("from", "Sender number or name (max 15 chars)")
                .with_example("15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("15559876543"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "vonage")?;
        let api_secret = config.require("api_secret", "vonage")?;
        let from = config.require("from", "vonage")?;
        let to = config.require("to", "vonage")?;

        let url = "https://rest.nexmo.com/sms/json";

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "api_key": api_key,
            "api_secret": api_secret,
            "from": from,
            "to": to,
            "text": text,
        });

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        // Vonage returns messages array with status per message
        let msg_status = raw
            .get("messages")
            .and_then(|m| m.get(0))
            .and_then(|m| m.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");

        if msg_status == "0" {
            Ok(SendResponse::success("vonage", "SMS sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_text = raw
                .get("messages")
                .and_then(|m| m.get(0))
                .and_then(|m| m.get("error-text"))
                .and_then(|e| e.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("vonage", format!("API error: {error_text}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
