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

    /// Build JSON body from message and config.
    /// If `body_template` is set, replaces `{message}` and `{title}` placeholders.
    /// Otherwise, builds `{"message": "...", "title": "..."}`.
    pub fn build_body(message: &Message, config: &ProviderConfig) -> Result<serde_json::Value, NotiError> {
        if let Some(template) = config.get("body_template") {
            let body_str = template
                .replace("{message}", &message.text)
                .replace("{title}", message.title.as_deref().unwrap_or(""));
            serde_json::from_str(&body_str)
                .map_err(|e| NotiError::Validation(format!("invalid body template JSON: {e}")))
        } else {
            let mut payload = json!({ "message": message.text });
            if let Some(ref title) = message.title {
                payload["title"] = json!(title);
            }
            Ok(payload)
        }
    }

    /// Resolve HTTP method from config (default: POST).
    pub fn resolve_method(config: &ProviderConfig) -> String {
        config.get("method").unwrap_or("POST").to_uppercase()
    }

    /// Resolve Content-Type from config (default: application/json).
    pub fn resolve_content_type(config: &ProviderConfig) -> &str {
        config.get("content_type").unwrap_or("application/json")
    }

    /// Parse retry count from config (default: 1).
    pub fn parse_retry_count(config: &ProviderConfig) -> u32 {
        config
            .get("retry")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1)
    }

    /// Build a SendResponse from HTTP status and body text.
    pub fn build_response(status: u16, raw_text: &str) -> SendResponse {
        let raw_json: Option<serde_json::Value> = serde_json::from_str(raw_text).ok();

        if (200..300).contains(&(status as usize)) {
            let mut resp = SendResponse::success("webhook", "request sent successfully")
                .with_status_code(status);
            if let Some(raw) = raw_json {
                resp = resp.with_raw_response(raw);
            }
            resp
        } else {
            let mut resp = SendResponse::failure("webhook", format!("HTTP {status}: {raw_text}"))
                .with_status_code(status);
            if let Some(raw) = raw_json {
                resp = resp.with_raw_response(raw);
            }
            resp
        }
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

        Ok(Self::build_response(status, &raw_text))
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
            ParamDef::optional("auth_type", "Authentication type: bearer, basic, api_key"),
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
        let method = Self::resolve_method(config);

        // If attachments present, use multipart form
        if message.has_attachments() {
            return self.send_multipart(message, url, &method, config).await;
        }

        let content_type = Self::resolve_content_type(config);
        let body = Self::build_body(message, config)?;

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
        let retry_count = Self::parse_retry_count(config);

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

                    if (200..300).contains(&(status as usize)) {
                        return Ok(Self::build_response(status, &raw_text));
                    } else {
                        last_error = Some((status, raw_text));
                    }
                }
                Err(e) => {
                    last_error = Some((0, e.to_string()));
                }
            }
        }

        if let Some((status, raw_text)) = last_error {
            Ok(Self::build_response(status, &raw_text))
        } else {
            Ok(SendResponse::failure(
                "webhook",
                "unknown error".to_string(),
            ))
        }
    }
}

