use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// WhatsApp Business Cloud API provider.
///
/// Sends messages through Meta's WhatsApp Business Cloud API.
///
/// API reference: https://developers.facebook.com/docs/whatsapp/cloud-api/messages/text-messages
pub struct WhatsAppProvider {
    client: Client,
}

impl WhatsAppProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for WhatsAppProvider {
    fn name(&self) -> &str {
        "whatsapp"
    }

    fn url_scheme(&self) -> &str {
        "whatsapp"
    }

    fn description(&self) -> &str {
        "WhatsApp Business Cloud API messaging"
    }

    fn example_url(&self) -> &str {
        "whatsapp://<access_token>@<phone_number_id>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required(
                "access_token",
                "WhatsApp Business API permanent access token",
            ),
            ParamDef::required(
                "phone_number_id",
                "Phone number ID from WhatsApp Business account",
            ),
            ParamDef::required(
                "to",
                "Recipient phone number in international format (e.g. +1234567890)",
            ),
            ParamDef::optional("api_version", "Graph API version (default: v21.0)"),
            ParamDef::optional(
                "preview_url",
                "Enable link previews (true/false, default: false)",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let access_token = config.require("access_token", "whatsapp")?;
        let phone_number_id = config.require("phone_number_id", "whatsapp")?;
        let to = config.require("to", "whatsapp")?;
        let api_version = config.get("api_version").unwrap_or("v21.0");
        let preview_url = config.get("preview_url").unwrap_or("false") == "true";

        let url = format!("https://graph.facebook.com/{api_version}/{phone_number_id}/messages");

        let body = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": preview_url,
                "body": message.text
            }
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("whatsapp", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("whatsapp", format!("WhatsApp API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
