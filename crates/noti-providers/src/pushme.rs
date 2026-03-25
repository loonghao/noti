use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// PushMe push notification service.
///
/// API reference: https://push.i-i.me/
pub struct PushMeProvider {
    client: Client,
}

impl PushMeProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushMeProvider {
    fn name(&self) -> &str {
        "pushme"
    }

    fn url_scheme(&self) -> &str {
        "pushme"
    }

    fn description(&self) -> &str {
        "PushMe push notification service"
    }

    fn example_url(&self) -> &str {
        "pushme://<push_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("push_key", "PushMe push key"),
            ParamDef::optional(
                "type",
                "Message type: text, markdown, image (default: text)",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;

        let push_key = config.require("push_key", "pushme")?;
        let title = message.title.as_deref().unwrap_or("noti");
        let msg_type = config.get("type").unwrap_or("text");

        let body = serde_json::json!({
            "push_key": push_key,
            "title": title,
            "content": message.text,
            "type": msg_type
        });

        let resp = self
            .client
            .post("https://push.i-i.me/")
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("pushme", "push notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("pushme", format!("PushMe API error: {raw}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
