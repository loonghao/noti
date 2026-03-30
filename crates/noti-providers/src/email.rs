use async_trait::async_trait;
use lettre::message::{Attachment as LettreAttachment, MultiPart, SinglePart, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor};
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};

/// Email (SMTP) notification provider via lettre.
#[derive(Default)]
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
        "smtp://user:pass@host:587?to=recipient@example.com"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "SMTP server host").with_example("smtp.gmail.com"),
            ParamDef::required("username", "SMTP username / email"),
            ParamDef::required("password", "SMTP password or app-specific password"),
            ParamDef::required("to", "Recipient email address"),
            ParamDef::optional("from", "Sender email (defaults to username)"),
            ParamDef::optional("port", "SMTP port (default: 587)"),
            ParamDef::optional("cc", "CC recipients, comma-separated"),
            ParamDef::optional("bcc", "BCC recipients, comma-separated"),
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

        let host = config.require("host", "email")?;
        let username = config.require("username", "email")?;
        let password = config.require("password", "email")?;
        let to = config.require("to", "email")?;
        let from = config.get("from").unwrap_or(username);
        let port: u16 = config.get("port").unwrap_or("587").parse().unwrap_or(587);
        let subject = message.title.as_deref().unwrap_or("noti notification");

        let mut builder = LettreMessage::builder()
            .from(
                from.parse()
                    .map_err(|e| NotiError::Validation(format!("invalid from address: {e}")))?,
            )
            .to(to
                .parse()
                .map_err(|e| NotiError::Validation(format!("invalid to address: {e}")))?)
            .subject(subject);

        // CC
        if let Some(cc) = config.get("cc") {
            for addr in cc.split(',').map(|s| s.trim()) {
                builder = builder.cc(addr
                    .parse()
                    .map_err(|e| NotiError::Validation(format!("invalid cc address: {e}")))?);
            }
        }
        // BCC
        if let Some(bcc) = config.get("bcc") {
            for addr in bcc.split(',').map(|s| s.trim()) {
                builder = builder.bcc(
                    addr.parse()
                        .map_err(|e| NotiError::Validation(format!("invalid bcc address: {e}")))?,
                );
            }
        }

        // Build body with or without attachments
        let email = if message.has_attachments() {
            let body_part = match message.format {
                MessageFormat::Html => SinglePart::builder()
                    .content_type(ContentType::TEXT_HTML)
                    .body(message.text.clone()),
                _ => SinglePart::builder()
                    .content_type(ContentType::TEXT_PLAIN)
                    .body(message.text.clone()),
            };

            let mut multipart = MultiPart::mixed().singlepart(body_part);
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let file_name = attachment.effective_file_name();
                let content_type = ContentType::parse(&attachment.effective_mime())
                    .unwrap_or(ContentType::TEXT_PLAIN);
                let lettre_attachment = LettreAttachment::new(file_name).body(data, content_type);
                multipart = multipart.singlepart(lettre_attachment);
            }

            builder
                .multipart(multipart)
                .map_err(|e| NotiError::provider("email", format!("build email error: {e}")))?
        } else {
            match message.format {
                MessageFormat::Html => builder
                    .header(ContentType::TEXT_HTML)
                    .body(message.text.clone())
                    .map_err(|e| NotiError::provider("email", format!("build email error: {e}")))?,
                _ => builder
                    .header(ContentType::TEXT_PLAIN)
                    .body(message.text.clone())
                    .map_err(|e| NotiError::provider("email", format!("build email error: {e}")))?,
            }
        };

        let creds = Credentials::new(username.to_string(), password.to_string());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|e| NotiError::provider("email", format!("SMTP setup error: {e}")))?
            .port(port)
            .credentials(creds)
            .build();

        match mailer.send(email).await {
            Ok(_) => Ok(SendResponse::success("email", "email sent successfully")),
            Err(e) => Ok(SendResponse::failure("email", format!("SMTP error: {e}"))),
        }
    }
}
