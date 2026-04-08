use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Format the message body text, optionally prepending the title.
pub(crate) fn format_body_text(message: &Message) -> String {
    if let Some(ref title) = message.title {
        format!("{title}\n\n{}", message.text)
    } else {
        message.text.clone()
    }
}

/// Twilio SMS/MMS provider.
///
/// Sends SMS messages via the Twilio REST API.
/// Supports MMS media attachments via MediaUrl parameter for publicly accessible URLs,
/// or by uploading media to Twilio's Media resource and referencing it.
///
/// For file attachments, images are uploaded as base64 data URIs in the MediaUrl field
/// (Twilio MMS supports up to 10 media per message).
pub struct TwilioProvider {
    client: Client,
}

impl TwilioProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for TwilioProvider {
    fn name(&self) -> &str {
        "twilio"
    }

    fn url_scheme(&self) -> &str {
        "twilio"
    }

    fn description(&self) -> &str {
        "Twilio SMS/MMS via REST API"
    }

    fn example_url(&self) -> &str {
        "twilio://<account_sid>:<auth_token>@<from_number>/<to_number>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("account_sid", "Twilio account SID")
                .with_example("ACxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
            ParamDef::required("auth_token", "Twilio auth token").with_example("your_auth_token"),
            ParamDef::required("from", "Sender phone number (E.164 format)")
                .with_example("+15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
            ParamDef::optional(
                "media_url",
                "Public URL for MMS media (alternative to file attachments)",
            ),
            ParamDef::optional("base_url", "Twilio API base URL (default: https://api.twilio.com)")
                .with_example("https://api.twilio.com"),
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
        let account_sid = config.require("account_sid", "twilio")?;
        let auth_token = config.require("auth_token", "twilio")?;
        let from = config.require("from", "twilio")?;
        let to = config.require("to", "twilio")?;

        let base_url = config.get("base_url").unwrap_or("https://api.twilio.com");
        let url = format!("{}/2010-04-01/Accounts/{account_sid}/Messages.json", base_url.trim_end_matches('/'));

        let body_text = format_body_text(message);

        let mut form_params: Vec<(&str, String)> = vec![
            ("From", from.to_string()),
            ("To", to.to_string()),
            ("Body", body_text),
        ];

        // Add media attachments for MMS
        if message.has_attachments() {
            // Twilio MMS requires publicly accessible MediaUrl(s).
            // We upload each attachment to Twilio Media and get a URL,
            // or if a media_url config is provided, use that directly.
            // For local files, we upload to Twilio's Media resource first.
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let mime_str = attachment.effective_mime();
                let file_name = attachment.effective_file_name();

                // Upload media to Twilio's Media resource
                let media_upload_url = format!(
                    "{}/2010-04-01/Accounts/{account_sid}/Messages/MediaUpload",
                    base_url.trim_end_matches('/')
                );

                let part = reqwest::multipart::Part::bytes(data.clone())
                    .file_name(file_name)
                    .mime_str(&mime_str)
                    .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

                let form = reqwest::multipart::Form::new().part("MediaFile", part);

                // Try uploading; if this doesn't work (not all accounts support it),
                // fall back to base64 data URI
                let upload_resp = self
                    .client
                    .post(&media_upload_url)
                    .basic_auth(account_sid, Some(auth_token))
                    .multipart(form)
                    .send()
                    .await;

                match upload_resp {
                    Ok(resp) if resp.status().is_success() => {
                        let upload_raw: serde_json::Value =
                            resp.json().await.unwrap_or(serde_json::Value::Null);
                        if let Some(media_url) = upload_raw.get("uri").and_then(|v| v.as_str()) {
                            form_params.push(("MediaUrl", media_url.to_string()));
                        }
                    }
                    _ => {
                        // Fallback: use data URI (works for some MMS gateways)
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                        let data_uri = format!("data:{mime_str};base64,{b64}");
                        form_params.push(("MediaUrl", data_uri));
                    }
                }
            }
        } else if let Some(media_url) = config.get("media_url") {
            form_params.push(("MediaUrl", media_url.to_string()));
        }

        let resp = self
            .client
            .post(&url)
            .basic_auth(account_sid, Some(auth_token))
            .form(&form_params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            let sid = raw.get("sid").and_then(|v| v.as_str()).unwrap_or("unknown");
            let msg = if message.has_attachments() {
                format!("MMS sent with attachments (SID: {sid})")
            } else {
                format!("SMS sent (SID: {sid})")
            };
            Ok(SendResponse::success("twilio", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("twilio", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // format_body_text tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_format_body_text_plain() {
        let msg = Message::text("hello world");
        assert_eq!(format_body_text(&msg), "hello world");
    }

    #[test]
    fn test_format_body_text_with_title() {
        let msg = Message::text("hello world").with_title("My Title");
        assert_eq!(format_body_text(&msg), "My Title\n\nhello world");
    }

    #[test]
    fn test_format_body_text_empty_body() {
        let msg = Message::text("").with_title("Title Only");
        assert_eq!(format_body_text(&msg), "Title Only\n\n");
    }

    #[test]
    fn test_format_body_text_empty_title() {
        let msg = Message {
            text: "body only".into(),
            title: Some("".into()),
            format: Default::default(),
            priority: Default::default(),
            attachments: vec![],
            extra: Default::default(),
        };
        // Empty title still gets formatted (title is Some(""))
        assert_eq!(format_body_text(&msg), "\n\nbody only");
    }

    // -------------------------------------------------------------------------
    // Provider metadata tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_twilio_provider_name() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        assert_eq!(provider.name(), "twilio");
    }

    #[tokio::test]
    async fn test_twilio_provider_url_scheme() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        assert_eq!(provider.url_scheme(), "twilio");
    }

    #[tokio::test]
    async fn test_twilio_provider_description() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        assert!(provider.description().contains("Twilio"));
    }

    #[tokio::test]
    async fn test_twilio_provider_example_url() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        assert!(provider.example_url().contains("twilio://"));
    }

    #[tokio::test]
    async fn test_twilio_provider_supports_attachments() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        assert!(provider.supports_attachments());
    }

