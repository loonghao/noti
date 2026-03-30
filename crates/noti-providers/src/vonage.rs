use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Vonage (formerly Nexmo) SMS/MMS provider.
///
/// Sends SMS messages via the Vonage SMS API.
/// Supports MMS media (images) via the Vonage Messages API.
pub struct VonageProvider {
    client: Client,
}

impl VonageProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for VonageProvider {
    fn name(&self) -> &str {
        "vonage"
    }

    fn url_scheme(&self) -> &str {
        "vonage"
    }

    fn description(&self) -> &str {
        "Vonage (Nexmo) SMS/MMS via REST API"
    }

    fn example_url(&self) -> &str {
        "vonage://<api_key>:<api_secret>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Vonage API key").with_example("abc12345"),
            ParamDef::required("api_secret", "Vonage API secret").with_example("xyz9876543"),
            ParamDef::required("from", "Sender number or name (max 15 chars)")
                .with_example("15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("15559876543"),
            ParamDef::optional(
                "application_id",
                "Vonage Application ID (required for MMS via Messages API)",
            ),
            ParamDef::optional(
                "private_key",
                "Vonage Application private key (required for MMS)",
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
        let api_key = config.require("api_key", "vonage")?;
        let api_secret = config.require("api_secret", "vonage")?;
        let from = config.require("from", "vonage")?;
        let to = config.require("to", "vonage")?;

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        // If we have image attachments, send as MMS via Messages API
        if message.has_attachments() {
            if let Some(img) = message
                .attachments
                .iter()
                .find(|a| a.kind == AttachmentKind::Image)
            {
                let data = img.read_bytes().await?;
                let mime_str = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let data_uri = format!("data:{mime_str};base64,{b64}");

                // Use Vonage Messages API for MMS
                let payload = json!({
                    "message_type": "image",
                    "image": {
                        "url": data_uri,
                        "caption": text,
                    },
                    "to": to,
                    "from": from,
                    "channel": "mms",
                });

                let resp = self
                    .client
                    .post("https://api.nexmo.com/v1/messages")
                    .basic_auth(api_key, Some(api_secret))
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| NotiError::Network(e.to_string()))?;

                let status = resp.status().as_u16();
                let raw: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| {
                        NotiError::Network(format!("failed to parse response: {e}"))
                    })?;

                if (200..300).contains(&status) {
                    return Ok(
                        SendResponse::success("vonage", "MMS sent with image")
                            .with_status_code(status)
                            .with_raw_response(raw),
                    );
                } else {
                    let error_msg = raw
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    return Ok(
                        SendResponse::failure(
                            "vonage",
                            format!("MMS API error: {error_msg}"),
                        )
                        .with_status_code(status)
                        .with_raw_response(raw),
                    );
                }
            }
            // Fall through to SMS for non-image attachments (mention in text)
        }

        // Standard SMS via Vonage SMS API
        let mut sms_text = text;
        for attachment in &message.attachments {
            let file_name = attachment.effective_file_name();
            sms_text.push_str(&format!("\n📎 {file_name}"));
        }

        let payload = json!({
            "api_key": api_key,
            "api_secret": api_secret,
            "from": from,
            "to": to,
            "text": sms_text,
        });

        let resp = self
            .client
            .post("https://rest.nexmo.com/sms/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let msg_status = raw
            .get("messages")
            .and_then(|m| m.get(0))
            .and_then(|m| m.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");

        if msg_status == "0" {
            Ok(SendResponse::success("vonage", "SMS sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_text = raw
                .get("messages")
                .and_then(|m| m.get(0))
                .and_then(|m| m.get("error-text"))
                .and_then(|e| e.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("vonage", format!("API error: {error_text}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
