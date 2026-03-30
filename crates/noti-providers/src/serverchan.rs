use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// ServerChan (Server酱) push notification provider.
///
/// Supports image attachments embedded as base64 data URIs in the markdown
/// `desp` field. Non-image attachments are listed as file references.
pub struct ServerChanProvider {
    client: Client,
}

impl ServerChanProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ServerChanProvider {
    fn name(&self) -> &str {
        "serverchan"
    }

    fn url_scheme(&self) -> &str {
        "serverchan"
    }

    fn description(&self) -> &str {
        "ServerChan (Server酱) push to WeChat"
    }

    fn example_url(&self) -> &str {
        "serverchan://<send_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("send_key", "ServerChan SendKey (SCT...)")
                .with_example("SCTxxxxxxxxxxx"),
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
        let send_key = config.require("send_key", "serverchan")?;

        let url = format!("https://sctapi.ftqq.com/{send_key}.send");

        let title = message.title.as_deref().unwrap_or("Notification");

        // Build desp with embedded attachments
        let desp = if message.has_attachments() {
            let mut md = message.text.clone();
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    md.push_str(&format!("\n\n![{file_name}](data:{mime};base64,{b64})"));
                } else {
                    md.push_str(&format!("\n\n📎 **Attachment:** {file_name}"));
                }
            }
            md
        } else {
            message.text.clone()
        };

        let form = vec![("title", title.to_string()), ("desp", desp)];

        let resp = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let code = raw.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code == 0 {
            Ok(
                SendResponse::success("serverchan", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("serverchan", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
