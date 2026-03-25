use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// BulkVS SMS provider.
///
/// API reference: https://portal.bulkvs.com/api-docs/
pub struct BulkVsProvider {
    client: Client,
}

impl BulkVsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BulkVsProvider {
    fn name(&self) -> &str {
        "bulkvs"
    }

    fn url_scheme(&self) -> &str {
        "bulkvs"
    }

    fn description(&self) -> &str {
        "BulkVS SMS messaging via REST API"
    }

    fn example_url(&self) -> &str {
        "bulkvs://<username>:<password>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("username", "BulkVS account username"),
            ParamDef::required("password", "BulkVS account password"),
            ParamDef::required("from", "Sender phone number (must be a BulkVS number)"),
            ParamDef::required("to", "Recipient phone number"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let username = config.require("username", "bulkvs")?;
        let password = config.require("password", "bulkvs")?;
        let from = config.require("from", "bulkvs")?;
        let to = config.require("to", "bulkvs")?;

        let body = serde_json::json!({
            "AuthenticationCredentials": {
                "Username": username,
                "Password": password
            },
            "Message": {
                "From": from,
                "To": [to],
                "Body": message.text
            }
        });

        let resp = self
            .client
            .post("https://portal.bulkvs.com/api/3.0/message")
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(SendResponse::success("bulkvs", "SMS sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("bulkvs", format!("BulkVS API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
