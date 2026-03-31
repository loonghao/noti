use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// Synology Chat incoming webhook provider.
///
/// Supports image attachments embedded as base64 data URIs in the message text.
/// Non-image attachments are encoded as base64 and included as data references.
///
/// API reference: https://kb.synology.com/en-global/DSM/tutorial/How_to_configure_webhooks_in_Synology_Chat
pub struct SynologyProvider {
    client: Client,
}

impl SynologyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SynologyProvider {
    fn name(&self) -> &str {
        "synology"
    }

    fn url_scheme(&self) -> &str {
        "synology"
    }

    fn description(&self) -> &str {
        "Synology Chat incoming webhook"
    }

    fn example_url(&self) -> &str {
        "synology://<token>@<host>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Synology NAS host (e.g. nas.local or 192.168.1.1)"),
            ParamDef::required("token", "Incoming webhook token"),
            ParamDef::optional("port", "HTTPS port (default: 5001)"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
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

        let host = config.require("host", "synology")?;
        let token = config.require("token", "synology")?;
        let port = config.get("port").unwrap_or("5001");
        let scheme = config.get("scheme").unwrap_or("https");

        let url = format!(
            "{scheme}://{host}:{port}/webapi/entry.cgi?api=SYNO.Chat.External&method=incoming&version=2&token=%22{token}%22"
        );

        // Build payload with optional file_url for attachments
        let mut payload_obj = serde_json::json!({"text": message.text});

        if message.has_attachments() {
            let mut text = message.text.clone();
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    text.push_str(&format!("\n\n![{file_name}](data:{mime};base64,{b64})"));
                } else {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    text.push_str(&format!("\n📎 {file_name} (data:{mime};base64,{b64})"));
                }
            }
            payload_obj["text"] = serde_json::json!(text);
        }

        let body = format!("payload={payload_obj}");

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("synology", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("synology", format!("Synology Chat API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
