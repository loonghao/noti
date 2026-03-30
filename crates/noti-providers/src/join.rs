use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Join by joaoapps push notification provider.
///
/// Join lets you push notifications between Android devices,
/// Chrome, and other platforms via a simple API.
///
/// API docs: <https://joaoapps.com/join/api/>
pub struct JoinProvider {
    client: Client,
}

impl JoinProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for JoinProvider {
    fn name(&self) -> &str {
        "join"
    }

    fn url_scheme(&self) -> &str {
        "join"
    }

    fn description(&self) -> &str {
        "Join by joaoapps push notifications"
    }

    fn example_url(&self) -> &str {
        "join://<api_key>/<device_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Join API key"),
            ParamDef::optional(
                "device_id",
                "Target device ID or group (default: group.all)",
            )
            .with_example("group.all"),
            ParamDef::optional("icon", "Notification icon URL"),
            ParamDef::optional("smallicon", "Small notification icon URL"),
            ParamDef::optional("url", "URL to open on the device"),
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
        let api_key = config.require("api_key", "join")?;
        let device_id = config.get("device_id").unwrap_or("group.all");

        let mut params = vec![
            ("apikey", api_key.to_string()),
            ("deviceId", device_id.to_string()),
            ("text", message.text.clone()),
        ];

        if let Some(ref title) = message.title {
            params.push(("title", title.clone()));
        }
        if let Some(icon) = config.get("icon") {
            params.push(("icon", icon.to_string()));
        }
        if let Some(smallicon) = config.get("smallicon") {
            params.push(("smallicon", smallicon.to_string()));
        }
        if let Some(url) = config.get("url") {
            params.push(("url", url.to_string()));
        }

        // Handle image attachments via the image parameter
        if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            params.push(("image", format!("data:{mime};base64,{b64}")));
        }

        let resp = self
            .client
            .get("https://joinjoaomgcd.appspot.com/_ah/api/messaging/v1/sendPush")
            .query(&params)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or(json!({"error": "failed to parse response"}));

        let success_val = raw
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if success_val || (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("join", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("errorMessage")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("join", format!("API error: {error}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
