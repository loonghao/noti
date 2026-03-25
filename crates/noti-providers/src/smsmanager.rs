use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// SMS Manager notification provider.
///
/// Uses the SMS Manager REST API to send SMS messages.
/// API docs: https://www.smsmanager.com/docs/api
pub struct SmsManagerProvider {
    client: Client,
}

impl SmsManagerProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SmsManagerProvider {
    fn name(&self) -> &str {
        "smsmanager"
    }

    fn url_scheme(&self) -> &str {
        "smsmanager"
    }

    fn description(&self) -> &str {
        "SMS Manager — bulk SMS messaging via REST API"
    }

    fn example_url(&self) -> &str {
        "smsmanager://<api_key>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "SMS Manager API key"),
            ParamDef::required("to", "Destination phone number").with_example("+15551234567"),
            ParamDef::optional("from", "Sender ID or phone number"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "smsmanager")?;
        let to = config.require("to", "smsmanager")?;

        let mut payload = json!({
            "apikey": api_key,
            "message": message.text,
            "number": to,
            "type": "default",
        });

        if let Some(from) = config.get("from") {
            payload["sender"] = json!(from);
        }

        let resp = self
            .client
            .post("https://http-api.smsmanager.com/Send")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(SendResponse::success("smsmanager", "SMS sent successfully")
                .with_status_code(status))
        } else {
            Ok(
                SendResponse::failure("smsmanager", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
