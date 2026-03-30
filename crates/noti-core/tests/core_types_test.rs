use noti_core::{Message, MessageFormat, NotiError, ParamDef, ProviderConfig, SendResponse};
use rstest::rstest;

// ======================== MessageFormat tests ========================

#[rstest]
#[case(MessageFormat::Text, "text")]
#[case(MessageFormat::Markdown, "markdown")]
#[case(MessageFormat::Html, "html")]
fn test_message_format_display(#[case] format: MessageFormat, #[case] expected: &str) {
    assert_eq!(format.to_string(), expected);
}

#[rstest]
#[case("text", MessageFormat::Text)]
#[case("TEXT", MessageFormat::Text)]
#[case("Text", MessageFormat::Text)]
#[case("markdown", MessageFormat::Markdown)]
#[case("MARKDOWN", MessageFormat::Markdown)]
#[case("md", MessageFormat::Markdown)]
#[case("MD", MessageFormat::Markdown)]
#[case("html", MessageFormat::Html)]
#[case("HTML", MessageFormat::Html)]
fn test_message_format_from_str_valid(#[case] input: &str, #[case] expected: MessageFormat) {
    let parsed: MessageFormat = input.parse().unwrap();
    assert_eq!(parsed, expected);
}

#[rstest]
#[case("json")]
#[case("xml")]
#[case("")]
#[case("unknown")]
fn test_message_format_from_str_invalid(#[case] input: &str) {
    let result = input.parse::<MessageFormat>();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("unknown message format"));
}

#[rstest]
fn test_message_format_default() {
    let format = MessageFormat::default();
    assert_eq!(format, MessageFormat::Text);
}

#[rstest]
fn test_message_format_serde_roundtrip() {
    let text = serde_json::to_string(&MessageFormat::Text).unwrap();
    assert_eq!(text, "\"text\"");
    let md = serde_json::to_string(&MessageFormat::Markdown).unwrap();
    assert_eq!(md, "\"markdown\"");
    let html = serde_json::to_string(&MessageFormat::Html).unwrap();
    assert_eq!(html, "\"html\"");

    let parsed: MessageFormat = serde_json::from_str("\"text\"").unwrap();
    assert_eq!(parsed, MessageFormat::Text);
    let parsed: MessageFormat = serde_json::from_str("\"markdown\"").unwrap();
    assert_eq!(parsed, MessageFormat::Markdown);
    let parsed: MessageFormat = serde_json::from_str("\"html\"").unwrap();
    assert_eq!(parsed, MessageFormat::Html);
}

// ======================== Message tests ========================

#[rstest]
fn test_message_text_constructor() {
    let msg = Message::text("hello world");
    assert_eq!(msg.text, "hello world");
    assert_eq!(msg.title, None);
    assert_eq!(msg.format, MessageFormat::Text);
    assert!(msg.extra.is_empty());
}

#[rstest]
fn test_message_markdown_constructor() {
    let msg = Message::markdown("# Title\nBody");
    assert_eq!(msg.text, "# Title\nBody");
    assert_eq!(msg.format, MessageFormat::Markdown);
    assert_eq!(msg.title, None);
}

#[rstest]
fn test_message_with_title() {
    let msg = Message::text("body").with_title("My Title");
    assert_eq!(msg.title, Some("My Title".to_string()));
}

#[rstest]
fn test_message_with_format() {
    let msg = Message::text("body").with_format(MessageFormat::Html);
    assert_eq!(msg.format, MessageFormat::Html);
}

#[rstest]
fn test_message_with_extra() {
    let msg = Message::text("body")
        .with_extra("key1", serde_json::json!("value1"))
        .with_extra("key2", serde_json::json!(42));
    assert_eq!(msg.extra.get("key1"), Some(&serde_json::json!("value1")));
    assert_eq!(msg.extra.get("key2"), Some(&serde_json::json!(42)));
    assert_eq!(msg.extra.len(), 2);
}

