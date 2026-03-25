use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// MessageBird SMS provider.
///
/// Sends SMS messages via the MessageBird REST API.
/// MessageBird is a global cloud communications platform.
///
/// API reference: <https://developers.messagebird.com/api/sms-messaging/>
pub struct MessageBirdProvider {
    client: Client,
}

impl MessageBirdProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for MessageBirdProvider {
    fn name(&self) -> &str {
        "messagebird"
    }

    fn url_scheme(&self) -> &str {
        "msgbird"
    }

    fn description(&self) -> &str {
        "MessageBird SMS via REST API"
    }

    fn example_url(&self) -> &str {
        "msgbird://<access_key>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_key", "MessageBird access key")
                .with_example("your-access-key"),
            ParamDef::required("from", "Sender name or phone number").with_example("MyApp"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15551234567"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_key = config.require("access_key", "messagebird")?;
        let from = config.require("from", "messagebird")?;
        let to = config.require("to", "messagebird")?;

        let url = "https://rest.messagebird.com/messages";

        let body_text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "originator": from,
            "recipients": [to],
            "body": body_text,
        });

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("AccessKey {access_key}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("messagebird", "SMS sent via MessageBird")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("errors")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|e| e.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("messagebird", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
