use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;

/// ServerChan (Server酱) push notification provider.
///
/// Supports image attachments embedded as base64 data URIs in the markdown
/// `desp` field. Non-image attachments are listed as file references.
pub struct ServerChanProvider {
    client: Client,
}

impl ServerChanProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

/// Builds the ServerChan API URL from send_key and optional base_url override.
fn serverchan_url(send_key: &str, config: &ProviderConfig) -> String {
    let base = config
        .get("base_url")
        .unwrap_or("https://sctapi.ftqq.com");
    format!("{}/{send_key}.send", base.trim_end_matches('/'))
}

/// Extracts the title from a message, defaulting to "Notification".
fn serverchan_title(message: &Message) -> &str {
    message.title.as_deref().unwrap_or("Notification")
}

/// Parses the ServerChan API response code.
fn parse_response_code(raw: &serde_json::Value) -> i64 {
    raw.get("code").and_then(|v| v.as_i64()).unwrap_or(-1)
}

/// Extracts the error message from a failed ServerChan API response.
fn parse_error_message(raw: &serde_json::Value) -> String {
    raw.get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown error")
        .to_string()
}

#[async_trait]
impl NotifyProvider for ServerChanProvider {
    fn name(&self) -> &str {
        "serverchan"
    }

    fn url_scheme(&self) -> &str {
        "serverchan"
    }

    fn description(&self) -> &str {
        "ServerChan (Server酱) push to WeChat"
    }

    fn example_url(&self) -> &str {
        "serverchan://<send_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("send_key", "ServerChan SendKey (SCT...)")
                .with_example("SCTxxxxxxxxxxx"),
            ParamDef::optional("base_url", "ServerChan API base URL (default: https://sctapi.ftqq.com)")
                .with_example("https://sctapi.ftqq.com"),
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
        let send_key = config.require("send_key", "serverchan")?;

        let url = serverchan_url(send_key, config);
        let title = serverchan_title(message);

        // Build desp with embedded attachments
        let desp = if message.has_attachments() {
            let mut md = message.text.clone();
            for attachment in &message.attachments {
                let file_name = attachment.effective_file_name();
                if attachment.kind == AttachmentKind::Image {
                    let data = attachment.read_bytes().await?;
                    let mime = attachment.effective_mime();
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                    md.push_str(&format!("\n\n![{file_name}](data:{mime};base64,{b64})"));
                } else {
                    md.push_str(&format!("\n\n📎 **Attachment:** {file_name}"));
                }
            }
            md
        } else {
            message.text.clone()
        };

        let form = vec![("title", title.to_string()), ("desp", desp)];

        let resp = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("serverchan", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let code = parse_response_code(&raw);
        if code == 0 {
            Ok(
                SendResponse::success("serverchan", "message sent successfully")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = parse_error_message(&raw);
            Ok(
                SendResponse::failure("serverchan", format!("API error: {msg}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serverchan_url_default() {
        let config = ProviderConfig::new().set("send_key", "SCT123");
        let url = serverchan_url("SCT123", &config);
        assert_eq!(url, "https://sctapi.ftqq.com/SCT123.send");
    }

    #[test]
    fn test_serverchan_url_custom_base() {
        let config = ProviderConfig::new()
            .set("send_key", "SCT123")
            .set("base_url", "https://custom.example.com");
        let url = serverchan_url("SCT123", &config);
        assert_eq!(url, "https://custom.example.com/SCT123.send");
    }

    #[test]
    fn test_serverchan_url_trailing_slash_stripped() {
        let config = ProviderConfig::new()
            .set("send_key", "SCT123")
            .set("base_url", "https://custom.example.com/");
        let url = serverchan_url("SCT123", &config);
        assert_eq!(url, "https://custom.example.com/SCT123.send");
    }

    #[test]
    fn test_serverchan_title_with_title() {
        let msg = Message::text("body").with_title("Alert");
        assert_eq!(serverchan_title(&msg), "Alert");
    }

    #[test]
    fn test_serverchan_title_without_title() {
        let msg = Message::text("body");
        assert_eq!(serverchan_title(&msg), "Notification");
    }

    #[test]
    fn test_parse_response_code_success() {
        let raw = serde_json::json!({"code": 0, "message": "success"});
        assert_eq!(parse_response_code(&raw), 0);
    }

    #[test]
    fn test_parse_response_code_error() {
        let raw = serde_json::json!({"code": 40001, "message": "bad key"});
        assert_eq!(parse_response_code(&raw), 40001);
    }

    #[test]
    fn test_parse_response_code_missing() {
        let raw = serde_json::json!({"message": "something"});
        assert_eq!(parse_response_code(&raw), -1);
    }

    #[test]
    fn test_parse_error_message_with_message() {
        let raw = serde_json::json!({"code": 40001, "message": "invalid key"});
        assert_eq!(parse_error_message(&raw), "invalid key");
    }

    #[test]
    fn test_parse_error_message_missing() {
        let raw = serde_json::json!({"code": 40001});
        assert_eq!(parse_error_message(&raw), "unknown error");
    }

    #[test]
    fn test_serverchan_provider_name() {
        let provider = ServerChanProvider::new(Client::new());
        assert_eq!(provider.name(), "serverchan");
    }

    #[test]
    fn test_serverchan_provider_url_scheme() {
        let provider = ServerChanProvider::new(Client::new());
        assert_eq!(provider.url_scheme(), "serverchan");
    }

    #[test]
    fn test_serverchan_provider_description() {
        let provider = ServerChanProvider::new(Client::new());
        assert!(!provider.description().is_empty());
    }

    #[test]
    fn test_serverchan_provider_example_url() {
        let provider = ServerChanProvider::new(Client::new());
        assert!(!provider.example_url().is_empty());
    }

    #[test]
    fn test_serverchan_provider_supports_attachments() {
        let provider = ServerChanProvider::new(Client::new());
        assert!(provider.supports_attachments());
    }

    #[test]
    fn test_serverchan_params_has_required_fields() {
        let provider = ServerChanProvider::new(Client::new());
        let params = provider.params();
        assert!(params.iter().any(|p| p.name == "send_key" && p.required));
    }

    #[test]
    fn test_serverchan_params_has_optional_base_url() {
        let provider = ServerChanProvider::new(Client::new());
        let params = provider.params();
        assert!(params.iter().any(|p| p.name == "base_url" && !p.required));
    }

    #[test]
    fn test_serverchan_params_count() {
        let provider = ServerChanProvider::new(Client::new());
        let params = provider.params();
        assert_eq!(params.len(), 2); // 1 required + 1 optional
    }

    #[test]
    fn test_validate_config_full() {
        let provider = ServerChanProvider::new(Client::new());
        let config = ProviderConfig::new().set("send_key", "SCT123");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[test]
    fn test_validate_config_missing_send_key() {
        let provider = ServerChanProvider::new(Client::new());
        let config = ProviderConfig::new();
        assert!(provider.validate_config(&config).is_err());
    }

    #[test]
    fn test_validate_config_empty() {
        let provider = ServerChanProvider::new(Client::new());
        assert!(provider.validate_config(&ProviderConfig::new()).is_err());
    }

    #[test]
    fn test_validate_config_with_optional_params() {
        let provider = ServerChanProvider::new(Client::new());
        let config = ProviderConfig::new()
            .set("send_key", "SCT123")
            .set("base_url", "https://custom.example.com");
        assert!(provider.validate_config(&config).is_ok());
    }
}
