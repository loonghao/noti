use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Spike.sh incident management provider.
///
/// Creates incidents via the Spike.sh integration API.
/// Spike.sh is a modern incident management and alerting platform.
///
/// API reference: <https://docs.spike.sh/integration-guides/integrate-any-tool-via-webhook>
pub struct SpikeProvider {
    client: Client,
}

impl SpikeProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SpikeProvider {
    fn name(&self) -> &str {
        "spike"
    }

    fn url_scheme(&self) -> &str {
        "spike"
    }

    fn description(&self) -> &str {
        "Spike.sh incident management and alerting"
    }

    fn example_url(&self) -> &str {
        "spike://<webhook_url>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Spike.sh integration webhook URL")
                .with_example("https://hooks.spike.sh/custom/xxx"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_url = config.require("webhook_url", "spike")?;

        let title = message.title.as_deref().unwrap_or("Alert");

        let payload = json!({
            "title": title,
            "message": message.text,
            "status": "open",
        });

        let resp = self
            .client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("spike", "incident sent to Spike.sh")
                .with_status_code(status)
                .with_raw_response(json!({"body": body})))
        } else {
            Ok(SendResponse::failure("spike", format!("API error: {body}"))
                .with_status_code(status)
                .with_raw_response(json!({"body": body})))
        }
    }
}
