use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Parse Platform push notification provider.
///
/// Uses the Parse REST API to send push notifications.
/// Supports image attachments via `image` field in push data.
/// API docs: https://docs.parseplatform.org/rest/guide/#push-notifications
pub struct ParseProvider {
    client: Client,
}

impl ParseProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ParseProvider {
    fn name(&self) -> &str {
        "parse"
    }

    fn url_scheme(&self) -> &str {
        "parse"
    }

    fn description(&self) -> &str {
        "Parse Platform push notifications via REST API"
    }

    fn example_url(&self) -> &str {
        "parse://<app_id>:<rest_api_key>@<host>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("app_id", "Parse Application ID"),
            ParamDef::required("rest_api_key", "Parse REST API key"),
            ParamDef::optional("host", "Parse Server host (default: api.parse.com)")
                .with_example("api.parse.com"),
            ParamDef::optional("channel", "Push channel name (default: broadcasts to all)"),
            ParamDef::optional("base_url", "Override base URL for API requests (takes precedence over host)"),
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
        let app_id = config.require("app_id", "parse")?;
        let rest_api_key = config.require("rest_api_key", "parse")?;

        let host = config.get("host").unwrap_or("api.parse.com");

        let url = if let Some(base_url) = config.get("base_url") {
            format!("{base_url}/1/push")
        } else {
            format!("https://{host}/1/push")
        };

        let title = message.title.as_deref().unwrap_or("noti");

        let where_clause = if let Some(channel) = config.get("channel") {
            json!({ "channels": channel })
        } else {
            json!({})
        };

        let mut data = json!({
            "alert": message.text,
            "title": title,
        });

        // Embed first image attachment as base64 data URI in push data
        if let Some(img) = message.first_image() {
            if let Ok(bytes) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                data["image"] = json!(format!("data:{mime};base64,{b64}"));
            }
        }

        let payload = json!({
            "where": where_clause,
            "data": data
        });



        let resp = self
            .client
            .post(&url)
            .header("X-Parse-Application-Id", app_id)
            .header("X-Parse-REST-API-Key", rest_api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("parse", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("parse", "push notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(SendResponse::failure("parse", format!("API error: {body}"))
                .with_status_code(status)
                .with_raw_response(json!({ "body": body })))
        }
    }
}
