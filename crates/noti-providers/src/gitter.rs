use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Gitter chat provider.
///
/// Sends messages to Gitter rooms via the Gitter REST API.
/// Gitter is a developer-focused chat platform (now part of Matrix/Element).
/// Supports attachments by embedding base64 data URIs in Markdown — images
/// are rendered inline, and other files are sent as download links.
///
/// API reference: <https://developer.gitter.im/docs/messages-resource>
pub struct GitterProvider {
    client: Client,
}

impl GitterProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for GitterProvider {
    fn name(&self) -> &str {
        "gitter"
    }

    fn url_scheme(&self) -> &str {
        "gitter"
    }

    fn description(&self) -> &str {
        "Gitter developer chat via REST API"
    }

    fn example_url(&self) -> &str {
        "gitter://<token>/<room_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token", "Gitter personal access token")
                .with_example("your-access-token"),
            ParamDef::required("room_id", "Gitter room ID").with_example("5xxxxxxxxxxxxxxxxxxxxx"),
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
        let token = config.require("token", "gitter")?;
        let room_id = config.require("room_id", "gitter")?;

        let url = format!("https://api.gitter.im/v1/rooms/{room_id}/chatMessages");

        let mut text = if let Some(ref title) = message.title {
            format!("**{title}**\n{}", message.text)
        } else {
            message.text.clone()
        };

        // Embed attachments as Markdown image/links with base64 data URIs
        for attachment in &message.attachments {
            let data = attachment.read_bytes().await?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            let mime = attachment.effective_mime();
            let file_name = attachment.effective_file_name();
            let data_uri = format!("data:{mime};base64,{b64}");

            match attachment.kind {
                AttachmentKind::Image => {
                    text.push_str(&format!("\n\n![{file_name}]({data_uri})"));
                }
                _ => {
                    text.push_str(&format!("\n\n📎 [{file_name}]({data_uri})"));
                }
            }
        }

        let payload = json!({
            "text": text
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("gitter", "message sent to Gitter")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("gitter", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
