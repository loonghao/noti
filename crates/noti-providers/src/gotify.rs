use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Gotify self-hosted push notification provider.
pub struct GotifyProvider {
    client: Client,
}

impl GotifyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for GotifyProvider {
    fn name(&self) -> &str {
        "gotify"
    }

    fn url_scheme(&self) -> &str {
        "gotify"
    }

    fn description(&self) -> &str {
        "Gotify self-hosted push notifications"
    }

    fn example_url(&self) -> &str {
        "gotify://<host>/<app_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Gotify server URL")
                .with_example("https://gotify.example.com"),
            ParamDef::required("app_token", "Gotify application token").with_example("AKbfx..."),
            ParamDef::optional("priority", "Message priority (default: 5)").with_example("8"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let host = config.require("host", "gotify")?;
        let app_token = config.require("app_token", "gotify")?;

        let url = format!("{}/message", host.trim_end_matches('/'));

        let priority: i32 = config.get("priority").unwrap_or("5").parse().unwrap_or(5);

        let content_type = match message.format {
            MessageFormat::Markdown => "text/markdown",
            MessageFormat::Html => "text/html",
            MessageFormat::Text => "text/plain",
        };

        let mut payload = json!({
            "message": message.text,
            "priority": priority,
            "extras": {
                "client::display": {
                    "contentType": content_type
                }
            }
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        let resp = self
            .client
            .post(&url)
            .header("X-Gotify-Key", app_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("gotify", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("gotify", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
