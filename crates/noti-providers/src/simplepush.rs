use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// SimplePush notification provider.
///
/// Sends push notifications via SimplePush.io.
/// Supports image, video, and GIF attachments via the `attachments` field
/// in the JSON API using base64 data URIs.
///
/// API reference: <https://simplepush.io/api>
pub struct SimplePushProvider {
    client: Client,
}

impl SimplePushProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SimplePushProvider {
    fn name(&self) -> &str {
        "simplepush"
    }

    fn url_scheme(&self) -> &str {
        "simplepush"
    }

    fn description(&self) -> &str {
        "SimplePush.io push notifications"
    }

    fn example_url(&self) -> &str {
        "simplepush://<key>"
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("key", "SimplePush key").with_example("HuxgBB"),
            ParamDef::optional("event", "Event name for filtering").with_example("alerts"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let key = config.require("key", "simplepush")?;

        let title = message.title.as_deref().unwrap_or("Notification");

        let mut payload = json!({
            "key": key,
            "title": title,
            "msg": message.text,
        });

        if let Some(event) = config.get("event") {
            payload["event"] = json!(event);
        }

        // Add attachments as base64 data URIs in the attachments array
        if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                if let Ok(data) = attachment.read_bytes().await {
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    attachments_json.push(json!(format!("data:{mime};base64,{b64}")));
                }
            }
            if !attachments_json.is_empty() {
                payload["attachments"] = json!(attachments_json);
            }
        }

        let resp = self
            .client
            .post("https://simplepu.sh")
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("simplepush", e))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("simplepush", "push sent successfully")
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        } else {
            Ok(SendResponse::failure(
                "simplepush",
                format!("API error (HTTP {status}): {body_text}"),
            )
            .with_status_code(status)
            .with_raw_response(json!({ "body": body_text })))
        }
    }
}
