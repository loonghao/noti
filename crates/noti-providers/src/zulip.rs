use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// Zulip messaging provider.
///
/// Sends messages via the Zulip API using a bot's email and API key.
/// Supports both stream and direct messages, with file attachment upload.
pub struct ZulipProvider {
    client: Client,
}

impl ZulipProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Upload a file to Zulip and return the markdown link for embedding.
    async fn upload_file(
        &self,
        domain: &str,
        bot_email: &str,
        api_key: &str,
        attachment: &noti_core::Attachment,
    ) -> Result<String, NotiError> {
        let url = format!("https://{domain}/api/v1/user_uploads");
        let data = attachment.read_bytes().await?;
        let file_name = attachment.effective_file_name();
        let mime_str = attachment.effective_mime();

        let part = reqwest::multipart::Part::bytes(data)
            .file_name(file_name.clone())
            .mime_str(&mime_str)
            .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

        let form = reqwest::multipart::Form::new().part("filename", part);

        let resp = self
            .client
            .post(&url)
            .basic_auth(bot_email, Some(api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

        let uri = raw
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NotiError::provider("zulip", "file upload failed: missing uri"))?;

        Ok(format!("[{file_name}]({uri})"))
    }
}

#[async_trait]
impl NotifyProvider for ZulipProvider {
    fn name(&self) -> &str {
        "zulip"
    }

    fn url_scheme(&self) -> &str {
        "zulip"
    }

    fn description(&self) -> &str {
        "Zulip team chat via Bot API"
    }

    fn example_url(&self) -> &str {
        "zulip://<bot_email>:<api_key>@<organization>.zulipchat.com/<stream>/<topic>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("domain", "Zulip server domain (e.g. yourorg.zulipchat.com)")
                .with_example("yourorg.zulipchat.com"),
            ParamDef::required("bot_email", "Bot email address")
                .with_example("my-bot@yourorg.zulipchat.com"),
            ParamDef::required("api_key", "Bot API key").with_example("abc123def456"),
            ParamDef::optional("stream", "Target stream name (for stream messages)")
                .with_example("general"),
            ParamDef::optional("topic", "Message topic within stream")
                .with_example("notifications"),
            ParamDef::optional(
                "to",
                "User email for direct messages (alternative to stream)",
            )
            .with_example("user@example.com"),
            ParamDef::optional("type", "Message type: stream or direct (default: stream)")
                .with_example("stream"),
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
        let domain = config.require("domain", "zulip")?;
        let bot_email = config.require("bot_email", "zulip")?;
        let api_key = config.require("api_key", "zulip")?;

        let url = format!("https://{domain}/api/v1/messages");

        let msg_type = config.get("type").unwrap_or("stream");
        let mut content = match message.format {
            MessageFormat::Markdown | MessageFormat::Html => {
                if let Some(ref title) = message.title {
                    format!("**{title}**\n\n{}", message.text)
                } else {
                    message.text.clone()
                }
            }
            MessageFormat::Text => {
                if let Some(ref title) = message.title {
                    format!("{title}\n\n{}", message.text)
                } else {
                    message.text.clone()
                }
            }
        };

        // Upload attachments and append markdown links
        if message.has_attachments() {
            for attachment in &message.attachments {
                let link = self
                    .upload_file(domain, bot_email, api_key, attachment)
                    .await?;
                content.push_str(&format!("\n{link}"));
            }
        }

        let mut form: Vec<(&str, String)> =
            vec![("type", msg_type.to_string()), ("content", content)];

        if msg_type == "direct" || msg_type == "private" {
            if let Some(to) = config.get("to") {
                form.push(("to", format!("[{to:?}]")));
            }
        } else {
            if let Some(stream) = config.get("stream") {
                form.push(("to", stream.to_string()));
            }
            let topic = config.get("topic").unwrap_or("notification");
            form.push(("topic", topic.to_string()));
        }

        let resp = self
            .client
            .post(&url)
            .basic_auth(bot_email, Some(api_key))
            .form(&form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let result = raw
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("error");
        if result == "success" {
            Ok(SendResponse::success("zulip", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("zulip", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
