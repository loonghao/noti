use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Chanify push notification provider.
///
/// Sends push notifications to iOS/Android devices via Chanify.
/// Supports both the public service and self-hosted servers.
///
/// API reference: <https://github.com/nicknisi/chanify>
pub struct ChanifyProvider {
    client: Client,
}

impl ChanifyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ChanifyProvider {
    fn name(&self) -> &str {
        "chanify"
    }

    fn url_scheme(&self) -> &str {
        "chanify"
    }

    fn description(&self) -> &str {
        "Chanify iOS/Android push notifications"
    }

    fn example_url(&self) -> &str {
        "chanify://<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token", "Chanify device token").with_example("your-device-token"),
            ParamDef::optional("server", "Self-hosted Chanify server URL")
                .with_example("https://chanify.example.com"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token = config.require("token", "chanify")?;

        let server = config.get("server").unwrap_or("https://api.chanify.net");

        let url = format!("{server}/v1/sender/{token}");

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut form = vec![("text", text.as_str())];
        if let Some(ref title) = message.title {
            form.push(("title", title.as_str()));
        }

        let resp = self
            .client
            .post(&url)
            .form(&form)
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
                SendResponse::success("chanify", "push notification sent via Chanify")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("chanify", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"body": body})),
            )
        }
    }
}
