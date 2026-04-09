use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Emby media server notification provider.
///
/// Uses the Emby REST API to send notifications to users.
/// Supports image attachments via the `Url` field in the notification payload,
/// using base64 data URIs for inline image display.
///
/// API docs: https://github.com/MediaBrowser/Emby/wiki/Emby-Server-API
pub struct EmbyProvider {
    client: Client,
}

impl EmbyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for EmbyProvider {
    fn name(&self) -> &str {
        "emby"
    }

    fn url_scheme(&self) -> &str {
        "emby"
    }

    fn description(&self) -> &str {
        "Emby media server notifications via REST API"
    }

    fn example_url(&self) -> &str {
        "emby://<api_key>@<host>/<user_id>"
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Emby API key"),
            ParamDef::required("host", "Emby server hostname or IP (e.g. localhost:8096)")
                .with_example("localhost:8096"),
            ParamDef::optional("user_id", "Target user ID for the notification"),
            ParamDef::optional("scheme", "URL scheme: http or https (default: http)")
                .with_example("http"),
            ParamDef::optional("base_url", "Override base URL for the Emby server")
                .with_example("http://localhost:8096"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "emby")?;
        let host = config.require("host", "emby")?;

        let scheme = config.get("scheme").unwrap_or("http");
        let default_base = format!("{scheme}://{host}");
        let base_url = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or(default_base);

        let name = message.title.as_deref().unwrap_or("noti");

        // Build description with attachment info
        let mut description = message.text.clone();
        if message.has_attachments() {
            let non_images: Vec<_> = message
                .attachments
                .iter()
                .filter(|a| a.kind != AttachmentKind::Image)
                .collect();
            for att in &non_images {
                description.push_str(&format!("\n📎 {}", att.effective_file_name()));
            }
        }

        let mut payload = json!({
            "Name": name,
            "Description": description,
            "Category": "Plugin",
            "NotificationType": "TaskCompleted"
        });

        // Embed first image as a data URI in the Url field
        if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                payload["Url"] = json!(format!("data:{mime};base64,{b64}"));
            }
        }

        let mut url = format!("{base_url}/Notifications/Admin");

        // If user_id is specified, send to specific user
        if let Some(user_id) = config.get("user_id") {
            url = format!("{base_url}/Notifications/{user_id}");
        }

        let resp = self
            .client
            .post(&url)
            .header("X-Emby-Token", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("emby", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("emby", "notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(SendResponse::failure("emby", format!("API error: {body}"))
                .with_status_code(status)
                .with_raw_response(json!({ "body": body })))
        }
    }
}
