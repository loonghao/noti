use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Opsgenie provider.
///
/// Creates alerts via the Opsgenie Alert API v2.
/// Requires a GenieKey (API key) from Opsgenie Integration settings.
///
/// API reference: <https://docs.opsgenie.com/docs/alert-api#create-alert>
pub struct OpsgenieProvider {
    client: Client,
}

impl OpsgenieProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for OpsgenieProvider {
    fn name(&self) -> &str {
        "opsgenie"
    }

    fn url_scheme(&self) -> &str {
        "opsgenie"
    }

    fn description(&self) -> &str {
        "Atlassian Opsgenie alerts via API v2"
    }

    fn example_url(&self) -> &str {
        "opsgenie://<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Opsgenie GenieKey (integration API key)")
                .with_example("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"),
            ParamDef::optional("region", "API region: 'us' (default) or 'eu'").with_example("us"),
            ParamDef::optional("priority", "Alert priority: P1-P5 (default: P3)")
                .with_example("P2"),
            ParamDef::optional("alias", "Alert alias for deduplication")
                .with_example("my-alert-001"),
            ParamDef::optional("tags", "Comma-separated tags").with_example("critical,network"),
            ParamDef::optional("entity", "Entity field for the alert").with_example("my-service"),
            ParamDef::optional("responders", "Comma-separated responder names or IDs")
                .with_example("team1,user@example.com"),
            ParamDef::optional("base_url", "Base URL override for API (default: auto from region)")
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
        let api_key = config.require("api_key", "opsgenie")?;
        let region = config.get("region").unwrap_or("us");

        let base_url = config.get("base_url").map(|s| s.trim_end_matches('/').to_string()).unwrap_or_else(|| {
            match region {
                "eu" => "https://api.eu.opsgenie.com".to_string(),
                _ => "https://api.opsgenie.com".to_string(),
            }
        });
        let url = format!("{base_url}/v2/alerts");

        let alert_message = message
            .title
            .clone()
            .unwrap_or_else(|| message.text.chars().take(130).collect());

        let mut payload = json!({
            "message": alert_message,
            "description": message.text,
        });

        if let Some(priority) = config.get("priority") {
            payload["priority"] = json!(priority);
        }

        if let Some(alias) = config.get("alias") {
            payload["alias"] = json!(alias);
        }

        if let Some(tags) = config.get("tags") {
            let tag_list: Vec<&str> = tags.split(',').map(|s| s.trim()).collect();
            payload["tags"] = json!(tag_list);
        }

        if let Some(entity) = config.get("entity") {
            payload["entity"] = json!(entity);
        }

        if let Some(responders) = config.get("responders") {
            let resp_list: Vec<serde_json::Value> = responders
                .split(',')
                .map(|s| {
                    let s = s.trim();
                    if s.contains('@') {
                        json!({"username": s, "type": "user"})
                    } else {
                        json!({"name": s, "type": "team"})
                    }
                })
                .collect();
            payload["responders"] = json!(resp_list);
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("GenieKey {api_key}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            let request_id = raw
                .get("requestId")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            // Upload attachments if present (using alert alias or dedup key)
            if message.has_attachments() {
                // OpsGenie needs an alert identifier to attach files
                let alert_id = config
                    .get("alias")
                    .map(|s| format!("alias={s}"))
                    .unwrap_or_else(|| request_id.to_string());

                let attach_url = format!("{base_url}/v2/alerts/{alert_id}/attachments");

                for attachment in &message.attachments {
                    let data = attachment.read_bytes().await?;
                    let file_name = attachment.effective_file_name();
                    let mime_str = attachment.effective_mime();

                    let part = reqwest::multipart::Part::bytes(data)
                        .file_name(file_name)
                        .mime_str(&mime_str)
                        .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

                    let form = reqwest::multipart::Form::new().part("file", part);

                    let _attach_resp = self
                        .client
                        .post(&attach_url)
                        .header("Authorization", format!("GenieKey {api_key}"))
                        .multipart(form)
                        .send()
                        .await
                        .map_err(|e| NotiError::Network(e.to_string()))?;
                }
            }

            let msg = if message.has_attachments() {
                format!("alert created with attachments (requestId: {request_id})")
            } else {
                format!("alert created (requestId: {request_id})")
            };
            Ok(SendResponse::success("opsgenie", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error_msg = raw
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("opsgenie", format!("API error: {error_msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
