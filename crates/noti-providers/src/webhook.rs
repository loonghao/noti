use async_trait::async_trait;
use noti_core::{Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse};
use reqwest::Client;
use serde_json::json;

/// Generic HTTP webhook provider.
pub struct WebhookProvider {
    client: Client,
}

impl WebhookProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl NotifyProvider for WebhookProvider {
    fn name(&self) -> &str {
        "webhook"
    }

    fn url_scheme(&self) -> &str {
        "webhook"
    }

    fn description(&self) -> &str {
        "Generic HTTP webhook (POST JSON)"
    }

    fn example_url(&self) -> &str {
        "webhook://example.com/api/notify"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("url", "Webhook URL to POST to")
                .with_example("https://example.com/api/notify"),
            ParamDef::optional("method", "HTTP method (default: POST)").with_example("POST"),
            ParamDef::optional(
                "content_type",
                "Content-Type header (default: application/json)",
            )
            .with_example("application/json"),
            ParamDef::optional(
                "headers",
                "Extra headers as key:value pairs, comma-separated",
            )
            .with_example("Authorization:Bearer xxx,X-Custom:value"),
            ParamDef::optional(
                "body_template",
                "Custom JSON body template. Use {message} and {title} as placeholders",
            ),
        ]
    }

    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        self.validate_config(config)?;
        let url = config.require("url", "webhook")?;
        let method = config.get("method").unwrap_or("POST").to_uppercase();
        let content_type = config.get("content_type").unwrap_or("application/json");

        // Build request body
        let body = if let Some(template) = config.get("body_template") {
            let body_str = template
                .replace("{message}", &message.text)
                .replace("{title}", message.title.as_deref().unwrap_or(""));
            serde_json::from_str(&body_str)
                .map_err(|e| NotiError::Validation(format!("invalid body template JSON: {e}")))?
        } else {
            let mut payload = json!({ "message": message.text });
            if let Some(ref title) = message.title {
                payload["title"] = json!(title);
            }
            payload
        };

        // Build the request
        let mut request = match method.as_str() {
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "PATCH" => self.client.patch(url),
            _ => {
                return Err(NotiError::Validation(format!(
                    "unsupported HTTP method: {method}"
                )));
            }
        };

        request = request.header("Content-Type", content_type);

        // Parse extra headers
        if let Some(headers) = config.get("headers") {
            for pair in headers.split(',') {
                if let Some((k, v)) = pair.split_once(':') {
                    request = request.header(k.trim(), v.trim());
                }
            }
        }

        let resp = request
            .json(&body)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw_text = resp
            .text()
            .await
            .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

        let raw_json: Option<serde_json::Value> = serde_json::from_str(&raw_text).ok();

        if (200..300).contains(&(status as usize)) {
            let mut resp = SendResponse::success("webhook", "request sent successfully")
                .with_status_code(status);
            if let Some(raw) = raw_json {
                resp = resp.with_raw_response(raw);
            }
            Ok(resp)
        } else {
            let mut resp = SendResponse::failure("webhook", format!("HTTP {status}: {raw_text}"))
                .with_status_code(status);
            if let Some(raw) = raw_json {
                resp = resp.with_raw_response(raw);
            }
            Ok(resp)
        }
    }
}
