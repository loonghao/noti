use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Form webhook provider.
///
/// Sends a form-encoded notification payload to any HTTP endpoint.
/// Supports file attachments via multipart/form-data upload.
///
/// The form fields:
/// - `title` (if provided)
/// - `message` (the notification text)
/// - `type` (default: "info")
/// - `file[]` (attachments, when present switches to multipart/form-data)
pub struct FormWebhookProvider {
    client: Client,
}

impl FormWebhookProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for FormWebhookProvider {
    fn name(&self) -> &str {
        "form"
    }

    fn url_scheme(&self) -> &str {
        "form"
    }

    fn description(&self) -> &str {
        "Generic form webhook (POST form-encoded payload to any URL)"
    }

    fn example_url(&self) -> &str {
        "form://<host>/<path>"
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
        let default_url = config.require("url", "form")?;
        let target_url = config
            .get("base_url")
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| default_url.trim_end_matches('/').to_string());
        let method = config.get("method").unwrap_or("POST").to_uppercase();
        let noti_type = config.get("type").unwrap_or("info");

        let resp = if message.has_attachments() {
            // Use multipart form for file attachments
            let mut form = reqwest::multipart::Form::new()
                .text("message", message.text.clone())
                .text("type", noti_type.to_string());

            if let Some(ref title) = message.title {
                form = form.text("title", title.clone());
            }

            for attachment in &message.attachments {
                let data = attachment.read_bytes().await?;
                let file_name = attachment.effective_file_name();
                let mime_str = attachment.effective_mime();

                let part = reqwest::multipart::Part::bytes(data)
                    .file_name(file_name)
                    .mime_str(&mime_str)
                    .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

                form = form.part("file[]", part);
            }

            let mut request = match method.as_str() {
                "PUT" => self.client.put(&target_url),
                "PATCH" => self.client.patch(&target_url),
                _ => self.client.post(&target_url),
            };

            if let Some(headers) = config.get("header") {
                for pair in headers.split(';') {
                    if let Some((k, v)) = pair.split_once('=') {
                        request = request.header(k.trim(), v.trim());
                    }
                }
            }

            request
                .multipart(form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?
        } else {
            // Standard form-encoded request
            let mut form_data: Vec<(&str, &str)> =
                vec![("message", &message.text), ("type", noti_type)];

            let title_val;
            if let Some(ref title) = message.title {
                title_val = title.clone();
                form_data.push(("title", &title_val));
            }

            let mut request = match method.as_str() {
                "PUT" => self.client.put(&target_url),
                "PATCH" => self.client.patch(&target_url),
                _ => self.client.post(&target_url),
            };

            if let Some(headers) = config.get("header") {
                for pair in headers.split(';') {
                    if let Some((k, v)) = pair.split_once('=') {
                        request = request.header(k.trim(), v.trim());
                    }
                }
            }

            request
                .form(&form_data)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?
        };

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        let raw = serde_json::from_str::<serde_json::Value>(&body)
            .unwrap_or_else(|_| json!({"body": body}));

        if (200..300).contains(&(status as usize)) {
            Ok(
                SendResponse::success("form", "form webhook delivered successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            Ok(
                SendResponse::failure("form", format!("HTTP {status}: {body}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
