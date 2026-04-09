use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// Mailgun transactional email provider.
///
/// Sends emails via the Mailgun REST API.
/// Supports file attachments via multipart form upload.
pub struct MailgunProvider {
    client: Client,
}

impl MailgunProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for MailgunProvider {
    fn name(&self) -> &str {
        "mailgun"
    }

    fn url_scheme(&self) -> &str {
        "mailgun"
    }

    fn description(&self) -> &str {
        "Mailgun transactional email API"
    }

    fn example_url(&self) -> &str {
        "mailgun://<api_key>@<domain>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Mailgun API key")
                .with_example("key-xxxxxxxxxxxxxxxxxxxx"),
            ParamDef::required("domain", "Mailgun sending domain").with_example("mg.example.com"),
            ParamDef::required("to", "Recipient email address").with_example("user@example.com"),
            ParamDef::optional("from", "Sender name and email")
                .with_example("noti <noti@mg.example.com>"),
            ParamDef::optional("region", "Mailgun region: us or eu (default: us)")
                .with_example("us"),
            ParamDef::optional("base_url", "Base URL override for API (default: auto from region)")
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
        let api_key = config.require("api_key", "mailgun")?;
        let domain = config.require("domain", "mailgun")?;
        let to = config.require("to", "mailgun")?;
        let default_from = format!("noti <noti@{domain}>");
        let from = config.get("from").unwrap_or(&default_from);
        let region = config.get("region").unwrap_or("us");
        let subject = message.title.as_deref().unwrap_or("Notification");

        let api_base = config.get("base_url").map(|s| s.trim_end_matches('/')).unwrap_or_else(|| {
            match region {
                "eu" => "https://api.eu.mailgun.net/v3",
                _ => "https://api.mailgun.net/v3",
            }
        });

        let url = format!("{api_base}/{domain}/messages");

        if message.has_attachments() {
            // Use multipart form for attachments
            let mut form = reqwest::multipart::Form::new()
                .text("from", from.to_string())
                .text("to", to.to_string())
                .text("subject", subject.to_string());

            match message.format {
                MessageFormat::Html => {
                    form = form.text("html", message.text.clone());
                }
                _ => {
                    form = form.text("text", message.text.clone());
                }
            }

            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let file_name = attachment.effective_file_name();
                let mime_str = attachment.effective_mime();

                let part = reqwest::multipart::Part::bytes(data)
                    .file_name(file_name)
                    .mime_str(&mime_str)
                    .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

                form = form.part("attachment", part);
            }

            let resp = self
                .client
                .post(&url)
                .basic_auth("api", Some(api_key))
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("mailgun", e))?;

            return Self::parse_response(resp).await;
        }

        // No attachments — use simple form POST
        let mut form = vec![
            ("from", from.to_string()),
            ("to", to.to_string()),
            ("subject", subject.to_string()),
        ];

        match message.format {
            MessageFormat::Html => {
                form.push(("html", message.text.clone()));
            }
            _ => {
                form.push(("text", message.text.clone()));
            }
        }

        let resp = self
            .client
            .post(&url)
            .basic_auth("api", Some(api_key))
            .form(&form)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("mailgun", e))?;

        Self::parse_response(resp).await
    }
}

impl MailgunProvider {
    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
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
                "mailgun",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(SendResponse::success("mailgun", "email sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("mailgun", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
