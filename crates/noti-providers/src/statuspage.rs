use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Atlassian Statuspage.io incident notification provider.
///
/// Statuspage lets you create and manage incidents on your status page.
/// This provider creates incidents via the Statuspage REST API.
/// Supports image attachments by embedding base64 data URIs as markdown
/// images in the incident body.
///
/// API Reference: <https://developer.statuspage.io/#tag/incidents>
pub struct StatuspageProvider {
    client: Client,
}

impl StatuspageProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for StatuspageProvider {
    fn name(&self) -> &str {
        "statuspage"
    }

    fn url_scheme(&self) -> &str {
        "statuspage"
    }

    fn description(&self) -> &str {
        "Atlassian Statuspage.io incident management"
    }

    fn example_url(&self) -> &str {
        "statuspage://<api_key>@<page_id>"
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Statuspage API key (OAuth token)")
                .with_example("your_api_key"),
            ParamDef::required("page_id", "Statuspage page ID").with_example("your_page_id"),
            ParamDef::optional(
                "status",
                "Incident status: investigating, identified, monitoring, resolved (default: investigating)",
            )
            .with_example("investigating"),
            ParamDef::optional(
                "impact",
                "Incident impact: none, minor, major, critical (default: minor)",
            )
            .with_example("minor"),
            ParamDef::optional("component_ids", "Comma-separated component IDs to affect"),
            ParamDef::optional(
                "component_status",
                "Component status: operational, degraded_performance, partial_outage, major_outage",
            )
            .with_example("degraded_performance"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let api_key = config.require("api_key", "statuspage")?;
        let page_id = config.require("page_id", "statuspage")?;

        let status = config.get("status").unwrap_or("investigating");
        let impact = config.get("impact").unwrap_or("minor");

        let url = format!("https://api.statuspage.io/v1/pages/{page_id}/incidents");

        let incident_name = message
            .title
            .as_deref()
            .unwrap_or("Incident reported by noti");

        // Build body with embedded attachments
        let mut body = message.text.clone();
        if message.has_attachments() {
            body.push('\n');
            for attachment in &message.attachments {
                if attachment.kind == AttachmentKind::Image {
                    if let Ok(data) = attachment.read_bytes().await {
                        let mime = attachment.effective_mime();
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                        body.push_str(&format!(
                            "\n![{}](data:{mime};base64,{b64})",
                            attachment.effective_file_name()
                        ));
                    }
                } else {
                    body.push_str(&format!("\n📎 {}", attachment.effective_file_name()));
                }
            }
        }

        let mut incident = json!({
            "incident": {
                "name": incident_name,
                "status": status,
                "impact_override": impact,
                "body": body,
            }
        });

        // Add component IDs and status if provided
        if let Some(component_ids) = config.get("component_ids") {
            let ids: Vec<&str> = component_ids.split(',').map(|s| s.trim()).collect();
            incident["incident"]["component_ids"] = json!(ids);

            if let Some(comp_status) = config.get("component_status") {
                let mut components = serde_json::Map::new();
                for id in &ids {
                    components.insert(id.to_string(), json!(comp_status));
                }
                incident["incident"]["components"] = json!(components);
            }
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("OAuth {api_key}"))
            .json(&incident)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status_code = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or_else(|_| json!({"status": status_code}));

        if (200..300).contains(&status_code) {
            Ok(
                SendResponse::success("statuspage", "incident created successfully")
                    .with_status_code(status_code)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("statuspage", format!("API error: {msg}"))
                    .with_status_code(status_code)
                    .with_raw_response(raw),
            )
        }
    }
}
