use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Emby media server notification provider.
///
/// Uses the Emby REST API to send notifications to users.
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

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Emby API key"),
            ParamDef::required("host", "Emby server hostname or IP (e.g. localhost:8096)")
                .with_example("localhost:8096"),
            ParamDef::optional("user_id", "Target user ID for the notification"),
            ParamDef::optional("scheme", "URL scheme: http or https (default: http)")
                .with_example("http"),
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
        let base_url = format!("{scheme}://{host}");

        let name = message.title.as_deref().unwrap_or("noti");
        let description = &message.text;

        let payload = json!({
            "Name": name,
            "Description": description,
            "Category": "Plugin",
            "NotificationType": "TaskCompleted"
        });

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
            .map_err(|e| NotiError::Network(e.to_string()))?;

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
