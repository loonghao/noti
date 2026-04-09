use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// MessageBird SMS/MMS provider.
///
/// Sends SMS/MMS messages via the MessageBird REST API.
/// Supports MMS with mediaUrls for image/video attachments.
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
        "MessageBird SMS/MMS via REST API"
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
            ParamDef::optional(
                "media_url",
                "Public URL for MMS media (alternative to file attachments)",
            ),
            ParamDef::optional("base_url", "API base URL override (default: https://rest.messagebird.com)")
                .with_example("https://rest.messagebird.com"),
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
        let access_key = config.require("access_key", "messagebird")?;
        let from = config.require("from", "messagebird")?;
        let to = config.require("to", "messagebird")?;

        let base_url = config
            .get("base_url")
            .unwrap_or("https://rest.messagebird.com")
            .trim_end_matches('/');
        let url = format!("{base_url}/messages");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "originator": from,
            "recipients": [to],
            "body": body_text,
        });

        // Add MMS media attachments
        if message.has_attachments() {
            let mut media_urls = Vec::new();
            for attachment in &message.attachments {
                if matches!(
                    attachment.kind,
                    AttachmentKind::Image | AttachmentKind::Video | AttachmentKind::Audio
                ) {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    media_urls.push(format!("data:{mime};base64,{b64}"));
                }
            }
            if !media_urls.is_empty() {
                payload["type"] = json!("mms");
                payload["mediaUrls"] = json!(media_urls);
            }
        } else if let Some(media_url) = config.get("media_url") {
            payload["type"] = json!("mms");
            payload["mediaUrls"] = json!([media_url]);
        }

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
            let msg = if message.has_attachments() {
                "MMS sent with media via MessageBird"
            } else {
                "SMS sent via MessageBird"
            };
            Ok(SendResponse::success("messagebird", msg)
                .with_status_code(status)
                .with_raw_response(raw))
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
