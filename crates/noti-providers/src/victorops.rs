use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Splunk On-Call (formerly VictorOps) provider.
///
/// Creates incidents via the Splunk On-Call REST endpoint for monitoring tool integrations.
/// Supports image attachments via `image_url` in the alert payload.
///
/// API reference: <https://help.victorops.com/knowledge-base/rest-endpoint-integration-guide/>
pub struct VictorOpsProvider {
    client: Client,
}

impl VictorOpsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for VictorOpsProvider {
    fn name(&self) -> &str {
        "victorops"
    }

    fn url_scheme(&self) -> &str {
        "victorops"
    }

    fn description(&self) -> &str {
        "Splunk On-Call (VictorOps) incident management"
    }

    fn example_url(&self) -> &str {
        "victorops://<api_key>/<routing_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "VictorOps REST API key").with_example("your-api-key"),
            ParamDef::required("routing_key", "VictorOps routing key").with_example("my-team"),
            ParamDef::optional(
                "message_type",
                "Alert type: CRITICAL, WARNING, ACKNOWLEDGEMENT, INFO, RECOVERY",
            )
            .with_example("CRITICAL"),
            ParamDef::optional("base_url", "Override base URL for API requests"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "victorops")?;
        let routing_key = config.require("routing_key", "victorops")?;

        let base_url = config.get("base_url").unwrap_or("https://alert.victorops.com");
        let url = format!("{base_url}/integrations/generic/20131114/alert/{api_key}/{routing_key}");

        let message_type = config.get("message_type").unwrap_or("CRITICAL");

        let title = message.title.as_deref().unwrap_or("Alert");

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut payload = json!({
            "message_type": message_type,
            "entity_id": format!("noti-{timestamp}"),
            "entity_display_name": title,
            "state_message": message.text,
            "monitoring_tool": "noti-cli",
        });

        // Embed first image attachment as base64 data URI in image_url
        if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                payload["image_url"] = json!(format!("data:{mime};base64,{b64}"));
            }
        }

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
            let result = raw
                .get("result")
                .and_then(|v| v.as_str())
                .unwrap_or("accepted");
            if result == "success" || result == "accepted" {
                Ok(
                    SendResponse::success("victorops", "incident sent to Splunk On-Call")
                        .with_status_code(status)
                        .with_raw_response(raw),
                )
            } else {
                Ok(
                    SendResponse::failure("victorops", format!("unexpected result: {result}"))
                        .with_status_code(status)
                        .with_raw_response(raw),
                )
            }
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("victorops", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
