use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Plivo SMS/MMS provider.
///
/// Sends SMS/MMS messages via the Plivo REST API.
/// Plivo is a global cloud communication platform for voice and messaging.
/// Supports MMS with media_urls for image/video/audio attachments.
///
/// API reference: <https://www.plivo.com/docs/sms/api/message#send-a-message>
pub struct PlivoProvider {
    client: Client,
}

impl PlivoProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PlivoProvider {
    fn name(&self) -> &str {
        "plivo"
    }

    fn url_scheme(&self) -> &str {
        "plivo"
    }

    fn description(&self) -> &str {
        "Plivo SMS/MMS via REST API"
    }

    fn example_url(&self) -> &str {
        "plivo://<auth_id>:<auth_token>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("auth_id", "Plivo Auth ID").with_example("MAXXXXXXXXXXXXXXXXXX"),
            ParamDef::required("auth_token", "Plivo Auth Token").with_example("your-auth-token"),
            ParamDef::required("from", "Sender phone number (E.164 format)")
                .with_example("+15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
            ParamDef::optional(
                "media_url",
                "Public URL for MMS media (alternative to file attachments)",
            ),
            ParamDef::optional("base_url", "API base URL override (default: https://api.plivo.com)")
                .with_example("https://api.plivo.com"),
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
        let auth_id = config.require("auth_id", "plivo")?;
        let auth_token = config.require("auth_token", "plivo")?;
        let from = config.require("from", "plivo")?;
        let to = config.require("to", "plivo")?;

        let base_url = config
            .get("base_url")
            .unwrap_or("https://api.plivo.com")
            .trim_end_matches('/');
        let url = format!("{base_url}/v1/Account/{auth_id}/Message/");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "src": from,
            "dst": to,
            "text": body_text,
        });

        // Add MMS media attachments
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
                payload["type"] = json!("mms");
                payload["media_urls"] = json!(media_urls);
            }
        } else if let Some(media_url) = config.get("media_url") {
            payload["type"] = json!("mms");
            payload["media_urls"] = json!([media_url]);
        }

        let resp = self
            .client
            .post(&url)
            .basic_auth(auth_id, Some(auth_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("plivo", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            let msg = if message.has_attachments() {
                "MMS sent with attachments via Plivo"
            } else {
                "SMS sent via Plivo"
            };
            Ok(SendResponse::success("plivo", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("plivo", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
