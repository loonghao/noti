use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Microsoft Teams incoming webhook provider.
///
/// Uses the Power Automate / Workflows webhook connector (the modern replacement
/// for the deprecated Office 365 connectors).  The webhook URL is the full
/// URL obtained when configuring a "Post to a channel when a webhook request
/// is received" workflow in Teams.
///
/// Supports image attachments via Adaptive Card Image elements (base64 data URI).
pub struct TeamsProvider {
    client: Client,
}

impl TeamsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for TeamsProvider {
    fn name(&self) -> &str {
        "teams"
    }

    fn url_scheme(&self) -> &str {
        "teams"
    }

    fn description(&self) -> &str {
        "Microsoft Teams via incoming webhook"
    }

    fn example_url(&self) -> &str {
        "teams://<webhook_url_encoded>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("webhook_url", "Teams incoming webhook URL")
                .with_example("https://xxx.webhook.office.com/webhookb2/..."),
            ParamDef::optional("theme_color", "Hex color for the card accent (e.g. 0076D7)"),
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
        let webhook_url = config.require("webhook_url", "teams")?;
        let theme_color = config.get("theme_color").unwrap_or("0076D7");

        let text_block = json!({
            "type": "TextBlock",
            "text": message.text,
            "wrap": true
        });

        let mut body_items = Vec::new();

        // Add title if present
        if let Some(ref title) = message.title {
            body_items.push(json!({
                "type": "TextBlock",
                "size": "Medium",
                "weight": "Bolder",
                "text": title,
                "color": "Accent"
            }));
        }

        body_items.push(text_block);

        // Add image attachments as Adaptive Card Image elements
        if message.has_attachments() {
            for attachment in &message.attachments {
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime_str = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let data_uri = format!("data:{mime_str};base64,{b64}");

                    body_items.push(json!({
                        "type": "Image",
                        "url": data_uri,
                        "altText": attachment.effective_file_name(),
                        "size": "Large"
                    }));
                } else {
                    // For non-image files, show as a text block with filename
                    let file_name = attachment.effective_file_name();
                    body_items.push(json!({
                        "type": "TextBlock",
                        "text": format!("📎 Attachment: {file_name}"),
                        "wrap": true,
                        "isSubtle": true
                    }));
                }
            }
        }

        let payload = json!({
            "type": "message",
            "attachments": [{
                "contentType": "application/vnd.microsoft.card.adaptive",
                "content": {
                    "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
                    "type": "AdaptiveCard",
                    "version": "1.4",
                    "body": body_items,
                    "msteams": {
                        "width": "Full",
                        "themeColor": theme_color
                    }
                }
            }]
        });

        let resp = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("teams", e))?;

        let status = resp.status().as_u16();

        // Check for 429 rate limiting
        if status == 429 {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::http_helpers::handle_http_error(
                "teams",
                status,
                &body,
                retry_after.as_deref(),
            ));
        }

        let body_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(SendResponse::success("teams", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(json!({ "body": body_text })))
        } else {
            Ok(
                SendResponse::failure("teams", format!("API error (HTTP {status}): {body_text}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body_text })),
            )
        }
    }
}
