use async_trait::async_trait;
use base64::Engine;
use noti_core::{
    AttachmentKind, Message, NotiError, NotifyProvider, ParamDef, ProviderConfig, SendResponse,
};
use reqwest::Client;
use serde_json::json;

/// Firebase Cloud Messaging (FCM) provider.
///
/// Sends push notifications via Google's FCM Legacy HTTP API.
///
/// API reference: <https://firebase.google.com/docs/cloud-messaging/send-message>
pub struct FcmProvider {
    client: Client,
}

impl FcmProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

/// FCM endpoint URL.
fn fcm_url() -> &'static str {
    "https://fcm.googleapis.com/fcm/send"
}

/// Build the FCM notification dict from message and config.
///
/// Does NOT handle image attachments (those require async I/O to read bytes).
fn build_fcm_notification(message: &Message, config: &ProviderConfig) -> serde_json::Value {
    let mut notification = json!({
        "body": message.text,
    });

    if let Some(ref title) = message.title {
        notification["title"] = json!(title);
    }
    if let Some(icon) = config.get("icon") {
        notification["icon"] = json!(icon);
    }
    if let Some(sound) = config.get("sound") {
        notification["sound"] = json!(sound);
    } else {
        notification["sound"] = json!("default");
    }
    if let Some(click_action) = config.get("click_action") {
        notification["click_action"] = json!(click_action);
    }
    if let Some(image) = config.get("image") {
        notification["image"] = json!(image);
    }

    notification
}

/// Parse TTL string to u64, returning default if None or invalid.
fn parse_ttl(ttl_opt: Option<&str>) -> u64 {
    ttl_opt
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(2419200)
}

/// FCM message priority. Defaults to "high".
fn fcm_priority(config: &ProviderConfig) -> &'static str {
    match config.get("priority") {
        Some("normal") => "normal",
        _ => "high",
    }
}

#[async_trait]
impl NotifyProvider for FcmProvider {
    fn name(&self) -> &str {
        "fcm"
    }

    fn url_scheme(&self) -> &str {
        "fcm"
    }

    fn description(&self) -> &str {
        "Firebase Cloud Messaging push notifications"
    }

