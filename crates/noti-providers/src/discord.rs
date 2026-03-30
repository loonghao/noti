use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use reqwest::multipart;
use serde_json::json;

/// Discord webhook provider.
pub struct DiscordProvider {
    client: Client,
}

impl DiscordProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for DiscordProvider {
    fn name(&self) -> &str {
        "discord"
    }

    fn url_scheme(&self) -> &str {
        "discord"
    }

    fn description(&self) -> &str {
        "Discord via webhooks"
    }

    fn example_url(&self) -> &str {
        "discord://<webhook_id>/<webhook_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_id", "Discord webhook ID").with_example("1234567890"),
            ParamDef::required("webhook_token", "Discord webhook token")
                .with_example("abcdefg_hijklmn"),
            ParamDef::optional("username", "Override the default bot username"),
            ParamDef::optional("avatar_url", "Override the default bot avatar"),
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
        let webhook_id = config.require("webhook_id", "discord")?;
        let webhook_token = config.require("webhook_token", "discord")?;

        let url = format!("https://discord.com/api/webhooks/{webhook_id}/{webhook_token}");

        let resp = if message.has_attachments() {
            // Multipart upload with file attachments
            let mut payload = match message.format {
                MessageFormat::Markdown | MessageFormat::Html => {
                    if let Some(ref title) = message.title {
                        json!({
                            "embeds": [{
                                "title": title,
                                "description": message.text
                            }]
                        })
                    } else {
                        json!({ "content": message.text })
                    }
                }
                MessageFormat::Text => {
                    json!({ "content": message.text })
                }
            };

            if let Some(username) = config.get("username") {
                payload["username"] = json!(username);
            }
            if let Some(avatar) = config.get("avatar_url") {
                payload["avatar_url"] = json!(avatar);
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
            // Text-only JSON payload
            let mut payload = match message.format {
                MessageFormat::Markdown | MessageFormat::Html => {
                    if let Some(ref title) = message.title {
                        json!({
                            "embeds": [{
                                "title": title,
                                "description": message.text
                            }]
                        })
                    } else {
                        json!({ "content": message.text })
                    }
                }
                MessageFormat::Text => {
                    json!({ "content": message.text })
                }
            };

            if let Some(username) = config.get("username") {
                payload["username"] = json!(username);
            }
            if let Some(avatar) = config.get("avatar_url") {
                payload["avatar_url"] = json!(avatar);
            }

            self.client
                .post(&url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?
        };

        let status = resp.status().as_u16();

        // Discord returns 204 No Content on success
        if status == 204 || status == 200 {
            Ok(
                SendResponse::success("discord", "message sent successfully")
                    .with_status_code(status),
            )
        } else {
            let raw: serde_json::Value = resp.json().await.unwrap_or(json!({}));
            let msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("discord", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
