use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Threema Gateway provider.
///
/// Sends text messages via Threema Gateway (basic mode).
/// Supports file attachments via the blob upload API.
///
/// API reference: <https://gateway.threema.ch/en/developer/api>
pub struct ThreemaProvider {
    client: Client,
}

impl ThreemaProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ThreemaProvider {
    fn name(&self) -> &str {
        "threema"
    }

    fn url_scheme(&self) -> &str {
        "threema"
    }

    fn description(&self) -> &str {
        "Threema Gateway secure messaging"
    }

    fn example_url(&self) -> &str {
        "threema://<gateway_id>:<api_secret>@<recipient_id>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("gateway_id", "Threema Gateway ID (starts with *)")
                .with_example("*MY_GW"),
            ParamDef::required("api_secret", "Threema Gateway API secret"),
            ParamDef::required("to", "Recipient Threema ID (8 characters)")
                .with_example("ABCD1234"),
            ParamDef::optional("to_phone", "Recipient phone number (alternative lookup)"),
            ParamDef::optional("to_email", "Recipient email (alternative lookup)"),
            ParamDef::optional("base_url", "Threema Gateway API base URL (default: https://msgapi.threema.ch)")
                .with_example("https://msgapi.threema.ch"),
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
        let gateway_id = config.require("gateway_id", "threema")?;
        let api_secret = config.require("api_secret", "threema")?;

        let base = config.get("base_url").unwrap_or("https://msgapi.threema.ch");
        let base_url = base.trim_end_matches('/');

        // If has attachments, upload blob and send file message
        if message.has_attachments() {
            let attachment = &message.attachments[0];
            let data = attachment.read_bytes().await?;

            // Upload blob
            let blob_url = format!("{base_url}/upload_blob");
            let part = reqwest::multipart::Part::bytes(data)
                .file_name(attachment.effective_file_name())
                .mime_str(&attachment.effective_mime())
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;

            let form = reqwest::multipart::Form::new().part("blob", part);

            let blob_resp = self
                .client
                .post(blob_url)
                .multipart(form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            let blob_status = blob_resp.status().as_u16();
            let blob_id = blob_resp
                .text()
                .await
                .map_err(|e| NotiError::Network(format!("failed to read blob response: {e}")))?;

            if !(200..300).contains(&(blob_status as usize)) {
                return Ok(SendResponse::failure(
                    "threema",
                    format!("blob upload failed: {blob_id}"),
                )
                .with_status_code(blob_status));
            }

            // Send file message referencing the blob
            let send_url = format!("{base_url}/send_simple");
            let text = if !message.text.is_empty() {
                format!(
                    "{}\n\n[Attachment: {}]",
                    message.text,
                    attachment.effective_file_name()
                )
            } else {
                format!("[Attachment: {}]", attachment.effective_file_name())
            };

            let mut send_form = vec![
                ("from", gateway_id.to_string()),
                ("secret", api_secret.to_string()),
                ("text", text),
            ];

            if let Some(to_phone) = config.get("to_phone") {
                send_form.push(("phone", to_phone.to_string()));
            } else if let Some(to_email) = config.get("to_email") {
                send_form.push(("email", to_email.to_string()));
            } else {
                let to = config.require("to", "threema")?;
                send_form.push(("to", to.to_string()));
            }

            let resp = self
                .client
                .post(send_url)
                .form(&send_form)
                .send()
                .await
                .map_err(|e| NotiError::Network(e.to_string()))?;

            let status = resp.status().as_u16();
            let body = resp
                .text()
                .await
                .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

            return if (200..300).contains(&(status as usize)) {
                Ok(
                    SendResponse::success("threema", "message with attachment sent")
                        .with_status_code(status)
                        .with_raw_response(serde_json::json!({"message_id": body.trim(), "blob_id": blob_id.trim()})),
                )
            } else {
                Ok(
                    SendResponse::failure("threema", format!("API error ({status}): {body}"))
                        .with_status_code(status)
                        .with_raw_response(serde_json::json!({"body": body})),
                )
            };
        }

        let url = format!("{base_url}/send_simple");

        let text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut form = vec![
            ("from", gateway_id.to_string()),
            ("secret", api_secret.to_string()),
            ("text", text),
        ];

        if let Some(to_phone) = config.get("to_phone") {
            form.push(("phone", to_phone.to_string()));
        } else if let Some(to_email) = config.get("to_email") {
            form.push(("email", to_email.to_string()));
        } else {
            let to = config.require("to", "threema")?;
            form.push(("to", to.to_string()));
        }

        let resp = self
            .client
            .post(url)
            .form(&form)
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
                SendResponse::success("threema", "message sent via Threema Gateway")
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"message_id": body.trim()})),
            )
        } else {
            Ok(
                SendResponse::failure("threema", format!("API error ({status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"body": body})),
            )
        }
    }
}
