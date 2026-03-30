use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Pushy cross-platform push notification provider.
///
/// Sends push notifications via the Pushy REST API.
/// Reliable cross-platform push notification service.
pub struct PushyProvider {
    client: Client,
}

impl PushyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushyProvider {
    fn name(&self) -> &str {
        "pushy"
    }

    fn url_scheme(&self) -> &str {
        "pushy"
    }

    fn description(&self) -> &str {
        "Pushy cross-platform push notifications"
    }

    fn example_url(&self) -> &str {
        "pushy://<api_key>/<device_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Pushy secret API key").with_example("your-api-key"),
            ParamDef::required("device_token", "Target device token or topic")
                .with_example("device-token-here"),
            ParamDef::optional("sound", "Notification sound file name").with_example("ping.aiff"),
            ParamDef::optional("badge", "Badge count (iOS)").with_example("1"),
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
        let api_key = config.require("api_key", "pushy")?;
        let device_token = config.require("device_token", "pushy")?;

        let url = format!("https://api.pushy.me/push?api_key={api_key}");

        let title = message.title.as_deref().unwrap_or("noti");

        let mut notification = json!({
            "body": message.text,
            "title": title
        });

        if let Some(sound) = config.get("sound") {
            notification["sound"] = json!(sound);
        }
        if let Some(badge) = config.get("badge") {
            if let Ok(b) = badge.parse::<u32>() {
                notification["badge"] = json!(b);
            }
        }

        // Handle image attachments
        if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            notification["image"] = json!(format!("data:{mime};base64,{b64}"));
        }

        let mut data_payload = json!({
            "message": message.text,
            "title": title
        });

        // Include attachment metadata in data payload
        if message.has_attachments() {
            let file_names: Vec<String> = message
                .attachments
                .iter()
                .map(|a| a.effective_file_name())
                .collect();
            data_payload["attachments"] = json!(file_names);
        }

        let payload = json!({
            "to": device_token,
            "data": data_payload,
            "notification": notification
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let success = raw
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if success || (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("pushy", "push notification sent via Pushy")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("pushy", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
