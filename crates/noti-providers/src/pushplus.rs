use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Pushplus push notification provider.
///
/// Sends push notifications via Pushplus (pushplus.plus), a popular
/// Chinese push notification service supporting WeChat, SMS, email, etc.
///
/// API reference: <https://www.pushplus.plus/doc/>
pub struct PushplusProvider {
    client: Client,
}

impl PushplusProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushplusProvider {
    fn name(&self) -> &str {
        "pushplus"
    }

    fn url_scheme(&self) -> &str {
        "pushplus"
    }

    fn description(&self) -> &str {
        "Pushplus push notifications (WeChat/SMS/Email)"
    }

    fn example_url(&self) -> &str {
        "pushplus://<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token", "Pushplus user token").with_example("your-token"),
            ParamDef::optional("topic", "Group topic code for multi-user push")
                .with_example("my-topic"),
            ParamDef::optional("template", "Message template: html, txt, json, markdown")
                .with_example("html"),
            ParamDef::optional("channel", "Push channel: wechat, webhook, mail, sms")
                .with_example("wechat"),
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
        let token = config.require("token", "pushplus")?;

        let url = "http://www.pushplus.plus/send";

        let title = message.title.as_deref().unwrap_or("Notification");

        // Build content with embedded images for attachments
        let content = if message.has_attachments() {
            let mut html = format!("<p>{}</p>", message.text);
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    html.push_str(&format!(
                        "<p><img src=\"data:{mime};base64,{b64}\" alt=\"{file_name}\" /></p>"
                    ));
                } else {
                    html.push_str(&format!("<p>📎 Attachment: {file_name}</p>"));
                }
            }
            html
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "token": token,
            "title": title,
            "content": content,
        });

        // Force HTML template when attachments are present
        if message.has_attachments() {
            payload["template"] = json!("html");
        }

        if let Some(topic) = config.get("topic") {
            payload["topic"] = json!(topic);
        }
        if let Some(template) = config.get("template") {
            payload["template"] = json!(template);
        }
        if let Some(channel) = config.get("channel") {
            payload["channel"] = json!(channel);
        }

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        let code = raw.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code == 200 {
            Ok(
                SendResponse::success("pushplus", "notification sent via Pushplus")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("pushplus", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
