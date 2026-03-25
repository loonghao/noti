use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Dot. IoT e-ink display notification provider.
///
/// Dot. is an e-ink display device that can receive push notifications
/// via a REST API. It supports text and image modes for displaying
/// content on the device screen.
///
/// API Reference: <https://dot.mindreset.tech>
pub struct DotProvider {
    client: Client,
}

impl DotProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for DotProvider {
    fn name(&self) -> &str {
        "dot"
    }

    fn url_scheme(&self) -> &str {
        "dot"
    }

    fn description(&self) -> &str {
        "Dot. IoT e-ink display notifications"
    }

    fn example_url(&self) -> &str {
        "dot://<token>@<device_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token", "Dot. API token (dot_app_...)").with_example("dot_app_xxx"),
            ParamDef::required("device_id", "Dot. device serial number (12 hex chars)")
                .with_example("aabbccddeeff"),
            ParamDef::optional("signature", "Footer text displayed on device"),
            ParamDef::optional("mode", "Display mode: text (default) or image"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token = config.require("token", "dot")?;
        let device_id = config.require("device_id", "dot")?;

        let mode = config.get("mode").unwrap_or("text");

        let url = format!("https://gateway.getdot.app/api/device/{device_id}/{mode}");

        let mut payload = json!({
            "body": message.text,
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        if let Some(signature) = config.get("signature") {
            payload["signature"] = json!(signature);
        }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"status": status}));

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("dot", "notification sent to Dot. device")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("error")
                .or_else(|| raw.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("dot", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
