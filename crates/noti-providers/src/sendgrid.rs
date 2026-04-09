use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// SendGrid email provider.
///
/// Sends transactional email via the SendGrid Mail Send API v3.
/// Supports file attachments encoded as base64 in the JSON payload.
///
/// API reference: <https://docs.sendgrid.com/api-reference/mail-send/mail-send>
pub struct SendGridProvider {
    client: Client,
}

impl SendGridProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SendGridProvider {
    fn name(&self) -> &str {
        "sendgrid"
    }

    fn url_scheme(&self) -> &str {
        "sendgrid"
    }

    fn description(&self) -> &str {
        "SendGrid transactional email via Mail Send API v3"
    }

    fn example_url(&self) -> &str {
        "sendgrid://<api_key>@<from_email>/<to_email>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "SendGrid API key").with_example("SG.xxxx"),
            ParamDef::required("from", "Sender email address").with_example("sender@example.com"),
            ParamDef::required("to", "Recipient email address")
                .with_example("recipient@example.com"),
            ParamDef::optional("from_name", "Sender display name").with_example("Noti Bot"),
            ParamDef::optional("to_name", "Recipient display name").with_example("John Doe"),
            ParamDef::optional("cc", "CC email address(es), comma-separated")
                .with_example("cc@example.com"),
            ParamDef::optional("bcc", "BCC email address(es), comma-separated")
                .with_example("bcc@example.com"),
            ParamDef::optional("base_url", "Base URL override for API (default: https://api.sendgrid.com)")
                .with_example("http://localhost:8080"),
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
        let api_key = config.require("api_key", "sendgrid")?;
        let from_email = config.require("from", "sendgrid")?;
        let to_email = config.require("to", "sendgrid")?;

        let subject = message
            .title
            .clone()
            .unwrap_or_else(|| "Notification from noti".to_string());

        let mut from_obj = json!({"email": from_email});
        if let Some(from_name) = config.get("from_name") {
            from_obj["name"] = json!(from_name);
        }

        let mut to_obj = json!({"email": to_email});
        if let Some(to_name) = config.get("to_name") {
            to_obj["name"] = json!(to_name);
        }

        let mut personalizations = json!({
            "to": [to_obj]
        });

        if let Some(cc) = config.get("cc") {
            let cc_list: Vec<serde_json::Value> =
                cc.split(',').map(|e| json!({"email": e.trim()})).collect();
            personalizations["cc"] = json!(cc_list);
        }

        if let Some(bcc) = config.get("bcc") {
            let bcc_list: Vec<serde_json::Value> =
                bcc.split(',').map(|e| json!({"email": e.trim()})).collect();
            personalizations["bcc"] = json!(bcc_list);
        }

        let content = match message.format {
            MessageFormat::Html => {
                json!([{"type": "text/html", "value": message.text}])
            }
            _ => {
                json!([{"type": "text/plain", "value": message.text}])
            }
        };

        let mut payload = json!({
            "personalizations": [personalizations],
            "from": from_obj,
            "subject": subject,
            "content": content,
        });

        // Add attachments as base64-encoded content
        if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                attachments_json.push(json!({
                    "content": b64,
                    "filename": attachment.effective_file_name(),
                    "type": attachment.effective_mime(),
                    "disposition": "attachment"
                }));
            }
            payload["attachments"] = json!(attachments_json);
        }

        let base_url = config.get("base_url").unwrap_or("https://api.sendgrid.com").trim_end_matches('/');
        let resp = self
            .client
            .post(format!("{base_url}/v3/mail/send"))
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("sendgrid", e))?;

        let status = resp.status().as_u16();

        // Check for 429 rate limiting
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = crate::http_helpers::read_response_body("sendgrid", resp).await;
            return Err(crate::http_helpers::handle_http_error(
                "sendgrid",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }

        // SendGrid returns 202 Accepted with empty body on success
        if status == 202 {
            Ok(
                SendResponse::success("sendgrid", "email accepted for delivery")
                    .with_status_code(status),
            )
        } else {
            let raw: serde_json::Value = resp
                .json()
                .await
                .unwrap_or_else(|_| json!({"error": "failed to parse error response"}));
            let error_msg = raw
                .get("errors")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("sendgrid", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
