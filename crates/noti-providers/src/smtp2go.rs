use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// SMTP2Go transactional email provider.
///
/// Sends email via SMTP2Go's REST API.
///
/// API reference: <https://www.smtp2go.com/docs/api/sending/>
pub struct Smtp2GoProvider {
    client: Client,
}

impl Smtp2GoProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for Smtp2GoProvider {
    fn name(&self) -> &str {
        "smtp2go"
    }

    fn url_scheme(&self) -> &str {
        "smtp2go"
    }

    fn description(&self) -> &str {
        "SMTP2Go transactional email"
    }

    fn example_url(&self) -> &str {
        "smtp2go://<api_key>@<from_email>/<to_email>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "SMTP2Go API key"),
            ParamDef::required("from", "Sender email address").with_example("noti@example.com"),
            ParamDef::required("to", "Recipient email address").with_example("user@example.com"),
            ParamDef::optional("cc", "CC recipients (comma-separated)"),
            ParamDef::optional("bcc", "BCC recipients (comma-separated)"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "smtp2go")?;
        let from = config.require("from", "smtp2go")?;
        let to = config.require("to", "smtp2go")?;

        let url = "https://api.smtp2go.com/v3/email/send";

        let subject = message.title.as_deref().unwrap_or("Notification from noti");

        let is_html = message.format == noti_core::MessageFormat::Html;

        let mut payload = json!({
            "api_key": api_key,
            "sender": from,
            "to": [to],
            "subject": subject,
        });

        if is_html {
            payload["html_body"] = json!(message.text);
        } else {
            payload["text_body"] = json!(message.text);
        }

        if let Some(cc) = config.get("cc") {
            let cc_list: Vec<&str> = cc.split(',').map(|e| e.trim()).collect();
            payload["cc"] = json!(cc_list);
        }

        if let Some(bcc) = config.get("bcc") {
            let bcc_list: Vec<&str> = bcc.split(',').map(|e| e.trim()).collect();
            payload["bcc"] = json!(bcc_list);
        }

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let succeeded = raw
            .get("data")
            .and_then(|d| d.get("succeeded"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if succeeded > 0 {
            Ok(SendResponse::success("smtp2go", "email sent via SMTP2Go")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("data")
                .and_then(|d| d.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("smtp2go", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
