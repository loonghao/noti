use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// 46elks SMS provider.
///
/// API reference: https://46elks.com/docs/send-sms
pub struct FortySixElksProvider {
    client: Client,
}

impl FortySixElksProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FortySixElksProvider {
    fn name(&self) -> &str {
        "46elks"
    }

    fn url_scheme(&self) -> &str {
        "46elks"
    }

    fn description(&self) -> &str {
        "46elks SMS messaging via REST API"
    }

    fn example_url(&self) -> &str {
        "46elks://<api_username>:<api_password>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_username", "46elks API username"),
            ParamDef::required("api_password", "46elks API password"),
            ParamDef::required("from", "Sender phone number or alphanumeric sender ID"),
            ParamDef::required("to", "Recipient phone number in E.164 format"),
            ParamDef::optional("flash", "Send as flash SMS (yes/no, default: no)"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let api_username = config.require("api_username", "46elks")?;
        let api_password = config.require("api_password", "46elks")?;
        let from = config.require("from", "46elks")?;
        let to = config.require("to", "46elks")?;

        let mut params = vec![
            ("from", from),
            ("to", to),
            ("message", message.text.as_str()),
        ];

        if let Some(flash) = config.get("flash") {
            params.push(("flashsms", flash));
        }

        let resp = self
            .client
            .post("https://api.46elks.com/a1/sms")
            .basic_auth(api_username, Some(api_password))
            .form(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(SendResponse::success("46elks", "SMS sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("46elks", format!("46elks API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
