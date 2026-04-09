use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// JSON webhook provider.
///
/// Sends a JSON-formatted notification payload to any HTTP endpoint.
/// Supports file attachments as base64-encoded data in the JSON payload.
///
/// The JSON body structure:
/// ```json
/// {
///   "title": "...",
///   "message": "...",
///   "type": "info",
///   "attachments": [{"name": "file.png", "mime": "image/png", "data": "base64..."}]
/// }
/// ```
pub struct JsonWebhookProvider {
    client: Client,
}

impl JsonWebhookProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for JsonWebhookProvider {
    fn name(&self) -> &str {
        "json"
    }

    fn url_scheme(&self) -> &str {
        "json"
    }

    fn description(&self) -> &str {
        "Generic JSON webhook (POST JSON payload to any URL)"
    }

    fn example_url(&self) -> &str {
        "json://<host>/<path>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("url", "Target webhook URL")
                .with_example("https://example.com/api/notify"),
            ParamDef::optional("method", "HTTP method (default: POST)").with_example("POST"),
            ParamDef::optional(
                "header",
                "Extra headers as key=value pairs, semicolon-separated",
            )
            .with_example("X-Api-Key=abc;X-Custom=val"),
            ParamDef::optional("type", "Notification type field (default: info)")
                .with_example("warning"),
            ParamDef::optional("base_url", "Override the target URL (takes precedence over url)")
                .with_example("https://example.com/api/notify"),
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
        let default_url = config.require("url", "json")?;
        let target_url = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| default_url.trim_end_matches('/').to_string());
        let method = config.get("method").unwrap_or("POST").to_uppercase();
        let noti_type = config.get("type").unwrap_or("info");

        let mut payload = json!({
            "message": message.text,
            "type": noti_type,
        });

        if let Some(ref title) = message.title {
            payload["title"] = json!(title);
        }

        // Add attachments as base64-encoded data in the JSON payload
        if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                attachments_json.push(json!({
                    "name": attachment.effective_file_name(),
                    "mime": attachment.effective_mime(),
                    "data": b64,
                }));
            }
            payload["attachments"] = json!(attachments_json);
        }

        let mut request = match method.as_str() {
            "PUT" => self.client.put(&target_url),
            "PATCH" => self.client.patch(&target_url),
            _ => self.client.post(&target_url),
        };

        // Add custom headers
        if let Some(headers) = config.get("header") {
            for pair in headers.split(';') {
                if let Some((k, v)) = pair.split_once('=') {
                    request = request.header(k.trim(), v.trim());
                }
            }
        }

        let resp = request
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        let raw = serde_json::from_str::<serde_json::Value>(&body)
            .unwrap_or_else(|_| json!({"body": body}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("json", "JSON webhook delivered successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("json", format!("HTTP {status}: {body}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
