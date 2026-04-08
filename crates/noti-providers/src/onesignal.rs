use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::{json, Value};

/// OneSignal push notification provider.
///
/// Sends push notifications via the OneSignal REST API.
/// Can target all users, segments, or specific player IDs.
pub struct OneSignalProvider {
    client: Client,
}

impl OneSignalProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

/// Returns the OneSignal API endpoint URL.
fn onesignal_url() -> &'static str {
    "https://onesignal.com/api/v1/notifications"
}

/// Parses target segments from config, returning the list of segments to target.
/// Default is "Subscribed Users" if no player_ids are specified.
fn parse_target_segments(config: &ProviderConfig) -> Vec<String> {
    if config.get("player_ids").is_some() {
        // Player IDs are handled separately in the payload
        return vec![];
    }
    let segments = config.get("include_segments").unwrap_or("Subscribed Users");
    segments.split(',').map(|s| s.trim().to_string()).collect()
}

/// Parses player IDs from config, returning them if set.
fn parse_player_ids(config: &ProviderConfig) -> Option<Vec<String>> {
    config.get("player_ids").map(|s| {
        s.split(',').map(|s| s.trim().to_string()).collect()
    })
}

/// Parses the recipients count from a successful OneSignal API response.
fn parse_recipients(response: &Value) -> u64 {
    response.get("recipients").and_then(|v| v.as_u64()).unwrap_or(0)
}

/// Extracts error message from a failed OneSignal API response.
fn parse_errors(response: &Value) -> String {
    response
        .get("errors")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "unknown error".to_string())
}

#[async_trait]
impl NotifyProvider for OneSignalProvider {
    fn name(&self) -> &str {
        "onesignal"
    }

    fn url_scheme(&self) -> &str {
        "onesignal"
    }

    fn description(&self) -> &str {
        "OneSignal push notification platform"
    }

    fn example_url(&self) -> &str {
        "onesignal://<app_id>:<api_key>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("app_id", "OneSignal application ID").with_example("your-app-id"),
            ParamDef::required("api_key", "OneSignal REST API key").with_example("your-api-key"),
            ParamDef::optional(
                "include_segments",
                "Target segments (comma-separated, default: Subscribed Users)",
            )
            .with_example("Subscribed Users"),
            ParamDef::optional("player_ids", "Target player IDs (comma-separated)")
                .with_example("player-id-1,player-id-2"),
            ParamDef::optional("url", "URL to open on notification click")
                .with_example("https://example.com"),
            ParamDef::optional("image", "Image URL for the notification")
                .with_example("https://example.com/img.png"),
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
        let app_id = config.require("app_id", "onesignal")?;
        let api_key = config.require("api_key", "onesignal")?;

        let url = onesignal_url();

        let mut payload = json!({
            "app_id": app_id,
            "contents": {"en": message.text}
        });

        if let Some(ref title) = message.title {
            payload["headings"] = json!({"en": title});
        }

        // Target: player IDs or segments
        if let Some(player_ids) = parse_player_ids(config) {
            payload["include_player_ids"] = json!(player_ids);
        } else {
            let segments = parse_target_segments(config);
            payload["included_segments"] = json!(segments);
        }

        if let Some(click_url) = config.get("url") {
            payload["url"] = json!(click_url);
        }

        // Handle image from config or attachments
        if let Some(image) = config.get("image") {
            payload["big_picture"] = json!(image);
            payload["ios_attachments"] = json!({"image": image});
        } else if let Some(image_att) = message
            .attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
        {
            let data = image_att.read_bytes().await?;
            let mime = image_att.effective_mime();
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            let data_uri = format!("data:{mime};base64,{b64}");
            payload["big_picture"] = json!(data_uri);
            payload["ios_attachments"] = json!({"image": data_uri});
            payload["chrome_web_image"] = json!(data_uri);
        }

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Basic {api_key}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotiError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let raw: Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        if (200..300).contains(&(status as usize)) {
            let recipients = parse_recipients(&raw);
            Ok(SendResponse::success(
                "onesignal",
                format!("notification sent to {recipients} recipients"),
            )
            .with_status_code(status)
            .with_raw_response(raw))
        } else {
            let errors = parse_errors(&raw);
            Ok(
                SendResponse::failure("onesignal", format!("API error: {errors}"))
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noti_core::ProviderConfig;

    fn make_config() -> ProviderConfig {
        ProviderConfig::new()
    }

    // ======================== URL tests ========================

    #[test]
    fn test_onesignal_url() {
        assert_eq!(onesignal_url(), "https://onesignal.com/api/v1/notifications");
    }

    // ======================== Target parsing tests ========================

    #[test]
    fn test_parse_player_ids_with_value() {
        let cfg = make_config().set("player_ids", "id1, id2 , id3");
        let ids = parse_player_ids(&cfg);
        assert_eq!(ids, Some(vec![String::from("id1"), String::from("id2"), String::from("id3")]));
    }

    #[test]
    fn test_parse_player_ids_missing() {
        let cfg = make_config();
        assert_eq!(parse_player_ids(&cfg), None);
    }

    #[test]
    fn test_parse_target_segments_default() {
        let cfg = make_config();
        let segments = parse_target_segments(&cfg);
        assert_eq!(segments, vec![String::from("Subscribed Users")]);
    }

    #[test]
    fn test_parse_target_segments_custom() {
        let cfg = make_config().set("include_segments", "Active, VIP, Test");
        let segments = parse_target_segments(&cfg);
        assert_eq!(segments, vec![String::from("Active"), String::from("VIP"), String::from("Test")]);
    }

    #[test]
    fn test_parse_target_segments_with_player_ids_returns_empty() {
        let cfg = make_config()
            .set("player_ids", "id1,id2")
            .set("include_segments", "Active, VIP");
        // When player_ids is set, segments should be empty (player_ids take precedence)
        let segments = parse_target_segments(&cfg);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_parse_target_segments_whitespace_trimming() {
        let cfg = make_config().set("include_segments", "  Active  ,  VIP  ");
        let segments = parse_target_segments(&cfg);
        assert_eq!(segments, vec![String::from("Active"), String::from("VIP")]);
    }

    // ======================== Response parsing tests ========================

    #[test]
    fn test_parse_recipients_with_value() {
        let resp = json!({"recipients": 42});
        assert_eq!(parse_recipients(&resp), 42);
    }

    #[test]
    fn test_parse_recipients_missing() {
        let resp = json!({"id": "some-id"});
        assert_eq!(parse_recipients(&resp), 0);
    }

    #[test]
    fn test_parse_recipients_null() {
        let resp = json!({"recipients": null});
        assert_eq!(parse_recipients(&resp), 0);
    }

    #[test]
    fn test_parse_recipients_large_number() {
        let resp = json!({"recipients": 18446744073709551615u64});
        assert_eq!(parse_recipients(&resp), 18446744073709551615u64);
    }

    #[test]
    fn test_parse_errors_with_array() {
        let resp = json!({"errors": ["Invalid app_id", "Missing api_key"]});
        assert_eq!(parse_errors(&resp), "Invalid app_id, Missing api_key");
    }

    #[test]
    fn test_parse_errors_missing() {
        let resp = json!({"id": "some-id"});
        assert_eq!(parse_errors(&resp), "unknown error");
    }

    #[test]
    fn test_parse_errors_empty_array() {
        let resp = json!({"errors": []});
        assert_eq!(parse_errors(&resp), "");
    }

    #[test]
    fn test_parse_errors_non_string_elements() {
        let resp = json!({"errors": [123, null, "valid"]});
        assert_eq!(parse_errors(&resp), "valid");
    }

    #[test]
    fn test_parse_errors_single_error() {
        let resp = json!({"errors": ["Something went wrong"]});
        assert_eq!(parse_errors(&resp), "Something went wrong");
    }

    // ======================== Provider metadata tests ========================

    #[test]
    fn test_onesignal_provider_name() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        assert_eq!(provider.name(), "onesignal");
    }

    #[test]
    fn test_onesignal_url_scheme() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        assert_eq!(provider.url_scheme(), "onesignal");
    }

    #[test]
    fn test_onesignal_description() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        assert!(provider.description().contains("OneSignal"));
    }

    #[test]
    fn test_onesignal_example_url() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        assert!(provider.example_url().contains("onesignal://"));
    }

