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
            ParamDef::optional(
                "thread_ts",
                "Thread reply ts (timestamp) to reply in a thread",
            ),
            ParamDef::optional(
                "ephemeral_user",
                "User ID for ephemeral message (visible only to that user)",
            ),
            ParamDef::optional("send_at", "Unix timestamp for scheduled message"),
            ParamDef::optional("blocks", "Raw Block Kit JSON array for structured messages")
                .with_example(
                    r#"[{"type":"section","text":{"type":"mrkdwn","text":"*Build passed*"}}]"#,
                ),
            ParamDef::optional("base_url", "Slack API base URL (default: https://slack.com)")
                .with_example("https://slack.com"),
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

        // Use chat.postMessage API for thread_ts, ephemeral, or scheduled messages
        let thread_ts = config.get("thread_ts");
        let ephemeral_user = config.get("ephemeral_user");
        let send_at = config.get("send_at");

        if thread_ts.is_some() || ephemeral_user.is_some() || send_at.is_some() {
            if let Some(bot_token) = config.get("bot_token") {
                return self.send_via_api(message, config, bot_token).await;
            }
            return Err(NotiError::Validation(
                "bot_token required for thread_ts, ephemeral_user, or send_at".to_string(),
            ));
        }

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
                // Check for raw blocks JSON
                if let Some(blocks_json) = config.get("blocks") {
                    let blocks: serde_json::Value = serde_json::from_str(blocks_json)
                        .map_err(|e| NotiError::Validation(format!("invalid blocks JSON: {e}")))?;
                    json!({
                        "text": message.text,
                        "blocks": blocks
                    })
                } else {
                    json!({ "text": message.text })
                }
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
            .map_err(|e| crate::http_helpers::classify_reqwest_error("slack", e))?;

        let status = resp.status().as_u16();
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if status == 429 {
            return Err(crate::http_helpers::handle_http_error(
                "slack",
                status,
                &body_text,
                retry_after.as_deref(),
            ));
        }

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
    /// Returns the Slack API base URL, optionally overridden by config.
    fn api_base_url(config: &ProviderConfig) -> String {
        config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| "https://slack.com".to_string())
    }

    /// Upload files to Slack using the files.uploadV2 equivalent (files.upload).
    async fn send_with_files(
        &self,
        message: &Message,
        bot_token: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let channel = config.get("channel").unwrap_or("general");
        let base_url = Self::api_base_url(config);

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
                .post(format!("{base_url}/api/files.upload"))
                .bearer_auth(bot_token)
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("slack", e))?;

            let status = resp.status().as_u16();
            if status == 429 {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let body = resp.text().await.unwrap_or_default();
                return Err(crate::http_helpers::handle_http_error(
                    "slack",
                    status,
                    &body,
                    retry_after.as_deref(),
                ));
            }
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

    /// Send message via Slack API (chat.postMessage/chat.postEphemeral/schat.scheduleMessage).
    async fn send_via_api(
        &self,
        message: &Message,
        config: &ProviderConfig,
        bot_token: &str,
    ) -> Result<SendResponse, NotiError> {
        let channel = config.require("channel", "slack")?;
        let thread_ts = config.get("thread_ts");
        let ephemeral_user = config.get("ephemeral_user");
        let send_at = config.get("send_at");
        let base_url = Self::api_base_url(config);

        let mut payload = match message.format {
            MessageFormat::Markdown => {
                json!({
                    "channel": channel,
                    "text": message.text,
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
                json!({
                    "channel": channel,
                    "text": message.text
                })
            }
        };

        if let Some(ts) = thread_ts {
            payload["thread_ts"] = json!(ts);
        }
        if let Some(icon) = config.get("icon_emoji") {
            payload["icon_emoji"] = json!(icon);
        }
        if let Some(username) = config.get("username") {
            payload["username"] = json!(username);
        }

        let api_method = if ephemeral_user.is_some() {
            "chat.postEphemeral"
        } else if send_at.is_some() {
            "chat.scheduleMessage"
        } else {
            "chat.postMessage"
        };

        if let Some(user) = ephemeral_user {
            payload["user"] = json!(user);
        }
        if let Some(ts) = send_at {
            payload["post_at"] = json!(ts);
        }

        let resp = self
            .client
            .post(format!("{base_url}/api/{api_method}"))
            .bearer_auth(bot_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("slack", e))?;

        let status = resp.status().as_u16();
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::http_helpers::handle_http_error(
                "slack",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let ok = raw.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if ok {
            Ok(SendResponse::success("slack", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let err = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("slack", format!("API error: {err}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
