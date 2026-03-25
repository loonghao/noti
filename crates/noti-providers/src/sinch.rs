use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Sinch SMS provider.
///
/// Sends SMS messages via the Sinch REST API.
/// Requires service plan ID, API token, sender, and recipient.
pub struct SinchProvider {
    client: Client,
}

impl SinchProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SinchProvider {
    fn name(&self) -> &str {
        "sinch"
    }

    fn url_scheme(&self) -> &str {
        "sinch"
    }

    fn description(&self) -> &str {
        "Sinch SMS via REST API"
    }

    fn example_url(&self) -> &str {
        "sinch://<service_plan_id>:<api_token>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("service_plan_id", "Sinch service plan ID")
                .with_example("your-plan-id"),
            ParamDef::required("api_token", "Sinch API token").with_example("your-api-token"),
            ParamDef::required("from", "Sender phone number or short code")
                .with_example("+15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
            ParamDef::optional("region", "API region: us or eu (default: us)").with_example("us"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let service_plan_id = config.require("service_plan_id", "sinch")?;
        let api_token = config.require("api_token", "sinch")?;
        let from = config.require("from", "sinch")?;
        let to = config.require("to", "sinch")?;
        let region = config.get("region").unwrap_or("us");

        let base_url = match region {
            "eu" => "https://eu.sms.api.sinch.com",
            _ => "https://us.sms.api.sinch.com",
        };

        let url = format!("{base_url}/xms/v1/{service_plan_id}/batches");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "from": from,
            "to": [to],
            "body": body_text
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(api_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            let batch_id = raw.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
            Ok(
                SendResponse::success("sinch", format!("SMS sent (batch: {batch_id})"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("sinch", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
