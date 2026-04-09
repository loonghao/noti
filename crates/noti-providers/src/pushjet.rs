use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Pushjet push notification provider.
///
/// Sends push notifications via a Pushjet server.
/// Supports image attachments via base64 data URI in the link field.
///
/// API reference: <https://pushjet.io/docs/api>
pub struct PushjetProvider {
    client: Client,
}

impl PushjetProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushjetProvider {
    fn name(&self) -> &str {
        "pushjet"
    }

    fn url_scheme(&self) -> &str {
        "pushjet"
    }

    fn description(&self) -> &str {
        "Pushjet push notification service"
    }

    fn example_url(&self) -> &str {
        "pushjet://<secret_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("secret", "Pushjet service secret key"),
            ParamDef::optional(
                "server",
                "Pushjet server URL (default: https://api.pushjet.io)",
            )
            .with_example("https://api.pushjet.io"),
            ParamDef::optional("level", "Notification importance level 1-5 (default: 3)")
                .with_example("3"),
            ParamDef::optional("link", "URL to attach to notification"),
            ParamDef::optional("base_url", "Override base URL for API requests (takes precedence over server)"),
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
        let secret = config.require("secret", "pushjet")?;
        let server = config.get("base_url").or_else(|| config.get("server")).unwrap_or("https://api.pushjet.io");
        let level = config
            .get("level")
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(3);

        let url = format!("{}/message", server.trim_end_matches('/'));

        let mut payload = json!({
            "secret": secret,
            "message": message.text,
            "level": level,
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        // Use attachment as link (base64 data URI for images)
        if message.has_attachments() {
            if let Some(attachment) = message
                .attachments
                .iter()
                .find(|a| matches!(a.kind, AttachmentKind::Image))
            {
                let data = attachment.read_bytes().await?;
                let mime = attachment.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                payload["link"] = json!(format!("data:{mime};base64,{b64}"));
            }
        } else if let Some(link) = config.get("link") {
            payload["link"] = json!(link);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            let msg = if message.has_attachments() {
                "notification with image sent via Pushjet"
            } else {
                "notification sent via Pushjet"
            };
            Ok(SendResponse::success("pushjet", msg).with_status_code(status))
        } else {
            Ok(
                SendResponse::failure("pushjet", format!("API error ({status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        }
    }
}