#[rstest]
fn test_message_builder_chain() {
    let msg = Message::text("hello")
        .with_title("Title")
        .with_format(MessageFormat::Markdown)
        .with_extra("priority", serde_json::json!("high"));
    assert_eq!(msg.text, "hello");
    assert_eq!(msg.title, Some("Title".to_string()));
    assert_eq!(msg.format, MessageFormat::Markdown);
    assert_eq!(msg.extra.get("priority"), Some(&serde_json::json!("high")));
}

#[rstest]
fn test_message_serde_roundtrip() {
    let msg = Message::text("hello").with_title("Title");
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.text, "hello");
    assert_eq!(parsed.title, Some("Title".to_string()));
    assert_eq!(parsed.format, MessageFormat::Text);
}

#[rstest]
fn test_message_serde_skip_empty_extra() {
    let msg = Message::text("hello");
    let json = serde_json::to_string(&msg).unwrap();
    // extra should not be serialized when empty
    assert!(!json.contains("extra"));
}

#[rstest]
fn test_message_serde_skip_none_title() {
    let msg = Message::text("hello");
    let json = serde_json::to_string(&msg).unwrap();
    // title should not be serialized when None
    assert!(!json.contains("title"));
}

// ======================== NotiError tests ========================

#[rstest]
fn test_noti_error_config_display() {
    let err = NotiError::Config("bad config".to_string());
    assert_eq!(err.to_string(), "config error: bad config");
}

#[rstest]
fn test_noti_error_provider_display() {
    let err = NotiError::Provider {
        provider: "wecom".to_string(),
        message: "auth failed".to_string(),
    };
    assert_eq!(err.to_string(), "provider error (wecom): auth failed");
}

#[rstest]
fn test_noti_error_url_parse_display() {
    let err = NotiError::UrlParse("missing scheme".to_string());
    assert_eq!(err.to_string(), "url parse error: missing scheme");
}

#[rstest]
fn test_noti_error_network_display() {
    let err = NotiError::Network("timeout".to_string());
    assert_eq!(err.to_string(), "network error: timeout");
}

#[rstest]
fn test_noti_error_validation_display() {
    let err = NotiError::Validation("missing key".to_string());
    assert_eq!(err.to_string(), "validation error: missing key");
}

#[rstest]
fn test_noti_error_provider_convenience_constructor() {
    let err = NotiError::provider("slack", "rate limited");
    match err {
        NotiError::Provider { provider, message } => {
            assert_eq!(provider, "slack");
            assert_eq!(message, "rate limited");
        }
        _ => panic!("expected Provider variant"),
    }
}

// ======================== ParamDef tests ========================

#[rstest]
fn test_param_def_required() {
    let param = ParamDef::required("api_key", "Your API key");
    assert_eq!(param.name, "api_key");
    assert_eq!(param.description, "Your API key");
    assert!(param.required);
    assert_eq!(param.example, None);
}

#[rstest]
fn test_param_def_optional() {
    let param = ParamDef::optional("channel", "Override channel");
    assert_eq!(param.name, "channel");
    assert_eq!(param.description, "Override channel");
    assert!(!param.required);
    assert_eq!(param.example, None);
}

#[rstest]
fn test_param_def_with_example() {
    let param = ParamDef::required("key", "desc").with_example("abc-123");
    assert_eq!(param.example, Some("abc-123".to_string()));
}

#[rstest]
fn test_param_def_serde_roundtrip() {
    let param = ParamDef::required("key", "desc").with_example("example");
    let json = serde_json::to_string(&param).unwrap();
    let parsed: ParamDef = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "key");
    assert_eq!(parsed.description, "desc");
    assert!(parsed.required);
    assert_eq!(parsed.example, Some("example".to_string()));
}

#[rstest]
fn test_param_def_serde_skip_none_example() {
    let param = ParamDef::required("key", "desc");
    let json = serde_json::to_string(&param).unwrap();
    assert!(!json.contains("example"));
}

