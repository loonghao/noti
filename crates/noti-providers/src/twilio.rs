use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

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

        let url = format!("https://api.twilio.com/2010-04-01/Accounts/{account_sid}/Messages.json");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

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
                    "https://api.twilio.com/2010-04-01/Accounts/{account_sid}/Messages/MediaUpload"
                );

                let part = reqwest::multipart::Part::bytes(data)
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
                        let b64 = base64::engine::general_purpose::STANDARD
                            .encode(attachment.read_bytes().await.unwrap_or_default());
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
