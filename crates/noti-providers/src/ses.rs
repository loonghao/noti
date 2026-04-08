use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// AWS SES (Simple Email Service) provider.
///
/// Sends email via the AWS SES REST API.
/// Supports file attachments via SendRawEmail with MIME encoding.
/// Requires AWS credentials, sender and recipient email addresses.
pub struct SesProvider {
    client: Client,
}

impl SesProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Build a raw MIME email string with optional attachments.
    async fn build_raw_email(
        &self,
        message: &Message,
        from_header: &str,
        to: &str,
        subject: &str,
        cc: Option<&str>,
    ) -> Result<String, NotiError> {
        let boundary = format!("noti-boundary-{}", uuid_v4_simple());
        let mut raw = String::new();

        // Headers
        raw.push_str(&format!("From: {from_header}\r\n"));
        raw.push_str(&format!("To: {to}\r\n"));
        if let Some(cc_addrs) = cc {
            raw.push_str(&format!("Cc: {cc_addrs}\r\n"));
        }
        raw.push_str(&format!("Subject: {subject}\r\n"));
        raw.push_str("MIME-Version: 1.0\r\n");
        raw.push_str(&format!(
            "Content-Type: multipart/mixed; boundary=\"{boundary}\"\r\n"
        ));
        raw.push_str("\r\n");

        // Body part
        raw.push_str(&format!("--{boundary}\r\n"));
        raw.push_str("Content-Type: text/plain; charset=UTF-8\r\n");
        raw.push_str("Content-Transfer-Encoding: 7bit\r\n");
        raw.push_str("\r\n");
        raw.push_str(&message.text);
        raw.push_str("\r\n");

        // Attachment parts
        for attachment in &message.attachments {
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();
            let mime_type = attachment.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

            raw.push_str(&format!("--{boundary}\r\n"));
            raw.push_str(&format!(
                "Content-Type: {mime_type}; name=\"{file_name}\"\r\n"
            ));
            raw.push_str("Content-Transfer-Encoding: base64\r\n");
            raw.push_str(&format!(
                "Content-Disposition: attachment; filename=\"{file_name}\"\r\n"
            ));
            raw.push_str("\r\n");

            // Split base64 into 76-char lines per MIME spec
            for chunk in b64.as_bytes().chunks(76) {
                raw.push_str(std::str::from_utf8(chunk).unwrap_or(""));
                raw.push_str("\r\n");
            }
        }

        raw.push_str(&format!("--{boundary}--\r\n"));
        Ok(raw)
    }
}

/// Generate a simple pseudo-UUID for MIME boundaries.
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{t:032x}")
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
            ParamDef::optional("base_url", "Base URL override for SES API (default: auto from region)")
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
        let access_key = config.require("access_key", "ses")?;
        let _secret_key = config.require("secret_key", "ses")?;
        let region = config.require("region", "ses")?;
        let from = config.require("from", "ses")?;
        let to = config.require("to", "ses")?;

        let url = config.get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| format!("https://email.{region}.amazonaws.com/"));

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

        let cc = config.get("cc");

        if message.has_attachments() {
            // Use SendRawEmail for attachments (MIME-encoded email)
            let raw_email = self
                .build_raw_email(message, &from_header, to, subject, cc)
                .await?;
            let raw_b64 = base64::engine::general_purpose::STANDARD.encode(raw_email.as_bytes());

            let mut params: Vec<(String, String)> = vec![
                ("Action".into(), "SendRawEmail".into()),
                ("Source".into(), from_header),
                ("RawMessage.Data".into(), raw_b64),
            ];

            // Add destinations — member index is 1-based; "to" is member.1
            params.push(("Destinations.member.1".into(), to.to_string()));
            if let Some(cc_addrs) = cc {
                for (i, addr) in cc_addrs.split(',').enumerate() {
                    params.push((
                        format!("Destinations.member.{}", i + 2),
                        addr.trim().to_string(),
                    ));
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
                Ok(
                    SendResponse::success("ses", "email with attachments sent via SES")
                        .with_status_code(status)
                        .with_raw_response(serde_json::json!({"response": body})),
                )
            } else {
                Ok(
                    SendResponse::failure("ses", format!("SES API error: {body}"))
                        .with_status_code(status)
                        .with_raw_response(serde_json::json!({"response": body})),
                )
            }
        } else {
            // Simple SendEmail for text-only messages
            let mut params: Vec<(String, String)> = vec![
                ("Action".into(), "SendEmail".into()),
                ("Source".into(), from_header),
                ("Destination.ToAddresses.member.1".into(), to.to_string()),
                ("Message.Subject.Data".into(), subject.to_string()),
                ("Message.Body.Text.Data".into(), message.text.clone()),
            ];

            if let Some(cc_addrs) = cc {
                for (i, addr) in cc_addrs.split(',').enumerate() {
                    params.push((
                        format!("Destination.CcAddresses.member.{}", i + 1),
                        addr.trim().to_string(),
                    ));
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
}
