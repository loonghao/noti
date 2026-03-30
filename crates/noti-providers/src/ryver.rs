use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig,
    SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Ryver team messaging provider.
///
/// Sends messages via Ryver incoming webhooks.
/// Ryver is a team collaboration platform with chat and task management.
pub struct RyverProvider {
    client: Client,
}

impl RyverProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for RyverProvider {
    fn name(&self) -> &str {
        "ryver"
    }

    fn url_scheme(&self) -> &str {
        "ryver"
    }

    fn description(&self) -> &str {
        "Ryver team messaging via incoming webhooks"
    }

    fn example_url(&self) -> &str {
        "ryver://<organization>/<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("organization", "Ryver organization name").with_example("mycompany"),
            ParamDef::required("token", "Incoming webhook token")
                .with_example("your-webhook-token"),
            ParamDef::optional("webhook_type", "Type: forum or team (default: forum)")
                .with_example("forum"),
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
        let organization = config.require("organization", "ryver")?;
        let token = config.require("token", "ryver")?;
        let _webhook_type = config.get("webhook_type").unwrap_or("forum");

        let url = format!("https://{organization}.ryver.com/application/24/incoming/{token}");

        let mut body_text = if let Some(ref title) = message.title {
            if matches!(message.format, MessageFormat::Markdown) {
                format!("**{title}**\n\n{}", message.text)
            } else {
                format!("{title}\n\n{}", message.text)
            }
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
                    body_text.push_str(&format!("\n\n![{file_name}](data:{mime};base64,{b64})"));
                } else {
                    body_text.push_str(&format!("\n\n📎 **Attachment:** {file_name}"));
                }
            }
        }

        let payload = json!({
            "body": body_text
        });

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
            Ok(SendResponse::success("ryver", "message sent to Ryver")
                .with_status_code(status)
                .with_raw_response(json!({"response": body})))
        } else {
            Ok(SendResponse::failure("ryver", format!("API error: {body}"))
                .with_status_code(status)
                .with_raw_response(json!({"response": body})))
        }
    }
}
