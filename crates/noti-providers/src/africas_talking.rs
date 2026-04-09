use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Africa's Talking SMS provider.
///
/// API reference: https://developers.africastalking.com/docs/sms/sending
pub struct AfricasTalkingProvider {
    client: Client,
}

impl AfricasTalkingProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for AfricasTalkingProvider {
    fn name(&self) -> &str {
        "africastalking"
    }

    fn url_scheme(&self) -> &str {
        "africastalking"
    }

    fn description(&self) -> &str {
        "Africa's Talking SMS gateway"
    }

    fn example_url(&self) -> &str {
        "africastalking://<username>:<api_key>@<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("username", "Africa's Talking application username"),
            ParamDef::required("api_key", "Africa's Talking API key"),
            ParamDef::required("to", "Recipient phone number in international format"),
            ParamDef::optional("from", "Sender ID (shortcode or alphanumeric)"),
            ParamDef::optional(
                "sandbox",
                "Use sandbox environment (true/false, default: false)",
            ),
            ParamDef::optional("base_url", "API base URL override (default: https://api.africastalking.com)")
                .with_example("https://api.africastalking.com"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let username = config.require("username", "africastalking")?;
        let api_key = config.require("api_key", "africastalking")?;
        let to = config.require("to", "africastalking")?;
        let sandbox = config.get("sandbox").unwrap_or("false") == "true";

        let base_url = if let Some(base) = config.get("base_url") {
            base.trim_end_matches('/').to_string()
        } else if sandbox {
            "https://api.sandbox.africastalking.com/version1/messaging".to_string()
        } else {
            "https://api.africastalking.com/version1/messaging".to_string()
        };

        let mut params: Vec<(&str, &str)> = vec![
            ("username", username),
            ("to", to),
            ("message", message.text.as_str()),
        ];

        if let Some(from) = config.get("from") {
            params.push(("from", from));
        }

        let resp = self
            .client
            .post(base_url)
            .header("apiKey", api_key)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("africas_talking", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("africastalking", "SMS sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(SendResponse::failure(
                "africastalking",
                format!("Africa's Talking API error: {raw}"),
            )
            .with_status_code(status)
            .with_raw_response(raw))
        }
    }
}
