use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Google Chat (formerly Hangouts Chat) webhook provider.
///
/// Uses the Google Chat Spaces webhook URL to post messages.
/// The webhook URL looks like:
/// `https://chat.googleapis.com/v1/spaces/<space>/messages?key=<key>&token=<token>`
pub struct GoogleChatProvider {
    client: Client,
}

impl GoogleChatProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for GoogleChatProvider {
    fn name(&self) -> &str {
        "googlechat"
    }

    fn url_scheme(&self) -> &str {
        "gchat"
    }

    fn description(&self) -> &str {
        "Google Chat via space webhook"
    }

    fn example_url(&self) -> &str {
        "gchat://<space>/<key>/<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Google Chat space webhook URL").with_example(
                "https://chat.googleapis.com/v1/spaces/AAAA/messages?key=xxx&token=yyy",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_url = config.require("webhook_url", "googlechat")?;

        // Google Chat supports a simple text field and cards.
        // For markdown/rich content, use cardsV2.
        let payload = match message.format {
            MessageFormat::Markdown | MessageFormat::Html => {
                // Google Chat supports a subset of markup in `text` field
                let mut text = String::new();
                if let Some(ref title) = message.title {
                    text.push_str(&format!("*{title}*\n\n"));
                }
                text.push_str(&message.text);
                json!({ "text": text })
            }
            MessageFormat::Text => {
                let mut text = String::new();
                if let Some(ref title) = message.title {
                    text.push_str(&format!("{title}\n\n"));
                }
                text.push_str(&message.text);
                json!({ "text": text })
            }
        };

        let resp = self
            .client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("googlechat", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error_msg = raw
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("googlechat", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
