use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Slack incoming webhook provider.
pub struct SlackProvider {
    client: Client,
}

impl SlackProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SlackProvider {
    fn name(&self) -> &str {
        "slack"
    }

    fn url_scheme(&self) -> &str {
        "slack"
    }

    fn description(&self) -> &str {
        "Slack via incoming webhooks"
    }

    fn example_url(&self) -> &str {
        "slack://<token_a>/<token_b>/<token_c>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Slack incoming webhook URL")
                .with_example("https://hooks.slack.com/services/T.../B.../xxx"),
            ParamDef::optional("channel", "Override the default channel"),
            ParamDef::optional("username", "Override the default username"),
            ParamDef::optional("icon_emoji", "Override the default icon emoji"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_url = config.require("webhook_url", "slack")?;

        let mut payload = match message.format {
            MessageFormat::Markdown => {
                // Slack uses mrkdwn format
                json!({
                    "blocks": [{
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": message.text
                        }
                    }]
                })
            }
            _ => {
                json!({ "text": message.text })
            }
        };

        // Add optional overrides
        if let Some(channel) = config.get("channel") {
            payload["channel"] = json!(channel);
        }
        if let Some(username) = config.get("username") {
            payload["username"] = json!(username);
        }
        if let Some(icon) = config.get("icon_emoji") {
            payload["icon_emoji"] = json!(icon);
        }

        let resp = self
            .client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if status == 200 && body_text == "ok" {
            Ok(
                SendResponse::success("slack", "message sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("slack", format!("API error: {body_text}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        }
    }
}
