use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;

/// Generic XML webhook provider.
///
/// Sends notifications as XML-encoded POST requests to any
/// endpoint that accepts XML payloads.
pub struct XmlWebhookProvider {
    client: Client,
}

impl XmlWebhookProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for XmlWebhookProvider {
    fn name(&self) -> &str {
        "xml"
    }

    fn url_scheme(&self) -> &str {
        "xml"
    }

    fn description(&self) -> &str {
        "Generic XML webhook (POST XML)"
    }

    fn example_url(&self) -> &str {
        "xml://<host>/<path>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("url", "Target webhook URL").with_example("https://example.com/api"),
            ParamDef::optional("method", "HTTP method: POST, PUT, PATCH (default: POST)")
                .with_example("POST"),
            ParamDef::optional(
                "header",
                "Custom headers (semicolon-separated key=value pairs)",
            )
            .with_example("X-Api-Key=abc;X-Custom=val"),
            ParamDef::optional("type", "Notification type field (default: info)")
                .with_example("info"),
            ParamDef::optional("root", "XML root element name (default: notification)")
                .with_example("notification"),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let url = config.require("url", "xml")?;
        let method_str = config.get("method").unwrap_or("POST").to_uppercase();
        let noti_type = config.get("type").unwrap_or("info");
        let root = config.get("root").unwrap_or("notification");

        let method = match method_str.as_str() {
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            _ => reqwest::Method::POST,
        };

        // Build XML body
        let title_xml = match &message.title {
            Some(t) => format!("<title>{}</title>", escape_xml(t)),
            None => String::new(),
        };
        let xml_body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{root}>{title_xml}<message>{}</message><type>{}</type></{root}>",
            escape_xml(&message.text),
            escape_xml(noti_type),
        );

        let mut req = self
            .client
            .request(method, url)
            .header("Content-Type", "application/xml; charset=utf-8");

        // Add custom headers
        if let Some(headers) = config.get("header") {
            for pair in headers.split(';') {
                if let Some((k, v)) = pair.split_once('=') {
                    req = req.header(k.trim(), v.trim());
                }
            }
        }

        let resp = req
            .body(xml_body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            Ok(SendResponse::success("xml", "message sent successfully")
                .with_status_code(status)
                .with_raw_response(serde_json::json!({"response": body})))
        } else {
            Ok(
                SendResponse::failure("xml", format!("HTTP {status}: {body}"))
                    .with_status_code(status)
                    .with_raw_response(serde_json::json!({"response": body})),
            )
        }
    }
}

/// Simple XML escaping for text content.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
