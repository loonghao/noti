use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// SimplePush notification provider.
///
/// Sends push notifications via SimplePush.io.
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

        let url = "https://api.simplepush.io/send";

        let title = message.title.as_deref().unwrap_or("Notification");

        let mut form: Vec<(&str, &str)> =
            vec![("key", key), ("title", title), ("msg", &message.text)];

        let event_val;
        if let Some(event) = config.get("event") {
            event_val = event.to_string();
            form.push(("event", &event_val));
        }

        let resp = self
            .client
            .post(url)
            .form(&form)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

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
