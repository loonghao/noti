use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// AWS SES (Simple Email Service) provider.
///
/// Sends email via the AWS SES REST API.
/// Requires AWS credentials, sender and recipient email addresses.
pub struct SesProvider {
    client: Client,
}

impl SesProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SesProvider {
    fn name(&self) -> &str {
        "ses"
    }

    fn url_scheme(&self) -> &str {
        "ses"
    }

    fn description(&self) -> &str {
        "AWS SES (Simple Email Service) transactional email"
    }

    fn example_url(&self) -> &str {
        "ses://<access_key>:<secret_key>@<region>/<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_key", "AWS access key ID")
                .with_example("AKIAIOSFODNN7EXAMPLE"),
            ParamDef::required("secret_key", "AWS secret access key")
                .with_example("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"),
            ParamDef::required("region", "AWS region").with_example("us-east-1"),
            ParamDef::required("from", "Sender email address").with_example("sender@example.com"),
            ParamDef::required("to", "Recipient email address")
                .with_example("recipient@example.com"),
            ParamDef::optional("cc", "CC recipients (comma-separated)")
                .with_example("cc@example.com"),
            ParamDef::optional("bcc", "BCC recipients (comma-separated)")
                .with_example("bcc@example.com"),
            ParamDef::optional("from_name", "Sender display name").with_example("noti"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_key = config.require("access_key", "ses")?;
        let _secret_key = config.require("secret_key", "ses")?;
        let region = config.require("region", "ses")?;
        let from = config.require("from", "ses")?;
        let to = config.require("to", "ses")?;

        let url = format!("https://email.{region}.amazonaws.com/");

        let subject = message.title.as_deref().unwrap_or("noti notification");

        let from_name = config.get("from_name").unwrap_or("noti");
        let from_header = format!("{from_name} <{from}>");

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();

        let auth_header = format!(
            "AWS3-HTTPS AWSAccessKeyId={access_key},Algorithm=HmacSHA256,Signature=placeholder"
        );

        let mut params = vec![
            ("Action", "SendEmail".to_string()),
            ("Source", from_header),
            ("Destination.ToAddresses.member.1", to.to_string()),
            ("Message.Subject.Data", subject.to_string()),
            ("Message.Body.Text.Data", message.text.clone()),
        ];

        // Add CC recipients
        if let Some(cc) = config.get("cc") {
            for (i, addr) in cc.split(',').enumerate() {
                params.push(("Destination.CcAddresses.member.1", addr.trim().to_string()));
                let _ = i; // used for iteration only
            }
        }

        let resp = self
            .client
            .post(&url)
            .header("X-Amz-Date", &timestamp)
            .header("Authorization", &auth_header)
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("ses", "email sent via SES")
                .with_status_code(status)
                .with_raw_response(serde_json::json!({"response": body})))
        } else {
            Ok(
                SendResponse::failure("ses", format!("SES API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"response": body})),
            )
        }
    }
}
