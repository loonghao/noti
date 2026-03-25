use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Twilio SMS provider.
///
/// Sends SMS messages via the Twilio REST API.
/// Requires account SID, auth token, sender phone number, and recipient phone number.
pub struct TwilioProvider {
    client: Client,
}

impl TwilioProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for TwilioProvider {
    fn name(&self) -> &str {
        "twilio"
    }

    fn url_scheme(&self) -> &str {
        "twilio"
    }

    fn description(&self) -> &str {
        "Twilio SMS via REST API"
    }

    fn example_url(&self) -> &str {
        "twilio://<account_sid>:<auth_token>@<from_number>/<to_number>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("account_sid", "Twilio account SID")
                .with_example("ACxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
            ParamDef::required("auth_token", "Twilio auth token").with_example("your_auth_token"),
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
        let account_sid = config.require("account_sid", "twilio")?;
        let auth_token = config.require("auth_token", "twilio")?;
        let from = config.require("from", "twilio")?;
        let to = config.require("to", "twilio")?;

        let url = format!("https://api.twilio.com/2010-04-01/Accounts/{account_sid}/Messages.json");

        // Twilio uses form-encoded body
        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let params = [("From", from), ("To", to), ("Body", &body_text)];

        let resp = self
            .client
            .post(&url)
            .basic_auth(account_sid, Some(auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            let sid = raw.get("sid").and_then(|v| v.as_str()).unwrap_or("unknown");
            Ok(
                SendResponse::success("twilio", format!("SMS sent (SID: {sid})"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error_msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("twilio", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
