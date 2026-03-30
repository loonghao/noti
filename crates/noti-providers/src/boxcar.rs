use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Boxcar iOS/Android push notification provider.
///
/// Boxcar is a push notification service that supports iOS and Android devices.
/// It provides a simple REST API for sending push notifications.
/// Supports image attachments via `icon_url` with base64 data URI.
///
/// API Reference: <https://boxcar.io/developer>
pub struct BoxcarProvider {
    client: Client,
}

impl BoxcarProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BoxcarProvider {
    fn name(&self) -> &str {
        "boxcar"
    }

    fn url_scheme(&self) -> &str {
        "boxcar"
    }

    fn description(&self) -> &str {
        "Boxcar push notifications for iOS/Android"
    }

    fn example_url(&self) -> &str {
        "boxcar://<access_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "Boxcar user access token")
                .with_example("your_access_token"),
            ParamDef::optional("sound", "Notification sound name").with_example("bird-1"),
            ParamDef::optional("source_name", "Source name for the notification")
                .with_example("noti"),
            ParamDef::optional("icon_url", "URL of notification icon"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_token = config.require("access_token", "boxcar")?;

        let url = "https://new.boxcar.io/api/notifications";

        let mut payload = json!({
            "user_credentials": access_token,
            "notification": {
                "alert": message.text,
            }
        });

        if let Some(ref title) = message.title {
            payload["notification"]["title"] = json!(title);
        }

        if let Some(sound) = config.get("sound") {
            payload["notification"]["sound"] = json!(sound);
        }

        if let Some(source) = config.get("source_name") {
            payload["notification"]["source_name"] = json!(source);
        } else {
            payload["notification"]["source_name"] = json!("noti");
        }

        // Use explicit icon_url or embed first image attachment as data URI
        if let Some(icon) = config.get("icon_url") {
            payload["notification"]["icon_url"] = json!(icon);
        } else if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                payload["notification"]["icon_url"] = json!(format!("data:{mime};base64,{b64}"));
            }
        }

        let resp = self
            .client
            .post(url)
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
                SendResponse::success("boxcar", "push notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("boxcar", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