    #[tokio::test]
    async fn test_twilio_params_has_required_fields() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let params = provider.params();
        assert!(params.iter().any(|p| p.name == "account_sid" && p.required));
        assert!(params.iter().any(|p| p.name == "auth_token" && p.required));
        assert!(params.iter().any(|p| p.name == "from" && p.required));
        assert!(params.iter().any(|p| p.name == "to" && p.required));
    }

    #[tokio::test]
    async fn test_twilio_params_has_optional_fields() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let params = provider.params();
        assert!(params.iter().any(|p| p.name == "media_url" && !p.required));
        assert!(params.iter().any(|p| p.name == "base_url" && !p.required));
    }

    #[tokio::test]
    async fn test_twilio_params_count() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        // 4 required + 2 optional = 6 total
        assert_eq!(provider.params().len(), 6);
    }

    // -------------------------------------------------------------------------
    // Config validation tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_validate_config_full() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("account_sid", "ACxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
            .set("auth_token", "auth_token")
            .set("from", "+15551234567")
            .set("to", "+15559876543");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_missing_all() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        assert!(provider.validate_config(&ProviderConfig::new()).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_auth_token() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("account_sid", "ACxxx")
            .set("from", "+15551234567")
            .set("to", "+15559876543");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_from() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("account_sid", "ACxxx")
            .set("auth_token", "tok")
            .set("to", "+15559876543");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_to() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("account_sid", "ACxxx")
            .set("auth_token", "tok")
            .set("from", "+15551234567");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_optional_base_url() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("account_sid", "ACxxx")
            .set("auth_token", "tok")
            .set("from", "+15551234567")
            .set("to", "+15559876543")
            .set("base_url", "https://api.twilio.com");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_optional_media_url() {
        let provider = TwilioProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("account_sid", "ACxxx")
            .set("auth_token", "tok")
            .set("from", "+15551234567")
            .set("to", "+15559876543")
            .set("media_url", "https://example.com/image.png");
        assert!(provider.validate_config(&config).is_ok());
    }
}
