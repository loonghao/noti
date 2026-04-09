use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Resend email provider.
///
/// Sends transactional emails via Resend, a modern email API for developers.
/// Supports file attachments encoded as base64 in the JSON payload.
///
/// API reference: <https://resend.com/docs/api-reference/emails/send-email>
pub struct ResendProvider {
    client: Client,
}

impl ResendProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ResendProvider {
    fn name(&self) -> &str {
        "resend"
    }

    fn url_scheme(&self) -> &str {
        "resend"
    }

    fn description(&self) -> &str {
        "Resend modern transactional email API"
    }

    fn example_url(&self) -> &str {
        "resend://<api_key>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Resend API key").with_example("re_123abc"),
            ParamDef::required("from", "Sender email address").with_example("noti@yourdomain.com"),
            ParamDef::required("to", "Recipient email address").with_example("user@example.com"),
            ParamDef::optional("reply_to", "Reply-to email address")
                .with_example("reply@example.com"),
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
        let api_key = config.require("api_key", "resend")?;
        let from = config.require("from", "resend")?;
        let to = config.require("to", "resend")?;

        let url = "https://api.resend.com/emails";

        let subject = message.title.as_deref().unwrap_or("Notification");

        let mut payload = json!({
            "from": from,
            "to": [to],
            "subject": subject,
            "text": message.text,
        });

        if let Some(reply_to) = config.get("reply_to") {
            payload["reply_to"] = json!([reply_to]);
        }

        // Add attachments as base64-encoded content
        if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                attachments_json.push(json!({
                    "content": b64,
                    "filename": attachment.effective_file_name(),
                }));
            }
            payload["attachments"] = json!(attachments_json);
        }

        let resp = self
            .client
            .post(url)
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("resend", e))?;

        let status = resp.status().as_u16();

        // Check for 429 rate limiting
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::http_helpers::handle_http_error(
                "resend",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }

        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("resend", "email sent via Resend")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("resend", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
