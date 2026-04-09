use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// D7 Networks SMS/MMS provider.
///
/// Sends messages via the D7 Networks REST API.
/// Supports media attachments via WhatsApp/Viber channels and data URIs.
pub struct D7NetworksProvider {
    client: Client,
}

impl D7NetworksProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for D7NetworksProvider {
    fn name(&self) -> &str {
        "d7sms"
    }

    fn url_scheme(&self) -> &str {
        "d7sms"
    }

    fn description(&self) -> &str {
        "D7 Networks SMS/messaging gateway"
    }

    fn example_url(&self) -> &str {
        "d7sms://<api_token>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_token", "D7 Networks API token").with_example("your-api-token"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
            ParamDef::optional("from", "Sender ID / phone number").with_example("SMSINFO"),
            ParamDef::optional("channel", "Channel: sms, whatsapp, viber (default: sms)")
                .with_example("sms"),
            ParamDef::optional(
                "media_url",
                "Public URL for media attachment (alternative to file attachments)",
            ),
            ParamDef::optional("base_url", "API base URL override (default: https://api.d7networks.com)")
                .with_example("https://api.d7networks.com"),
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
        let api_token = config.require("api_token", "d7sms")?;
        let to = config.require("to", "d7sms")?;
        let from = config.get("from").unwrap_or("SMSINFO");
        let channel = config.get("channel").unwrap_or("sms");

        let base_url = config
            .get("base_url")
            .unwrap_or("https://api.d7networks.com")
            .trim_end_matches('/');
        let url = format!("{base_url}/messages/v1/send");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut msg_type = "text";
        let mut media_data = None;

        // Handle attachments
        if message.has_attachments() {
            if let Some(attachment) = message.attachments.iter().find(|a| {
                matches!(
                    a.kind,
                    AttachmentKind::Image | AttachmentKind::Video | AttachmentKind::Audio
                )
            }) {
                let data = attachment.read_bytes().await?;
                let mime = attachment.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                media_data = Some(json!({
                    "media_url": format!("data:{mime};base64,{b64}"),
                    "media_type": mime
                }));
                msg_type = "media";
            }
        } else if let Some(media_url) = config.get("media_url") {
            media_data = Some(json!({
                "media_url": media_url
            }));
            msg_type = "media";
        }

        let mut msg_payload = json!({
            "channel": channel,
            "recipients": [to],
            "content": body_text,
            "msg_type": msg_type,
            "data_coding": "text"
        });

        if let Some(ref media) = media_data {
            msg_payload["media"] = media.clone();
        }

        let payload = json!({
            "messages": [msg_payload],
            "message_globals": {
                "originator": from,
                "report_url": ""
            }
        });

        let resp = self
            .client
            .post(url)
            .bearer_auth(api_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("d7networks", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            let msg = if message.has_attachments() {
                "message sent with media via D7 Networks"
            } else {
                "SMS sent via D7 Networks"
            };
            Ok(SendResponse::success("d7sms", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("d7sms", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
