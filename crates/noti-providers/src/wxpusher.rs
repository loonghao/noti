use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// WxPusher provider.
///
/// Sends push notifications via WxPusher, a WeChat push notification service.
/// Messages are delivered to WeChat Official Account followers.
///
/// API reference: <https://wxpusher.zjiecode.com/docs/>
pub struct WxPusherProvider {
    client: Client,
}

impl WxPusherProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for WxPusherProvider {
    fn name(&self) -> &str {
        "wxpusher"
    }

    fn url_scheme(&self) -> &str {
        "wxpusher"
    }

    fn description(&self) -> &str {
        "WxPusher WeChat push notifications"
    }

    fn example_url(&self) -> &str {
        "wxpusher://<app_token>/<uid>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("app_token", "WxPusher application token").with_example("AT_xxxx"),
            ParamDef::required("uid", "Target user UID").with_example("UID_xxxx"),
            ParamDef::optional("topic_id", "Topic ID for topic-based push").with_example("12345"),
            ParamDef::optional("content_type", "Content type: 1=text, 2=html, 3=markdown")
                .with_example("1"),
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
        let app_token = config.require("app_token", "wxpusher")?;
        let uid = config.require("uid", "wxpusher")?;

        let url = "https://wxpusher.zjiecode.com/api/send/message";

        let content_type = config
            .get("content_type")
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(1);

        // Build content with embedded images for attachments
        let (content, effective_content_type) = if message.has_attachments() {
            let mut html = if let Some(ref title) = message.title {
                format!("<h3>{title}</h3><p>{}</p>", message.text)
            } else {
                format!("<p>{}</p>", message.text)
            };
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    html.push_str(&format!(
                        "<p><img src=\"data:{mime};base64,{b64}\" alt=\"{file_name}\" style=\"max-width:100%\" /></p>"
                    ));
                } else {
                    html.push_str(&format!("<p>📎 Attachment: {file_name}</p>"));
                }
            }
            (html, 2) // contentType 2 = HTML
        } else {
            let content = if let Some(ref title) = message.title {
                format!("{title}\n\n{}", message.text)
            } else {
                message.text.clone()
            };
            (content, content_type)
        };

        let mut payload = json!({
            "appToken": app_token,
            "content": content,
            "contentType": effective_content_type,
            "uids": [uid],
        });

        if let Some(ref title) = message.title {
            payload["summary"] = json!(title);
        }

        if let Some(topic_id) = config.get("topic_id") {
            if let Ok(tid) = topic_id.parse::<i64>() {
                payload["topicIds"] = json!([tid]);
            }
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
        if code == 1000 {
            Ok(
                SendResponse::success("wxpusher", "message sent via WxPusher")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("wxpusher", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
