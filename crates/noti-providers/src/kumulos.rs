use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Kumulos push notification provider.
///
/// Uses the Kumulos Push API to send push notifications.
/// Supports rich media push with image attachments.
/// API docs: https://docs.kumulos.com/messaging/api/
pub struct KumulosProvider {
    client: Client,
}

impl KumulosProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for KumulosProvider {
    fn name(&self) -> &str {
        "kumulos"
    }

    fn url_scheme(&self) -> &str {
        "kumulos"
    }

    fn description(&self) -> &str {
        "Kumulos push notifications via Push API"
    }

    fn example_url(&self) -> &str {
        "kumulos://<api_key>:<server_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Kumulos application API key"),
            ParamDef::required("server_key", "Kumulos server key"),
            ParamDef::optional(
                "channel",
                "Broadcast channel ID (default sends to all users)",
            ),
            ParamDef::optional("base_url", "Override base URL for the Kumulos API")
                .with_example("https://messages.kumulos.com"),
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
        let api_key = config.require("api_key", "kumulos")?;
        let server_key = config.require("server_key", "kumulos")?;

        let title = message.title.as_deref().unwrap_or("noti");

        let mut broadcast = json!({
            "title": title,
            "message": message.text,
        });

        if let Some(channel) = config.get("channel") {
            broadcast["broadcast"] = json!(false);
            broadcast["channel"] = json!(channel);
        } else {
            broadcast["broadcast"] = json!(true);
        }

        // Embed first image attachment as rich push media
        if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let data_uri = format!("data:{mime};base64,{b64}");
                broadcast["data"] = json!({ "picture": data_uri });
            }
        }

        let default_base = "https://messages.kumulos.com";
        let base = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| default_base.to_string());

        let url = format!("{base}/v2/app-api-keys/{api_key}/messages");

        let resp = self
            .client
            .post(&url)
            .basic_auth(api_key, Some(server_key))
            .json(&broadcast)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("kumulos", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("kumulos", "push notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("kumulos", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
