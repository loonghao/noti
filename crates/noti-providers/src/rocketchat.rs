use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Rocket.Chat incoming webhook provider.
///
/// Uses Rocket.Chat's incoming webhook integration.
/// The webhook URL is obtained from Administration > Integrations > New Integration.
pub struct RocketChatProvider {
    client: Client,
}

impl RocketChatProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for RocketChatProvider {
    fn name(&self) -> &str {
        "rocketchat"
    }

    fn url_scheme(&self) -> &str {
        "rocketchat"
    }

    fn description(&self) -> &str {
        "Rocket.Chat via incoming webhook"
    }

    fn example_url(&self) -> &str {
        "rocketchat://<host>/<token_a>/<token_b>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Rocket.Chat server host (e.g. chat.example.com)")
                .with_example("chat.example.com"),
            ParamDef::required("token_a", "Webhook token part A").with_example("abcdef"),
            ParamDef::required("token_b", "Webhook token part B").with_example("ghijkl"),
            ParamDef::optional("channel", "Target channel or user (e.g. #general, @user)"),
            ParamDef::optional("username", "Override the posting username"),
            ParamDef::optional("icon_url", "Override the posting avatar URL"),
            ParamDef::optional("port", "Server port (default: 443)"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let host = config.require("host", "rocketchat")?;
        let token_a = config.require("token_a", "rocketchat")?;
        let token_b = config.require("token_b", "rocketchat")?;
        let url_scheme = config.get("scheme").unwrap_or("https");
        let port = config.get("port").unwrap_or("443");

        let webhook_url = format!("{url_scheme}://{host}:{port}/hooks/{token_a}/{token_b}");

        // Rocket.Chat supports markdown in the text field
        let text = match (&message.format, &message.title) {
            (MessageFormat::Markdown, Some(title)) => {
                format!("### {title}\n\n{}", message.text)
            }
            (_, Some(title)) => {
                format!("**{title}**\n\n{}", message.text)
            }
            _ => message.text.clone(),
        };

        let mut payload = json!({ "text": text });

        if let Some(channel) = config.get("channel") {
            payload["channel"] = json!(channel);
        }
        if let Some(username) = config.get("username") {
            payload["alias"] = json!(username);
        }
        if let Some(icon_url) = config.get("icon_url") {
            payload["avatar"] = json!(icon_url);
        }

        let resp = self
            .client
            .post(&webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({ "error": "failed to parse response" }));

        let success = raw
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || (200..300).contains(&status);

        if success {
            Ok(
                SendResponse::success("rocketchat", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error_msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("rocketchat", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
