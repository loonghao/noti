use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helper functions (exported for unit testing)
// ---------------------------------------------------------------------------

/// Build the Sinch API URL from service_plan_id and base URL.
pub fn sinch_url(service_plan_id: &str, api_base_url: &str) -> String {
    format!("{api_base_url}/xms/v1/{service_plan_id}/batches")
}

/// Format message body text, prepending title if present.
pub fn sinch_body_text(message: &Message) -> String {
    if let Some(ref title) = message.title {
        format!("{title}\n\n{}", message.text)
    } else {
        message.text.clone()
    }
}

/// Get the Sinch API base URL for a given region.
pub fn sinch_region_base_url(region: &str) -> String {
    match region {
        "eu" => "https://eu.sms.api.sinch.com".to_string(),
        _ => "https://us.sms.api.sinch.com".to_string(),
    }
}

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
            sinch_region_base_url(region)
        };

        let url = sinch_url(service_plan_id, &api_base_url);
        let body_text = sinch_body_text(message);

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
            .map_err(|e| crate::http_helpers::classify_reqwest_error("sinch", e))?;

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

// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // sinch_url tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sinch_url_basic() {
        let url = sinch_url("plan123", "https://us.sms.api.sinch.com");
        assert_eq!(url, "https://us.sms.api.sinch.com/xms/v1/plan123/batches");
    }

    #[test]
    fn test_sinch_url_with_custom_base() {
        let url = sinch_url("my-plan", "https://custom.api.example.com");
        assert_eq!(url, "https://custom.api.example.com/xms/v1/my-plan/batches");
    }

    #[test]
    fn test_sinch_url_with_eu_base() {
        let url = sinch_url("eu-plan", "https://eu.sms.api.sinch.com");
        assert_eq!(url, "https://eu.sms.api.sinch.com/xms/v1/eu-plan/batches");
    }

    // -------------------------------------------------------------------------
    // sinch_body_text tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sinch_body_text_plain() {
        let msg = Message::text("hello world");
        assert_eq!(sinch_body_text(&msg), "hello world");
    }

    #[test]
    fn test_sinch_body_text_with_title() {
        let msg = Message::text("hello world").with_title("My Title");
        assert_eq!(sinch_body_text(&msg), "My Title\n\nhello world");
    }

    #[test]
    fn test_sinch_body_text_empty_body_with_title() {
        let msg = Message::text("").with_title("Title Only");
        assert_eq!(sinch_body_text(&msg), "Title Only\n\n");
    }

    #[test]
    fn test_sinch_body_text_empty_title() {
        let msg = Message {
            text: "body only".into(),
            title: Some("".into()),
            format: Default::default(),
            priority: Default::default(),
            attachments: vec![],
            extra: Default::default(),
        };
        // Empty title still gets formatted (title is Some(""))
        assert_eq!(sinch_body_text(&msg), "\n\nbody only");
    }

    // -------------------------------------------------------------------------
    // sinch_region_base_url tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sinch_region_base_url_us() {
        assert_eq!(
            sinch_region_base_url("us"),
            "https://us.sms.api.sinch.com"
        );
    }

    #[test]
    fn test_sinch_region_base_url_eu() {
        assert_eq!(
            sinch_region_base_url("eu"),
            "https://eu.sms.api.sinch.com"
        );
    }

    #[test]
    fn test_sinch_region_base_url_unknown_defaults_to_us() {
        // Unknown region falls back to US
        assert_eq!(
            sinch_region_base_url("ap"),
            "https://us.sms.api.sinch.com"
        );
        assert_eq!(
            sinch_region_base_url(""),
            "https://us.sms.api.sinch.com"
        );
        assert_eq!(
            sinch_region_base_url("unknown"),
            "https://us.sms.api.sinch.com"
        );
    }

    // -------------------------------------------------------------------------
    // SinchProvider trait implementation tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_sinch_provider_name() {
        let provider = SinchProvider::new(reqwest::Client::new());
        assert_eq!(provider.name(), "sinch");
    }

    #[test]
    fn test_sinch_provider_url_scheme() {
        let provider = SinchProvider::new(reqwest::Client::new());
        assert_eq!(provider.url_scheme(), "sinch");
    }

    #[test]
    fn test_sinch_provider_description() {
        let provider = SinchProvider::new(reqwest::Client::new());
        assert!(provider.description().contains("Sinch"));
    }

    #[test]
    fn test_sinch_provider_example_url() {
        let provider = SinchProvider::new(reqwest::Client::new());
        assert!(provider.example_url().contains("sinch://"));
    }

    #[test]
    fn test_sinch_provider_supports_attachments() {
        let provider = SinchProvider::new(reqwest::Client::new());
        assert!(provider.supports_attachments());
    }

    #[test]
    fn test_sinch_params_has_required_fields() {
        let provider = SinchProvider::new(reqwest::Client::new());
        let params = provider.params();
        assert!(params.iter().any(|p| p.name == "service_plan_id" && p.required));
        assert!(params.iter().any(|p| p.name == "api_token" && p.required));
        assert!(params.iter().any(|p| p.name == "from" && p.required));
        assert!(params.iter().any(|p| p.name == "to" && p.required));
    }

    #[test]
    fn test_sinch_params_has_optional_fields() {
        let provider = SinchProvider::new(reqwest::Client::new());
        let params = provider.params();
        assert!(params.iter().any(|p| p.name == "region" && !p.required));
        assert!(params.iter().any(|p| p.name == "media_url" && !p.required));
        assert!(params.iter().any(|p| p.name == "base_url" && !p.required));
    }

    #[test]
    fn test_sinch_params_count() {
        let provider = SinchProvider::new(reqwest::Client::new());
        // 4 required + 3 optional = 7 total
        assert_eq!(provider.params().len(), 7);
    }
}
