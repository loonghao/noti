use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Notica provider.
///
/// Sends browser push notifications via Notica.
///
/// API reference: <https://notica.us/>
pub struct NoticaProvider {
    client: Client,
}

impl NoticaProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NoticaProvider {
    fn name(&self) -> &str {
        "notica"
    }

    fn url_scheme(&self) -> &str {
        "notica"
    }

    fn description(&self) -> &str {
        "Notica browser push notifications"
    }

    fn example_url(&self) -> &str {
        "notica://<token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::required("token", "Notica notification token").with_example("abc123")]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token = config.require("token", "notica")?;

        let url = format!("https://notica.us/?{token}");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let params = [("payload", body_text.as_str())];

        let resp = self
            .client
            .post(&url)
            .form(&params)
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
                SendResponse::success("notica", "notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("notica", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"body": body})),
            )
        }
    }
}
