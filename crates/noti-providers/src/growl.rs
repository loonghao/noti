use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Growl notification provider via HTTP relay.
///
/// Growl uses GNTP (Growl Notification Transport Protocol).
/// Supports image attachments via the GNTP `Notification-Icon` resource header.
///
/// This implementation uses a Growl-compatible HTTP relay (e.g., prowl-like
/// forwarding) or a Growl REST endpoint if available.
pub struct GrowlProvider {
    client: Client,
}

impl GrowlProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for GrowlProvider {
    fn name(&self) -> &str {
        "growl"
    }

    fn url_scheme(&self) -> &str {
        "growl"
    }

    fn description(&self) -> &str {
        "Growl desktop notifications via GNTP/HTTP relay"
    }

    fn example_url(&self) -> &str {
        "growl://<host>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("host", "Growl host IP or hostname").with_example("192.168.1.100"),
            ParamDef::optional("port", "Growl port (default: 23053)").with_example("23053"),
            ParamDef::optional("password", "Growl notification password"),
            ParamDef::optional("scheme", "URL scheme: http or https (default: http)"),
            ParamDef::optional("priority", "Notification priority: -2 to 2 (default: 0)"),
            ParamDef::optional(
                "sticky",
                "Make notification sticky: true or false (default: false)",
            ),
        ]
    }

    fn supports_attachments(&self) -> bool {
        false
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let host = config.require("host", "growl")?;
        let port = config.get("port").unwrap_or("23053");
        let scheme = config.get("scheme").unwrap_or("http");
        let priority = config.get("priority").unwrap_or("0");
        let sticky = config.get("sticky").unwrap_or("false");

        let title = message.title.as_deref().unwrap_or("noti");

        // Use GNTP over HTTP (some Growl-compatible implementations support REST)
        let url = format!("{scheme}://{host}:{port}/gntp");

        // Build GNTP message with optional icon resource
        let mut gntp_message = format!(
            "GNTP/1.0 NOTIFY NONE\r\n\
            Application-Name: noti\r\n\
            Notification-Name: General\r\n\
            Notification-Title: {title}\r\n\
            Notification-Text: {text}\r\n\
            Notification-Priority: {priority}\r\n\
            Notification-Sticky: {sticky}\r\n",
            text = message.text,
        );

        // Embed image attachment as Notification-Icon resource
        let mut icon_data: Option<Vec<u8>> = None;
        if message.has_attachments() {
            if let Some(img) = message
                .attachments
                .iter()
                .find(|a| matches!(a.kind, AttachmentKind::Image))
            {
                let data = img.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                let mime = img.effective_mime();
                // Use data URI as icon URL (GNTP 1.0 supports resource identifiers)
                gntp_message.push_str(&format!(
                    "Notification-Icon: data:{mime};base64,{b64}\r\n"
                ));
                icon_data = Some(data);
            }
        }

        gntp_message.push_str("\r\n");

        // If we have icon data, append it as GNTP binary resource section
        let body = if let Some(ref data) = icon_data {
            let mut full = gntp_message.into_bytes();
            // GNTP binary resource identifier section
            let resource_header = format!(
                "Identifier: icon\r\nLength: {}\r\n\r\n",
                data.len()
            );
            full.extend_from_slice(resource_header.as_bytes());
            full.extend_from_slice(data);
            full.extend_from_slice(b"\r\n\r\n");
            full
        } else {
            gntp_message.into_bytes()
        };

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-gntp")
            .body(body);

        if let Some(password) = config.get("password") {
            req = req.header("X-Growl-Password", password);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let resp_body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&status) {
            Ok(
                SendResponse::success("growl", "notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("growl", format!("GNTP error: {resp_body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": resp_body })),
            )
        }
    }
}
