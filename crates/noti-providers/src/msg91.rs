use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// MSG91 SMS provider.
///
/// Sends SMS via MSG91 (Indian SMS gateway).
///
/// API reference: <https://docs.msg91.com/reference/send-sms>
pub struct Msg91Provider {
    client: Client,
}

impl Msg91Provider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for Msg91Provider {
    fn name(&self) -> &str {
        "msg91"
    }

    fn url_scheme(&self) -> &str {
        "msg91"
    }

    fn description(&self) -> &str {
        "MSG91 SMS gateway (India)"
    }

    fn example_url(&self) -> &str {
        "msg91://<authkey>/<sender_id>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("authkey", "MSG91 authentication key"),
            ParamDef::required("sender", "Sender ID (6 characters)").with_example("NOTIAP"),
            ParamDef::required("to", "Recipient phone number with country code")
                .with_example("919876543210"),
            ParamDef::optional(
                "route",
                "SMS route: 1=Promotional, 4=Transactional (default: 4)",
            )
            .with_example("4"),
            ParamDef::optional("country", "Default country code (default: 91)").with_example("91"),
            ParamDef::optional("DLT_TE_ID", "DLT template entity ID (required for India)"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let authkey = config.require("authkey", "msg91")?;
        let sender = config.require("sender", "msg91")?;
        let to = config.require("to", "msg91")?;

        let base_url = config
            .get("base_url")
            .unwrap_or("https://control.msg91.com")
            .trim_end_matches('/');
        let url = format!("{base_url}/api/v5/flow/");

        let route = config.get("route").unwrap_or("4");
        let country = config.get("country").unwrap_or("91");

        let text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "sender": sender,
            "route": route,
            "country": country,
            "sms": [
                {
                    "message": text,
                    "to": [to],
                }
            ],
        });

        if let Some(dlt_te_id) = config.get("DLT_TE_ID") {
            payload["DLT_TE_ID"] = json!(dlt_te_id);
        }

        let resp = self
            .client
            .post(url)
            .header("authkey", authkey)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let msg_type = raw.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if msg_type == "success" || (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("msg91", "SMS sent via MSG91")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("msg91", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
