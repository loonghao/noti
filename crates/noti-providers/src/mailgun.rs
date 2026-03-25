use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// Mailgun transactional email provider.
///
/// Sends emails via the Mailgun REST API.
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
        ]
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

        let api_base = match region {
            "eu" => "https://api.eu.mailgun.net/v3",
            _ => "https://api.mailgun.net/v3",
        };

        let url = format!("{api_base}/{domain}/messages");

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
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
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
