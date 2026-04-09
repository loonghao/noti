use async_trait::async_trait;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// LINE Notify provider.
///
/// Sends notifications via the LINE Notify API.
/// Users generate a personal access token at https://notify-bot.line.me/.
/// Supports image uploads via multipart form.
pub struct LineProvider {
    client: Client,
}

impl LineProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Build LINE Notify API URL with optional base_url override.
    fn line_api_url(config: &ProviderConfig) -> String {
        let base = config
            .get("base_url")
            .unwrap_or("https://notify-api.line.me");
        let base = base.trim_end_matches('/');
        format!("{base}/api/notify")
    }

    /// Format message text with optional title prefix.
    fn format_message_text(message: &Message) -> String {
        if let Some(ref title) = message.title {
            format!("\n{title}\n{}", message.text)
        } else {
            format!("\n{}", message.text)
        }
    }
}

#[async_trait]
impl NotifyProvider for LineProvider {
    fn name(&self) -> &str {
        "line"
    }

    fn url_scheme(&self) -> &str {
        "line"
    }

    fn description(&self) -> &str {
        "LINE Notify push notifications"
    }

    fn example_url(&self) -> &str {
        "line://<access_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "LINE Notify personal access token")
                .with_example("xxxxxxxxxxxxxxxxxxxx"),
            ParamDef::optional("base_url", "LINE Notify API base URL (default: https://notify-api.line.me)")
                .with_example("https://notify-api.line.me"),
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
        let access_token = config.require("access_token", "line")?;

        let url = Self::line_api_url(config);

        let text = Self::format_message_text(message);

        // If there's an image attachment, use multipart form with imageFile
        if let Some(img) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = img.read_bytes().await?;
            let file_name = img.effective_file_name();
            let mime_str = img.effective_mime();

            let file_part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

            let form = reqwest::multipart::Form::new()
                .text("message", text)
                .part("imageFile", file_part);

            let resp = self
                .client
                .post(url)
                .header("Authorization", format!("Bearer {access_token}"))
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("line", e))?;

            return Self::parse_response(resp).await;
        }

        // If there are non-image attachments, mention them in the text
        if message.has_attachments() {
            let mut full_text = text;
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                full_text.push_str(&format!("\n📎 {file_name}"));
            }

            let form = [("message", full_text.as_str())];

            let resp = self
                .client
                .post(url)
                .header("Authorization", format!("Bearer {access_token}"))
                .form(&form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("line", e))?;

            return Self::parse_response(resp).await;
        }

        // Text-only
        let form = [("message", text.as_str())];

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .form(&form)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("line", e))?;

        Self::parse_response(resp).await
    }
}

impl LineProvider {
    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let code = raw.get("status").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code == 200 {
            Ok(
                SendResponse::success("line", "notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("line", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
