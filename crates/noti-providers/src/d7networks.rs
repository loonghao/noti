use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// D7 Networks SMS provider.
///
/// Sends SMS messages via the D7 Networks REST API.
/// Requires an API token/key.
pub struct D7NetworksProvider {
    client: Client,
}

impl D7NetworksProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for D7NetworksProvider {
    fn name(&self) -> &str {
        "d7sms"
    }

    fn url_scheme(&self) -> &str {
        "d7sms"
    }

    fn description(&self) -> &str {
        "D7 Networks SMS gateway"
    }

    fn example_url(&self) -> &str {
        "d7sms://<api_token>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_token", "D7 Networks API token").with_example("your-api-token"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
            ParamDef::optional("from", "Sender ID / phone number").with_example("SMSINFO"),
            ParamDef::optional("channel", "Channel: sms, whatsapp, viber (default: sms)")
                .with_example("sms"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_token = config.require("api_token", "d7sms")?;
        let to = config.require("to", "d7sms")?;
        let from = config.get("from").unwrap_or("SMSINFO");
        let channel = config.get("channel").unwrap_or("sms");

        let url = "https://api.d7networks.com/messages/v1/send";

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "messages": [{
                "channel": channel,
                "recipients": [to],
                "content": body_text,
                "msg_type": "text",
                "data_coding": "text"
            }],
            "message_globals": {
                "originator": from,
                "report_url": ""
            }
        });

        let resp = self
            .client
            .post(url)
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
            Ok(SendResponse::success("d7sms", "SMS sent via D7 Networks")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("detail")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("d7sms", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
