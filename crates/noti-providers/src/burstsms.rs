use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// BurstSMS provider.
///
/// Sends SMS messages via the BurstSMS (Transmit SMS) REST API.
/// BurstSMS is an Australian-based SMS gateway service.
///
/// API reference: <https://burstsms.com/api>
pub struct BurstSmsProvider {
    client: Client,
}

impl BurstSmsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BurstSmsProvider {
    fn name(&self) -> &str {
        "burstsms"
    }

    fn url_scheme(&self) -> &str {
        "burstsms"
    }

    fn description(&self) -> &str {
        "BurstSMS (Transmit SMS) gateway via REST API"
    }

    fn example_url(&self) -> &str {
        "burstsms://<api_key>:<api_secret>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "BurstSMS API key").with_example("your-api-key"),
            ParamDef::required("api_secret", "BurstSMS API secret").with_example("your-api-secret"),
            ParamDef::required("from", "Sender caller ID or phone number").with_example("MyApp"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+61412345678"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "burstsms")?;
        let api_secret = config.require("api_secret", "burstsms")?;
        let from = config.require("from", "burstsms")?;
        let to = config.require("to", "burstsms")?;

        let url = "https://api.transmitsms.com/send-sms.json";

        let body_text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let params = [("message", body_text.as_str()), ("to", to), ("from", from)];

        let resp = self
            .client
            .post(url)
            .basic_auth(api_key, Some(api_secret))
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            let error_code = raw.get("error").and_then(|v| v.get("code"));
            if error_code.is_some() && error_code != Some(&json!("SUCCESS")) {
                let msg = raw
                    .get("error")
                    .and_then(|v| v.get("description"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                return Ok(
                    SendResponse::failure("burstsms", format!("API error: {msg}"))
                        .with_status_code(status)
                        .with_raw_response(raw),
                );
            }
            Ok(SendResponse::success("burstsms", "SMS sent via BurstSMS")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("burstsms", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