    fn example_url(&self) -> &str {
        "fcm://<server_key>/<device_token>"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::required("server_key", "FCM server key (legacy API key)")
                .with_example("AAAA..."),
            ParamDef::required("device_token", "Target device registration token")
                .with_example("dQw4..."),
            ParamDef::optional("topic", "FCM topic name (alternative to device_token)")
                .with_example("news"),
            ParamDef::optional("condition", "FCM condition expression for topic targeting"),
            ParamDef::optional(
                "priority",
                "Message priority: high or normal (default: high)",
            )
            .with_example("high"),
            ParamDef::optional("collapse_key", "Collapse key for message grouping"),
            ParamDef::optional("ttl", "Time-to-live in seconds (default: 2419200)")
                .with_example("3600"),
            ParamDef::optional("icon", "Notification icon name"),
            ParamDef::optional("sound", "Notification sound (default: default)")
                .with_example("default"),
            ParamDef::optional("click_action", "Action on notification click"),
            ParamDef::optional("image", "Image URL for rich notification"),
            ParamDef::optional("base_url", "Base URL override for FCM API (default: https://fcm.googleapis.com/fcm)")
                .with_example("http://localhost:8080"),
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
        let server_key = config.require("server_key", "fcm")?;

        let url = config.get("base_url")
            .map(|s| {
                let base = s.trim_end_matches('/');
                format!("{base}/send")
            })
            .unwrap_or_else(|| fcm_url().to_string());

        let mut notification = build_fcm_notification(message, config);

        // If there's an image attachment, use it as the notification image
        if config.get("image").is_none() {
            if let Some(img) = message
                .attachments
                .iter()
                .find(|a| a.kind == AttachmentKind::Image)
            {
                let data = img.read_bytes().await?;
                let mime_str = img.effective_mime();
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                notification["image"] = json!(format!("data:{mime_str};base64,{b64}"));
            }
        }

        // Add non-image file info as data payload for client-side handling
        let file_attachments: Vec<_> = message
            .attachments
            .iter()
            .filter(|a| a.kind != AttachmentKind::Image)
            .collect();
        let has_file_attachments = !file_attachments.is_empty();

        let mut payload = json!({
            "notification": notification,
        });

        // Target: device_token, topic, or condition
        if let Some(topic) = config.get("topic") {
            payload["to"] = json!(format!("/topics/{topic}"));
        } else if let Some(condition) = config.get("condition") {
            payload["condition"] = json!(condition);
        } else {
            let device_token = config.require("device_token", "fcm")?;
            payload["to"] = json!(device_token);
        }

        payload["priority"] = json!(fcm_priority(config));

        if let Some(collapse_key) = config.get("collapse_key") {
            payload["collapse_key"] = json!(collapse_key);
        }
        if let Some(ttl) = config.get("ttl") {
            payload["time_to_live"] = json!(parse_ttl(Some(ttl)));
        }

        // Add file attachment info as data payload for client-side handling
        if has_file_attachments {
            let file_names: Vec<String> = file_attachments
                .iter()
                .map(|a| a.effective_file_name())
                .collect();
            payload["data"] = json!({
                "attachment_count": file_names.len(),
                "attachment_names": file_names.join(","),
            });
        }

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("key={server_key}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| crate::http_helpers::classify_reqwest_error("fcm", e))?;

        let status = resp.status().as_u16();
        let raw: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| NotiError::Network(format!("failed to parse response: {e}")))?;

        let success_count = raw.get("success").and_then(|v| v.as_i64()).unwrap_or(0);
        if success_count > 0 {
            Ok(
                SendResponse::success("fcm", "push notification sent via FCM")
                    .with_status_code(status)
                    .with_raw_response(raw),
            )
        } else {
            let msg = raw
                .get("results")
                .and_then(|r| r.as_array())
                .and_then(|arr| arr.first())
                .and_then(|r| r.get("error"))
                .and_then(|e| e.as_str())
                .unwrap_or("unknown error");
            Ok(SendResponse::failure("fcm", format!("FCM error: {msg}"))
                .with_status_code(status)
                .with_raw_response(raw))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // fcm_url tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fcm_url() {
        assert_eq!(fcm_url(), "https://fcm.googleapis.com/fcm/send");
    }

    // -------------------------------------------------------------------------
    // build_fcm_notification tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_build_fcm_notification_basic() {
        let msg = Message::text("hello world");
        let config = ProviderConfig::new();
        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["body"], "hello world");
        assert!(notification["title"].is_null());
        assert_eq!(notification["sound"], "default");
        assert!(notification["icon"].is_null());
        assert!(notification["click_action"].is_null());
        assert!(notification["image"].is_null());
    }

    #[test]
    fn test_build_fcm_notification_with_title() {
        let msg = Message::text("hello world").with_title("Notification Title");
        let config = ProviderConfig::new();
        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["body"], "hello world");
        assert_eq!(notification["title"], "Notification Title");
    }

    #[test]
    fn test_build_fcm_notification_with_icon() {
        let msg = Message::text("hello");
        let config = ProviderConfig::new().set("icon", "ic_notification");
        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["icon"], "ic_notification");
    }

    #[test]
    fn test_build_fcm_notification_custom_sound() {
        let msg = Message::text("hello");
        let config = ProviderConfig::new().set("sound", "custom.caf");
        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["sound"], "custom.caf");
    }

    #[test]
    fn test_build_fcm_notification_default_sound() {
        let msg = Message::text("hello");
        let config = ProviderConfig::new();
        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["sound"], "default");
    }

    #[test]
    fn test_build_fcm_notification_with_click_action() {
        let msg = Message::text("hello");
        let config = ProviderConfig::new().set("click_action", "OPEN_ACTIVITY");
        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["click_action"], "OPEN_ACTIVITY");
    }

    #[test]
    fn test_build_fcm_notification_with_image_url() {
        let msg = Message::text("hello");
        let config = ProviderConfig::new().set("image", "https://example.com/image.png");
        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["image"], "https://example.com/image.png");
    }

    #[test]
    fn test_build_fcm_notification_all_fields() {
        let msg = Message::text("hello world").with_title("Title");
        let config = ProviderConfig::new()
            .set("icon", "ic_stat_icon")
            .set("sound", "custom.caf")
            .set("click_action", "FLUTTER_NOTIFICATION_CLICK")
            .set("image", "https://example.com/banner.png");

        let notification = build_fcm_notification(&msg, &config);

        assert_eq!(notification["body"], "hello world");
        assert_eq!(notification["title"], "Title");
        assert_eq!(notification["icon"], "ic_stat_icon");
        assert_eq!(notification["sound"], "custom.caf");
        assert_eq!(notification["click_action"], "FLUTTER_NOTIFICATION_CLICK");
        assert_eq!(notification["image"], "https://example.com/banner.png");
    }

    // -------------------------------------------------------------------------
    // parse_ttl tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_ttl_valid() {
        assert_eq!(parse_ttl(Some("3600")), 3600);
        assert_eq!(parse_ttl(Some("0")), 0);
        assert_eq!(parse_ttl(Some("86400")), 86400);
        assert_eq!(parse_ttl(Some("2419200")), 2419200);
    }

    #[test]
    fn test_parse_ttl_none() {
        assert_eq!(parse_ttl(None), 2419200);
    }

    #[test]
    fn test_parse_ttl_invalid() {
        // Non-numeric string falls back to default
        assert_eq!(parse_ttl(Some("invalid")), 2419200);
        assert_eq!(parse_ttl(Some("")), 2419200);
        assert_eq!(parse_ttl(Some("-100")), 2419200);
    }

    #[test]
    fn test_parse_ttl_large_number() {
        assert_eq!(parse_ttl(Some("9999999999")), 9999999999);
    }

    // -------------------------------------------------------------------------
    // fcm_priority tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fcm_priority_high() {
        let config = ProviderConfig::new();
        assert_eq!(fcm_priority(&config), "high");
    }

    #[test]
    fn test_fcm_priority_explicit_high() {
        let config = ProviderConfig::new().set("priority", "high");
        assert_eq!(fcm_priority(&config), "high");
    }

    #[test]
    fn test_fcm_priority_normal() {
        let config = ProviderConfig::new().set("priority", "normal");
        assert_eq!(fcm_priority(&config), "normal");
    }

    #[test]
    fn test_fcm_priority_unknown_falls_back_to_high() {
        // Any non-"normal" value falls back to "high"
        let config = ProviderConfig::new().set("priority", "urgent");
        assert_eq!(fcm_priority(&config), "high");

        let config = ProviderConfig::new().set("priority", "low");
        assert_eq!(fcm_priority(&config), "high");
    }

    // -------------------------------------------------------------------------
    // FcmProvider trait implementation tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_fcm_provider_name() {
        let provider = FcmProvider::new(reqwest::Client::new());
        assert_eq!(provider.name(), "fcm");
    }

    #[test]
    fn test_fcm_provider_url_scheme() {
        let provider = FcmProvider::new(reqwest::Client::new());
        assert_eq!(provider.url_scheme(), "fcm");
    }

    #[test]
    fn test_fcm_provider_description() {
        let provider = FcmProvider::new(reqwest::Client::new());
        assert!(provider.description().contains("Firebase"));
    }

    #[test]
    fn test_fcm_provider_example_url() {
        let provider = FcmProvider::new(reqwest::Client::new());
        assert!(provider.example_url().contains("fcm://"));
    }

    #[test]
    fn test_fcm_provider_supports_attachments() {
        let provider = FcmProvider::new(reqwest::Client::new());
        assert!(provider.supports_attachments());
    }

    #[test]
    fn test_fcm_provider_params_contains_required() {
        let provider = FcmProvider::new(reqwest::Client::new());
        let params = provider.params();

        // Should have required params: server_key, device_token
        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();
        assert!(param_names.contains(&"server_key"));
        assert!(param_names.contains(&"device_token"));
    }

    #[test]
    fn test_fcm_provider_params_contains_optional() {
        let provider = FcmProvider::new(reqwest::Client::new());
        let params = provider.params();

        let param_names: Vec<_> = params.iter().map(|p| p.name.as_str()).collect();
        // Optional params
        assert!(param_names.contains(&"topic"));
        assert!(param_names.contains(&"condition"));
        assert!(param_names.contains(&"priority"));
        assert!(param_names.contains(&"collapse_key"));
        assert!(param_names.contains(&"ttl"));
        assert!(param_names.contains(&"icon"));
        assert!(param_names.contains(&"sound"));
        assert!(param_names.contains(&"click_action"));
        assert!(param_names.contains(&"image"));
    }

    #[test]
    fn test_fcm_provider_params_count() {
        let provider = FcmProvider::new(reqwest::Client::new());
        let params = provider.params();
        // 2 required + 10 optional = 12 total
        assert_eq!(params.len(), 12);
    }
}
