use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Gitter chat provider.
///
/// Sends messages to Gitter rooms via the Gitter REST API.
/// Gitter is a developer-focused chat platform (now part of Matrix/Element).
///
/// API reference: <https://developer.gitter.im/docs/messages-resource>
pub struct GitterProvider {
    client: Client,
}

impl GitterProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for GitterProvider {
    fn name(&self) -> &str {
        "gitter"
    }

    fn url_scheme(&self) -> &str {
        "gitter"
    }

    fn description(&self) -> &str {
        "Gitter developer chat via REST API"
    }

    fn example_url(&self) -> &str {
        "gitter://<token>/<room_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("token", "Gitter personal access token")
                .with_example("your-access-token"),
            ParamDef::required("room_id", "Gitter room ID").with_example("5xxxxxxxxxxxxxxxxxxxxx"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let token = config.require("token", "gitter")?;
        let room_id = config.require("room_id", "gitter")?;

        let url = format!("https://api.gitter.im/v1/rooms/{room_id}/chatMessages");

        let text = if let Some(ref title) = message.title {
            format!("**{title}**\n{}", message.text)
        } else {
            message.text.clone()
        };

        let payload = json!({
            "text": text
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("gitter", "message sent to Gitter")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("gitter", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
