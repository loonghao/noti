use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Twist team messaging provider.
///
/// Sends messages via Twist incoming webhooks.
/// Twist is an async-first team communication tool by Doist.
pub struct TwistProvider {
    client: Client,
}

impl TwistProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for TwistProvider {
    fn name(&self) -> &str {
        "twist"
    }

    fn url_scheme(&self) -> &str {
        "twist"
    }

    fn description(&self) -> &str {
        "Twist team messaging via incoming webhooks"
    }

    fn example_url(&self) -> &str {
        "twist://<token_a>/<token_b>/<token_c>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Twist integration webhook URL")
                .with_example("https://twist.com/api/v3/integration_incoming/post_data?install_id=XXX&install_token=YYY"),
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
        let webhook_url = config.require("webhook_url", "twist")?;

        let mut content = if let Some(ref title) = message.title {
            format!("**{title}**\n{}", message.text)
        } else {
            message.text.clone()
        };

        // Embed images in markdown and list file attachments
        if message.has_attachments() {
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    content.push_str(&format!("\n\n![{file_name}](data:{mime};base64,{b64})"));
                } else {
                    content.push_str(&format!("\n\n📎 **Attachment:** {file_name}"));
                }
            }
        }

        let payload = json!({
            "content": content
        });

        let resp = self
            .client
            .post(webhook_url)
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
            Ok(SendResponse::success("twist", "message sent to Twist")
                .with_status_code(status)
                .with_raw_response(json!({"response": body})))
        } else {
            Ok(SendResponse::failure("twist", format!("API error: {body}"))
                .with_status_code(status)
                .with_raw_response(json!({"response": body})))
        }
    }
}
