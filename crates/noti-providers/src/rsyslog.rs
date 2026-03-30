use async_trait::async_trait;
use base64::Engine;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Remote Syslog notification provider (via HTTP relay).
///
/// Sends syslog-style notifications via a syslog HTTP relay service
/// such as Papertrail, Loggly, or any syslog-to-HTTP gateway.
/// Supports attachments by embedding base64-encoded data in the JSON
/// payload under the `attachments` field.
pub struct RsyslogProvider {
    client: Client,
}

impl RsyslogProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for RsyslogProvider {
    fn name(&self) -> &str {
        "rsyslog"
    }

    fn url_scheme(&self) -> &str {
        "rsyslog"
    }

    fn description(&self) -> &str {
        "Remote Syslog notifications via HTTP relay"
    }

    fn example_url(&self) -> &str {
        "rsyslog://<host>/<token>"
    }

    fn supports_attachments(&self) -> bool {
        true
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Syslog HTTP relay host").with_example("logs.example.com"),
            ParamDef::optional("token", "Authentication token for the syslog relay"),
            ParamDef::optional("port", "Port number (default: 514)"),
            ParamDef::optional("scheme", "URL scheme: http or https (default: https)"),
            ParamDef::optional("facility", "Syslog facility (default: user)"),
            ParamDef::optional(
                "severity",
                "Syslog severity: emerg, alert, crit, err, warning, notice, info, debug (default: info)",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let host = config.require("host", "rsyslog")?;
        let scheme = config.get("scheme").unwrap_or("https");
        let severity = config.get("severity").unwrap_or("info");
        let facility = config.get("facility").unwrap_or("user");

        let title = message.title.as_deref().unwrap_or("noti");

        let mut payload = json!({
            "message": format!("[{severity}] {title}: {}", message.text),
            "facility": facility,
            "severity": severity,
            "hostname": "noti-cli",
            "tag": "noti",
        });

        // Add attachments as base64-encoded data in the JSON payload
        if message.has_attachments() {
            let mut attachments_json = Vec::new();
            for attachment in &message.attachments {
                if let Ok(data) = attachment.read_bytes().await {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    attachments_json.push(json!({
                        "name": attachment.effective_file_name(),
                        "mime": attachment.effective_mime(),
                        "data": b64,
                    }));
                }
            }
            if !attachments_json.is_empty() {
                payload["attachments"] = json!(attachments_json);
            }
        }

        let mut url = format!("{scheme}://{host}");
        if let Some(port) = config.get("port") {
            url = format!("{scheme}://{host}:{port}");
        }

        let mut req = self.client.post(&url).json(&payload);

        if let Some(token) = config.get("token") {
            req = req.header("Authorization", format!("Bearer {token}"));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("rsyslog", "syslog message sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("rsyslog", format!("API error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
