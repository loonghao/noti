use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// NotificationAPI multi-channel notification provider.
///
/// NotificationAPI provides a unified API to trigger email, SMS, phone,
/// push, and in-app notifications through a single endpoint. It supports
/// multiple regions (US, CA, EU) and template-based notifications.
///
/// API Reference: <https://www.notificationapi.com>
pub struct NotificationApiProvider {
    client: Client,
}

impl NotificationApiProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NotificationApiProvider {
    fn name(&self) -> &str {
        "notificationapi"
    }

    fn url_scheme(&self) -> &str {
        "napi"
    }

    fn description(&self) -> &str {
        "NotificationAPI multi-channel notifications (email, SMS, push, in-app)"
    }

    fn example_url(&self) -> &str {
        "napi://<client_id>/<client_secret>/<user_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("client_id", "NotificationAPI client ID")
                .with_example("your_client_id"),
            ParamDef::required("client_secret", "NotificationAPI client secret")
                .with_example("your_client_secret"),
            ParamDef::required("user_id", "Target user identifier")
                .with_example("user@example.com"),
            ParamDef::optional(
                "notification_type",
                "Notification type ID (default: apprise)",
            )
            .with_example("order_tracking"),
            ParamDef::optional("region", "API region: us (default), ca, or eu"),
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
        let client_id = config.require("client_id", "notificationapi")?;
        let client_secret = config.require("client_secret", "notificationapi")?;
        let user_id = config.require("user_id", "notificationapi")?;

        let notification_type = config.get("notification_type").unwrap_or("apprise");
        let region = config.get("region").unwrap_or("us");

        let base_url = match region {
            "ca" => "https://api.ca.notificationapi.com",
            "eu" => "https://api.eu.notificationapi.com",
            _ => "https://api.notificationapi.com",
        };

        let url = format!("{base_url}/{client_id}/sender");

        // Build Basic auth from client_id:client_secret
        let auth = base64::engine::general_purpose::STANDARD
            .encode(format!("{client_id}:{client_secret}"));

        let title = message
            .title
            .clone()
            .unwrap_or_else(|| "Notification".to_string());

        let mut merge_tags = json!({
            "appTitle": title,
            "appBody": message.text,
        });

        // Add image attachment as imageUrl merge tag
        if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            merge_tags["imageUrl"] = json!(format!("data:{mime};base64,{b64}"));
        }

        // Add non-image attachment info to body
        let non_images: Vec<_> = message
            .attachments
            .iter()
            .filter(|a| a.kind != AttachmentKind::Image)
            .collect();
        if !non_images.is_empty() {
            let mut body = message.text.clone();
            for att in &non_images {
                body.push_str(&format!("\n📎 Attachment: {}", att.effective_file_name()));
            }
            merge_tags["appBody"] = json!(body);
        }

        let payload = json!({
            "notificationId": notification_type,
            "user": {
                "id": user_id,
            },
            "mergeTags": merge_tags
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Basic {auth}"))
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
            Ok(SendResponse::success(
                "notificationapi",
                "multi-channel notification sent successfully",
            )
            .with_status_code(status)
            .with_raw_response(raw))
        } else {
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("notificationapi", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
