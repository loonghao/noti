use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// BurstSMS provider.
///
/// Sends SMS/MMS messages via the BurstSMS (Transmit SMS) REST API.
/// Supports MMS via the send-mms.json endpoint for image attachments.
///
/// API reference: <https://burstsms.com/api>
pub struct BurstSmsProvider {
    client: Client,
}

impl BurstSmsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BurstSmsProvider {
    fn name(&self) -> &str {
        "burstsms"
    }

    fn url_scheme(&self) -> &str {
        "burstsms"
    }

    fn description(&self) -> &str {
        "BurstSMS (Transmit SMS) gateway via REST API"
    }

    fn example_url(&self) -> &str {
        "burstsms://<api_key>:<api_secret>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "BurstSMS API key").with_example("your-api-key"),
            ParamDef::required("api_secret", "BurstSMS API secret").with_example("your-api-secret"),
            ParamDef::required("from", "Sender caller ID or phone number").with_example("MyApp"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+61412345678"),
            ParamDef::optional(
                "media_url",
                "Public URL for MMS image (alternative to file attachments)",
            ),
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
        let api_key = config.require("api_key", "burstsms")?;
        let api_secret = config.require("api_secret", "burstsms")?;
        let from = config.require("from", "burstsms")?;
        let to = config.require("to", "burstsms")?;

        let body_text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let has_media = message.has_attachments() || config.get("media_url").is_some();

        // Use MMS endpoint for media attachments
        let endpoint = if has_media {
            "https://api.transmitsms.com/send-mms.json"
        } else {
            "https://api.transmitsms.com/send-sms.json"
        };

        let mut params: Vec<(&str, String)> = vec![
            ("message", body_text),
            ("to", to.to_string()),
            ("from", from.to_string()),
        ];

        if message.has_attachments() {
            if let Some(attachment) = message
                .attachments
                .iter()
                .find(|a| matches!(a.kind, AttachmentKind::Image))
            {
                let data = attachment.read_bytes().await?;
                let mime = attachment.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                params.push(("image", format!("data:{mime};base64,{b64}")));
            }
        } else if let Some(media_url) = config.get("media_url") {
            params.push(("image", media_url.to_string()));
        }

        let resp = self
            .client
            .post(endpoint)
            .basic_auth(api_key, Some(api_secret))
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            let error_code = raw.get("error").and_then(|v| v.get("code"));
            if error_code.is_some() && error_code != Some(&json!("SUCCESS")) {
                let msg = raw
                    .get("error")
                    .and_then(|v| v.get("description"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                return Ok(
                    SendResponse::failure("burstsms", format!("API error: {msg}"))
                        .with_status_code(status)
                        .with_raw_response(raw),
                );
            }
            let msg = if has_media {
                "MMS sent with image via BurstSMS"
            } else {
                "SMS sent via BurstSMS"
            };
            Ok(SendResponse::success("burstsms", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("burstsms", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
