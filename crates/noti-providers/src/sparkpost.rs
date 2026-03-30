use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// SparkPost transactional email provider.
///
/// Sends email through the SparkPost Transmissions API.
/// Supports HTML and text content, CC/BCC, and custom sender names.
///
/// API docs: <https://developers.sparkpost.com/api/transmissions/>
pub struct SparkPostProvider {
    client: Client,
}

impl SparkPostProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SparkPostProvider {
    fn name(&self) -> &str {
        "sparkpost"
    }

    fn url_scheme(&self) -> &str {
        "sparkpost"
    }

    fn description(&self) -> &str {
        "SparkPost transactional email API"
    }

    fn example_url(&self) -> &str {
        "sparkpost://<api_key>@<from_email>/<to_email>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "SparkPost API key"),
            ParamDef::required("from", "Sender email address").with_example("noti@example.com"),
            ParamDef::required("to", "Recipient email address").with_example("user@example.com"),
            ParamDef::optional("from_name", "Sender display name").with_example("noti"),
            ParamDef::optional("region", "API region: us or eu (default: us)").with_example("us"),
            ParamDef::optional("cc", "CC email addresses (comma-separated)"),
            ParamDef::optional("bcc", "BCC email addresses (comma-separated)"),
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
        let api_key = config.require("api_key", "sparkpost")?;
        let from_email = config.require("from", "sparkpost")?;
        let to_email = config.require("to", "sparkpost")?;
        let from_name = config.get("from_name").unwrap_or("noti");
        let region = config.get("region").unwrap_or("us");
        let subject = message
            .title
            .clone()
            .unwrap_or_else(|| "Notification".into());

        let base_url = if region == "eu" {
            "https://api.eu.sparkpost.com/api/v1/transmissions"
        } else {
            "https://api.sparkpost.com/api/v1/transmissions"
        };

        // Build recipients list
        let mut recipients = vec![json!({"address": {"email": to_email}})];

        if let Some(cc) = config.get("cc") {
            for addr in cc.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                recipients.push(json!({"address": {"email": addr, "header_to": to_email}}));
            }
        }

        let mut content = if matches!(message.format, MessageFormat::Html) {
            json!({
                "from": {"name": from_name, "email": from_email},
                "subject": subject,
                "html": message.text,
            })
        } else {
            json!({
                "from": {"name": from_name, "email": from_email},
                "subject": subject,
                "text": message.text,
            })
        };

        // Add file attachments as base64-encoded content
        if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                attachments_json.push(json!({
                    "name": attachment.effective_file_name(),
                    "type": attachment.effective_mime(),
                    "data": b64,
                }));
            }
            content["attachments"] = json!(attachments_json);
        }

        let payload = json!({
            "recipients": recipients,
            "content": content,
        });

        let resp = self
            .client
            .post(base_url)
            .header("Authorization", api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or(json!({"error": "failed to parse response"}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("sparkpost", "email sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let errors = raw
                .get("errors")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("sparkpost", format!("API error: {errors}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
