use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// SFR SMS notification provider.
///
/// Sends free SMS via SFR's API (French carrier).
/// Similar to Free Mobile, this is a French carrier-specific SMS service.
pub struct SfrProvider {
    client: Client,
}

impl SfrProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SfrProvider {
    fn name(&self) -> &str {
        "sfr"
    }

    fn url_scheme(&self) -> &str {
        "sfr"
    }

    fn description(&self) -> &str {
        "SFR free SMS notification (French carrier)"
    }

    fn example_url(&self) -> &str {
        "sfr://<phone>:<password>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("phone", "SFR phone number (10 digits)").with_example("0612345678"),
            ParamDef::required("password", "SFR account password or API key"),
            ParamDef::optional("base_url", "Override base URL for API requests"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let phone = config.require("phone", "sfr")?;
        let password = config.require("password", "sfr")?;

        let base_url = config.get("base_url").unwrap_or("https://www.sfr.fr");
        let url = format!("{base_url}/xmscomposer/mc/envoyer-texto-texto/");

        let resp = self
            .client
            .post(url)
            .form(&[
                ("login", phone),
                ("pwd", password),
                ("msg", message.text.as_str()),
            ])
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("sfr", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(SendResponse::success("sfr", "SMS sent successfully").with_status_code(status))
        } else {
            Ok(SendResponse::failure("sfr", format!("API error: {body}"))
                .with_status_code(status)
                .with_raw_response(json!({ "body": body })))
        }
    }
}
