use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Seven (formerly sms77) SMS provider.
///
/// Sends SMS via the Seven.io REST API.
///
/// API reference: <https://docs.seven.io/en/rest-api/endpoints/sms>
pub struct SevenProvider {
    client: Client,
}

impl SevenProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SevenProvider {
    fn name(&self) -> &str {
        "seven"
    }

    fn url_scheme(&self) -> &str {
        "seven"
    }

    fn description(&self) -> &str {
        "Seven (sms77) SMS gateway"
    }

    fn example_url(&self) -> &str {
        "seven://<api_key>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Seven.io API key"),
            ParamDef::required("to", "Recipient phone number (E.164)").with_example("+15559876543"),
            ParamDef::optional("from", "Sender name or phone number").with_example("noti"),
            ParamDef::optional("flash", "Send as flash SMS: 1 or 0 (default: 0)"),
            ParamDef::optional("foreign_id", "Your custom foreign ID for tracking"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "seven")?;
        let to = config.require("to", "seven")?;

        let url = "https://gateway.seven.io/api/sms";

        let text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let mut params = vec![
            ("to", to.to_string()),
            ("text", text),
            ("json", "1".to_string()),
        ];

        if let Some(from) = config.get("from") {
            params.push(("from", from.to_string()));
        }
        if let Some(flash) = config.get("flash") {
            params.push(("flash", flash.to_string()));
        }
        if let Some(foreign_id) = config.get("foreign_id") {
            params.push(("foreign_id", foreign_id.to_string()));
        }

        let resp = self
            .client
            .post(url)
            .header("X-Api-Key", api_key)
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let success_code = raw.get("success").and_then(|v| v.as_str()).unwrap_or("");

        if success_code == "100" || (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("seven", "SMS sent via Seven.io")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("messages")
                .and_then(|m| m.as_array())
                .and_then(|arr| arr.first())
                .and_then(|m| m.get("error_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("seven", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
