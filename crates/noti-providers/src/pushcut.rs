use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Pushcut notification provider.
///
/// Pushcut is an iOS app for automation notifications.
/// API docs: https://www.pushcut.io/support_notifications.html
pub struct PushcutProvider {
    client: Client,
}

impl PushcutProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushcutProvider {
    fn name(&self) -> &str {
        "pushcut"
    }

    fn url_scheme(&self) -> &str {
        "pushcut"
    }

    fn description(&self) -> &str {
        "Pushcut iOS automation notifications"
    }

    fn example_url(&self) -> &str {
        "pushcut://<api_key>/<notification_name>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Pushcut API key (from Pushcut Account settings)")
                .with_example("pk_abc123"),
            ParamDef::required(
                "notification_name",
                "Name of the Pushcut notification to trigger",
            )
            .with_example("My Notification"),
            ParamDef::optional("url", "URL to open when the notification is tapped"),
            ParamDef::optional("image", "URL of an image to attach to the notification"),
            ParamDef::optional("sound", "Custom notification sound name"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        false
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "pushcut")?;
        let notification_name = config.require("notification_name", "pushcut")?;

        let title = message.title.as_deref().unwrap_or("noti");

        let mut payload = json!({
            "title": title,
            "text": message.text,
        });

        if let Some(url) = config.get("url") {
            payload["url"] = json!(url);
        }

        // Handle image from config or attachments
        if let Some(image) = config.get("image") {
            payload["image"] = json!(image);
        } else if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            payload["image"] = json!(format!("data:{mime};base64,{b64}"));
        }

        if let Some(sound) = config.get("sound") {
            payload["sound"] = json!(sound);
        }

        // URL-encode the notification name for the API path
        let encoded_name = notification_name.replace(' ', "%20");
        let url = format!("https://api.pushcut.io/v1/notifications/{encoded_name}");

        let resp = self
            .client
            .post(&url)
            .header("API-Key", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("pushcut", "notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("pushcut", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