    #[test]
    fn test_onesignal_supports_attachments() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        assert!(provider.supports_attachments());
    }

    #[test]
    fn test_onesignal_params_required_fields() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        let params = provider.params();
        let required: Vec<_> = params.iter().filter(|p| p.required).collect();
        assert_eq!(required.len(), 2);
        assert!(required.iter().any(|p| p.name == "app_id"));
        assert!(required.iter().any(|p| p.name == "api_key"));
    }

    #[test]
    fn test_onesignal_params_optional_fields() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        let params = provider.params();
        let optional: Vec<_> = params.iter().filter(|p| !p.required).collect();
        assert_eq!(optional.len(), 4);
        assert!(optional.iter().any(|p| p.name == "include_segments"));
        assert!(optional.iter().any(|p| p.name == "player_ids"));
        assert!(optional.iter().any(|p| p.name == "url"));
        assert!(optional.iter().any(|p| p.name == "image"));
    }

    #[test]
    fn test_onesignal_params_count() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        assert_eq!(provider.params().len(), 6);
    }

    // ======================== Config validation tests ========================

    #[tokio::test]
    async fn test_validate_config_full() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("app_id", "test-app-id")
            .set("api_key", "test-api-key");
        assert!(provider.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_missing_app_id() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new().set("api_key", "test-api-key");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_missing_api_key() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new().set("app_id", "test-app-id");
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_empty() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new();
        assert!(provider.validate_config(&config).is_err());
    }

    #[tokio::test]
    async fn test_validate_config_with_optional_params() {
        let provider = OneSignalProvider::new(reqwest::Client::new());
        let config = ProviderConfig::new()
            .set("app_id", "test-app-id")
            .set("api_key", "test-api-key")
            .set("include_segments", "Active Users")
            .set("url", "https://example.com");
        assert!(provider.validate_config(&config).is_ok());
    }
}
