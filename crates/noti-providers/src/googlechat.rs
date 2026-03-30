use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig,
    SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Google Chat (formerly Hangouts Chat) webhook provider.
///
/// Uses the Google Chat Spaces webhook URL to post messages.
/// Supports image attachments via cardsV2 Image widget with base64 data URI.
/// Non-image attachments are listed as file references in the text.
///
/// The webhook URL looks like:
/// `https://chat.googleapis.com/v1/spaces/<space>/messages?key=<key>&token=<token>`
pub struct GoogleChatProvider {
    client: Client,
}

impl GoogleChatProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for GoogleChatProvider {
    fn name(&self) -> &str {
        "googlechat"
    }

    fn url_scheme(&self) -> &str {
        "gchat"
    }

    fn description(&self) -> &str {
        "Google Chat via space webhook"
    }

    fn example_url(&self) -> &str {
        "gchat://<space>/<key>/<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Google Chat space webhook URL").with_example(
                "https://chat.googleapis.com/v1/spaces/AAAA/messages?key=xxx&token=yyy",
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
        let webhook_url = config.require("webhook_url", "googlechat")?;

        let mut text = String::new();
        if let Some(ref title) = message.title {
            match message.format {
                MessageFormat::Markdown | MessageFormat::Html => {
                    text.push_str(&format!("*{title}*\n\n"));
                }
                MessageFormat::Text => {
                    text.push_str(&format!("{title}\n\n"));
                }
            }
        }
        text.push_str(&message.text);

        // Build payload — use cardsV2 when image attachments are present
        let payload = if message.has_attachments() {
            let mut widgets = Vec::new();

            // Text widget
            widgets.push(json!({
                "textParagraph": { "text": text }
            }));

            // Image widgets for image attachments
            for attachment in &message.attachments {
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    widgets.push(json!({
                        "image": {
                            "imageUrl": format!("data:{mime};base64,{b64}"),
                            "altText": attachment.effective_file_name()
                        }
                    }));
                } else {
                    let file_name = attachment.effective_file_name();
                    widgets.push(json!({
                        "textParagraph": {
                            "text": format!("📎 Attachment: {file_name}")
                        }
                    }));
                }
            }

            json!({
                "cardsV2": [{
                    "cardId": "noti-attachment",
                    "card": {
                        "sections": [{
                            "widgets": widgets
                        }]
                    }
                }]
            })
        } else {
            json!({ "text": text })
        };

        let resp = self
            .client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("googlechat", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error_msg = raw
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("googlechat", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
