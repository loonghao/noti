use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Sinch SMS/MMS provider.
///
/// Sends SMS/MMS messages via the Sinch REST API.
/// Supports MMS with media_body for image/video attachments.
pub struct SinchProvider {
    client: Client,
}

impl SinchProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for SinchProvider {
    fn name(&self) -> &str {
        "sinch"
    }

    fn url_scheme(&self) -> &str {
        "sinch"
    }

    fn description(&self) -> &str {
        "Sinch SMS/MMS via REST API"
    }

    fn example_url(&self) -> &str {
        "sinch://<service_plan_id>:<api_token>@<from>/<to>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("service_plan_id", "Sinch service plan ID")
                .with_example("your-plan-id"),
            ParamDef::required("api_token", "Sinch API token").with_example("your-api-token"),
            ParamDef::required("from", "Sender phone number or short code")
                .with_example("+15551234567"),
            ParamDef::required("to", "Recipient phone number (E.164 format)")
                .with_example("+15559876543"),
            ParamDef::optional("region", "API region: us or eu (default: us)").with_example("us"),
            ParamDef::optional(
                "media_url",
                "Public URL for MMS media (alternative to file attachments)",
            ),
            ParamDef::optional(
                "base_url",
                "Override API base URL (useful for testing or Twilio-compatible APIs)",
            )
            .with_example("https://api.sms.sinch.com"),
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
        let service_plan_id = config.require("service_plan_id", "sinch")?;
        let api_token = config.require("api_token", "sinch")?;
        let from = config.require("from", "sinch")?;
        let to = config.require("to", "sinch")?;
        let region = config.get("region").unwrap_or("us");

        let api_base_url = if let Some(base_url) = config.get("base_url") {
            base_url.to_string()
        } else {
            match region {
                "eu" => "https://eu.sms.api.sinch.com".to_string(),
                _ => "https://us.sms.api.sinch.com".to_string(),
            }
        };

        let url = format!("{api_base_url}/xms/v1/{service_plan_id}/batches");

        let body_text = if let Some(ref title) = message.title {
            format!("{title}\n\n{}", message.text)
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "from": from,
            "to": [to],
            "body": body_text
        });

        // Add MMS media attachments
        if message.has_attachments() {
            let mut media_body = Vec::new();
            for attachment in &message.attachments {
                if matches!(
                    attachment.kind,
                    AttachmentKind::Image | AttachmentKind::Video | AttachmentKind::Audio
                ) {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    media_body.push(json!({
                        "url": format!("data:{mime};base64,{b64}"),
                        "content_type": mime
                    }));
                }
            }
            if !media_body.is_empty() {
                payload["type"] = json!("mt_media");
                payload["media_body"] = json!(media_body);
            }
        } else if let Some(media_url) = config.get("media_url") {
            payload["type"] = json!("mt_media");
            payload["media_body"] = json!([{
                "url": media_url
            }]);
        }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(api_token)
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
            let batch_id = raw.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
            let msg = if message.has_attachments() {
                format!("MMS sent with attachments (batch: {batch_id})")
            } else {
                format!("SMS sent (batch: {batch_id})")
            };
            Ok(SendResponse::success("sinch", msg)
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("sinch", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
