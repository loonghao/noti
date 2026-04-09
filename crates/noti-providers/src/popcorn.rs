use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// PopcornNotify SMS provider.
///
/// Sends SMS messages via the PopcornNotify API.
///
/// API reference: <https://popcornnotify.com/api>
pub struct PopcornProvider {
    client: Client,
}

impl PopcornProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PopcornProvider {
    fn name(&self) -> &str {
        "popcorn"
    }

    fn url_scheme(&self) -> &str {
        "popcorn"
    }

    fn description(&self) -> &str {
        "PopcornNotify SMS messaging"
    }

    fn example_url(&self) -> &str {
        "popcorn://<api_key>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "PopcornNotify API key").with_example("your-api-key"),
            ParamDef::required("from", "Sender phone number").with_example("+15551234567"),
            ParamDef::required("to", "Recipient phone number").with_example("+15559876543"),
            ParamDef::optional("base_url", "Override base URL for API requests"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "popcorn")?;
        let from = config.require("from", "popcorn")?;
        let to = config.require("to", "popcorn")?;

        let base_url = config.get("base_url").unwrap_or("https://muncher.popcornnotify.com");
        let url = format!("{base_url}/send/message");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "ApiKey": api_key,
            "From": from,
            "To": to,
            "Text": body_text,
        });

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("popcorn", "SMS sent via PopcornNotify")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("Message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("popcorn", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
