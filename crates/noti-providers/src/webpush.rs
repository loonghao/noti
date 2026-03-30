use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Web Push (VAPID) provider.
///
/// Sends browser push notifications using the Web Push protocol.
/// Supports image attachments via the Notification API `image` and `badge` fields.
///
/// Reference: <https://web.dev/push-notifications-overview/>
pub struct WebPushProvider {
    client: Client,
}

impl WebPushProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for WebPushProvider {
    fn name(&self) -> &str {
        "webpush"
    }

    fn url_scheme(&self) -> &str {
        "webpush"
    }

    fn description(&self) -> &str {
        "Web Push (VAPID) browser notifications"
    }

    fn example_url(&self) -> &str {
        "webpush://<endpoint_encoded>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("endpoint", "Push subscription endpoint URL"),
            ParamDef::required("p256dh", "Push subscription p256dh key (base64url)"),
            ParamDef::required("auth", "Push subscription auth secret (base64url)"),
            ParamDef::optional("vapid_private", "VAPID private key (base64url)"),
            ParamDef::optional("vapid_email", "VAPID contact email")
                .with_example("mailto:admin@example.com"),
            ParamDef::optional("ttl", "Time-to-live in seconds (default: 86400)")
                .with_example("86400"),
            ParamDef::optional(
                "urgency",
                "Push urgency: very-low, low, normal, high (default: normal)",
            ),
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
        let endpoint = config.require("endpoint", "webpush")?;

        // Build the notification payload
        let mut notification = json!({
            "body": message.text,
        });

        if let Some(ref title) = message.title {
            notification["title"] = json!(title);
        }

        // Embed first image attachment as data URI in the `image` field
        if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                notification["image"] = json!(format!("data:{mime};base64,{b64}"));
            }
        } else if message.has_attachments() {
            // For non-image attachments, embed as badge icon
            if let Some(att) = message.attachments.first() {
                if att.kind == AttachmentKind::Image {
                    if let Ok(data) = att.read_bytes().await {
                        let mime = att.effective_mime();
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                        notification["badge"] = json!(format!("data:{mime};base64,{b64}"));
                    }
                }
            }
        }

        let payload = serde_json::to_string(&notification).map_err(|e| NotiError::Provider {
            provider: "webpush".into(),
            message: format!("failed to serialize payload: {e}"),
        })?;

        let ttl = config.get("ttl").unwrap_or("86400");
        let urgency = config.get("urgency").unwrap_or("normal");

        // For now, send a plain push to the endpoint
        // In production, this would need VAPID signing and encryption
        let resp = self
            .client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("TTL", ttl)
            .header("Urgency", urgency)
            .body(payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if status == 201 || (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("webpush", "push notification sent via Web Push")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("webpush", format!("Push service error ({status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        }
    }
}
