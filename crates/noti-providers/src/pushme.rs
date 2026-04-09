use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// PushMe push notification service.
///
/// API reference: https://push.i-i.me/
pub struct PushMeProvider {
    client: Client,
}

impl PushMeProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushMeProvider {
    fn name(&self) -> &str {
        "pushme"
    }

    fn url_scheme(&self) -> &str {
        "pushme"
    }

    fn description(&self) -> &str {
        "PushMe push notification service"
    }

    fn example_url(&self) -> &str {
        "pushme://<push_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("push_key", "PushMe push key"),
            ParamDef::optional(
                "type",
                "Message type: text, markdown, image (default: text)",
            ),
            ParamDef::optional("base_url", "PushMe API base URL (default: https://push.i-i.me)")
                .with_example("https://push.i-i.me"),
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

        let push_key = config.require("push_key", "pushme")?;
        let title = message.title.as_deref().unwrap_or("noti");
        let msg_type = config.get("type").unwrap_or("text");

        // If there's an image attachment, send as image type
        let (content, effective_type) = if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            (format!("data:{mime};base64,{b64}"), "image")
        } else if message.has_attachments() {
            // Non-image attachments: include file info in markdown
            let mut md = message.text.clone();
            for att in &message.attachments {
                md.push_str(&format!(
                    "\n\n📎 **Attachment:** {}",
                    att.effective_file_name()
                ));
            }
            (md, "markdown")
        } else {
            (message.text.clone(), msg_type)
        };

        let body = serde_json::json!({
            "push_key": push_key,
            "title": title,
            "content": content,
            "type": effective_type
        });

        let base = config.get("base_url").unwrap_or("https://push.i-i.me");
        let api_url = format!("{}/", base.trim_end_matches('/'));

        let resp = self
            .client
            .post(&api_url)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("pushme", "push notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("pushme", format!("PushMe API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
