use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Nextcloud Talk chat message provider.
///
/// API reference: https://nextcloud-talk.readthedocs.io/en/latest/chat/
pub struct NcTalkProvider {
    client: Client,
}

impl NcTalkProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NcTalkProvider {
    fn name(&self) -> &str {
        "nctalk"
    }

    fn url_scheme(&self) -> &str {
        "nctalk"
    }

    fn description(&self) -> &str {
        "Nextcloud Talk chat messaging via OCS API"
    }

    fn example_url(&self) -> &str {
        "nctalk://<user>:<password>@<host>/<room_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("user", "Nextcloud username"),
            ParamDef::required("password", "Nextcloud password or app token"),
            ParamDef::required("host", "Nextcloud server host (e.g. cloud.example.com)"),
            ParamDef::required("room_token", "Talk room/conversation token"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let user = config.require("user", "nctalk")?;
        let password = config.require("password", "nctalk")?;
        let host = config.require("host", "nctalk")?;
        let room_token = config.require("room_token", "nctalk")?;
        let scheme = config.get("scheme").unwrap_or("https");

        let url = format!("{scheme}://{host}/ocs/v2.php/apps/spreed/api/v1/chat/{room_token}");

        let body = serde_json::json!({ "message": message.text });

        let resp = self
            .client
            .post(&url)
            .basic_auth(user, Some(password))
            .header("OCS-APIRequest", "true")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(SendResponse::success("nctalk", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(
                SendResponse::failure("nctalk", format!("Nextcloud Talk API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
