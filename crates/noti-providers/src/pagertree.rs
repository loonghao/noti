use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// PagerTree incident management provider.
///
/// Creates incidents/alerts via PagerTree integration webhook.
///
/// API reference: <https://pagertree.com/docs/integration/incoming>
pub struct PagerTreeProvider {
    client: Client,
}

impl PagerTreeProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PagerTreeProvider {
    fn name(&self) -> &str {
        "pagertree"
    }

    fn url_scheme(&self) -> &str {
        "pagertree"
    }

    fn description(&self) -> &str {
        "PagerTree incident management webhook"
    }

    fn example_url(&self) -> &str {
        "pagertree://<integration_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("integration_id", "PagerTree integration ID")
                .with_example("your-integration-id"),
            ParamDef::optional("urgency", "Incident urgency: low, medium, high, critical")
                .with_example("high"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let integration_id = config.require("integration_id", "pagertree")?;

        let url = format!("https://api.pagertree.com/integration/{integration_id}");

        let title = message.title.as_deref().unwrap_or("Alert");

        let urgency = config.get("urgency").unwrap_or("high");

        let payload = json!({
            "event_type": "create",
            "Id": format!("noti-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()),
            "Title": title,
            "Description": message.text,
            "Urgency": urgency,
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
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
            Ok(
                SendResponse::success("pagertree", "incident created in PagerTree")
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        } else {
            Ok(
                SendResponse::failure("pagertree", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        }
    }
}
