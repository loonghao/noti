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
            ParamDef::optional("thread_id", "Thread ID to send message to"),
            ParamDef::optional("wait", "Wait for response (true/false) to get message ID"),
            ParamDef::optional("embed_title", "Embed title"),
            ParamDef::optional("embed_color", "Embed color (hex, e.g. 0xFF0000)"),
            ParamDef::optional("embed_description", "Embed description text"),
            ParamDef::optional("embed_footer", "Embed footer text"),
            ParamDef::optional("embed_thumbnail", "Embed thumbnail URL"),
            ParamDef::optional(
                "embed_field",
                "Embed field (can be repeated, format: title:value)",
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
        let webhook_id = config.require("webhook_id", "discord")?;
        let webhook_token = config.require("webhook_token", "discord")?;

        let mut url = format!("https://discord.com/api/webhooks/{webhook_id}/{webhook_token}");

        // Add query parameters
        let mut query_params = Vec::new();
        if let Some(thread_id) = config.get("thread_id") {
            query_params.push(("thread_id", thread_id));
        }
        if config.get("wait") == Some("true") {
            query_params.push(("wait", "true"));
        }

        if !query_params.is_empty() {
            let query_string = query_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{url}?{query_string}");
        }

        // Build payload with optional rich embeds
        let payload = self.build_payload(message, config);

        let resp = if message.has_attachments() {
            // Multipart upload with file attachments
            let mut payload_with_user = payload;
            if let Some(username) = config.get("username") {
                payload_with_user["username"] = json!(username);
            }
            if let Some(avatar) = config.get("avatar_url") {
                payload_with_user["avatar_url"] = json!(avatar);
            }

            let mut form = multipart::Form::new().text(
                "payload_json",
                serde_json::to_string(&payload_with_user)
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
            let mut payload_with_user = payload;
            if let Some(username) = config.get("username") {
                payload_with_user["username"] = json!(username);
            }
            if let Some(avatar) = config.get("avatar_url") {
                payload_with_user["avatar_url"] = json!(avatar);
            }

            self.client
                .post(&url)
                .json(&payload_with_user)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?
        };

        let status = resp.status().as_u16();

        // Discord returns 204 No Content on success (or 200 with message object if wait=true)
        if status == 204 || status == 200 {
            let raw: serde_json::Value = if status == 200 {
                resp.json().await.unwrap_or(json!({}))
            } else {
                json!({})
            };
            let mut resp = SendResponse::success("discord", "message sent successfully")
                .with_status_code(status);
            if !raw.is_null() {
                resp = resp.with_raw_response(raw);
            }
            Ok(resp)
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

impl DiscordProvider {
    /// Build the JSON payload, optionally including rich embeds.
    fn build_payload(&self, message: &Message, config: &ProviderConfig) -> serde_json::Value {
        // Check if any embed params are provided
        let has_embed = config.get("embed_title").is_some()
            || config.get("embed_color").is_some()
            || config.get("embed_description").is_some()
            || config.get("embed_footer").is_some()
            || config.get("embed_thumbnail").is_some()
            || config.get("embed_field").is_some();

        if has_embed {
            let mut embed = serde_json::Map::new();

            if let Some(title) = config.get("embed_title") {
                embed.insert("title".to_string(), json!(title));
            }
            if let Some(color_str) = config.get("embed_color") {
                // Parse hex color (e.g., "0xFF0000" or "FF0000")
                let color_str = color_str.trim_start_matches("0x");
                if let Ok(color) = u32::from_str_radix(color_str, 16) {
                    embed.insert("color".to_string(), json!(color));
                }
            }
            if let Some(desc) = config.get("embed_description") {
                embed.insert("description".to_string(), json!(desc));
            } else if !message.text.is_empty() {
                embed.insert("description".to_string(), json!(message.text));
            }
            if let Some(title) = &message.title {
                if config.get("embed_title").is_none() {
                    embed.insert("title".to_string(), json!(title));
                }
            }

            if let Some(footer) = config.get("embed_footer") {
                embed.insert("footer".to_string(), json!({ "text": footer }));
            }
            if let Some(thumbnail) = config.get("embed_thumbnail") {
                embed.insert("thumbnail".to_string(), json!({ "url": thumbnail }));
            }

            // Parse embed fields (format: "title:value", can be repeated)
            if let Some(fields_str) = config.get("embed_field") {
                let fields: Vec<serde_json::Value> = fields_str
                    .split(',')
                    .filter_map(|f| {
                        let f = f.trim();
                        if let Some((field_title, field_value)) = f.split_once(':') {
                            Some(json!({
                                "name": field_title.trim(),
                                "value": field_value.trim(),
                                "inline": false
                            }))
                        } else {
                            None
                        }
                    })
                    .collect();
                if !fields.is_empty() {
                    embed.insert("fields".to_string(), json!(fields));
                }
            }

            return json!({ "embeds": [embed] });
        }

        // No embed params, use standard content/embed logic
        match message.format {
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
        }
    }
}
