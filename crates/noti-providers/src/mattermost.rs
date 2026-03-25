use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Mattermost incoming webhook provider.
///
/// Mattermost webhooks accept JSON with `text`, `channel`, `username`,
/// and `icon_url` fields. Markdown is natively supported in the `text` field.
pub struct MattermostProvider {
    client: Client,
}

impl MattermostProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for MattermostProvider {
    fn name(&self) -> &str {
        "mattermost"
    }

    fn url_scheme(&self) -> &str {
        "mattermost"
    }

    fn description(&self) -> &str {
        "Mattermost via incoming webhook"
    }

    fn example_url(&self) -> &str {
        "mattermost://<host>/<hook_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required(
                "host",
                "Mattermost server host (e.g. mattermost.example.com)",
            )
            .with_example("mattermost.example.com"),
            ParamDef::required("hook_id", "Incoming webhook ID").with_example("abcdef1234567890"),
            ParamDef::optional("channel", "Override the default channel"),
            ParamDef::optional("username", "Override the posting username"),
            ParamDef::optional("icon_url", "Override the posting icon URL"),
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
        let host = config.require("host", "mattermost")?;
        let hook_id = config.require("hook_id", "mattermost")?;
        let url_scheme = config.get("scheme").unwrap_or("https");
        let port = config.get("port").unwrap_or("443");

        let webhook_url = format!("{url_scheme}://{host}:{port}/hooks/{hook_id}");

        // Mattermost supports markdown natively in the text field
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
            payload["username"] = json!(username);
        }
        if let Some(icon_url) = config.get("icon_url") {
            payload["icon_url"] = json!(icon_url);
        }

        let resp = self
            .client
            .post(&webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("mattermost", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        } else {
            Ok(SendResponse::failure(
                "mattermost",
                format!("API error (HTTP {status}): {body_text}"),
            )
            .with_status_code(status)
            .with_raw_response(json!({ "body": body_text })))
        }
    }
}
