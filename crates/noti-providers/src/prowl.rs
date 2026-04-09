use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Prowl iOS push notification provider.
///
/// Prowl is a push notification client for iOS that receives push
/// notifications from the Prowl API. It supports priority levels,
/// URLs, and application names. Supports image attachments via
/// base64 data URI in the url field.
///
/// API docs: <https://www.prowlapp.com/api.php>
pub struct ProwlProvider {
    client: Client,
}

impl ProwlProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for ProwlProvider {
    fn name(&self) -> &str {
        "prowl"
    }

    fn url_scheme(&self) -> &str {
        "prowl"
    }

    fn description(&self) -> &str {
        "Prowl iOS push notifications"
    }

    fn example_url(&self) -> &str {
        "prowl://<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("api_key", "Prowl API key")
                .with_example("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"),
            ParamDef::optional("provider_key", "Provider API key (for higher rate limits)"),
            ParamDef::optional(
                "priority",
                "Priority: -2 (Very Low) to 2 (Emergency), default 0",
            )
            .with_example("1"),
            ParamDef::optional("url", "URL to attach to the notification"),
            ParamDef::optional("application", "Application name (default: noti)")
                .with_example("noti"),
            ParamDef::optional("base_url", "Override base URL for API requests"),
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
        let api_key = config.require("api_key", "prowl")?;
        let application = config.get("application").unwrap_or("noti");
        let priority = config.get("priority").unwrap_or("0");
        let base_url = config.get("base_url").unwrap_or("https://api.prowlapp.com");

        // Embed first image attachment as base64 in description
        let mut description = message.text.clone();
        for attachment in &message.attachments {
            if attachment.kind == noti_core::AttachmentKind::Image {
                if let Ok(data) = attachment.read_bytes().await {
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    let name = attachment.effective_file_name();
                    description.push_str(&format!("\n\n[Image: {name}] data:{mime};base64,{b64}"));
                }
            }
        }

        let mut form = vec![
            ("apikey", api_key.to_string()),
            ("application", application.to_string()),
            ("event", message.title.clone().unwrap_or_default()),
            ("description", description),
            ("priority", priority.to_string()),
        ];

        if let Some(provider_key) = config.get("provider_key") {
            form.push(("providerkey", provider_key.to_string()));
        }

        // Use explicit url config or embed first image as data URI
        if let Some(url) = config.get("url") {
            form.push(("url", url.to_string()));
        } else if let Some(img) = message.first_image() {
            if let Ok(data) = img.read_bytes().await {
                let mime = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                form.push(("url", format!("data:{mime};base64,{b64}")));
            }
        }

        let resp = self
            .client
            .post(format!("{base_url}/publicapi/add"))
            .form(&form)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("prowl", e))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("prowl", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(serde_json::json!({"response": body})))
        } else {
            Ok(
                SendResponse::failure("prowl", format!("API error (HTTP {status}): {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"response": body})),
            )
        }
    }
}
