use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Revolt messaging provider.
///
/// Sends messages via the Revolt REST API using a bot token.
/// Revolt is an open-source alternative to Discord.
/// Supports file attachments via the Autumn file server.
pub struct RevoltProvider {
    client: Client,
}

impl RevoltProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Upload a file to Revolt's Autumn file server and return the attachment ID.
    async fn upload_file(
        &self,
        api_url: &str,
        bot_token: &str,
        attachment: &noti_core::Attachment,
    ) -> Result<String, NotiError> {
        // Autumn is the file server for Revolt, default URL is https://autumn.revolt.chat
        let autumn_url = format!(
            "{}/attachments",
            api_url
                .replace("api.revolt.chat", "autumn.revolt.chat")
                .replace("/api", "")
        );

        let data = attachment.read_bytes().await?;
        let file_name = attachment.effective_file_name();
        let mime_str = attachment.effective_mime();

        let part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name)
            .mime_str(&mime_str)
            .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

        let form = reqwest::multipart::Form::new().part("file", part);

        let resp = self
            .client
            .post(&autumn_url)
            .header("x-bot-token", bot_token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

        raw.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| NotiError::provider("revolt", "file upload failed: missing id"))
    }
}

#[async_trait]
impl NotifyProvider for RevoltProvider {
    fn name(&self) -> &str {
        "revolt"
    }

    fn url_scheme(&self) -> &str {
        "revolt"
    }

    fn description(&self) -> &str {
        "Revolt chat via Bot API"
    }

    fn example_url(&self) -> &str {
        "revolt://<bot_token>/<channel_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("bot_token", "Revolt bot token").with_example("xxx..."),
            ParamDef::required("channel_id", "Revolt channel ID to post to")
                .with_example("01HXXXXXXXXX"),
            ParamDef::optional(
                "api_url",
                "Revolt API URL (default: https://api.revolt.chat)",
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
        let bot_token = config.require("bot_token", "revolt")?;
        let channel_id = config.require("channel_id", "revolt")?;
        let api_url = config.get("api_url").unwrap_or("https://api.revolt.chat");

        let url = format!("{api_url}/channels/{channel_id}/messages");

        let content = if let Some(ref title) = message.title {
            format!("**{title}**\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "content": content,
        });

        // Upload attachments and include IDs
        if message.has_attachments() {
            let mut attachment_ids = Vec::new();
            for attachment in &message.attachments {
                let id = self.upload_file(api_url, bot_token, attachment).await?;
                attachment_ids.push(id);
            }
            payload["attachments"] = json!(attachment_ids);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-bot-token", bot_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));

        if (200..300).contains(&status) {
            Ok(SendResponse::success("revolt", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_type = raw
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("revolt", format!("API error: {error_type}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
