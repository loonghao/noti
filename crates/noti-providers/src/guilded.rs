use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use reqwest::multipart;
use serde_json::json;

/// Guilded webhook provider.
///
/// Sends messages via Guilded webhooks (Discord-compatible).
/// Guilded is a chat platform for gaming communities.
/// Supports file attachments via multipart upload.
pub struct GuildedProvider {
    client: Client,
}

impl GuildedProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for GuildedProvider {
    fn name(&self) -> &str {
        "guilded"
    }

    fn url_scheme(&self) -> &str {
        "guilded"
    }

    fn description(&self) -> &str {
        "Guilded chat webhooks (Discord-compatible)"
    }

    fn example_url(&self) -> &str {
        "guilded://<webhook_id>/<webhook_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_id", "Guilded webhook ID").with_example("your-webhook-id"),
            ParamDef::required("webhook_token", "Guilded webhook token")
                .with_example("your-webhook-token"),
            ParamDef::optional("username", "Override webhook display name").with_example("noti"),
            ParamDef::optional("avatar_url", "Override webhook avatar URL")
                .with_example("https://example.com/avatar.png"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let webhook_id = config.require("webhook_id", "guilded")?;
        let webhook_token = config.require("webhook_token", "guilded")?;

        let url = format!("https://media.guilded.gg/webhooks/{webhook_id}/{webhook_token}");

        let content = if let Some(ref title) = message.title {
            format!("**{title}**\n{}", message.text)
        } else {
            message.text.clone()
        };

        let resp = if message.has_attachments() {
            let mut payload = json!({ "content": content });

            if let Some(username) = config.get("username") {
                payload["username"] = json!(username);
            }
            if let Some(avatar_url) = config.get("avatar_url") {
                payload["avatar_url"] = json!(avatar_url);
            }

            let mut form = multipart::Form::new().text(
                "payload_json",
                serde_json::to_string(&payload)
                    .map_err(|e| NotiError::Network(format!("JSON error: {e}")))?,
            );

            for (i, attachment) in message.attachments.iter().enumerate() {
                let data = attachment.read_bytes().await?;
                let file_name = attachment.effective_file_name();
                let mime_str = attachment.effective_mime();
                let part = multipart::Part::bytes(data)
                    .file_name(file_name)
                    .mime_str(&mime_str)
                    .map_err(|e| NotiError::Network(format!("invalid MIME type: {e}")))?;
                form = form.part(format!("files[{i}]"), part);
            }

            self.client
                .post(&url)
                .multipart(form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?
        } else {
            let mut payload = json!({ "content": content });

            if let Some(username) = config.get("username") {
                payload["username"] = json!(username);
            }
            if let Some(avatar_url) = config.get("avatar_url") {
                payload["avatar_url"] = json!(avatar_url);
            }

            self.client
                .post(&url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?
        };

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        let raw =
            serde_json::from_str::<serde_json::Value>(&body).unwrap_or(json!({"response": body}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("guilded", "message sent to Guilded")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("guilded", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