impl WebhookProvider {
    /// Apply authentication based on auth_type parameter.
    pub fn apply_auth(
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

#[cfg(test)]
mod tests {
    use super::*;
    use noti_core::MessageFormat;

    // ---- build_body tests ----

    #[test]
    fn test_build_body_plain_text() {
        let message = Message::text("hello world");
        let config = ProviderConfig::new();
        let body = WebhookProvider::build_body(&message, &config).unwrap();
        assert_eq!(body["message"], "hello world");
        assert!(body.get("title").is_none());
    }

    #[test]
    fn test_build_body_with_title() {
        let message = Message::text("body text").with_title("Alert");
        let config = ProviderConfig::new();
        let body = WebhookProvider::build_body(&message, &config).unwrap();
        assert_eq!(body["message"], "body text");
        assert_eq!(body["title"], "Alert");
    }

    #[test]
    fn test_build_body_template_with_placeholders() {
        let message = Message::text("hello").with_title("Hi");
        let config = ProviderConfig::new().set(
            "body_template",
            r#"{"text": "{message}", "heading": "{title}"}"#,
        );
        let body = WebhookProvider::build_body(&message, &config).unwrap();
        assert_eq!(body["text"], "hello");
        assert_eq!(body["heading"], "Hi");
    }

    #[test]
    fn test_build_body_template_message_only() {
        let message = Message::text("hello");
        let config = ProviderConfig::new().set(
            "body_template",
            r#"{"content": "{message}"}"#,
        );
        let body = WebhookProvider::build_body(&message, &config).unwrap();
        assert_eq!(body["content"], "hello");
    }

    #[test]
    fn test_build_body_template_invalid_json() {
        let message = Message::text("hello");
        let config = ProviderConfig::new().set("body_template", "not json");
        let result = WebhookProvider::build_body(&message, &config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid body template JSON"));
    }

    // ---- resolve_method tests ----

    #[test]
    fn test_resolve_method_default() {
        let config = ProviderConfig::new();
        assert_eq!(WebhookProvider::resolve_method(&config), "POST");
    }

    #[test]
    fn test_resolve_method_explicit_post() {
        let config = ProviderConfig::new().set("method", "POST");
        assert_eq!(WebhookProvider::resolve_method(&config), "POST");
    }

    #[test]
    fn test_resolve_method_put() {
        let config = ProviderConfig::new().set("method", "PUT");
        assert_eq!(WebhookProvider::resolve_method(&config), "PUT");
    }

    #[test]
    fn test_resolve_method_patch() {
        let config = ProviderConfig::new().set("method", "PATCH");
        assert_eq!(WebhookProvider::resolve_method(&config), "PATCH");
    }

    #[test]
    fn test_resolve_method_lowercase() {
        let config = ProviderConfig::new().set("method", "post");
        assert_eq!(WebhookProvider::resolve_method(&config), "POST");
    }

    // ---- resolve_content_type tests ----

    #[test]
    fn test_resolve_content_type_default() {
        let config = ProviderConfig::new();
        assert_eq!(WebhookProvider::resolve_content_type(&config), "application/json");
    }

    #[test]
    fn test_resolve_content_type_custom() {
        let config = ProviderConfig::new().set("content_type", "text/plain");
        assert_eq!(WebhookProvider::resolve_content_type(&config), "text/plain");
    }

    // ---- parse_retry_count tests ----

    #[test]
    fn test_parse_retry_count_default() {
        let config = ProviderConfig::new();
        assert_eq!(WebhookProvider::parse_retry_count(&config), 1);
    }

    #[test]
    fn test_parse_retry_count_explicit() {
        let config = ProviderConfig::new().set("retry", "3");
        assert_eq!(WebhookProvider::parse_retry_count(&config), 3);
    }

    #[test]
    fn test_parse_retry_count_invalid() {
        let config = ProviderConfig::new().set("retry", "abc");
        assert_eq!(WebhookProvider::parse_retry_count(&config), 1);
    }

    #[test]
    fn test_parse_retry_count_zero() {
        let config = ProviderConfig::new().set("retry", "0");
        assert_eq!(WebhookProvider::parse_retry_count(&config), 0);
    }

    // ---- build_response tests ----

    #[test]
    fn test_build_response_success_200() {
        let resp = WebhookProvider::build_response(200, r#"{"ok": true}"#);
        assert!(resp.success);
        assert_eq!(resp.status_code, Some(200));
        assert_eq!(resp.provider, "webhook");
        assert!(resp.raw_response.is_some());
    }

    #[test]
    fn test_build_response_success_201_no_json() {
        let resp = WebhookProvider::build_response(201, "Created");
        assert!(resp.success);
        assert_eq!(resp.status_code, Some(201));
        assert!(resp.raw_response.is_none());
    }

    #[test]
    fn test_build_response_failure_400() {
        let resp = WebhookProvider::build_response(400, r#"{"error": "bad request"}"#);
        assert!(!resp.success);
        assert_eq!(resp.status_code, Some(400));
        assert!(resp.raw_response.is_some());
    }

    #[test]
    fn test_build_response_failure_500() {
        let resp = WebhookProvider::build_response(500, "Internal Server Error");
        assert!(!resp.success);
        assert_eq!(resp.status_code, Some(500));
        assert!(resp.raw_response.is_none());
    }

    // ---- Provider metadata tests ----

    #[test]
    fn test_webhook_provider_name() {
        let provider = WebhookProvider::new(Client::new());
        assert_eq!(provider.name(), "webhook");
    }

    #[test]
    fn test_webhook_provider_url_scheme() {
        let provider = WebhookProvider::new(Client::new());
        assert_eq!(provider.url_scheme(), "webhook");
    }

    #[test]
    fn test_webhook_provider_description() {
        let provider = WebhookProvider::new(Client::new());
        assert!(!provider.description().is_empty());
    }

    #[test]
    fn test_webhook_provider_example_url() {
        let provider = WebhookProvider::new(Client::new());
        assert!(provider.example_url().starts_with("webhook://"));
    }

    #[test]
    fn test_webhook_provider_supports_attachments() {
        let provider = WebhookProvider::new(Client::new());
        assert!(provider.supports_attachments());
    }

    #[test]
    fn test_webhook_provider_params_required() {
        let provider = WebhookProvider::new(Client::new());
        let params = provider.params();
        let required: Vec<_> = params.iter().filter(|p| p.required).collect();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].name, "url");
    }

    #[test]
    fn test_webhook_provider_params_optional() {
        let provider = WebhookProvider::new(Client::new());
        let params = provider.params();
        let optional: Vec<_> = params.iter().filter(|p| !p.required).collect();
        assert_eq!(optional.len(), 8);
    }

    // ---- Config validation tests ----

    #[tokio::test]
    async fn test_validate_config_full() {
        let provider = WebhookProvider::new(Client::new());
        let config = ProviderConfig::new().set("url", "https://example.com/webhook");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_missing_url() {
        let provider = WebhookProvider::new(Client::new());
        let config = ProviderConfig::new();
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_with_optional_params() {
        let provider = WebhookProvider::new(Client::new());
        let config = ProviderConfig::new()
            .set("url", "https://example.com/webhook")
            .set("method", "POST")
            .set("content_type", "application/json")
            .set("headers", "X-Custom:value")
            .set("body_template", r#"{"msg": "{message}"}"#)
            .set("auth_type", "bearer")
            .set("auth_token", "tok123")
            .set("retry", "3")
            .set("retry_delay", "2");
        assert!(provider.validate_config(&config).is_ok());
    }
}
