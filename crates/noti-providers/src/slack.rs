use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Slack incoming webhook provider.
pub struct SlackProvider {
    client: Client,
}

impl SlackProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SlackProvider {
    fn name(&self) -> &str {
        "slack"
    }

    fn url_scheme(&self) -> &str {
        "slack"
    }

    fn description(&self) -> &str {
        "Slack via incoming webhooks"
    }

    fn example_url(&self) -> &str {
        "slack://<token_a>/<token_b>/<token_c>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Slack incoming webhook URL")
                .with_example("https://hooks.slack.com/services/T.../B.../xxx"),
            ParamDef::optional("channel", "Override the default channel"),
            ParamDef::optional("username", "Override the default username"),
            ParamDef::optional("icon_emoji", "Override the default icon emoji"),
            ParamDef::optional(
                "bot_token",
                "Slack Bot token (required for file uploads, xoxb-...)",
            ),
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
        let webhook_url = config.require("webhook_url", "slack")?;

        // If attachments are present and a bot_token is provided, use file upload API
        if message.has_attachments() {
            if let Some(bot_token) = config.get("bot_token") {
                return self.send_with_files(message, bot_token, config).await;
            }
            // Fall through to normal webhook if no bot_token — files will be ignored
        }

        let mut payload = match message.format {
            MessageFormat::Markdown => {
                json!({
                    "blocks": [{
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": message.text
                        }
                    }]
                })
            }
            _ => {
                json!({ "text": message.text })
            }
        };

        if let Some(channel) = config.get("channel") {
            payload["channel"] = json!(channel);
        }
        if let Some(username) = config.get("username") {
            payload["username"] = json!(username);
        }
        if let Some(icon) = config.get("icon_emoji") {
            payload["icon_emoji"] = json!(icon);
        }

        let resp = self
            .client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if status == 200 && body_text == "ok" {
            Ok(
                SendResponse::success("slack", "message sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("slack", format!("API error: {body_text}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        }
    }
}

impl SlackProvider {
    /// Upload files to Slack using the files.uploadV2 equivalent (files.upload).
    async fn send_with_files(
        &self,
        message: &Message,
        bot_token: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let channel = config.get("channel").unwrap_or("general");

        for attachment in &message.attachments {
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();

            let form = reqwest::multipart::Form::new()
                .text("channels", channel.to_string())
                .text("initial_comment", message.text.clone())
                .text("filename", file_name.clone())
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(data)
                        .file_name(file_name)
                        .mime_str(&attachment.effective_mime())
                        .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?,
                );

            let resp = self
                .client
                .post("https://slack.com/api/files.upload")
                .bearer_auth(bot_token)
                .multipart(form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            let status = resp.status().as_u16();
            let raw: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

            let ok = raw.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            if !ok {
                let err = raw
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                return Ok(
                    SendResponse::failure("slack", format!("file upload error: {err}"))
                        .with_status_code(status)
                        .with_raw_response(raw),
                );
            }
        }

        Ok(SendResponse::success(
            "slack",
            "message and file(s) sent successfully",
        ))
    }
}
