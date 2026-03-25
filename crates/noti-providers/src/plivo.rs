use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Plivo SMS provider.
///
/// Sends SMS messages via the Plivo REST API.
/// Plivo is a global cloud communication platform for voice and messaging.
///
/// API reference: <https://www.plivo.com/docs/sms/api/message#send-a-message>
pub struct PlivoProvider {
    client: Client,
}

impl PlivoProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PlivoProvider {
    fn name(&self) -> &str {
        "plivo"
    }

    fn url_scheme(&self) -> &str {
        "plivo"
    }

    fn description(&self) -> &str {
        "Plivo SMS via REST API"
    }

    fn example_url(&self) -> &str {
        "plivo://<auth_id>:<auth_token>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("auth_id", "Plivo Auth ID").with_example("MAXXXXXXXXXXXXXXXXXX"),
            ParamDef::required("auth_token", "Plivo Auth Token").with_example("your-auth-token"),
            ParamDef::required("from", "Sender phone number (E.164 format)")
                .with_example("+15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let auth_id = config.require("auth_id", "plivo")?;
        let auth_token = config.require("auth_token", "plivo")?;
        let from = config.require("from", "plivo")?;
        let to = config.require("to", "plivo")?;

        let url = format!("https://api.plivo.com/v1/Account/{auth_id}/Message/");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "src": from,
            "dst": to,
            "text": body_text,
        });

        let resp = self
            .client
            .post(&url)
            .basic_auth(auth_id, Some(auth_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("plivo", "SMS sent via Plivo")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("plivo", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
