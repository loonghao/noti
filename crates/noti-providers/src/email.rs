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

    /// Resolve the sender address (defaults to username if `from` not set).
    pub fn resolve_from(config: &ProviderConfig) -> &str {
        let username = config.get("username").unwrap_or("unknown");
        config.get("from").unwrap_or(username)
    }

    /// Resolve SMTP port (default: 587).
    pub fn resolve_port(config: &ProviderConfig) -> u16 {
        config.get("port").unwrap_or("587").parse().unwrap_or(587)
    }

    /// Resolve email subject (defaults to "noti notification" if title not set).
    pub fn resolve_subject(message: &Message) -> &str {
        message.title.as_deref().unwrap_or("noti notification")
    }

    /// Parse comma-separated email addresses into a Vec.
    pub fn parse_email_list(list: &str) -> Vec<&str> {
        list.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect()
    }

    /// Build the lettre email message (without sending).
    /// Extracted for unit testability of address parsing and header construction.
    pub fn build_email_message(
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<LettreMessage, NotiError> {
        let to = config.require("to", "email")?;
        let from = Self::resolve_from(config);
        let subject = Self::resolve_subject(message);

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
            for addr in Self::parse_email_list(cc) {
                builder = builder.cc(addr
                    .parse()
                    .map_err(|e| NotiError::Validation(format!("invalid cc address: {e}")))?);
            }
        }
        // BCC
        if let Some(bcc) = config.get("bcc") {
            for addr in Self::parse_email_list(bcc) {
                builder = builder.bcc(
                    addr.parse()
                        .map_err(|e| NotiError::Validation(format!("invalid bcc address: {e}")))?,
                );
            }
        }

        // Build body (without attachments — attachment handling requires async I/O)
        match message.format {
            MessageFormat::Html => builder
                .header(ContentType::TEXT_HTML)
                .body(message.text.clone())
                .map_err(|e| NotiError::provider("email", format!("build email error: {e}"))),
            _ => builder
                .header(ContentType::TEXT_PLAIN)
                .body(message.text.clone())
                .map_err(|e| NotiError::provider("email", format!("build email error: {e}"))),
        }
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
        let port = Self::resolve_port(config);

        // Build body with or without attachments
        let email = if message.has_attachments() {
            let to = config.require("to", "email")?;
            let from = Self::resolve_from(config);
            let subject = Self::resolve_subject(message);

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
                for addr in Self::parse_email_list(cc) {
                    builder = builder.cc(addr
                        .parse()
                        .map_err(|e| NotiError::Validation(format!("invalid cc address: {e}")))?);
                }
            }
            // BCC
            if let Some(bcc) = config.get("bcc") {
                for addr in Self::parse_email_list(bcc) {
                    builder = builder.bcc(
                        addr.parse()
                            .map_err(|e| NotiError::Validation(format!("invalid bcc address: {e}")))?,
                    );
                }
            }

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
            Self::build_email_message(message, config)?
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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- resolve_from tests ----

    #[test]
    fn test_resolve_from_with_explicit_from() {
        let config = ProviderConfig::new()
            .set("username", "user@gmail.com")
            .set("from", "custom@example.com");
        assert_eq!(EmailProvider::resolve_from(&config), "custom@example.com");
    }

    #[test]
    fn test_resolve_from_defaults_to_username() {
        let config = ProviderConfig::new().set("username", "user@gmail.com");
        assert_eq!(EmailProvider::resolve_from(&config), "user@gmail.com");
    }

    #[test]
    fn test_resolve_from_no_username() {
        let config = ProviderConfig::new();
        assert_eq!(EmailProvider::resolve_from(&config), "unknown");
    }

    // ---- resolve_port tests ----

    #[test]
    fn test_resolve_port_default() {
        let config = ProviderConfig::new();
        assert_eq!(EmailProvider::resolve_port(&config), 587);
    }

    #[test]
    fn test_resolve_port_explicit() {
        let config = ProviderConfig::new().set("port", "465");
        assert_eq!(EmailProvider::resolve_port(&config), 465);
    }

    #[test]
    fn test_resolve_port_invalid() {
        let config = ProviderConfig::new().set("port", "abc");
        assert_eq!(EmailProvider::resolve_port(&config), 587);
    }

    // ---- resolve_subject tests ----

    #[test]
    fn test_resolve_subject_with_title() {
        let message = Message::text("body").with_title("Alert!");
        assert_eq!(EmailProvider::resolve_subject(&message), "Alert!");
    }

    #[test]
    fn test_resolve_subject_default() {
        let message = Message::text("body");
        assert_eq!(EmailProvider::resolve_subject(&message), "noti notification");
    }

    // ---- parse_email_list tests ----

    #[test]
    fn test_parse_email_list_single() {
        let result = EmailProvider::parse_email_list("user@example.com");
        assert_eq!(result, vec!["user@example.com"]);
    }

    #[test]
    fn test_parse_email_list_multiple() {
        let result = EmailProvider::parse_email_list("a@example.com, b@example.com");
        assert_eq!(result, vec!["a@example.com", "b@example.com"]);
    }

    #[test]
    fn test_parse_email_list_with_spaces() {
        let result = EmailProvider::parse_email_list("  a@example.com ,  b@example.com  ");
        assert_eq!(result, vec!["a@example.com", "b@example.com"]);
    }

    #[test]
    fn test_parse_email_list_empty_segments() {
        let result = EmailProvider::parse_email_list("a@example.com,,b@example.com,");
        assert_eq!(result, vec!["a@example.com", "b@example.com"]);
    }

    // ---- build_email_message tests ----

    #[test]
    fn test_build_email_message_plain() {
        let message = Message::text("Hello world");
        let config = ProviderConfig::new()
            .set("to", "recipient@example.com")
            .set("from", "sender@example.com");
        let email = EmailProvider::build_email_message(&message, &config).unwrap();
        let buf = email.formatted();
        let formatted = String::from_utf8_lossy(&buf);
        assert!(formatted.contains("noti notification"));
    }

    #[test]
    fn test_build_email_message_with_title() {
        let message = Message::text("Hello world").with_title("Test Subject");
        let config = ProviderConfig::new()
            .set("to", "recipient@example.com")
            .set("from", "sender@example.com");
        let email = EmailProvider::build_email_message(&message, &config).unwrap();
        let buf = email.formatted();
        let formatted = String::from_utf8_lossy(&buf);
        assert!(formatted.contains("Test Subject"));
    }

    #[test]
    fn test_build_email_message_with_cc() {
        let message = Message::text("Hello");
        let config = ProviderConfig::new()
            .set("to", "recipient@example.com")
            .set("from", "sender@example.com")
            .set("cc", "cc1@example.com, cc2@example.com");
        let email = EmailProvider::build_email_message(&message, &config).unwrap();
        let buf = email.formatted();
        let formatted = String::from_utf8_lossy(&buf);
        assert!(formatted.contains("cc1@example.com"));
        assert!(formatted.contains("cc2@example.com"));
    }

    #[test]
    fn test_build_email_message_with_bcc() {
        let message = Message::text("Hello");
        let config = ProviderConfig::new()
            .set("to", "recipient@example.com")
            .set("from", "sender@example.com")
            .set("bcc", "bcc@example.com");
        // BCC is intentionally excluded from message headers by lettre
        let email = EmailProvider::build_email_message(&message, &config).unwrap();
        // Just verify it builds successfully
        assert!(!email.formatted().is_empty());
    }

    #[test]
    fn test_build_email_message_invalid_to() {
        let message = Message::text("Hello");
        let config = ProviderConfig::new()
            .set("to", "not-an-email")
            .set("from", "sender@example.com");
        let result = EmailProvider::build_email_message(&message, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid to address"));
    }

    #[test]
    fn test_build_email_message_invalid_from() {
        let message = Message::text("Hello");
        let config = ProviderConfig::new()
            .set("to", "recipient@example.com")
            .set("from", "bad-from");
        let result = EmailProvider::build_email_message(&message, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid from address"));
    }

    #[test]
    fn test_build_email_message_invalid_cc() {
        let message = Message::text("Hello");
        let config = ProviderConfig::new()
            .set("to", "recipient@example.com")
            .set("from", "sender@example.com")
            .set("cc", "bad-cc");
        let result = EmailProvider::build_email_message(&message, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid cc address"));
    }

    #[test]
    fn test_build_email_message_html_format() {
        let message = Message::text("<b>Bold</b>").with_format(MessageFormat::Html);
        let config = ProviderConfig::new()
            .set("to", "recipient@example.com")
            .set("from", "sender@example.com");
        let email = EmailProvider::build_email_message(&message, &config).unwrap();
        let buf = email.formatted();
        let formatted = String::from_utf8_lossy(&buf);
        // HTML content type should be in the message headers
        assert!(formatted.contains("text/html"));
    }

    // ---- Provider metadata tests ----

    #[test]
    fn test_email_provider_name() {
        let provider = EmailProvider::new();
        assert_eq!(provider.name(), "email");
    }

    #[test]
    fn test_email_provider_url_scheme() {
        let provider = EmailProvider::new();
        assert_eq!(provider.url_scheme(), "smtp");
    }

    #[test]
    fn test_email_provider_description() {
        let provider = EmailProvider::new();
        assert!(!provider.description().is_empty());
    }

    #[test]
    fn test_email_provider_example_url() {
        let provider = EmailProvider::new();
        assert!(provider.example_url().starts_with("smtp://"));
    }

    #[test]
    fn test_email_provider_supports_attachments() {
        let provider = EmailProvider::new();
        assert!(provider.supports_attachments());
    }

    #[test]
    fn test_email_provider_params_required() {
        let provider = EmailProvider::new();
        let params = provider.params();
        let required: Vec<_> = params.iter().filter(|p| p.required).collect();
        assert_eq!(required.len(), 4);
        let required_names: Vec<_> = required.iter().map(|p| p.name.as_str()).collect();
        assert!(required_names.contains(&"host"));
        assert!(required_names.contains(&"username"));
        assert!(required_names.contains(&"password"));
        assert!(required_names.contains(&"to"));
    }

    #[test]
    fn test_email_provider_params_optional() {
        let provider = EmailProvider::new();
        let params = provider.params();
        let optional: Vec<_> = params.iter().filter(|p| !p.required).collect();
        assert_eq!(optional.len(), 4);
    }

    // ---- Config validation tests ----

    #[tokio::test]
    async fn test_validate_config_full() {
        let provider = EmailProvider::new();
        let config = ProviderConfig::new()
            .set("host", "smtp.gmail.com")
            .set("username", "user@gmail.com")
            .set("password", "app-password")
            .set("to", "recipient@example.com");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_missing_host() {
        let provider = EmailProvider::new();
        let config = ProviderConfig::new()
            .set("username", "user@gmail.com")
            .set("password", "app-password")
            .set("to", "recipient@example.com");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_username() {
        let provider = EmailProvider::new();
        let config = ProviderConfig::new()
            .set("host", "smtp.gmail.com")
            .set("password", "app-password")
            .set("to", "recipient@example.com");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_password() {
        let provider = EmailProvider::new();
        let config = ProviderConfig::new()
            .set("host", "smtp.gmail.com")
            .set("username", "user@gmail.com")
            .set("to", "recipient@example.com");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_to() {
        let provider = EmailProvider::new();
        let config = ProviderConfig::new()
            .set("host", "smtp.gmail.com")
            .set("username", "user@gmail.com")
            .set("password", "app-password");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_empty() {
        let provider = EmailProvider::new();
        assert!(provider.validate_config(&ProviderConfig::new()).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_with_optional_params() {
        let provider = EmailProvider::new();
        let config = ProviderConfig::new()
            .set("host", "smtp.gmail.com")
            .set("username", "user@gmail.com")
            .set("password", "app-password")
            .set("to", "recipient@example.com")
            .set("from", "custom@example.com")
            .set("port", "465")
            .set("cc", "cc1@example.com, cc2@example.com")
            .set("bcc", "bcc@example.com");
        assert!(provider.validate_config(&config).is_ok());
    }
}
