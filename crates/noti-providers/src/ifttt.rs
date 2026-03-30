use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// IFTTT Webhook provider.
///
/// Triggers IFTTT Maker Webhooks (Webhooks service).
/// The event is triggered with value1=message, value2=title, value3=format/image.
/// Supports image attachments via base64 data URI in value3.
pub struct IftttProvider {
    client: Client,
}

impl IftttProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for IftttProvider {
    fn name(&self) -> &str {
        "ifttt"
    }

    fn url_scheme(&self) -> &str {
        "ifttt"
    }

    fn description(&self) -> &str {
        "IFTTT via Maker Webhooks"
    }

    fn example_url(&self) -> &str {
        "ifttt://<webhook_key>/<event_name>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_key", "IFTTT Webhooks service key")
                .with_example("dW7JkWxxxxxxxxxxxxxx"),
            ParamDef::required("event", "IFTTT event name to trigger").with_example("notification"),
            ParamDef::optional("value1", "Override value1 (default: message text)"),
            ParamDef::optional("value2", "Override value2 (default: title)"),
            ParamDef::optional(
                "value3",
                "Override value3 (default: format, or base64 image data URI if attachment present)",
            ),
        ]
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_key = config.require("webhook_key", "ifttt")?;
        let event = config.require("event", "ifttt")?;

        let url = format!("https://maker.ifttt.com/trigger/{event}/json/with/key/{webhook_key}");

        let value1 = config.get("value1").unwrap_or(&message.text).to_string();
        let value2 = config
            .get("value2")
            .map(|s| s.to_string())
            .or_else(|| message.title.clone())
            .unwrap_or_default();

        // If there's an image attachment and no explicit value3, embed it as data URI
        let value3 = if let Some(v3) = config.get("value3") {
            v3.to_string()
        } else if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                format!("data:{mime};base64,{b64}")
            } else {
                message.format.to_string()
            }
        } else {
            message.format.to_string()
        };

        let payload = json!({
            "value1": value1,
            "value2": value2,
            "value3": value3,
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("ifttt", "webhook triggered successfully")
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        } else {
            Ok(
                SendResponse::failure("ifttt", format!("API error (HTTP {status}): {body_text}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        }
    }
}
