use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// SendPulse transactional email via SMTP API.
///
/// Supports file attachments via base64-encoded content in the email payload.
///
/// API reference: https://sendpulse.com/integrations/api
pub struct SendPulseProvider {
    client: Client,
}

impl SendPulseProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SendPulseProvider {
    fn name(&self) -> &str {
        "sendpulse"
    }

    fn url_scheme(&self) -> &str {
        "sendpulse"
    }

    fn description(&self) -> &str {
        "SendPulse transactional email via SMTP API"
    }

    fn example_url(&self) -> &str {
        "sendpulse://<client_id>:<client_secret>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("client_id", "SendPulse REST API client ID"),
            ParamDef::required("client_secret", "SendPulse REST API client secret"),
            ParamDef::required("from", "Sender email address"),
            ParamDef::required("to", "Recipient email address"),
            ParamDef::optional("from_name", "Sender display name"),
            ParamDef::optional("to_name", "Recipient display name"),
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

        let client_id = config.require("client_id", "sendpulse")?;
        let client_secret = config.require("client_secret", "sendpulse")?;
        let from = config.require("from", "sendpulse")?;
        let to = config.require("to", "sendpulse")?;
        let from_name = config.get("from_name").unwrap_or("noti");
        let to_name = config.get("to_name").unwrap_or("");
        let subject = message.title.as_deref().unwrap_or("Notification from noti");

        // Step 1: Get access token
        let token_resp = self
            .client
            .post("https://api.sendpulse.com/oauth/access_token")
            .json(&json!({
                "grant_type": "client_credentials",
                "client_id": client_id,
                "client_secret": client_secret
            }))
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let token_data: serde_json::Value =
            token_resp.json().await.unwrap_or(serde_json::Value::Null);

        let access_token = token_data
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                NotiError::provider(
                    "sendpulse",
                    format!("failed to obtain access token: {token_data}"),
                )
            })?;

        // Step 2: Build email payload
        let mut email_payload = json!({
            "email": {
                "subject": subject,
                "from": { "name": from_name, "email": from },
                "to": [{ "name": to_name, "email": to }],
                "text": message.text,
                "html": format!("<p>{}</p>", message.text)
            }
        });

        // Step 3: Add attachments if present
        if message.has_attachments() {
            let mut attachments_map = serde_json::Map::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let file_name = attachment.effective_file_name();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                attachments_map.insert(file_name, json!(b64));
            }
            email_payload["email"]["attachments_binary"] =
                serde_json::Value::Object(attachments_map);
        }

        let resp = self
            .client
            .post("https://api.sendpulse.com/smtp/emails")
            .bearer_auth(access_token)
            .json(&email_payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            let msg = if message.has_attachments() {
                "email with attachments sent successfully"
            } else {
                "email sent successfully"
            };
            Ok(SendResponse::success("sendpulse", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("sendpulse", format!("SendPulse API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
