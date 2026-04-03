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

    /// Send a multipart request with file attachments.
    async fn send_multipart(
        &self,
        message: &Message,
        url: &str,
        method: &str,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError> {
        let mut form = reqwest::multipart::Form::new().text("message", message.text.clone());

        if let Some(ref title) = message.title {
            form = form.text("title", title.clone());
        }

        for attachment in &message.attachments {
            let data = attachment.read_bytes().await?;
            let file_name = attachment.effective_file_name();
            let mime_str = attachment.effective_mime();
            let part = reqwest::multipart::Part::bytes(data)
                .file_name(file_name)
                .mime_str(&mime_str)
                .map_err(|e| NotiError::Network(format!("MIME error: {e}")))?;
            form = form.part("file", part);
        }

        let mut request = match method {
            "POST" => self.client.post(url),
            "PUT" => self.client.put(url),
            "PATCH" => self.client.patch(url),
            _ => {
                return Err(NotiError::Validation(format!(
                    "unsupported HTTP method: {method}"
                )));
            }
        };

        // Apply authentication
        request = Self::apply_auth(request, config);

        if let Some(headers) = config.get("headers") {
            for pair in headers.split(',') {
                if let Some((k, v)) = pair.split_once(':') {
                    request = request.header(k.trim(), v.trim());
                }
            }
        }

        let resp = request
            .multipart(form)
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
            ParamDef::optional(
                "auth_type",
                "Authentication type: bearer, basic, api_key",
            ),
            ParamDef::optional("auth_token", "Authentication token/credentials"),
            ParamDef::optional("retry", "Number of retry attempts on failure").with_example("3"),
            ParamDef::optional("retry_delay", "Delay in seconds between retries").with_example("2"),
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
        let url = config.require("url", "webhook")?;
        let method = config.get("method").unwrap_or("POST").to_uppercase();

        // If attachments present, use multipart form
        if message.has_attachments() {
            return self.send_multipart(message, url, &method, config).await;
        }

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

        // Apply authentication
        request = Self::apply_auth(request, config);

        // Parse extra headers
        if let Some(headers) = config.get("headers") {
            for pair in headers.split(',') {
                if let Some((k, v)) = pair.split_once(':') {
                    request = request.header(k.trim(), v.trim());
                }
            }
        }

        // Retry logic (simple synchronous retry without delay)
        let retry_count = config
            .get("retry")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);

        let mut last_error = None;
        for _ in 0..retry_count {
            match request
                .try_clone()
                .ok_or_else(|| NotiError::Network("request clone failed".to_string()))?
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let raw_text = resp
                        .text()
                        .await
                        .map_err(|e| NotiError::Network(format!("failed to read response: {e}")))?;

                    let raw_json: Option<serde_json::Value> =
                        serde_json::from_str(&raw_text).ok();

                    if (200..300).contains(&(status as usize)) {
                        let mut resp =
                            SendResponse::success("webhook", "request sent successfully")
                                .with_status_code(status);
                        if let Some(raw) = raw_json {
                            resp = resp.with_raw_response(raw);
                        }
                        return Ok(resp);
                    } else {
                        last_error = Some((status, raw_text, raw_json));
                    }
                }
                Err(e) => {
                    last_error = Some((0, e.to_string(), None));
                }
            }
        }

        if let Some((status, raw_text, raw_json)) = last_error {
            let mut resp = if status > 0 {
                SendResponse::failure("webhook", format!("HTTP {status}: {raw_text}"))
                    .with_status_code(status)
            } else {
                SendResponse::failure("webhook", raw_text.clone())
                    .with_status_code(status)
            };
            if let Some(raw) = raw_json {
                resp = resp.with_raw_response(raw);
            }
            Ok(resp)
        } else {
            Ok(SendResponse::failure("webhook", "unknown error".to_string()))
        }
    }
}

impl WebhookProvider {
    /// Apply authentication based on auth_type parameter.
    fn apply_auth(
        request: reqwest::RequestBuilder,
        config: &ProviderConfig,
    ) -> reqwest::RequestBuilder {
        let auth_type = match config.get("auth_type") {
            Some(t) => t.to_lowercase(),
            None => return request,
        };

        let auth_token = match config.get("auth_token") {
            Some(t) => t,
            None => return request,
        };

        match auth_type.as_str() {
            "bearer" => request.bearer_auth(auth_token),
            "basic" => {
                // auth_token should be "username:password"
                if let Some((username, password)) = auth_token.split_once(':') {
                    request.basic_auth(username, Some(password))
                } else {
                    request.basic_auth(auth_token, None::<&str>)
                }
            }
            "api_key" => {
                // Use X-API-Key header by default
                request.header("X-API-Key", auth_token)
            }
            _ => request,
        }
    }
}
