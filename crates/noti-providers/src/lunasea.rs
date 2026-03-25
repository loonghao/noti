use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// LunaSea notification provider.
///
/// Sends push notifications to LunaSea app (self-hosted media management).
/// Uses Firebase Cloud Messaging through LunaSea's notification API.
pub struct LunaseaProvider {
    client: Client,
}

impl LunaseaProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for LunaseaProvider {
    fn name(&self) -> &str {
        "lunasea"
    }

    fn url_scheme(&self) -> &str {
        "lunasea"
    }

    fn description(&self) -> &str {
        "LunaSea self-hosted media push notifications"
    }

    fn example_url(&self) -> &str {
        "lunasea://<user_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("user_token", "LunaSea user token or device token")
                .with_example("your-user-token"),
            ParamDef::optional("target", "Target: user or device (default: user)")
                .with_example("user"),
            ParamDef::optional("image", "Image URL to include")
                .with_example("https://example.com/img.png"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let user_token = config.require("user_token", "lunasea")?;
        let target = config.get("target").unwrap_or("user");

        let url = format!("https://notify.lunasea.app/v1/custom/{target}/{user_token}");

        let mut payload = json!({
            "body": message.text
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        if let Some(image) = config.get("image") {
            payload["image"] = json!(image);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("lunasea", "notification sent via LunaSea")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("lunasea", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
