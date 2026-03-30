use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// SMSEagle hardware SMS gateway provider.
///
/// Sends SMS/MMS via SMSEagle hardware appliance HTTP API v2.
/// When attachments are present (images), the provider switches to the MMS endpoint.
///
/// API reference: <https://www.smseagle.eu/api/>
pub struct SmsEagleProvider {
    client: Client,
}

impl SmsEagleProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SmsEagleProvider {
    fn name(&self) -> &str {
        "smseagle"
    }

    fn url_scheme(&self) -> &str {
        "smseagle"
    }

    fn description(&self) -> &str {
        "SMSEagle hardware SMS/MMS gateway"
    }

    fn example_url(&self) -> &str {
        "smseagle://<access_token>@<host>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "SMSEagle device host/IP").with_example("192.168.1.100"),
            ParamDef::required("access_token", "SMSEagle API access token"),
            ParamDef::required("to", "Recipient phone number").with_example("+15559876543"),
            ParamDef::optional("scheme", "HTTP scheme: http or https (default: https)")
                .with_example("https"),
            ParamDef::optional("port", "Port number (default: auto)"),
            ParamDef::optional("priority", "Message priority: 0-9 (default: 0)").with_example("0"),
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
        let host = config.require("host", "smseagle")?;
        let access_token = config.require("access_token", "smseagle")?;
        let to = config.require("to", "smseagle")?;
        let scheme = config.get("scheme").unwrap_or("https");

        let text = if let Some(ref title) = message.title {
            format!("{title}\n{}", message.text)
        } else {
            message.text.clone()
        };

        let base_url = if let Some(port) = config.get("port") {
            format!("{scheme}://{host}:{port}")
        } else {
            format!("{scheme}://{host}")
        };

        // Use MMS endpoint when image attachments are present
        let has_media = message.has_attachments()
            && message
                .attachments
                .iter()
                .any(|a| matches!(a.kind, AttachmentKind::Image));

        let url = if has_media {
            format!("{base_url}/api/v2/messages/mms")
        } else {
            format!("{base_url}/api/v2/messages/sms")
        };

        let mut payload = serde_json::json!({
            "to": [to],
            "text": text,
        });

        if let Some(priority) = config.get("priority") {
            payload["priority"] = serde_json::json!(priority.parse::<u8>().unwrap_or(0));
        }

        // Attach base64-encoded images for MMS
        if has_media {
            let mut attachments = Vec::new();
            for attachment in &message.attachments {
                if matches!(attachment.kind, AttachmentKind::Image) {
                    let data = attachment.read_bytes().await?;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let mime = attachment.effective_mime();
                    let name = attachment.effective_file_name();
                    attachments.push(serde_json::json!({
                        "content": b64,
                        "content_type": mime,
                        "filename": name,
                    }));
                }
            }
            if !attachments.is_empty() {
                payload["attachments"] = serde_json::json!(attachments);
            }
        }

        let resp = self
            .client
            .post(&url)
            .header("access-token", access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        let msg_type = if has_media { "MMS" } else { "SMS" };

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("smseagle", format!("{msg_type} sent via SMSEagle"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"body": body})),
            )
        } else {
            Ok(
                SendResponse::failure("smseagle", format!("API error ({status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"body": body})),
            )
        }
    }
}
