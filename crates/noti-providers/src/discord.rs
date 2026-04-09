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
            ParamDef::optional("base_url", "Discord API base URL (default: https://discord.com)")
                .with_example("https://discord.com"),
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

        let mut url = discord_url(webhook_id, webhook_token, config);

        // Add query parameters
        let query_params = build_query_params(config);
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
                .map_err(|e| crate::http_helpers::classify_reqwest_error("discord", e))?
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
                .map_err(|e| crate::http_helpers::classify_reqwest_error("discord", e))?
        };

        let status = resp.status().as_u16();

        // Handle rate limiting (429) — Discord returns retry_after in JSON body
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = crate::http_helpers::read_response_body("discord", resp).await;
            // Discord also includes retry_after in the JSON body
            let retry_from_body = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("retry_after").and_then(|r| r.as_f64()))
                .map(|secs| secs as u64);
            let retry_secs = retry_after
                .as_deref()
                .and_then(|s| s.parse::<u64>().ok())
                .or(retry_from_body);
            return Err(NotiError::rate_limited("discord", retry_secs));
        }

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
                if let Some(color) = parse_embed_color(color_str) {
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
                let fields = parse_embed_fields(fields_str);
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

/// Build the Discord webhook URL, optionally using a custom base URL.
fn discord_url(webhook_id: &str, webhook_token: &str, config: &ProviderConfig) -> String {
    let base = config
        .get("base_url")
        .map(|s| s.trim_end_matches('/'))
        .unwrap_or("https://discord.com");
    format!("{base}/api/webhooks/{webhook_id}/{webhook_token}")
}

/// Build query parameters for the Discord webhook URL.
fn build_query_params(config: &ProviderConfig) -> Vec<(&'static str, String)> {
    let mut params = Vec::new();
    if let Some(thread_id) = config.get("thread_id") {
        params.push(("thread_id", thread_id.to_string()));
    }
    if config.get("wait") == Some("true") {
        params.push(("wait", "true".to_string()));
    }
    params
}

/// Parse a hex color string (e.g., "0xFF0000" or "FF0000") to a u32.
fn parse_embed_color(color_str: &str) -> Option<u32> {
    let color_str = color_str.trim_start_matches("0x");
    u32::from_str_radix(color_str, 16).ok()
}

/// Parse embed fields from a comma-separated string (format: "title:value,title2:value2").
fn parse_embed_fields(fields_str: &str) -> Vec<serde_json::Value> {
    fields_str
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
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> ProviderConfig {
        ProviderConfig::new()
            .set("webhook_id", "1234567890")
            .set("webhook_token", "abcdefg_hijklmn")
    }

    // ---- discord_url tests ----

    #[test]
    fn test_discord_url_default() {
        let config = make_config();
        let url = discord_url("1234567890", "abcdefg_hijklmn", &config);
        assert_eq!(
            url,
            "https://discord.com/api/webhooks/1234567890/abcdefg_hijklmn"
        );
    }

    #[test]
    fn test_discord_url_custom_base() {
        let config = make_config().set("base_url", "https://custom.discord.server.com");
        let url = discord_url("1234567890", "abcdefg_hijklmn", &config);
        assert_eq!(
            url,
            "https://custom.discord.server.com/api/webhooks/1234567890/abcdefg_hijklmn"
        );
    }

    #[test]
    fn test_discord_url_trailing_slash_stripped() {
        let config = make_config().set("base_url", "https://custom.server.com/");
        let url = discord_url("1234567890", "abcdefg_hijklmn", &config);
        assert_eq!(
            url,
            "https://custom.server.com/api/webhooks/1234567890/abcdefg_hijklmn"
        );
    }

    // ---- build_query_params tests ----

    #[test]
    fn test_build_query_params_empty() {
        let config = make_config();
        let params = build_query_params(&config);
        assert!(params.is_empty());
    }

    #[test]
    fn test_build_query_params_thread_id() {
        let config = make_config().set("thread_id", "123456");
        let params = build_query_params(&config);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].0, "thread_id");
        assert_eq!(params[0].1, "123456");
    }

    #[test]
    fn test_build_query_params_wait() {
        let config = make_config().set("wait", "true");
        let params = build_query_params(&config);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].0, "wait");
        assert_eq!(params[0].1, "true");
    }

    #[test]
    fn test_build_query_params_wait_false_ignored() {
        let config = make_config().set("wait", "false");
        let params = build_query_params(&config);
        assert!(params.is_empty());
    }

    #[test]
    fn test_build_query_params_thread_and_wait() {
        let config = make_config()
            .set("thread_id", "999")
            .set("wait", "true");
        let params = build_query_params(&config);
        assert_eq!(params.len(), 2);
    }

    // ---- parse_embed_color tests ----

    #[test]
    fn test_parse_embed_color_hex_only() {
        assert_eq!(parse_embed_color("FF0000"), Some(0xFF0000));
    }

    #[test]
    fn test_parse_embed_color_0x_prefix() {
        assert_eq!(parse_embed_color("0xFF0000"), Some(0xFF0000));
    }

    #[test]
    fn test_parse_embed_color_lowercase() {
        assert_eq!(parse_embed_color("ff0000"), Some(0xFF0000));
    }

    #[test]
    fn test_parse_embed_color_invalid() {
        assert_eq!(parse_embed_color("not-a-color"), None);
    }

    #[test]
    fn test_parse_embed_color_empty() {
        assert_eq!(parse_embed_color(""), None);
    }

    // ---- parse_embed_fields tests ----

    #[test]
    fn test_parse_embed_fields_single() {
        let fields = parse_embed_fields("CPU:85%");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0]["name"], "CPU");
        assert_eq!(fields[0]["value"], "85%");
        assert_eq!(fields[0]["inline"], false);
    }

    #[test]
    fn test_parse_embed_fields_multiple() {
        let fields = parse_embed_fields("CPU:85%, Memory:72%");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0]["name"], "CPU");
        assert_eq!(fields[0]["value"], "85%");
        assert_eq!(fields[1]["name"], "Memory");
        assert_eq!(fields[1]["value"], "72%");
    }

    #[test]
    fn test_parse_embed_fields_no_colon_ignored() {
        let fields = parse_embed_fields("no-colon, CPU:85%");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0]["name"], "CPU");
    }

    #[test]
    fn test_parse_embed_fields_empty() {
        let fields = parse_embed_fields("");
        assert!(fields.is_empty());
    }

    #[test]
    fn test_parse_embed_fields_whitespace_trimmed() {
        let fields = parse_embed_fields("  CPU  :  85%  ");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0]["name"], "CPU");
        assert_eq!(fields[0]["value"], "85%");
    }

    // ---- build_payload tests ----

    #[test]
    fn test_build_payload_text_plain() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("Hello Discord!");
        let config = ProviderConfig::new();
        let payload = provider.build_payload(&message, &config);
        assert_eq!(payload["content"], "Hello Discord!");
        assert!(payload.get("embeds").is_none());
    }

    #[test]
    fn test_build_payload_text_with_title() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("Details here").with_title("Alert");
        let config = ProviderConfig::new();
        let payload = provider.build_payload(&message, &config);
        // Text format with title still uses content
        assert_eq!(payload["content"], "Details here");
    }

    #[test]
    fn test_build_payload_markdown_no_title() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::markdown("Hello **bold**");
        let config = ProviderConfig::new();
        let payload = provider.build_payload(&message, &config);
        assert_eq!(payload["content"], "Hello **bold**");
    }

    #[test]
    fn test_build_payload_markdown_with_title() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::markdown("Hello **bold**").with_title("Alert");
        let config = ProviderConfig::new();
        let payload = provider.build_payload(&message, &config);
        // Markdown with title uses embeds
        assert!(payload.get("embeds").is_some());
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(embeds.len(), 1);
        assert_eq!(embeds[0]["title"], "Alert");
        assert_eq!(embeds[0]["description"], "Hello **bold**");
    }

    #[test]
    fn test_build_payload_with_embed_title() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("body text");
        let config = ProviderConfig::new().set("embed_title", "Custom Title");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(embeds[0]["title"], "Custom Title");
    }

    #[test]
    fn test_build_payload_with_embed_color() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("body");
        let config = ProviderConfig::new()
            .set("embed_title", "Title")
            .set("embed_color", "0xFF0000");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(embeds[0]["color"], 0xFF0000);
    }

    #[test]
    fn test_build_payload_with_embed_description() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("ignored body");
        let config = ProviderConfig::new()
            .set("embed_title", "Title")
            .set("embed_description", "Custom description");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(embeds[0]["description"], "Custom description");
    }

    #[test]
    fn test_build_payload_embed_description_fallback_to_text() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("message text");
        let config = ProviderConfig::new().set("embed_title", "Title");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        // When no embed_description, message.text is used
        assert_eq!(embeds[0]["description"], "message text");
    }

    #[test]
    fn test_build_payload_with_embed_footer() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("body");
        let config = ProviderConfig::new()
            .set("embed_title", "Title")
            .set("embed_footer", "Sent by bot");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(embeds[0]["footer"]["text"], "Sent by bot");
    }

    #[test]
    fn test_build_payload_with_embed_thumbnail() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("body");
        let config = ProviderConfig::new()
            .set("embed_title", "Title")
            .set("embed_thumbnail", "https://example.com/thumb.png");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(
            embeds[0]["thumbnail"]["url"],
            "https://example.com/thumb.png"
        );
    }

    #[test]
    fn test_build_payload_with_embed_fields() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("body");
        let config = ProviderConfig::new()
            .set("embed_title", "Status")
            .set("embed_field", "CPU:85%,Memory:72%");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        let fields = embeds[0]["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0]["name"], "CPU");
        assert_eq!(fields[0]["value"], "85%");
        assert_eq!(fields[1]["name"], "Memory");
        assert_eq!(fields[1]["value"], "72%");
    }

    #[test]
    fn test_build_payload_message_title_fills_embed_title_if_missing() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("body").with_title("Message Title");
        // When embed_color is set but embed_title is not, message.title fills in
        let config = ProviderConfig::new().set("embed_color", "0xFF0000");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(embeds[0]["title"], "Message Title");
    }

    #[test]
    fn test_build_payload_embed_title_takes_precedence_over_message_title() {
        let provider = DiscordProvider::new(Client::new());
        let message = Message::text("body").with_title("Message Title");
        let config =
            ProviderConfig::new().set("embed_title", "Embed Title").set("embed_color", "0xFF0000");
        let payload = provider.build_payload(&message, &config);
        let embeds = payload["embeds"].as_array().unwrap();
        assert_eq!(embeds[0]["title"], "Embed Title");
    }

    // ---- Provider metadata tests ----

    #[test]
    fn test_discord_provider_name() {
        let provider = DiscordProvider::new(Client::new());
        assert_eq!(provider.name(), "discord");
    }

    #[test]
    fn test_discord_provider_url_scheme() {
        let provider = DiscordProvider::new(Client::new());
        assert_eq!(provider.url_scheme(), "discord");
    }

    #[test]
    fn test_discord_provider_description() {
        let provider = DiscordProvider::new(Client::new());
        assert!(!provider.description().is_empty());
    }

    #[test]
    fn test_discord_provider_example_url() {
        let provider = DiscordProvider::new(Client::new());
        assert!(provider.example_url().contains("discord://"));
    }

    #[test]
    fn test_discord_provider_supports_attachments() {
        let provider = DiscordProvider::new(Client::new());
        assert!(provider.supports_attachments());
    }

    #[test]
    fn test_discord_provider_params_required() {
        let provider = DiscordProvider::new(Client::new());
        let params = provider.params();
        let required: Vec<_> = params.iter().filter(|p| p.required).collect();
        assert_eq!(required.len(), 2);
        let names: Vec<_> = required.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"webhook_id"));
        assert!(names.contains(&"webhook_token"));
    }

    #[test]
    fn test_discord_provider_params_optional() {
        let provider = DiscordProvider::new(Client::new());
        let params = provider.params();
        let optional: Vec<_> = params
            .iter()
            .filter(|p| !p.required)
            .map(|p| p.name.as_str())
            .collect();
        assert!(optional.contains(&"username"));
        assert!(optional.contains(&"avatar_url"));
        assert!(optional.contains(&"thread_id"));
        assert!(optional.contains(&"wait"));
        assert!(optional.contains(&"embed_title"));
        assert!(optional.contains(&"embed_color"));
        assert!(optional.contains(&"embed_description"));
        assert!(optional.contains(&"embed_footer"));
        assert!(optional.contains(&"embed_thumbnail"));
        assert!(optional.contains(&"embed_field"));
        assert!(optional.contains(&"base_url"));
    }

    #[test]
    fn test_discord_provider_params_count() {
        let provider = DiscordProvider::new(Client::new());
        assert_eq!(provider.params().len(), 13); // 2 required + 11 optional
    }

    // ---- Config validation tests ----

    #[test]
    fn test_validate_config_full() {
        let provider = DiscordProvider::new(Client::new());
        let config = ProviderConfig::new()
            .set("webhook_id", "123")
            .set("webhook_token", "abc");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_webhook_id() {
        let provider = DiscordProvider::new(Client::new());
        let config = ProviderConfig::new().set("webhook_token", "abc");
        assert!(provider.validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_missing_webhook_token() {
        let provider = DiscordProvider::new(Client::new());
        let config = ProviderConfig::new().set("webhook_id", "123");
        assert!(provider.validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_empty() {
        let provider = DiscordProvider::new(Client::new());
        assert!(provider.validate_config(&ProviderConfig::new()).is_err());
    }

    #[test]
    fn test_validate_config_with_optional_params() {
        let provider = DiscordProvider::new(Client::new());
        let config = ProviderConfig::new()
            .set("webhook_id", "123")
            .set("webhook_token", "abc")
            .set("username", "MyBot")
            .set("thread_id", "999")
            .set("wait", "true");
        assert!(provider.validate_config(&config).is_ok());
    }
}
