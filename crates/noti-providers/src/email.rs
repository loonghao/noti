use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};

/// Email notification provider via SMTP (using lettre).
pub struct EmailProvider;

impl EmailProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmailProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotifyProvider for EmailProvider {
    fn name(&self) -> &str {
        "email"
    }

    fn url_scheme(&self) -> &str {
        "smtp"
    }

    fn description(&self) -> &str {
        "Email via SMTP"
    }

    fn example_url(&self) -> &str {
        "smtp://user:pass@smtp.example.com:587?to=recipient@example.com"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "SMTP server hostname").with_example("smtp.gmail.com"),
            ParamDef::optional("port", "SMTP server port (default: 587)").with_example("587"),
            ParamDef::required("username", "SMTP username / from address")
                .with_example("user@gmail.com"),
            ParamDef::required("password", "SMTP password or app-specific password"),
            ParamDef::required("to", "Recipient email address")
                .with_example("recipient@example.com"),
            ParamDef::optional("from", "Sender display name"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let host = config.require("host", "email")?;
        let username = config.require("username", "email")?;
        let password = config.require("password", "email")?;
        let to = config.require("to", "email")?;
        let port: u16 = config
            .get("port")
            .unwrap_or("587")
            .parse()
            .map_err(|_| NotiError::Validation("invalid port number".into()))?;
        let from_name = config.get("from").unwrap_or(username);
        let subject = message.title.as_deref().unwrap_or("Notification");

        // Build the email
        use lettre::message::header::ContentType;
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

        let content_type = match message.format {
            MessageFormat::Html => ContentType::TEXT_HTML,
            _ => ContentType::TEXT_PLAIN,
        };

        let email = lettre::Message::builder()
            .from(
                format!("{from_name} <{username}>")
                    .parse()
                    .map_err(|e| NotiError::Validation(format!("invalid from address: {e}")))?,
            )
            .to(to
                .parse()
                .map_err(|e| NotiError::Validation(format!("invalid to address: {e}")))?)
            .subject(subject)
            .header(content_type)
            .body(message.text.clone())
            .map_err(|e| NotiError::provider("email", format!("failed to build email: {e}")))?;

        let creds = Credentials::new(username.to_string(), password.to_string());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|e| NotiError::provider("email", format!("SMTP relay error: {e}")))?
            .port(port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| NotiError::provider("email", format!("SMTP send error: {e}")))?;

        Ok(SendResponse::success(
            "email",
            format!("email sent to {to}"),
        ))
    }
}
