use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Microsoft Teams incoming webhook provider.
///
/// Uses the Power Automate / Workflows webhook connector (the modern replacement
/// for the deprecated Office 365 connectors).  The webhook URL is the full
/// URL obtained when configuring a "Post to a channel when a webhook request
/// is received" workflow in Teams.
pub struct TeamsProvider {
    client: Client,
}

impl TeamsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for TeamsProvider {
    fn name(&self) -> &str {
        "teams"
    }

    fn url_scheme(&self) -> &str {
        "teams"
    }

    fn description(&self) -> &str {
        "Microsoft Teams via incoming webhook"
    }

    fn example_url(&self) -> &str {
        "teams://<webhook_url_encoded>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Teams incoming webhook URL")
                .with_example("https://xxx.webhook.office.com/webhookb2/..."),
            ParamDef::optional("theme_color", "Hex color for the card accent (e.g. 0076D7)"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_url = config.require("webhook_url", "teams")?;
        let theme_color = config.get("theme_color").unwrap_or("0076D7");

        // Build Adaptive Card payload (works with modern Workflows webhooks)
        let text_block = match message.format {
            MessageFormat::Markdown => {
                json!({
                    "type": "TextBlock",
                    "text": message.text,
                    "wrap": true
                })
            }
            _ => {
                json!({
                    "type": "TextBlock",
                    "text": message.text,
                    "wrap": true
                })
            }
        };

        let mut body_items = Vec::new();

        // Add title if present
        if let Some(ref title) = message.title {
            body_items.push(json!({
                "type": "TextBlock",
                "size": "Medium",
                "weight": "Bolder",
                "text": title,
                "color": "Accent"
            }));
        }

        body_items.push(text_block);

        let payload = json!({
            "type": "message",
            "attachments": [{
                "contentType": "application/vnd.microsoft.card.adaptive",
                "content": {
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "type": "AdaptiveCard",
                    "version": "1.4",
                    "body": body_items,
                    "msteams": {
                        "width": "Full",
                        "themeColor": theme_color
                    }
                }
            }]
        });

        let resp = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
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
            Ok(SendResponse::success("teams", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(json!({ "body": body_text })))
        } else {
            Ok(
                SendResponse::failure("teams", format!("API error (HTTP {status}): {body_text}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        }
    }
}
