use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Growl notification provider via HTTP relay.
///
/// Growl uses GNTP (Growl Notification Transport Protocol).
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

        let gntp_message = format!(
            "GNTP/1.0 NOTIFY NONE\r\n\
            Application-Name: noti\r\n\
            Notification-Name: General\r\n\
            Notification-Title: {title}\r\n\
            Notification-Text: {text}\r\n\
            Notification-Priority: {priority}\r\n\
            Notification-Sticky: {sticky}\r\n\
            \r\n",
            text = message.text,
        );

        let mut req = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-gntp")
            .body(gntp_message);

        if let Some(password) = config.get("password") {
            req = req.header("X-Growl-Password", password);
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
                SendResponse::success("growl", "notification sent successfully")
                    .with_status_code(status),
            )
        } else {
            Ok(
                SendResponse::failure("growl", format!("GNTP error: {body}"))
                    .with_status_code(status)
                    .with_raw_response(json!({ "body": body })),
            )
        }
    }
}
