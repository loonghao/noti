use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// BulkVS SMS/MMS provider.
///
/// Supports MMS via MediaUrl parameter for sending images and media.
///
/// API reference: https://portal.bulkvs.com/api-docs/
pub struct BulkVsProvider {
    client: Client,
}

impl BulkVsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BulkVsProvider {
    fn name(&self) -> &str {
        "bulkvs"
    }

    fn url_scheme(&self) -> &str {
        "bulkvs"
    }

    fn description(&self) -> &str {
        "BulkVS SMS/MMS messaging via REST API"
    }

    fn example_url(&self) -> &str {
        "bulkvs://<username>:<password>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("username", "BulkVS account username"),
            ParamDef::required("password", "BulkVS account password"),
            ParamDef::required("from", "Sender phone number (must be a BulkVS number)"),
            ParamDef::required("to", "Recipient phone number"),
            ParamDef::optional(
                "media_url",
                "Public URL for MMS media (alternative to file attachments)",
            ),
            ParamDef::optional("base_url", "API base URL override (default: https://portal.bulkvs.com)")
                .with_example("https://portal.bulkvs.com"),
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

        let username = config.require("username", "bulkvs")?;
        let password = config.require("password", "bulkvs")?;
        let from = config.require("from", "bulkvs")?;
        let to = config.require("to", "bulkvs")?;

        let mut msg_obj = serde_json::json!({
            "From": from,
            "To": [to],
            "Body": message.text
        });

        // Add MMS media
        if message.has_attachments() {
            let mut media_urls = Vec::new();
            for attachment in &message.attachments {
                if matches!(
                    attachment.kind,
                    AttachmentKind::Image | AttachmentKind::Video | AttachmentKind::Audio
                ) {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    media_urls.push(format!("data:{mime};base64,{b64}"));
                }
            }
            if !media_urls.is_empty() {
                msg_obj["MediaUrl"] = serde_json::json!(media_urls);
            }
        } else if let Some(media_url) = config.get("media_url") {
            msg_obj["MediaUrl"] = serde_json::json!([media_url]);
        }

        let body = serde_json::json!({
            "AuthenticationCredentials": {
                "Username": username,
                "Password": password
            },
            "Message": msg_obj
        });

        let base_url = config
            .get("base_url")
            .unwrap_or("https://portal.bulkvs.com")
            .trim_end_matches('/');

        let resp = self
            .client
            .post(format!("{base_url}/api/3.0/message"))
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            let msg = if message.has_attachments() {
                "MMS sent with attachments via BulkVS"
            } else {
                "SMS sent successfully"
            };
            Ok(SendResponse::success("bulkvs", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("bulkvs", format!("BulkVS API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
