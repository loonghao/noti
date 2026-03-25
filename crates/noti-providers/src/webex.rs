use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Cisco Webex Teams messaging provider.
///
/// Sends messages via the Webex REST API using a bot access token.
pub struct WebexProvider {
    client: Client,
}

impl WebexProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for WebexProvider {
    fn name(&self) -> &str {
        "webex"
    }

    fn url_scheme(&self) -> &str {
        "webex"
    }

    fn description(&self) -> &str {
        "Cisco Webex Teams via Bot API"
    }

    fn example_url(&self) -> &str {
        "webex://<access_token>/<room_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("access_token", "Webex bot access token").with_example("NjY2..."),
            ParamDef::required("room_id", "Webex room/space ID to post to")
                .with_example("Y2lzY29zcGFy..."),
            ParamDef::optional("to_person_email", "Send a direct message to a person email"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let access_token = config.require("access_token", "webex")?;
        let room_id = config.require("room_id", "webex")?;

        let url = "https://webexapis.com/v1/messages";

        let text = if let Some(ref title) = message.title {
            format!("**{title}**\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "roomId": room_id,
        });

        // If direct message is specified, use toPersonEmail instead
        if let Some(email) = config.get("to_person_email") {
            payload = json!({
                "toPersonEmail": email,
            });
        }

        match message.format {
            MessageFormat::Markdown | MessageFormat::Html => {
                payload["markdown"] = json!(text);
            }
            MessageFormat::Text => {
                payload["text"] = json!(text);
            }
        }

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(SendResponse::success("webex", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("webex", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
