use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Firebase Cloud Messaging (FCM) provider.
///
/// Sends push notifications via Google's FCM Legacy HTTP API.
///
/// API reference: <https://firebase.google.com/docs/cloud-messaging/send-message>
pub struct FcmProvider {
    client: Client,
}

impl FcmProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FcmProvider {
    fn name(&self) -> &str {
        "fcm"
    }

    fn url_scheme(&self) -> &str {
        "fcm"
    }

    fn description(&self) -> &str {
        "Firebase Cloud Messaging push notifications"
    }

    fn example_url(&self) -> &str {
        "fcm://<server_key>/<device_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("server_key", "FCM server key (legacy API key)")
                .with_example("AAAA..."),
            ParamDef::required("device_token", "Target device registration token")
                .with_example("dQw4..."),
            ParamDef::optional("topic", "FCM topic name (alternative to device_token)")
                .with_example("news"),
            ParamDef::optional("condition", "FCM condition expression for topic targeting"),
            ParamDef::optional(
                "priority",
                "Message priority: high or normal (default: high)",
            )
            .with_example("high"),
            ParamDef::optional("collapse_key", "Collapse key for message grouping"),
            ParamDef::optional("ttl", "Time-to-live in seconds (default: 2419200)")
                .with_example("3600"),
            ParamDef::optional("icon", "Notification icon name"),
            ParamDef::optional("sound", "Notification sound (default: default)")
                .with_example("default"),
            ParamDef::optional("click_action", "Action on notification click"),
            ParamDef::optional("image", "Image URL for rich notification"),
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
        let server_key = config.require("server_key", "fcm")?;

        let url = "https://fcm.googleapis.com/fcm/send";

        let mut notification = json!({
            "body": message.text,
        });

        if let Some(ref title) = message.title {
            notification["title"] = json!(title);
        }
        if let Some(icon) = config.get("icon") {
            notification["icon"] = json!(icon);
        }
        if let Some(sound) = config.get("sound") {
            notification["sound"] = json!(sound);
        } else {
            notification["sound"] = json!("default");
        }
        if let Some(click_action) = config.get("click_action") {
            notification["click_action"] = json!(click_action);
        }
        if let Some(image) = config.get("image") {
            notification["image"] = json!(image);
        }

        // If there's an image attachment, use it as the notification image
        if config.get("image").is_none() {
            if let Some(img) = message
                .attachments
                .iter()
                .find(|a| a.kind == AttachmentKind::Image)
            {
                let data = img.read_bytes().await?;
                let mime_str = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                notification["image"] = json!(format!("data:{mime_str};base64,{b64}"));
            }
        }

        // Add non-image file info as data payload for client-side handling
        let file_attachments: Vec<_> = message
            .attachments
            .iter()
            .filter(|a| a.kind != AttachmentKind::Image)
            .collect();
        let has_file_attachments = !file_attachments.is_empty();

        let mut payload = json!({
            "notification": notification,
        });

        // Target: device_token, topic, or condition
        if let Some(topic) = config.get("topic") {
            payload["to"] = json!(format!("/topics/{topic}"));
        } else if let Some(condition) = config.get("condition") {
            payload["condition"] = json!(condition);
        } else {
            let device_token = config.require("device_token", "fcm")?;
            payload["to"] = json!(device_token);
        }

        if let Some(priority) = config.get("priority") {
            payload["priority"] = json!(priority);
        } else {
            payload["priority"] = json!("high");
        }
        if let Some(collapse_key) = config.get("collapse_key") {
            payload["collapse_key"] = json!(collapse_key);
        }
        if let Some(ttl) = config.get("ttl") {
            payload["time_to_live"] = json!(ttl.parse::<u64>().unwrap_or(2419200));
        }

        // Add file attachment info as data payload for client-side handling
        if has_file_attachments {
            let file_names: Vec<String> = file_attachments
                .iter()
                .map(|a| a.effective_file_name())
                .collect();
            payload["data"] = json!({
                "attachment_count": file_names.len(),
                "attachment_names": file_names.join(","),
            });
        }

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("key={server_key}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let success_count = raw.get("success").and_then(|v| v.as_i64()).unwrap_or(0);
        if success_count > 0 {
            Ok(
                SendResponse::success("fcm", "push notification sent via FCM")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("results")
                .and_then(|r| r.as_array())
                .and_then(|arr| arr.first())
                .and_then(|r| r.get("error"))
                .and_then(|e| e.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("fcm", format!("FCM error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
