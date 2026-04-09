use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// ntfy.sh push notification provider.
pub struct NtfyProvider {
    client: Client,
}

impl NtfyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NtfyProvider {
    fn name(&self) -> &str {
        "ntfy"
    }

    fn url_scheme(&self) -> &str {
        "ntfy"
    }

    fn description(&self) -> &str {
        "ntfy.sh push notifications"
    }

    fn example_url(&self) -> &str {
        "ntfy://<topic>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("topic", "ntfy topic name").with_example("my-alerts"),
            ParamDef::optional("server", "Custom ntfy server URL (default: ntfy.sh)"),
            ParamDef::optional("priority", "Message priority (1-5)"),
            ParamDef::optional("tags", "Comma-separated emoji tags"),
            ParamDef::optional("token", "Access token for authentication"),
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
        let topic = config.require("topic", "ntfy")?;
        let server = config.get("server").unwrap_or("https://ntfy.sh");
        let url = format!("{server}/{topic}");

        if message.has_attachments() {
            return self.send_with_attachment(message, &url, config).await;
        }

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(message.text.clone());

        if let Some(title) = &message.title {
            req = req.header("Title", title.as_str());
        }
        if let Some(priority) = config.get("priority") {
            req = req.header("Priority", priority);
        }
        if let Some(tags) = config.get("tags") {
            req = req.header("Tags", tags);
        }
        if let Some(token) = config.get("token") {
            req = req.bearer_auth(token);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("ntfy", e))?;

        let status = resp.status().as_u16();

        // Check for 429 rate limiting
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = crate::http_helpers::read_response_body("ntfy", resp).await;
            return Err(crate::http_helpers::handle_http_error(
                "ntfy",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }

        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({ "status": status }));

        if status == 200 {
            Ok(SendResponse::success("ntfy", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(SendResponse::failure("ntfy", format!("HTTP {status}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}

impl NtfyProvider {
    /// Send message with file attachments using PUT.
    /// ntfy supports one attachment per request, so we send the first file
    /// with the message text, and additional files as separate messages.
    async fn send_with_attachment(
        &self,
        message: &Message,
        url: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let mut last_response = None;

        for (i, attachment) in message.attachments.iter().enumerate() {
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();

            let mut req = self
                .client
                .put(url)
                .header("Filename", &file_name)
                .body(data);

            // Only first attachment gets the message text
            if i == 0 && !message.text.is_empty() {
                req = req.header("Message", message.text.as_str());
            }
            if i == 0 {
                if let Some(title) = &message.title {
                    req = req.header("Title", title.as_str());
                }
            }
            if let Some(priority) = config.get("priority") {
                req = req.header("Priority", priority);
            }
            if let Some(tags) = config.get("tags") {
                req = req.header("Tags", tags);
            }
            if let Some(token) = config.get("token") {
                req = req.bearer_auth(token);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("ntfy", e))?;

            let status = resp.status().as_u16();

            // Check for 429 rate limiting
            if status == 429 {
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let body = crate::http_helpers::read_response_body("ntfy", resp).await;
                return Err(crate::http_helpers::handle_http_error(
                    "ntfy",
                    status,
                    &body,
                    retry_after.as_deref(),
                ));
            }

            let raw: serde_json::Value = resp.json().await.unwrap_or(json!({ "status": status }));

            if status != 200 {
                return Ok(SendResponse::failure("ntfy", format!("HTTP {status}"))
                    .with_status_code(status)
                    .with_raw_response(raw));
            }
            last_response = Some((status, raw));
        }

        if let Some((status, raw)) = last_response {
            Ok(SendResponse::success("ntfy", "file(s) sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            Ok(SendResponse::success("ntfy", "message sent successfully"))
        }
    }
}
