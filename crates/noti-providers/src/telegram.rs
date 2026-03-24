use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Telegram Bot API provider.
pub struct TelegramProvider {
    client: Client,
}

impl TelegramProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
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
            ParamDef::optional("disable_notification", "Send silently (true/false)"),
            ParamDef::optional(
                "disable_web_page_preview",
                "Disable link previews (true/false)",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let bot_token = config.require("bot_token", "telegram")?;
        let chat_id = config.require("chat_id", "telegram")?;

        let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

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

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
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
}
