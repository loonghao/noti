use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Brevo (formerly Sendinblue) transactional email provider.
///
/// Sends email via Brevo's transactional email API v3.
///
/// API reference: <https://developers.brevo.com/reference/sendtransacemail>
pub struct BrevoProvider {
    client: Client,
}

impl BrevoProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BrevoProvider {
    fn name(&self) -> &str {
        "brevo"
    }

    fn url_scheme(&self) -> &str {
        "brevo"
    }

    fn description(&self) -> &str {
        "Brevo (Sendinblue) transactional email"
    }

    fn example_url(&self) -> &str {
        "brevo://<api_key>@<from_email>/<to_email>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Brevo API key"),
            ParamDef::required("from", "Sender email address").with_example("noti@example.com"),
            ParamDef::required("to", "Recipient email address").with_example("user@example.com"),
            ParamDef::optional("from_name", "Sender display name (default: noti)")
                .with_example("noti"),
            ParamDef::optional("to_name", "Recipient display name"),
            ParamDef::optional("cc", "CC recipients (comma-separated emails)"),
            ParamDef::optional("bcc", "BCC recipients (comma-separated emails)"),
            ParamDef::optional("reply_to", "Reply-to email address"),
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
        let api_key = config.require("api_key", "brevo")?;
        let from_email = config.require("from", "brevo")?;
        let to_email = config.require("to", "brevo")?;

        let url = "https://api.brevo.com/v3/smtp/email";

        let from_name = config.get("from_name").unwrap_or("noti");
        let subject = message.title.as_deref().unwrap_or("Notification from noti");

        let is_html = message.format == noti_core::MessageFormat::Html;
        let (content_key, content_val) = if is_html {
            ("htmlContent", message.text.as_str())
        } else {
            ("textContent", message.text.as_str())
        };

        let mut to_list = vec![json!({"email": to_email})];
        if let Some(to_name) = config.get("to_name") {
            to_list[0]["name"] = json!(to_name);
        }

        let mut payload = json!({
            "sender": {
                "name": from_name,
                "email": from_email,
            },
            "to": to_list,
            "subject": subject,
            content_key: content_val,
        });

        if let Some(cc) = config.get("cc") {
            let cc_list: Vec<serde_json::Value> =
                cc.split(',').map(|e| json!({"email": e.trim()})).collect();
            payload["cc"] = json!(cc_list);
        }

        if let Some(bcc) = config.get("bcc") {
            let bcc_list: Vec<serde_json::Value> =
                bcc.split(',').map(|e| json!({"email": e.trim()})).collect();
            payload["bcc"] = json!(bcc_list);
        }

        if let Some(reply_to) = config.get("reply_to") {
            payload["replyTo"] = json!({"email": reply_to});
        }

        // Add file attachments as base64-encoded content
        if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &data,
                );
                attachments_json.push(json!({
                    "content": b64,
                    "name": attachment.effective_file_name(),
                }));
            }
            payload["attachment"] = json!(attachments_json);
        }

        let resp = self
            .client
            .post(url)
            .header("api-key", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("brevo", "email sent via Brevo")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("brevo", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
