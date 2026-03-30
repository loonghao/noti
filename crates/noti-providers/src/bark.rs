use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Bark iOS push notification provider.
pub struct BarkProvider {
    client: Client,
}

impl BarkProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for BarkProvider {
    fn name(&self) -> &str {
        "bark"
    }

    fn url_scheme(&self) -> &str {
        "bark"
    }

    fn description(&self) -> &str {
        "Bark iOS push notifications"
    }

    fn example_url(&self) -> &str {
        "bark://<device_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("device_key", "Bark device key").with_example("xxxxxx"),
            ParamDef::optional("server", "Bark server URL (default: https://api.day.app)")
                .with_example("https://api.day.app"),
            ParamDef::optional("group", "Notification group name"),
            ParamDef::optional("sound", "Notification sound").with_example("alarm"),
            ParamDef::optional("icon", "Notification icon URL"),
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
        let device_key = config.require("device_key", "bark")?;
        let server = config.get("server").unwrap_or("https://api.day.app");

        let url = format!("{}/{}", server.trim_end_matches('/'), device_key);

        let mut payload = json!({
            "body": message.text,
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        if let Some(group) = config.get("group") {
            payload["group"] = json!(group);
        }
        if let Some(sound) = config.get("sound") {
            payload["sound"] = json!(sound);
        }

        // Handle icon from config or image attachments
        if let Some(icon) = config.get("icon") {
            payload["icon"] = json!(icon);
        } else if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            payload["icon"] = json!(format!("data:{mime};base64,{b64}"));
        }

        // Include attachment info in body text for non-image attachments
        if message.has_attachments() {
            let non_images: Vec<_> = message
                .attachments
                .iter()
                .filter(|a| a.kind != AttachmentKind::Image)
                .collect();
            if !non_images.is_empty() {
                let mut body = message.text.clone();
                for att in &non_images {
                    body.push_str(&format!("\n📎 {}", att.effective_file_name()));
                }
                payload["body"] = json!(body);
            }
        }

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

        let code = raw.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code == 200 {
            Ok(SendResponse::success("bark", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("bark", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
