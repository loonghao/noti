use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// 46elks SMS/MMS provider.
///
/// Supports MMS via the a1/mms endpoint for sending images.
///
/// API reference: https://46elks.com/docs/send-sms
pub struct FortySixElksProvider {
    client: Client,
}

impl FortySixElksProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FortySixElksProvider {
    fn name(&self) -> &str {
        "46elks"
    }

    fn url_scheme(&self) -> &str {
        "46elks"
    }

    fn description(&self) -> &str {
        "46elks SMS/MMS messaging via REST API"
    }

    fn example_url(&self) -> &str {
        "46elks://<api_username>:<api_password>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_username", "46elks API username"),
            ParamDef::required("api_password", "46elks API password"),
            ParamDef::required("from", "Sender phone number or alphanumeric sender ID"),
            ParamDef::required("to", "Recipient phone number in E.164 format"),
            ParamDef::optional("flash", "Send as flash SMS (yes/no, default: no)"),
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

        let api_username = config.require("api_username", "46elks")?;
        let api_password = config.require("api_password", "46elks")?;
        let from = config.require("from", "46elks")?;
        let to = config.require("to", "46elks")?;

        let has_media = message.has_attachments() || config.get("media_url").is_some();

        let base_url = config
            .get("base_url")
            .unwrap_or("https://api.46elks.com")
            .trim_end_matches('/');

        // Use MMS endpoint if we have attachments
        let endpoint = if has_media {
            format!("{base_url}/a1/mms")
        } else {
            format!("{base_url}/a1/sms")
        };

        let mut params: Vec<(&str, String)> = vec![
            ("from", from.to_string()),
            ("to", to.to_string()),
            ("message", message.text.clone()),
        ];

        if let Some(flash) = config.get("flash") {
            params.push(("flashsms", flash.to_string()));
        }

        // Add MMS image
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
            .basic_auth(api_username, Some(api_password))
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            let msg = if has_media {
                "MMS sent with image via 46elks"
            } else {
                "SMS sent successfully"
            };
            Ok(SendResponse::success("46elks", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("46elks", format!("46elks API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
