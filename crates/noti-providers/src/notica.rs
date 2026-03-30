use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Notica provider.
///
/// Sends browser push notifications via Notica.
/// Supports image attachments by embedding base64 data URIs in
/// the notification payload.
///
/// API reference: <https://notica.us/>
pub struct NoticaProvider {
    client: Client,
}

impl NoticaProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NoticaProvider {
    fn name(&self) -> &str {
        "notica"
    }

    fn url_scheme(&self) -> &str {
        "notica"
    }

    fn description(&self) -> &str {
        "Notica browser push notifications"
    }

    fn example_url(&self) -> &str {
        "notica://<token>"
    }

    fn supports_attachments(&self) -> bool {
        false
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::required("token", "Notica notification token").with_example("abc123")]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token = config.require("token", "notica")?;

        let url = format!("https://notica.us/?{token}");

        // Build notification text with embedded image data URIs
        let mut body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        // Append image data as base64 data URIs in the payload
        if message.has_attachments() {
            for attachment in &message.attachments {
                if let Ok(data) = attachment.read_bytes().await {
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    body_text.push_str(&format!(
                        "\n[{}] data:{mime};base64,{b64}",
                        attachment.effective_file_name()
                    ));
                }
            }
        }

        let params = [("payload", body_text.as_str())];

        let resp = self
            .client
            .post(&url)
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
                SendResponse::success("notica", "notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("notica", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"body": body})),
            )
        }
    }
}
