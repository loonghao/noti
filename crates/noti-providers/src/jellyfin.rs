use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Jellyfin media server notification provider.
///
/// Uses the Jellyfin REST API to send notifications to users.
/// API docs: https://api.jellyfin.org/
pub struct JellyfinProvider {
    client: Client,
}

impl JellyfinProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for JellyfinProvider {
    fn name(&self) -> &str {
        "jellyfin"
    }

    fn url_scheme(&self) -> &str {
        "jellyfin"
    }

    fn description(&self) -> &str {
        "Jellyfin media server notifications via REST API"
    }

    fn example_url(&self) -> &str {
        "jellyfin://<api_key>@<host>/<user_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Jellyfin API key"),
            ParamDef::required(
                "host",
                "Jellyfin server hostname or IP (e.g. localhost:8096)",
            )
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
        let api_key = config.require("api_key", "jellyfin")?;
        let host = config.require("host", "jellyfin")?;

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
            .header("Authorization", format!("MediaBrowser Token=\"{api_key}\""))
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
                SendResponse::success("jellyfin", "notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("jellyfin", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
