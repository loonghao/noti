use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Techulus Push notification provider.
///
/// Sends push notifications via the Techulus Push API.
/// Supports image attachments via the `image_url` field (base64 data URI).
pub struct TechulusProvider {
    client: Client,
}

impl TechulusProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for TechulusProvider {
    fn name(&self) -> &str {
        "techulus"
    }

    fn url_scheme(&self) -> &str {
        "push"
    }

    fn description(&self) -> &str {
        "Techulus Push notifications"
    }

    fn example_url(&self) -> &str {
        "push://<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Techulus Push API key").with_example("your-api-key"),
            ParamDef::optional("link", "URL to attach to the notification")
                .with_example("https://example.com"),
            ParamDef::optional("base_url", "Override base URL for API requests"),
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
        let api_key = config.require("api_key", "techulus")?;

        let base_url = config.get("base_url").unwrap_or("https://push.techulus.com");
        let url = format!("{base_url}/api/v1/notify");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "body": body_text
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        if let Some(link) = config.get("link") {
            payload["link"] = json!(link);
        }

        // Attach image as base64 data URI in image_url field
        if message.has_attachments() {
            if let Some(img) = message
                .attachments
                .iter()
                .find(|a| matches!(a.kind, AttachmentKind::Image))
            {
                let data = img.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let mime = img.effective_mime();
                payload["image_url"] = json!(format!("data:{mime};base64,{b64}"));
            }
        }

        let resp = self
            .client
            .post(url)
            .header("x-api-key", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("techulus", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("techulus", "push notification sent via Techulus")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("techulus", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
