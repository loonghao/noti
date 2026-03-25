use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Clickatell SMS provider.
///
/// Sends SMS messages via the Clickatell Platform REST API.
/// Requires an API key and recipient phone number.
pub struct ClickatellProvider {
    client: Client,
}

impl ClickatellProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ClickatellProvider {
    fn name(&self) -> &str {
        "clickatell"
    }

    fn url_scheme(&self) -> &str {
        "clickatell"
    }

    fn description(&self) -> &str {
        "Clickatell SMS/messaging gateway"
    }

    fn example_url(&self) -> &str {
        "clickatell://<api_key>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Clickatell API key").with_example("your-api-key"),
            ParamDef::required("to", "Recipient phone number (international format)")
                .with_example("15559876543"),
            ParamDef::optional("from", "Sender ID or phone number").with_example("+15551234567"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "clickatell")?;
        let to = config.require("to", "clickatell")?;

        let url = "https://platform.clickatell.com/messages";

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "messages": [{
                "channel": "sms",
                "to": to,
                "content": body_text
            }]
        });

        if let Some(from) = config.get("from") {
            payload["messages"][0]["from"] = json!(from);
        }

        let resp = self
            .client
            .post(url)
            .header("Authorization", api_key)
            .header("Accept", "application/json")
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
            Ok(
                SendResponse::success("clickatell", "SMS sent via Clickatell")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("clickatell", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
