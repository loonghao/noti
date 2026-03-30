use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Seven (formerly sms77) SMS provider.
///
/// Sends SMS via the Seven.io REST API. Supports file attachments via the
/// `files` parameter — each file is base64-encoded and a placeholder
/// `[[filename]]` is inserted into the SMS text. Seven generates a download
/// link that replaces the placeholder.
///
/// API reference: <https://docs.seven.io/en/rest-api/endpoints/sms>
pub struct SevenProvider {
    client: Client,
}

impl SevenProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SevenProvider {
    fn name(&self) -> &str {
        "seven"
    }

    fn url_scheme(&self) -> &str {
        "seven"
    }

    fn description(&self) -> &str {
        "Seven (sms77) SMS gateway"
    }

    fn example_url(&self) -> &str {
        "seven://<api_key>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Seven.io API key"),
            ParamDef::required("to", "Recipient phone number (E.164)").with_example("+15559876543"),
            ParamDef::optional("from", "Sender name or phone number").with_example("noti"),
            ParamDef::optional("flash", "Send as flash SMS: 1 or 0 (default: 0)"),
            ParamDef::optional("foreign_id", "Your custom foreign ID for tracking"),
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
        let api_key = config.require("api_key", "seven")?;
        let to = config.require("to", "seven")?;

        let url = "https://gateway.seven.io/api/sms";

        let mut text = if let Some(ref title) = message.title {
            format!("{title}: {}", message.text)
        } else {
            message.text.clone()
        };

        // Append file placeholders to the text for each attachment
        for attachment in &message.attachments {
            let file_name = attachment.effective_file_name();
            text.push_str(&format!(" [[{file_name}]]"));
        }

        let mut payload = json!({
            "to": to,
            "text": text,
            "json": 1,
        });

        if let Some(from) = config.get("from") {
            payload["from"] = json!(from);
        }
        if let Some(flash) = config.get("flash") {
            payload["flash"] = json!(flash);
        }
        if let Some(foreign_id) = config.get("foreign_id") {
            payload["foreign_id"] = json!(foreign_id);
        }

        // Add file attachments as base64-encoded content
        if message.has_attachments() {
            let mut files = Vec::new();
            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let file_name = attachment.effective_file_name();
                files.push(json!({
                    "name": file_name,
                    "contents": b64,
                }));
            }
            payload["files"] = json!(files);
        }

        let resp = self
            .client
            .post(url)
            .header("X-Api-Key", api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let success_code = raw.get("success").and_then(|v| v.as_str()).unwrap_or("");

        if success_code == "100" || (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("seven", "SMS sent via Seven.io")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let msg = raw
                .get("messages")
                .and_then(|m| m.as_array())
                .and_then(|arr| arr.first())
                .and_then(|m| m.get("error_text"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("seven", format!("API error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}
