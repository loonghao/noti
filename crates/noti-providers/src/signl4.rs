use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// SIGNL4 mobile alerting provider.
///
/// Sends alerts to SIGNL4, a mobile alerting and incident response platform.
/// Alerts are delivered to on-call teams via the SIGNL4 app.
/// Image attachments are embedded as base64 data URIs in the message body.
///
/// API reference: <https://connect.signl4.com/webhook/docs>
pub struct Signl4Provider {
    client: Client,
}

impl Signl4Provider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for Signl4Provider {
    fn name(&self) -> &str {
        "signl4"
    }

    fn url_scheme(&self) -> &str {
        "signl4"
    }

    fn description(&self) -> &str {
        "SIGNL4 mobile alerting and incident response"
    }

    fn example_url(&self) -> &str {
        "signl4://<team_secret>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("team_secret", "SIGNL4 team secret (webhook ID)")
                .with_example("your-team-secret"),
            ParamDef::optional(
                "s4_severity",
                "Alert severity: 0=info, 1=warning, 2=critical",
            )
            .with_example("2"),
            ParamDef::optional("s4_service", "Service name / category for the alert")
                .with_example("noti-cli"),
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
        let team_secret = config.require("team_secret", "signl4")?;

        let url = format!("https://connect.signl4.com/webhook/{team_secret}");

        let title = message.title.as_deref().unwrap_or("Alert");

        // Embed image attachments as base64 data URIs, list non-image files
        let message_text = if message.has_attachments() {
            let mut text = message.text.clone();
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    text.push_str(&format!(
                        "\n\n![{file_name}](data:{mime};base64,{b64})"
                    ));
                } else {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    text.push_str(&format!(
                        "\n📎 {file_name} (data:{mime};base64,{b64})"
                    ));
                }
            }
            text
        } else {
            message.text.clone()
        };

        let mut payload = json!({
            "Title": title,
            "Message": message_text,
        });

        if let Some(severity) = config.get("s4_severity") {
            payload["X-S4-Severity"] = json!(severity);
        }
        if let Some(service) = config.get("s4_service") {
            payload["X-S4-Service"] = json!(service);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp.json().await.unwrap_or(json!({"status": status}));

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("signl4", "alert sent to SIGNL4")
                .with_status_code(status)
                .with_raw_response(raw))
        } else {
            let error = raw
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Ok(
                SendResponse::failure("signl4", format!("API error: {error}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}
