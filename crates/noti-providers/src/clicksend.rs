use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// ClickSend SMS provider.
///
/// Sends SMS messages via the ClickSend REST API v3.
///
/// API reference: <https://developers.clicksend.com/docs/rest/v3/>
pub struct ClickSendProvider {
    client: Client,
}

impl ClickSendProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ClickSendProvider {
    fn name(&self) -> &str {
        "clicksend"
    }

    fn url_scheme(&self) -> &str {
        "clicksend"
    }

    fn description(&self) -> &str {
        "ClickSend SMS messaging gateway"
    }

    fn example_url(&self) -> &str {
        "clicksend://<username>:<api_key>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("username", "ClickSend account username"),
            ParamDef::required("api_key", "ClickSend API key"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15551234567"),
            ParamDef::optional("from", "Sender phone number or name").with_example("+15559876543"),
            ParamDef::optional("schedule", "Unix timestamp for scheduled delivery"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let username = config.require("username", "clicksend")?;
        let api_key = config.require("api_key", "clicksend")?;
        let to = config.require("to", "clicksend")?;

        let url = "https://rest.clicksend.com/v3/sms/send";

        let mut sms_message = json!({
            "body": message.text,
            "to": to,
            "source": "noti-cli",
        });

        if let Some(from) = config.get("from") {
            sms_message["from"] = json!(from);
        }
        if let Some(schedule) = config.get("schedule") {
            sms_message["schedule"] = json!(schedule.parse::<u64>().unwrap_or(0));
        }

        let payload = json!({
            "messages": [sms_message],
        });

        let auth =
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{api_key}"));

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Basic {auth}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let http_code = raw.get("http_code").and_then(|v| v.as_u64()).unwrap_or(0);

        if http_code == 200 {
            Ok(SendResponse::success("clicksend", "SMS sent via ClickSend")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("response_msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("clicksend", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

use base64::Engine;
