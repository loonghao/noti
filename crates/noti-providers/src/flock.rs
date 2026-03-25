use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Flock team messaging provider.
///
/// Sends messages via Flock incoming webhooks.
/// Flock is a team messaging and collaboration platform.
pub struct FlockProvider {
    client: Client,
}

impl FlockProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FlockProvider {
    fn name(&self) -> &str {
        "flock"
    }

    fn url_scheme(&self) -> &str {
        "flock"
    }

    fn description(&self) -> &str {
        "Flock team messaging via incoming webhooks"
    }

    fn example_url(&self) -> &str {
        "flock://<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token", "Flock incoming webhook token")
                .with_example("your-webhook-token"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token = config.require("token", "flock")?;

        let url = format!("https://api.flock.com/hooks/sendMessage/{token}");

        let text = if let Some(ref title) = message.title {
            format!("**{title}**\n{}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "text": text
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("flock", "message sent to Flock")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("flock", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
