use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// PagerTree incident management provider.
///
/// Creates incidents/alerts via PagerTree integration webhook.
/// Supports image attachments embedded as base64 data URI in description.
///
/// API reference: <https://pagertree.com/docs/integration/incoming>
pub struct PagerTreeProvider {
    client: Client,
}

impl PagerTreeProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PagerTreeProvider {
    fn name(&self) -> &str {
        "pagertree"
    }

    fn url_scheme(&self) -> &str {
        "pagertree"
    }

    fn description(&self) -> &str {
        "PagerTree incident management webhook"
    }

    fn example_url(&self) -> &str {
        "pagertree://<integration_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("integration_id", "PagerTree integration ID")
                .with_example("your-integration-id"),
            ParamDef::optional("urgency", "Incident urgency: low, medium, high, critical")
                .with_example("high"),
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
        let integration_id = config.require("integration_id", "pagertree")?;

        let url = format!("https://api.pagertree.com/integration/{integration_id}");

        let title = message.title.as_deref().unwrap_or("Alert");
        let urgency = config.get("urgency").unwrap_or("high");

        // Embed image attachments as base64 in description
        let mut description = message.text.clone();
        for attachment in &message.attachments {
            if attachment.kind == noti_core::AttachmentKind::Image {
                if let Ok(data) = attachment.read_bytes().await {
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let name = attachment.effective_file_name();
                    description.push_str(&format!(
                        "\n\n![{name}](data:{mime};base64,{b64})"
                    ));
                }
            }
        }

        let payload = json!({
            "event_type": "create",
            "Id": format!("noti-{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()),
            "Title": title,
            "Description": description,
            "Urgency": urgency,
        });

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("pagertree", "incident created in PagerTree")
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        } else {
            Ok(
                SendResponse::failure("pagertree", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({"body": body})),
            )
        }
    }
}
