use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// PagerDuty Events API v2 provider.
///
/// Triggers incidents or sends change events via the PagerDuty Events API v2.
/// Supports image attachments via the `images` field (base64 data URI or URL).
pub struct PagerDutyProvider {
    client: Client,
}

impl PagerDutyProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for PagerDutyProvider {
    fn name(&self) -> &str {
        "pagerduty"
    }

    fn url_scheme(&self) -> &str {
        "pagerduty"
    }

    fn description(&self) -> &str {
        "PagerDuty Events API v2"
    }

    fn example_url(&self) -> &str {
        "pagerduty://<integration_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required(
                "integration_key",
                "PagerDuty Events API v2 integration/routing key",
            )
            .with_example("R015xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
            ParamDef::optional(
                "severity",
                "Event severity: critical, error, warning, info (default: info)",
            )
            .with_example("warning"),
            ParamDef::optional("source", "Source of the event (default: noti)")
                .with_example("noti"),
            ParamDef::optional("component", "Component responsible"),
            ParamDef::optional("group", "Logical grouping"),
            ParamDef::optional("class", "Event class/type"),
            ParamDef::optional(
                "action",
                "Event action: trigger, acknowledge, resolve (default: trigger)",
            )
            .with_example("trigger"),
            ParamDef::optional("dedup_key", "Deduplication key for event correlation"),
            ParamDef::optional("base_url", "Base URL override for API (default: https://events.pagerduty.com)")
                .with_example("http://localhost:8080"),
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
        let integration_key = config.require("integration_key", "pagerduty")?;
        let severity = config.get("severity").unwrap_or("info");
        let source = config.get("source").unwrap_or("noti");
        let action = config.get("action").unwrap_or("trigger");

        let url = config.get("base_url")
            .map(|s| format!("{}/v2/enqueue", s.trim_end_matches('/')))
            .unwrap_or_else(|| "https://events.pagerduty.com/v2/enqueue".to_string());

        let summary = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "routing_key": integration_key,
            "event_action": action,
            "payload": {
                "summary": summary,
                "severity": severity,
                "source": source,
            }
        });

        if let Some(component) = config.get("component") {
            payload["payload"]["component"] = json!(component);
        }
        if let Some(group) = config.get("group") {
            payload["payload"]["group"] = json!(group);
        }
        if let Some(class) = config.get("class") {
            payload["payload"]["class"] = json!(class);
        }
        if let Some(dedup_key) = config.get("dedup_key") {
            payload["dedup_key"] = json!(dedup_key);
        }

        // Add image attachments via the `images` field
        if message.has_attachments() {
            let mut images = Vec::new();
            let mut links = Vec::new();

            for attachment in &message.attachments {
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime_str = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let data_uri = format!("data:{mime_str};base64,{b64}");
                    images.push(json!({
                        "src": data_uri,
                        "alt": attachment.effective_file_name()
                    }));
                } else {
                    // Non-image files referenced as links
                    let file_name = attachment.effective_file_name();
                    links.push(json!({
                        "href": format!("attachment://{file_name}"),
                        "text": format!("📎 {file_name}")
                    }));
                }
            }

            if !images.is_empty() {
                payload["images"] = json!(images);
            }
            if !links.is_empty() {
                payload["links"] = json!(links);
            }
        }

        let resp = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let resp_status = raw
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("error");

        if resp_status == "success" {
            Ok(
                SendResponse::success("pagerduty", "event submitted successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let error = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("pagerduty", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
