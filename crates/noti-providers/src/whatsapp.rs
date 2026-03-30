use async_trait::async_trait;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// WhatsApp Business Cloud API provider.
///
/// Sends messages through Meta's WhatsApp Business Cloud API.
/// Supports media messages (image, audio, video, document) via media upload.
///
/// API reference: https://developers.facebook.com/docs/whatsapp/cloud-api/messages
pub struct WhatsAppProvider {
    client: Client,
}

impl WhatsAppProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn graph_url(api_version: &str, phone_number_id: &str, path: &str) -> String {
        format!("https://graph.facebook.com/{api_version}/{phone_number_id}/{path}")
    }
}

#[async_trait]
impl NotifyProvider for WhatsAppProvider {
    fn name(&self) -> &str {
        "whatsapp"
    }

    fn url_scheme(&self) -> &str {
        "whatsapp"
    }

    fn description(&self) -> &str {
        "WhatsApp Business Cloud API messaging"
    }

    fn example_url(&self) -> &str {
        "whatsapp://<access_token>@<phone_number_id>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required(
                "access_token",
                "WhatsApp Business API permanent access token",
            ),
            ParamDef::required(
                "phone_number_id",
                "Phone number ID from WhatsApp Business account",
            ),
            ParamDef::required(
                "to",
                "Recipient phone number in international format (e.g. +1234567890)",
            ),
            ParamDef::optional("api_version", "Graph API version (default: v21.0)"),
            ParamDef::optional(
                "preview_url",
                "Enable link previews (true/false, default: false)",
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

        let access_token = config.require("access_token", "whatsapp")?;
        let phone_number_id = config.require("phone_number_id", "whatsapp")?;
        let to = config.require("to", "whatsapp")?;
        let api_version = config.get("api_version").unwrap_or("v21.0");

        // Handle media attachments
        if message.has_attachments() {
            let attachment = &message.attachments[0];
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();
            let mime_str = attachment.effective_mime();

            // Step 1: Upload media
            let upload_url = Self::graph_url(api_version, phone_number_id, "media");

            let file_part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

            let form = reqwest::multipart::Form::new()
                .text("messaging_product", "whatsapp")
                .part("file", file_part);

            let upload_resp = self
                .client
                .post(&upload_url)
                .bearer_auth(access_token)
                .multipart(form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            let upload_raw: serde_json::Value = upload_resp
                .json()
                .await
                .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

            let media_id = upload_raw
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| NotiError::provider("whatsapp", "no media id in upload response"))?;

            // Step 2: Send media message
            let (msg_type, media_key) = match attachment.kind {
                AttachmentKind::Image => ("image", "image"),
                AttachmentKind::Audio => ("audio", "audio"),
                AttachmentKind::Video => ("video", "video"),
                AttachmentKind::File => ("document", "document"),
            };

            let messages_url = Self::graph_url(api_version, phone_number_id, "messages");

            let mut media_obj = json!({ "id": media_id });
            if !message.text.is_empty() {
                media_obj["caption"] = json!(message.text);
            }

            let body = json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": msg_type,
                media_key: media_obj
            });

            let resp = self
                .client
                .post(&messages_url)
                .bearer_auth(access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            return Self::parse_response(resp).await;
        }

        // Text-only message
        let preview_url = config.get("preview_url").unwrap_or("false") == "true";
        let url = Self::graph_url(api_version, phone_number_id, "messages");

        let body = json!({
            "messaging_product": "whatsapp",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": preview_url,
                "body": message.text
            }
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        Self::parse_response(resp).await
    }
}

impl WhatsAppProvider {
    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("whatsapp", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("whatsapp", format!("WhatsApp API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
