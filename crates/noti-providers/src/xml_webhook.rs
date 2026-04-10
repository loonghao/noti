use async_trait::async_trait;
use base64::Engine;
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
            ParamDef::optional("base_url", "Override target URL (takes precedence over url param)"),
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
        let url = match config.get("base_url") {
            Some(u) => u,
            None => config.require("url", "xml")?,
        };
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

        // Build attachments XML if present
        let attachments_xml = if message.has_attachments() {
            let mut xml_parts = String::from("<attachments>");
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                let mime = attachment.effective_mime();
                let data = attachment.read_bytes().await?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                xml_parts.push_str(&format!(
                    "<attachment><filename>{}</filename><mimetype>{}</mimetype><content>{}</content></attachment>",
                    escape_xml(&file_name),
                    escape_xml(&mime),
                    b64
                ));
            }
            xml_parts.push_str("</attachments>");
            xml_parts
        } else {
            String::new()
        };

        let xml_body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<{root}>{title_xml}<message>{}</message><type>{}</type>{attachments_xml}</{root}>",
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
            .map_err(|e| crate::http_helpers::classify_reqwest_error("xml_webhook", e))?;

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
