use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Flock team messaging provider.
///
/// Sends messages via Flock incoming webhooks.
/// Flock webhooks only support JSON payloads (text and flockml).
/// File uploads require the Flock API with OAuth tokens.
pub struct FlockProvider {
    client: Client,
}

impl FlockProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FlockProvider {
    fn name(&self) -> &str {
        "flock"
    }

    fn url_scheme(&self) -> &str {
        "flock"
    }

    fn description(&self) -> &str {
        "Flock team messaging via incoming webhooks"
    }

    fn example_url(&self) -> &str {
        "flock://<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Flock incoming webhook URL")
                .with_example("https://api.flock.com/hooks/sendMessage/XXXXXX"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_url = config.require("webhook_url", "flock")?;

        let text = match message.format {
            MessageFormat::Html => {
                if let Some(ref title) = message.title {
                    format!("<b>{title}</b><br/>{}", message.text)
                } else {
                    message.text.clone()
                }
            }
            _ => {
                if let Some(ref title) = message.title {
                    format!("{title}\n{}", message.text)
                } else {
                    message.text.clone()
                }
            }
        };

        let payload = if matches!(message.format, MessageFormat::Html) {
            json!({ "flockml": text })
        } else {
            json!({ "text": text })
        };

        let resp = self
            .client
            .post(webhook_url)
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
                SendResponse::success("flock", "message sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("flock", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
