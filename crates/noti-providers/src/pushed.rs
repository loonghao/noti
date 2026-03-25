use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Pushed.co push notification provider.
///
/// Uses the Pushed.co REST API to send push notifications.
/// API docs: https://about.pushed.co/docs/api
pub struct PushedProvider {
    client: Client,
}

impl PushedProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushedProvider {
    fn name(&self) -> &str {
        "pushed"
    }

    fn url_scheme(&self) -> &str {
        "pushed"
    }

    fn description(&self) -> &str {
        "Pushed.co push notifications via REST API"
    }

    fn example_url(&self) -> &str {
        "pushed://<app_key>:<app_secret>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("app_key", "Pushed application key"),
            ParamDef::required("app_secret", "Pushed application secret"),
            ParamDef::optional(
                "target_type",
                "Target type: app, channel, or pushed_id (default: app)",
            ),
            ParamDef::optional(
                "target_alias",
                "Channel alias or pushed_id (when target_type is not app)",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let app_key = config.require("app_key", "pushed")?;
        let app_secret = config.require("app_secret", "pushed")?;

        let target_type = config.get("target_type").unwrap_or("app");

        let mut payload = json!({
            "app_key": app_key,
            "app_secret": app_secret,
            "target_type": target_type,
            "content": message.text,
        });

        if let Some(alias) = config.get("target_alias") {
            payload["target_alias"] = json!(alias);
        }

        let resp = self
            .client
            .post("https://api.pushed.co/1/push")
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
                SendResponse::success("pushed", "push notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("pushed", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
