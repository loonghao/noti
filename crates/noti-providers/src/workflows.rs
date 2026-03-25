use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Microsoft Power Automate / Workflows notification provider.
///
/// Sends notifications through Microsoft Power Automate (formerly Flow)
/// using Azure Logic Apps webhook endpoints. Messages are delivered as
/// Adaptive Cards via the Workflows connector (replacement for Teams
/// incoming webhooks).
///
/// API Reference: <https://learn.microsoft.com/en-us/power-automate/>
pub struct WorkflowsProvider {
    client: Client,
}

impl WorkflowsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for WorkflowsProvider {
    fn name(&self) -> &str {
        "workflows"
    }

    fn url_scheme(&self) -> &str {
        "workflows"
    }

    fn description(&self) -> &str {
        "Microsoft Power Automate / Workflows notifications"
    }

    fn example_url(&self) -> &str {
        "workflows://<host>:<port>/<workflow>/<signature>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Azure Logic Apps host")
                .with_example("prod-XX.westus.logic.azure.com"),
            ParamDef::optional("port", "Host port (default: 443)").with_example("443"),
            ParamDef::required("workflow", "Workflow ID from the webhook URL")
                .with_example("abc123def456"),
            ParamDef::required("signature", "Signature from the webhook URL sig= parameter")
                .with_example("xyzSignatureValue"),
            ParamDef::optional("api_version", "Power Automate API version")
                .with_example("2016-06-01"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let host = config.require("host", "workflows")?;
        let workflow = config.require("workflow", "workflows")?;
        let signature = config.require("signature", "workflows")?;

        let port = config.get("port").unwrap_or("443");
        let api_version = config.get("api_version").unwrap_or("2016-06-01");

        let url = format!(
            "https://{host}:{port}/workflows/{workflow}/triggers/manual/paths/invoke\
             ?api-version={api_version}\
             &sp=%2Ftriggers%2Fmanual%2Frun\
             &sv=1.0\
             &sig={signature}"
        );

        // Build Adaptive Card payload
        let title = message.title.clone().unwrap_or_else(|| "noti".to_string());
        let payload = json!({
            "type": "message",
            "attachments": [{
                "contentType": "application/vnd.microsoft.card.adaptive",
                "contentUrl": null,
                "content": {
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "type": "AdaptiveCard",
                    "version": "1.4",
                    "body": [
                        {
                            "type": "TextBlock",
                            "text": title,
                            "weight": "Bolder",
                            "size": "Medium"
                        },
                        {
                            "type": "TextBlock",
                            "text": message.text,
                            "wrap": true
                        }
                    ]
                }
            }]
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
        let body_text = resp.text().await.unwrap_or_default();
        let raw: serde_json::Value = serde_json::from_str(&body_text)
            .unwrap_or_else(|_| json!({"status": status, "body": body_text}));

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("workflows", "Power Automate notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("error")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
                .or_else(|| raw.get("body").and_then(|v| v.as_str()))
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("workflows", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
