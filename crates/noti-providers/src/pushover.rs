use async_trait::async_trait;
use noti_core::{
    Message, MessageFormat, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// Pushover push notification provider.
pub struct PushoverProvider {
    client: Client,
}

impl PushoverProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PushoverProvider {
    fn name(&self) -> &str {
        "pushover"
    }

    fn url_scheme(&self) -> &str {
        "pushover"
    }

    fn description(&self) -> &str {
        "Pushover push notifications"
    }

    fn example_url(&self) -> &str {
        "pushover://<user_key>/<api_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("user_key", "Pushover user/group key")
                .with_example("uQiRzpo4DXghDmr9QzzfQu27cmVRsG"),
            ParamDef::required("api_token", "Pushover application API token")
                .with_example("azGDORePK8gMaC0QOYAMyEEuzJnyUi"),
            ParamDef::optional("device", "Target device name"),
            ParamDef::optional("priority", "Priority: -2 to 2 (default: 0)").with_example("1"),
            ParamDef::optional("sound", "Notification sound name").with_example("pushover"),
            ParamDef::optional(
                "retry",
                "Retry interval in seconds (required for priority=2)",
            )
            .with_example("60"),
            ParamDef::optional("expire", "Expire time in seconds (required for priority=2)")
                .with_example("3600"),
            ParamDef::optional("url", "Supplementary URL"),
            ParamDef::optional("url_title", "URL title/description"),
            ParamDef::optional("ttl", "Message TTL in seconds"),
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
        let user_key = config.require("user_key", "pushover")?;
        let api_token = config.require("api_token", "pushover")?;

        let resp = if message.has_attachments() {
            // Use multipart form for file attachment
            let attachment = &message.attachments[0];
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();
            let mime_str = attachment.effective_mime();

            let file_part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

            let mut form = reqwest::multipart::Form::new()
                .text("token", api_token.to_string())
                .text("user", user_key.to_string())
                .text("message", message.text.clone())
                .part("attachment", file_part);

            if let Some(ref title) = message.title {
                form = form.text("title", title.clone());
            }
            if message.format == MessageFormat::Html {
                form = form.text("html", "1".to_string());
            }
            if let Some(device) = config.get("device") {
                form = form.text("device", device.to_string());
            }
            if let Some(priority) = config.get("priority") {
                form = form.text("priority", priority.to_string());
            }
            if let Some(sound) = config.get("sound") {
                form = form.text("sound", sound.to_string());
            }
            if let Some(retry) = config.get("retry") {
                form = form.text("retry", retry.to_string());
            }
            if let Some(expire) = config.get("expire") {
                form = form.text("expire", expire.to_string());
            }
            if let Some(url) = config.get("url") {
                form = form.text("url", url.to_string());
            }
            if let Some(url_title) = config.get("url_title") {
                form = form.text("url_title", url_title.to_string());
            }
            if let Some(ttl) = config.get("ttl") {
                form = form.text("ttl", ttl.to_string());
            }

            self.client
                .post("https://api.pushover.net/1/messages.json")
                .multipart(form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("pushover", e))?
        } else {
            // Standard form POST
            let mut form = vec![
                ("token", api_token.to_string()),
                ("user", user_key.to_string()),
                ("message", message.text.clone()),
            ];

            if let Some(ref title) = message.title {
                form.push(("title", title.clone()));
            }
            if message.format == MessageFormat::Html {
                form.push(("html", "1".to_string()));
            }
            if let Some(device) = config.get("device") {
                form.push(("device", device.to_string()));
            }
            if let Some(priority) = config.get("priority") {
                form.push(("priority", priority.to_string()));
            }
            if let Some(sound) = config.get("sound") {
                form.push(("sound", sound.to_string()));
            }
            if let Some(retry) = config.get("retry") {
                form.push(("retry", retry.to_string()));
            }
            if let Some(expire) = config.get("expire") {
                form.push(("expire", expire.to_string()));
            }
            if let Some(url) = config.get("url") {
                form.push(("url", url.to_string()));
            }
            if let Some(url_title) = config.get("url_title") {
                form.push(("url_title", url_title.to_string()));
            }
            if let Some(ttl) = config.get("ttl") {
                form.push(("ttl", ttl.to_string()));
            }

            self.client
                .post("https://api.pushover.net/1/messages.json")
                .form(&form)
                .send()
                .await
                .map_err(|e| crate::http_helpers::classify_reqwest_error("pushover", e))?
        };

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let ok = raw.get("status").and_then(|v| v.as_i64()).unwrap_or(0);
        if ok == 1 {
            Ok(
                SendResponse::success("pushover", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let errors = raw
                .get("errors")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "unknown error".to_string());
            Ok(
                SendResponse::failure("pushover", format!("API error: {errors}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
