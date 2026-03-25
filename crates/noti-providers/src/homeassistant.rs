use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Home Assistant notification provider.
///
/// Sends notifications via the Home Assistant REST API.
/// Supports the `notify` service to push messages to configured
/// notification targets (mobile_app, etc.).
///
/// API docs: <https://developers.home-assistant.io/docs/api/rest/>
pub struct HomeAssistantProvider {
    client: Client,
}

impl HomeAssistantProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for HomeAssistantProvider {
    fn name(&self) -> &str {
        "homeassistant"
    }

    fn url_scheme(&self) -> &str {
        "hassio"
    }

    fn description(&self) -> &str {
        "Home Assistant notifications"
    }

    fn example_url(&self) -> &str {
        "hassio://<access_token>@<host>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "Home Assistant long-lived access token"),
            ParamDef::required(
                "host",
                "Home Assistant host (e.g. homeassistant.local:8123)",
            ),
            ParamDef::optional("scheme", "HTTP scheme (default: http)").with_example("https"),
            ParamDef::optional(
                "target",
                "Notification service name (default: notify.notify)",
            )
            .with_example("notify.mobile_app_phone"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_token = config.require("access_token", "homeassistant")?;
        let host = config.require("host", "homeassistant")?;
        let scheme = config.get("scheme").unwrap_or("http");
        let target = config.get("target").unwrap_or("notify.notify");

        // Convert service name to API path: notify.mobile_app_phone -> notify/mobile_app_phone
        let service_path = target.replace('.', "/");
        let url = format!("{scheme}://{host}/api/services/{service_path}");

        let mut payload = json!({
            "message": message.text,
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or(json!({"error": "failed to parse response"}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("homeassistant", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("homeassistant", format!("API error (HTTP {status}): {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