// ======================== ProviderConfig tests ========================

#[rstest]
fn test_provider_config_new() {
    let config = ProviderConfig::new();
    assert!(config.values.is_empty());
}

#[rstest]
fn test_provider_config_default() {
    let config = ProviderConfig::default();
    assert!(config.values.is_empty());
}

#[rstest]
fn test_provider_config_set_and_get() {
    let config = ProviderConfig::new()
        .set("key1", "value1")
        .set("key2", "value2");
    assert_eq!(config.get("key1"), Some("value1"));
    assert_eq!(config.get("key2"), Some("value2"));
    assert_eq!(config.get("key3"), None);
}

#[rstest]
fn test_provider_config_set_overwrites() {
    let config = ProviderConfig::new()
        .set("key", "first")
        .set("key", "second");
    assert_eq!(config.get("key"), Some("second"));
}

#[rstest]
fn test_provider_config_require_success() {
    let config = ProviderConfig::new().set("key", "value");
    let result = config.require("key", "test_provider");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "value");
}

#[rstest]
fn test_provider_config_require_failure() {
    let config = ProviderConfig::new();
    let result = config.require("missing_key", "test_provider");
    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        NotiError::Validation(msg) => {
            assert!(msg.contains("missing_key"));
            assert!(msg.contains("test_provider"));
        }
        _ => panic!("expected Validation error"),
    }
}

#[rstest]
fn test_provider_config_serde_roundtrip() {
    let config = ProviderConfig::new()
        .set("key", "value")
        .set("token", "abc");
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ProviderConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.get("key"), Some("value"));
    assert_eq!(parsed.get("token"), Some("abc"));
}

// ======================== SendResponse tests ========================

#[rstest]
fn test_send_response_success() {
    let resp = SendResponse::success("wecom", "sent ok");
    assert!(resp.success);
    assert_eq!(resp.provider, "wecom");
    assert_eq!(resp.message, "sent ok");
    assert_eq!(resp.status_code, None);
    assert_eq!(resp.raw_response, None);
}

#[rstest]
fn test_send_response_failure() {
    let resp = SendResponse::failure("slack", "auth error");
    assert!(!resp.success);
    assert_eq!(resp.provider, "slack");
    assert_eq!(resp.message, "auth error");
}

#[rstest]
fn test_send_response_with_status_code() {
    let resp = SendResponse::success("wecom", "ok").with_status_code(200);
    assert_eq!(resp.status_code, Some(200));
}

#[rstest]
fn test_send_response_with_raw_response() {
    let raw = serde_json::json!({"errcode": 0});
    let resp = SendResponse::success("wecom", "ok").with_raw_response(raw.clone());
    assert_eq!(resp.raw_response, Some(raw));
}

#[rstest]
fn test_send_response_builder_chain() {
    let raw = serde_json::json!({"ok": true});
    let resp = SendResponse::failure("discord", "rate limited")
        .with_status_code(429)
        .with_raw_response(raw.clone());
    assert!(!resp.success);
    assert_eq!(resp.provider, "discord");
    assert_eq!(resp.message, "rate limited");
    assert_eq!(resp.status_code, Some(429));
    assert_eq!(resp.raw_response, Some(raw));
}

#[rstest]
fn test_send_response_serde_roundtrip() {
    let resp = SendResponse::success("test", "ok")
        .with_status_code(200)
        .with_raw_response(serde_json::json!({"result": "ok"}));
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SendResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.success, true);
    assert_eq!(parsed.provider, "test");
    assert_eq!(parsed.status_code, Some(200));
}

#[rstest]
fn test_send_response_serde_skip_none_fields() {
    let resp = SendResponse::success("test", "ok");
    let json = serde_json::to_string(&resp).unwrap();
    assert!(!json.contains("status_code"));
    assert!(!json.contains("raw_response"));
}
