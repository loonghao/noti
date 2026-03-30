use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Notifiarr notification provider.
///
/// Notifiarr is a notification aggregation service popular in the media server
/// community (Plex, Sonarr, Radarr, Lidarr, etc). It provides pass-through
/// notifications and integrates with Discord, Slack, Telegram, etc.
///
/// API Reference: <https://notifiarr.wiki/en/Website/Integrations/Passthrough>
pub struct NotifiarrProvider {
    client: Client,
}

impl NotifiarrProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for NotifiarrProvider {
    fn name(&self) -> &str {
        "notifiarr"
    }

    fn url_scheme(&self) -> &str {
        "notifiarr"
    }

    fn description(&self) -> &str {
        "Notifiarr media server notification aggregation"
    }

    fn example_url(&self) -> &str {
        "notifiarr://<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Notifiarr API key").with_example("your_api_key"),
            ParamDef::optional(
                "notification_type",
                "Notification type (default: passthrough)",
            )
            .with_example("passthrough"),
            ParamDef::optional("discord_channel", "Discord channel ID for routing"),
            ParamDef::optional("color", "Notification color (hex)").with_example("#00FF00"),
            ParamDef::optional("ping_user", "Discord user ID to ping"),
            ParamDef::optional("ping_role", "Discord role ID to ping"),
            ParamDef::optional("image", "Image URL to include"),
        ]
    }

    fn supports_attachments(&self) -> bool {
        false
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "notifiarr")?;

        let url = "https://notifiarr.com/api/v1/notification/passthrough";

        let mut notification = json!({
            "notification": {
                "update": false,
                "name": message.title.as_deref().unwrap_or("noti"),
            },
            "discord": {
                "text": {
                    "content": "",
                },
                "embeds": [{
                    "title": message.title.as_deref().unwrap_or("Notification"),
                    "text": message.text,
                }],
            },
        });

        if let Some(color) = config.get("color") {
            // Convert hex color to decimal
            let color_str = color.trim_start_matches('#');
            if let Ok(color_val) = u32::from_str_radix(color_str, 16) {
                notification["discord"]["embeds"][0]["color"] = json!(color_val);
            }
        }

        if let Some(channel) = config.get("discord_channel") {
            notification["discord"]["ids"] = json!({"channel": channel});
        }

        if let Some(user) = config.get("ping_user") {
            notification["discord"]["text"]["content"] = json!(format!("<@{user}>"));
        }

        if let Some(role) = config.get("ping_role") {
            notification["discord"]["text"]["content"] = json!(format!("<@&{role}>"));
        }

        if let Some(image) = config.get("image") {
            notification["discord"]["embeds"][0]["image"] = json!({"url": image});
        } else if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            let data_uri = format!("data:{mime};base64,{b64}");
            notification["discord"]["embeds"][0]["image"] = json!({"url": data_uri});
        }

        // Add non-image attachment info to the embed description
        let non_image_attachments: Vec<_> = message
            .attachments
            .iter()
            .filter(|a| a.kind != AttachmentKind::Image)
            .collect();
        if !non_image_attachments.is_empty() {
            let mut desc = message.text.clone();
            for att in &non_image_attachments {
                desc.push_str(&format!("\n📎 **Attachment:** {}", att.effective_file_name()));
            }
            notification["discord"]["embeds"][0]["text"] = json!(desc);
        }

        let resp = self
            .client
            .post(url)
            .header("x-api-key", api_key)
            .json(&notification)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"status": status}));

        let result_str = raw.get("result").and_then(|v| v.as_str()).unwrap_or("");

        if result_str == "success" || (200..300).contains(&status) {
            Ok(
                SendResponse::success("notifiarr", "notification sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("details")
                .and_then(|v| v.get("response"))
                .and_then(|v| v.as_str())
                .or_else(|| raw.get("result").and_then(|v| v.as_str()))
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("notifiarr", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
