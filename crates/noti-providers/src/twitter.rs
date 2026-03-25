use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Twitter/X notification provider.
///
/// Uses the X (Twitter) API v2 to post tweets or send DMs.
/// API docs: https://developer.x.com/en/docs/x-api
pub struct TwitterProvider {
    client: Client,
}

impl TwitterProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for TwitterProvider {
    fn name(&self) -> &str {
        "twitter"
    }

    fn url_scheme(&self) -> &str {
        "twitter"
    }

    fn description(&self) -> &str {
        "X (Twitter) — post tweets or send DMs via API v2"
    }

    fn example_url(&self) -> &str {
        "twitter://<bearer_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("bearer_token", "X (Twitter) API v2 Bearer token"),
            ParamDef::optional("mode", "Send mode: tweet or dm (default: tweet)"),
            ParamDef::optional("dm_user_id", "Recipient user ID for DM mode"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let bearer_token = config.require("bearer_token", "twitter")?;
        let mode = config.get("mode").unwrap_or("tweet");

        let (url, payload) = if mode == "dm" {
            let dm_user_id = config.require("dm_user_id", "twitter")?;
            (
                "https://api.x.com/2/dm_conversations/with/messages".to_string(),
                json!({
                    "participant_id": dm_user_id,
                    "message": {
                        "text": message.text
                    }
                }),
            )
        } else {
            (
                "https://api.x.com/2/tweets".to_string(),
                json!({ "text": message.text }),
            )
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {bearer_token}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("twitter", format!("{mode} sent successfully"))
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("twitter", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
