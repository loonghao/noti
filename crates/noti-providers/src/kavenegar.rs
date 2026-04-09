use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Kavenegar SMS provider (Iran).
///
/// Sends SMS messages via the Kavenegar REST API.
/// Popular SMS gateway in Iran.
pub struct KavenegarProvider {
    client: Client,
}

impl KavenegarProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for KavenegarProvider {
    fn name(&self) -> &str {
        "kavenegar"
    }

    fn url_scheme(&self) -> &str {
        "kavenegar"
    }

    fn description(&self) -> &str {
        "Kavenegar SMS gateway (Iran)"
    }

    fn example_url(&self) -> &str {
        "kavenegar://<api_key>/<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Kavenegar API key").with_example("your-api-key"),
            ParamDef::required("to", "Recipient phone number").with_example("09121234567"),
            ParamDef::optional("from", "Sender number (line number)").with_example("10004346"),
            ParamDef::optional("base_url", "API base URL override (default: https://api.kavenegar.com)")
                .with_example("https://api.kavenegar.com"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "kavenegar")?;
        let to = config.require("to", "kavenegar")?;

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let base_url = config
            .get("base_url")
            .unwrap_or("https://api.kavenegar.com")
            .trim_end_matches('/');
        let url = format!("{base_url}/v1/{api_key}/sms/send.json");

        let mut params = vec![("receptor", to.to_string()), ("message", body_text)];

        if let Some(from) = config.get("from") {
            params.push(("sender", from.to_string()));
        }

        let resp = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("kavenegar", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let return_status = raw
            .get("return")
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if return_status == 200 {
            Ok(SendResponse::success("kavenegar", "SMS sent via Kavenegar")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("return")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("kavenegar", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
