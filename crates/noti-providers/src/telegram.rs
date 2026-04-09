use async_trait::async_trait;
use noti_core::{
    AttachmentKind, Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig,
    SendResponse,
};
use reqwest::Client;
use reqwest::multipart;
use serde_json::json;

/// Telegram Bot API provider.
pub struct TelegramProvider {
    client: Client,
}

impl TelegramProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Returns the API base URL, using config override if set.
    fn base_url(config: &ProviderConfig) -> String {
        config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://api.telegram.org".to_string())
    }

    /// Send a text-only message via sendMessage.
    async fn send_text(
        &self,
        message: &Message,
        bot_token: &str,
        chat_id: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let url = format!("{}/bot{}/sendMessage", Self::base_url(config), bot_token);

        let parse_mode = match message.format {
            MessageFormat::Markdown => Some("MarkdownV2"),
            MessageFormat::Html => Some("HTML"),
            MessageFormat::Text => None,
        };

        let mut payload = json!({
            "chat_id": chat_id,
            "text": message.text,
        });

        if let Some(mode) = parse_mode {
            payload["parse_mode"] = json!(mode);
        }
        if config.get("disable_notification") == Some("true") {
            payload["disable_notification"] = json!(true);
        }
        if config.get("disable_web_page_preview") == Some("true") {
            payload["disable_web_page_preview"] = json!(true);
        }
        if let Some(thread_id) = config.get("thread_id") {
            payload["message_thread_id"] = json!(thread_id);
        }
        if config.get("protect") == Some("true") {
            payload["protect_content"] = json!(true);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("telegram", e))?;

        Self::parse_response(resp).await
    }

    /// Send file attachments via the appropriate Telegram endpoints.
    /// Sends the first attachment with the message caption, additional
    /// attachments are sent as separate documents.
    async fn send_attachment(
        &self,
        message: &Message,
        bot_token: &str,
        chat_id: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let mut last_response = None;

        for (i, attachment) in message.attachments.iter().enumerate() {
            let data = attachment.read_bytes().await?;

            let (method, field_name) = match attachment.kind {
                AttachmentKind::Image => ("sendPhoto", "photo"),
                AttachmentKind::Audio => ("sendAudio", "audio"),
                AttachmentKind::Video => ("sendVideo", "video"),
                AttachmentKind::File => ("sendDocument", "document"),
            };

            let url = format!("{}/bot{}/{}", Self::base_url(config), bot_token, method);
            let file_name = attachment.effective_file_name();
            let mime_str = attachment.effective_mime();

            let file_part = multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("invalid MIME type: {e}")))?;

            let mut form = multipart::Form::new()
                .text("chat_id", chat_id.to_string())
                .part(field_name.to_string(), file_part);

            // Only add caption to the first attachment
            if i == 0 && !message.text.is_empty() {
                form = form.text("caption", message.text.clone());
                match message.format {
                    MessageFormat::Markdown => {
                        form = form.text("parse_mode", "MarkdownV2".to_string());
                    }
                    MessageFormat::Html => {
                        form = form.text("parse_mode", "HTML".to_string());
                    }
                    MessageFormat::Text => {}
                }
            }

            if config.get("disable_notification") == Some("true") {
                form = form.text("disable_notification", "true".to_string());
            }
            if let Some(thread_id) = config.get("thread_id") {
                form = form.text("message_thread_id", thread_id.to_string());
            }
            if config.get("protect") == Some("true") {
                form = form.text("protect_content", "true".to_string());
            }

            let resp = self
                .client
                .post(&url)
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("telegram", e))?;

            let result = Self::parse_response(resp).await?;
            if !result.success {
                return Ok(result);
            }
            last_response = Some(result);
        }

        Ok(last_response
            .unwrap_or_else(|| SendResponse::success("telegram", "message sent successfully")))
    }

    async fn parse_response(resp: reqwest::Response) -> Result<SendResponse, NotiError> {
        let status = resp.status().as_u16();

        // Handle rate limiting (429)
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::http_helpers::handle_http_error(
                "telegram",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }

        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let ok = raw.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if ok {
            Ok(
                SendResponse::success("telegram", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let desc = raw
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("telegram", format!("API error: {desc}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }

    /// Send a chat action (typing, upload_photo, etc.) via sendChatAction.
    async fn send_chat_action(
        &self,
        bot_token: &str,
        chat_id: &str,
        action: &str,
        base_url: &str,
    ) -> Result<SendResponse, NotiError> {
        let url = format!("{}/bot{}/sendChatAction", base_url, bot_token);
        let payload = json!({
            "chat_id": chat_id,
            "action": action,
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("telegram", e))?;

        Self::parse_response(resp).await
    }

    /// Edit an existing message in-place via editMessageText.
    async fn edit_message(
        &self,
        message: &Message,
        bot_token: &str,
        chat_id: &str,
        message_id: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let url = format!(
            "{}/bot{}/editMessageText",
            Self::base_url(config),
            bot_token
        );

        let parse_mode = match message.format {
            MessageFormat::Markdown => Some("MarkdownV2"),
            MessageFormat::Html => Some("HTML"),
            MessageFormat::Text => None,
        };

        let msg_id: i64 = message_id
            .parse()
            .map_err(|_| NotiError::Validation(format!("invalid message_id: {message_id}")))?;

        let mut payload = json!({
            "chat_id": chat_id,
            "message_id": msg_id,
            "text": message.text,
        });

        if let Some(mode) = parse_mode {
            payload["parse_mode"] = json!(mode);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("telegram", e))?;

        Self::parse_response(resp).await
    }
}

#[async_trait]
impl NotifyProvider for TelegramProvider {
    fn name(&self) -> &str {
        "telegram"
    }

    fn url_scheme(&self) -> &str {
        "tg"
    }

    fn description(&self) -> &str {
        "Telegram via Bot API"
    }

    fn example_url(&self) -> &str {
        "tg://<bot_token>/<chat_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("bot_token", "Telegram bot token from @BotFather")
                .with_example("123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11"),
            ParamDef::required("chat_id", "Target chat ID").with_example("-1001234567890"),
            ParamDef::optional(
                "base_url",
                "Override API base URL (for testing or Telegram-compatible APIs)",
            )
            .with_example("https://api.telegram.org"),
            ParamDef::optional("disable_notification", "Send silently (true/false)"),
            ParamDef::optional(
                "disable_web_page_preview",
                "Disable link previews (true/false)",
            ),
            ParamDef::optional("thread_id", "Message thread ID for supergroup topics"),
            ParamDef::optional(
                "protect",
                "Protect content from forwarding/saving (true/false)",
            ),
            ParamDef::optional(
                "action",
                "Chat action to send (typing, upload_photo, upload_document, etc.)",
            )
            .with_example("typing"),
            ParamDef::optional(
                "edit_message_id",
                "Edit an existing message by its message ID (in-place update)",
            )
            .with_example("12345"),
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
        let bot_token = config.require("bot_token", "telegram")?;
        let chat_id = config.require("chat_id", "telegram")?;

        // Handle sendChatAction (typing indicators, etc.)
        if let Some(action) = config.get("action") {
            return self
                .send_chat_action(bot_token, chat_id, action, &Self::base_url(config))
                .await;
        }

        // Handle editMessageText (in-place update)
        if let Some(message_id) = config.get("edit_message_id") {
            return self
                .edit_message(message, bot_token, chat_id, message_id, config)
                .await;
        }

        if message.has_attachments() {
            self.send_attachment(message, bot_token, chat_id, config)
                .await
        } else {
            self.send_text(message, bot_token, chat_id, config).await
        }
    }
}
