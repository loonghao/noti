use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Apprise API provider.
///
/// Forwards notifications through a self-hosted Apprise API server,
/// which supports 100+ services natively.
///
/// API reference: <https://github.com/caronc/apprise-api>
pub struct AppriseProvider {
    client: Client,
}

impl AppriseProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for AppriseProvider {
    fn name(&self) -> &str {
        "apprise"
    }

    fn url_scheme(&self) -> &str {
        "apprise"
    }

    fn description(&self) -> &str {
        "Apprise API notification relay"
    }

    fn example_url(&self) -> &str {
        "apprise://<host>/<config_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Apprise API server URL")
                .with_example("http://localhost:8000"),
            ParamDef::optional("config_key", "Apprise persistent storage configuration key")
                .with_example("my-config"),
            ParamDef::optional("urls", "Apprise notification URLs (comma-separated)")
                .with_example("slack://token_a/token_b/token_c"),
            ParamDef::optional(
                "notification_type",
                "Notification type: info, success, warning, failure (default: info)",
            )
            .with_example("info"),
            ParamDef::optional("tag", "Filter tag for persistent config notifications"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let host = config.require("host", "apprise")?;

        let host = host.trim_end_matches('/');

        let notification_type = config.get("notification_type").unwrap_or("info");

        // Determine the endpoint: stateful (config key) or stateless
        let (url, mut payload) = if let Some(config_key) = config.get("config_key") {
            let url = format!("{host}/notify/{config_key}");
            let payload = json!({
                "body": message.text,
                "type": notification_type,
            });
            (url, payload)
        } else {
            let urls = config.require("urls", "apprise")?;
            let url = format!("{host}/notify");
            let payload = json!({
                "urls": urls,
                "body": message.text,
                "type": notification_type,
            });
            (url, payload)
        };

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        if let Some(tag) = config.get("tag") {
            payload["tag"] = json!(tag);
        }

        // Map message format
        match message.format {
            noti_core::MessageFormat::Markdown => {
                payload["format"] = json!("markdown");
            }
            noti_core::MessageFormat::Html => {
                payload["format"] = json!("html");
            }
            _ => {
                payload["format"] = json!("text");
            }
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("apprise", "notification sent via Apprise API")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("apprise", format!("API error ({status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        }
    }
}
