use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Mattermost incoming webhook provider.
///
/// Mattermost webhooks accept JSON with `text`, `channel`, `username`,
/// and `icon_url` fields. Markdown is natively supported in the `text` field.
///
/// For file attachments, requires a personal access token and channel_id to
/// use the file upload REST API.
pub struct MattermostProvider {
    client: Client,
}

impl MattermostProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for MattermostProvider {
    fn name(&self) -> &str {
        "mattermost"
    }

    fn url_scheme(&self) -> &str {
        "mattermost"
    }

    fn description(&self) -> &str {
        "Mattermost via incoming webhook"
    }

    fn example_url(&self) -> &str {
        "mattermost://<host>/<hook_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required(
                "host",
                "Mattermost server host (e.g. mattermost.example.com)",
            )
            .with_example("mattermost.example.com"),
            ParamDef::required("hook_id", "Incoming webhook ID").with_example("abcdef1234567890"),
            ParamDef::optional("channel", "Override the default channel"),
            ParamDef::optional("username", "Override the posting username"),
            ParamDef::optional("icon_url", "Override the posting icon URL"),
            ParamDef::optional("port", "Server port (default: 443)"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
            ParamDef::optional(
                "personal_token",
                "Personal access token (required for file uploads)",
            ),
            ParamDef::optional(
                "channel_id",
                "Channel ID (required for file uploads, different from channel name)",
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
        let host = config.require("host", "mattermost")?;
        let hook_id = config.require("hook_id", "mattermost")?;
        let url_scheme = config.get("scheme").unwrap_or("https");
        let port = config.get("port").unwrap_or("443");

        let base_url = format!("{url_scheme}://{host}:{port}");
        let webhook_url = format!("{base_url}/hooks/{hook_id}");

        // Handle file attachments via REST API
        if message.has_attachments() {
            if let (Some(token), Some(channel_id)) =
                (config.get("personal_token"), config.get("channel_id"))
            {
                return self
                    .send_with_files(message, &base_url, token, channel_id, config)
                    .await;
            }
            // Fall through to webhook if no token — mention files in text
        }

        // Mattermost supports markdown natively in the text field
        let text = match (&message.format, &message.title) {
            (MessageFormat::Markdown, Some(title)) => {
                format!("### {title}\n\n{}", message.text)
            }
            (_, Some(title)) => {
                format!("**{title}**\n\n{}", message.text)
            }
            _ => message.text.clone(),
        };

        let mut payload = json!({ "text": text });

        if let Some(channel) = config.get("channel") {
            payload["channel"] = json!(channel);
        }
        if let Some(username) = config.get("username") {
            payload["username"] = json!(username);
        }
        if let Some(icon_url) = config.get("icon_url") {
            payload["icon_url"] = json!(icon_url);
        }

        let resp = self
            .client
            .post(&webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("mattermost", e))?;

        let status = resp.status().as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("mattermost", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        } else {
            Ok(SendResponse::failure(
                "mattermost",
                format!("API error (HTTP {status}): {body_text}"),
            )
            .with_status_code(status)
            .with_raw_response(json!({ "body": body_text })))
        }
    }
}

impl MattermostProvider {
    /// Upload files via Mattermost REST API and create a post referencing them.
    async fn send_with_files(
        &self,
        message: &Message,
        base_url: &str,
        token: &str,
        channel_id: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let mut file_ids = Vec::new();

        for attachment in &message.attachments {
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();
            let mime_str = attachment.effective_mime();

            let part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

            let form = reqwest::multipart::Form::new()
                .text("channel_id", channel_id.to_string())
                .part("files", part);

            let upload_url = format!("{base_url}/api/v4/files");
            let resp = self
                .client
                .post(&upload_url)
                .header("Authorization", format!("Bearer {token}"))
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("mattermost", e))?;

            let raw: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| NotiError::Network(format!("upload parse error: {e}")))?;

            if let Some(infos) = raw.get("file_infos").and_then(|v| v.as_array()) {
                for info in infos {
                    if let Some(id) = info.get("id").and_then(|v| v.as_str()) {
                        file_ids.push(id.to_string());
                    }
                }
            }
        }

        // Create a post with the uploaded file IDs
        let text = match (&message.format, &message.title) {
            (MessageFormat::Markdown, Some(title)) => {
                format!("### {title}\n\n{}", message.text)
            }
            (_, Some(title)) => {
                format!("**{title}**\n\n{}", message.text)
            }
            _ => message.text.clone(),
        };

        let mut payload = json!({
            "channel_id": channel_id,
            "message": text,
            "file_ids": file_ids,
        });

        // Apply optional username override via props
        if let Some(username) = config.get("username") {
            payload["props"] = json!({
                "override_username": username,
            });
        }

        let post_url = format!("{base_url}/api/v4/posts");
        let resp = self
            .client
            .post(&post_url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("mattermost", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("mattermost", "message and file(s) sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("mattermost", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
