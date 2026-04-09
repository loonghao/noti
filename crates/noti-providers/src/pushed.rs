use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Pushed.co push notification provider.
///
/// Uses the Pushed.co REST API to send push notifications.
/// Supports image attachments via content_extra with base64 data URI.
/// API docs: https://about.pushed.co/docs/api
pub struct PushedProvider {
    client: Client,
}

impl PushedProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushedProvider {
    fn name(&self) -> &str {
        "pushed"
    }

    fn url_scheme(&self) -> &str {
        "pushed"
    }

    fn description(&self) -> &str {
        "Pushed.co push notifications via REST API"
    }

    fn example_url(&self) -> &str {
        "pushed://<app_key>:<app_secret>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("app_key", "Pushed application key"),
            ParamDef::required("app_secret", "Pushed application secret"),
            ParamDef::optional(
                "target_type",
                "Target type: app, channel, or pushed_id (default: app)",
            ),
            ParamDef::optional(
                "target_alias",
                "Channel alias or pushed_id (when target_type is not app)",
            ),
            ParamDef::optional("base_url", "Pushed API base URL (default: https://api.pushed.co)")
                .with_example("https://api.pushed.co"),
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
        let app_key = config.require("app_key", "pushed")?;
        let app_secret = config.require("app_secret", "pushed")?;

        let target_type = config.get("target_type").unwrap_or("app");

        let mut payload = json!({
            "app_key": app_key,
            "app_secret": app_secret,
            "target_type": target_type,
            "content": message.text,
        });

        if let Some(alias) = config.get("target_alias") {
            payload["target_alias"] = json!(alias);
        }

        // Embed first image attachment as base64 data URI in content_extra
        if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                payload["content_extra"] = json!(format!("data:{mime};base64,{b64}"));
                payload["content_type"] = json!("url");
            }
        }

        let base = config.get("base_url").unwrap_or("https://api.pushed.co");
        let api_url = format!("{}/1/push", base.trim_end_matches('/'));

        let resp = self
            .client
            .post(&api_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("pushed", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("pushed", "push notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("pushed", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
