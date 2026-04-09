use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Clickatell SMS/messaging provider.
///
/// Sends messages via the Clickatell Platform REST API.
/// Supports media attachments via WhatsApp channel.
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
            ParamDef::optional("channel", "Channel: sms or whatsapp (default: sms)")
                .with_example("sms"),
            ParamDef::optional(
                "media_url",
                "Public URL for media attachment (alternative to file attachments)",
            ),
            ParamDef::optional("base_url", "API base URL override (default: https://platform.clickatell.com)")
                .with_example("https://platform.clickatell.com"),
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
        let api_key = config.require("api_key", "clickatell")?;
        let to = config.require("to", "clickatell")?;
        let channel = config.get("channel").unwrap_or("sms");

        let url = if let Some(base) = config.get("base_url") {
            format!("{}/messages", base.trim_end_matches('/'))
        } else {
            "https://platform.clickatell.com/messages".to_string()
        };

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut msg_payload = json!({
            "channel": channel,
            "to": to,
            "content": body_text
        });

        if let Some(from) = config.get("from") {
            msg_payload["from"] = json!(from);
        }

        // Add media attachments
        if message.has_attachments() {
            if let Some(attachment) = message.attachments.iter().find(|a| {
                matches!(
                    a.kind,
                    AttachmentKind::Image | AttachmentKind::Video | AttachmentKind::Audio
                )
            }) {
                let data = attachment.read_bytes().await?;
                let mime = attachment.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                msg_payload["media"] = json!({
                    "url": format!("data:{mime};base64,{b64}"),
                    "type": mime
                });
            }
        } else if let Some(media_url) = config.get("media_url") {
            msg_payload["media"] = json!({
                "url": media_url
            });
        }

        let payload = json!({
            "messages": [msg_payload]
        });

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
            let msg = if message.has_attachments() {
                "message sent with media via Clickatell"
            } else {
                "SMS sent via Clickatell"
            };
            Ok(SendResponse::success("clickatell", msg)
                .with_status_code(status)
                .with_raw_response(raw))
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
