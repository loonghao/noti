use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// SpugPush webhook notification provider.
///
/// SpugPush is a simple webhook service from the Spug monitoring platform.
/// Supports image attachments embedded as base64 data URI in content.
///
/// API Reference: <https://push.spug.dev>
pub struct SpugPushProvider {
    client: Client,
}

impl SpugPushProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SpugPushProvider {
    fn name(&self) -> &str {
        "spugpush"
    }

    fn url_scheme(&self) -> &str {
        "spugpush"
    }

    fn description(&self) -> &str {
        "SpugPush webhook notifications (Spug monitoring)"
    }

    fn example_url(&self) -> &str {
        "spugpush://<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token", "SpugPush authentication token (32-64 chars)")
                .with_example("abc123def456ghi789jkl012mno345pq"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        false
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token = config.require("token", "spugpush")?;

        let url = format!("https://push.spug.dev/send/{token}");

        // Embed image attachments as base64 in content
        let mut content = message.text.clone();
        for attachment in &message.attachments {
            if attachment.kind == AttachmentKind::Image {
                if let Ok(data) = attachment.read_bytes().await {
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let name = attachment.effective_file_name();
                    content.push_str(&format!("\n\n![{name}](data:{mime};base64,{b64})"));
                }
            }
        }

        let mut payload = json!({
            "content": content,
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"status": status}));

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("spugpush", "notification sent via SpugPush")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("error")
                .or_else(|| raw.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("spugpush", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
