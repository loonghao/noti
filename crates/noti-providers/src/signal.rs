use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Signal Messenger notification provider via signal-cli REST API.
///
/// Uses the signal-cli REST API to send messages through Signal.
/// Requires a running signal-cli-rest-api instance.
/// Supports file attachments via base64 encoding.
///
/// API docs: <https://github.com/bbernhard/signal-cli-rest-api>
pub struct SignalProvider {
    client: Client,
}

impl SignalProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SignalProvider {
    fn name(&self) -> &str {
        "signal"
    }

    fn url_scheme(&self) -> &str {
        "signal"
    }

    fn description(&self) -> &str {
        "Signal Messenger via signal-cli REST API"
    }

    fn example_url(&self) -> &str {
        "signal://<from_number>/<to_number>?server=http://localhost:8080"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("from", "Sender phone number (registered in signal-cli)")
                .with_example("+1234567890"),
            ParamDef::required("to", "Recipient phone number or group ID")
                .with_example("+0987654321"),
            ParamDef::optional(
                "server",
                "signal-cli REST API server URL (default: http://localhost:8080)",
            )
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
        let from = config.require("from", "signal")?;
        let to = config.require("to", "signal")?;
        let server = config.get("server").unwrap_or("http://localhost:8080");

        let url = format!("{}/v2/send", server.trim_end_matches('/'));

        let mut text = message.text.clone();
        if let Some(ref title) = message.title {
            text = format!("*{title}*\n\n{text}");
        }

        let mut payload = json!({
            "message": text,
            "number": from,
            "recipients": [to],
        });

        // Add base64-encoded attachments
        if message.has_attachments() {
            let mut base64_attachments = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                let mime_str = attachment.effective_mime();
                let file_name = attachment.effective_file_name();
                base64_attachments.push(json!({
                    "contentType": mime_str,
                    "filename": file_name,
                    "base64": b64,
                }));
            }
            payload["base64_attachments"] = json!(base64_attachments);
        }

        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .unwrap_or(json!({"error": "failed to parse response"}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("signal", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("signal", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
