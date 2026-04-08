use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Rocket.Chat incoming webhook provider.
///
/// Uses Rocket.Chat's incoming webhook integration for text messages.
/// For file attachments, uses the REST API (requires auth_token + user_id + room_id).
pub struct RocketChatProvider {
    client: Client,
}

impl RocketChatProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Build the Rocket.Chat base URL with optional base_url override.
    fn rocketchat_base_url(config: &ProviderConfig) -> String {
        if let Some(base) = config.get("base_url") {
            let base = base.trim_end_matches('/');
            return base.to_string();
        }
        let host = config.get("host").unwrap_or("localhost");
        let url_scheme = config.get("scheme").unwrap_or("https");
        let port = config.get("port").unwrap_or("443");
        format!("{url_scheme}://{host}:{port}")
    }

    /// Upload files via Rocket.Chat REST API.
    async fn send_with_files(
        &self,
        message: &Message,
        base_url: &str,
        auth_token: &str,
        user_id: &str,
        room_id: &str,
    ) -> Result<SendResponse, NotiError> {
        for attachment in &message.attachments {
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();
            let mime_str = attachment.effective_mime();

            let part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

            let mut form = reqwest::multipart::Form::new().part("file", part);

            if !message.text.is_empty() {
                form = form.text("description", message.text.clone());
            }

            let upload_url = format!("{base_url}/api/v1/rooms.upload/{room_id}");
            let resp = self
                .client
                .post(&upload_url)
                .header("X-Auth-Token", auth_token)
                .header("X-User-Id", user_id)
                .multipart(form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            let status = resp.status().as_u16();
            let raw: serde_json::Value = resp
                .json()
                .await
                .unwrap_or_else(|_| json!({"error": "failed to parse response"}));

            let success = raw
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if !success {
                let error = raw
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                return Ok(SendResponse::failure(
                    "rocketchat",
                    format!("file upload error: {error}"),
                )
                .with_status_code(status)
                .with_raw_response(raw));
            }
        }

        Ok(SendResponse::success(
            "rocketchat",
            "message and file(s) sent successfully",
        ))
    }
}

#[async_trait]
impl NotifyProvider for RocketChatProvider {
    fn name(&self) -> &str {
        "rocketchat"
    }

    fn url_scheme(&self) -> &str {
        "rocketchat"
    }

    fn description(&self) -> &str {
        "Rocket.Chat via incoming webhook"
    }

    fn example_url(&self) -> &str {
        "rocketchat://<host>/<token_a>/<token_b>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Rocket.Chat server host (e.g. chat.example.com)")
                .with_example("chat.example.com"),
            ParamDef::required("token_a", "Webhook token part A").with_example("abcdef"),
            ParamDef::required("token_b", "Webhook token part B").with_example("ghijkl"),
            ParamDef::optional("channel", "Target channel or user (e.g. #general, @user)"),
            ParamDef::optional("username", "Override the posting username"),
            ParamDef::optional("icon_url", "Override the posting avatar URL"),
            ParamDef::optional("port", "Server port (default: 443)"),
            ParamDef::optional("scheme", "URL scheme: https or http (default: https)"),
            ParamDef::optional(
                "auth_token",
                "Personal auth token (required for file uploads)",
            ),
            ParamDef::optional("user_id", "User ID (required for file uploads)"),
            ParamDef::optional("room_id", "Room ID (required for file uploads)"),
            ParamDef::optional("base_url", "Override the full base URL (default: {scheme}://{host}:{port})")
                .with_example("https://chat.example.com"),
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
        let _host = config.require("host", "rocketchat")?;
        let token_a = config.require("token_a", "rocketchat")?;
        let token_b = config.require("token_b", "rocketchat")?;

        let base_url = Self::rocketchat_base_url(config);

        // Handle file attachments via REST API
        if message.has_attachments() {
            if let (Some(auth_token), Some(user_id), Some(room_id)) = (
                config.get("auth_token"),
                config.get("user_id"),
                config.get("room_id"),
            ) {
                return self
                    .send_with_files(message, &base_url, auth_token, user_id, room_id)
                    .await;
            }
            // Fall through to webhook if no auth credentials
        }

        let webhook_url = format!("{base_url}/hooks/{token_a}/{token_b}");

        // Rocket.Chat supports markdown in the text field
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
            payload["alias"] = json!(username);
        }
        if let Some(icon_url) = config.get("icon_url") {
            payload["avatar"] = json!(icon_url);
        }

        let resp = self
            .client
            .post(&webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({ "error": "failed to parse response" }));

        let success = raw
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || (200..300).contains(&status);

        if success {
            Ok(
                SendResponse::success("rocketchat", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error_msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("rocketchat", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
