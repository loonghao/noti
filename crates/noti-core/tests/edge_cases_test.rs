/// Additional edge case tests for noti-core types to improve coverage.
use noti_core::{
    AppConfig, Message, MessageFormat, NotiError, ParamDef, Profile, ProviderConfig, SendResponse,
};
use rstest::rstest;

// ======================== NotiError additional tests ========================

#[rstest]
fn test_noti_error_is_debug() {
    let err = NotiError::Config("test".into());
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("Config"));
}

#[rstest]
fn test_noti_error_provider_with_string_types() {
    let err = NotiError::provider(String::from("test_provider"), String::from("test_message"));
    assert_eq!(err.to_string(), "provider error (test_provider): test_message");
}

#[rstest]
fn test_noti_error_all_variants_are_error() {
    let errors: Vec<NotiError> = vec![
        NotiError::Config("c".into()),
        NotiError::Provider {
            provider: "p".into(),
            message: "m".into(),
        },
        NotiError::UrlParse("u".into()),
        NotiError::Network("n".into()),
        NotiError::Validation("v".into()),
    ];
    for err in &errors {
        // All errors should implement Display via thiserror
        let display = err.to_string();
        assert!(!display.is_empty());
    }
}

// ======================== Message additional tests ========================

#[rstest]
fn test_message_text_empty_string() {
    let msg = Message::text("");
    assert_eq!(msg.text, "");
}

#[rstest]
fn test_message_text_unicode() {
    let msg = Message::text("🔔 通知消息 — notification");
    assert!(msg.text.contains("🔔"));
    assert!(msg.text.contains("通知消息"));
}

#[rstest]
fn test_message_text_multiline() {
    let msg = Message::text("line1\nline2\nline3");
    assert!(msg.text.contains('\n'));
}

#[rstest]
fn test_message_markdown_html_format() {
    let msg = Message::text("body").with_format(MessageFormat::Html);
    assert_eq!(msg.format, MessageFormat::Html);
    assert_eq!(msg.format.to_string(), "html");
}

#[rstest]
fn test_message_with_extra_overwrite() {
    let msg = Message::text("test")
        .with_extra("key", serde_json::json!("first"))
        .with_extra("key", serde_json::json!("second"));
    assert_eq!(msg.extra.get("key"), Some(&serde_json::json!("second")));
    assert_eq!(msg.extra.len(), 1);
}

#[rstest]
fn test_message_with_extra_complex_value() {
    let msg = Message::text("test")
        .with_extra("nested", serde_json::json!({"a": [1, 2, 3]}));
    let val = msg.extra.get("nested").unwrap();
    assert!(val.is_object());
}

#[rstest]
fn test_message_serde_with_extra() {
    let msg = Message::text("hello")
        .with_extra("priority", serde_json::json!("high"))
        .with_extra("count", serde_json::json!(42));
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("extra"));
    assert!(json.contains("priority"));

    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.extra.get("priority"), Some(&serde_json::json!("high")));
    assert_eq!(parsed.extra.get("count"), Some(&serde_json::json!(42)));
}

#[rstest]
fn test_message_serde_with_format_roundtrip() {
    for format in [MessageFormat::Text, MessageFormat::Markdown, MessageFormat::Html] {
        let msg = Message::text("test").with_format(format.clone());
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.format, format);
    }
}

#[rstest]
fn test_message_format_clone() {
    let format = MessageFormat::Markdown;
    let cloned = format.clone();
    assert_eq!(format, cloned);
}

#[rstest]
fn test_message_clone() {
    let msg = Message::text("hello")
        .with_title("Title")
        .with_format(MessageFormat::Html)
        .with_extra("key", serde_json::json!("val"));
    let cloned = msg.clone();
    assert_eq!(cloned.text, msg.text);
    assert_eq!(cloned.title, msg.title);
    assert_eq!(cloned.format, msg.format);
    assert_eq!(cloned.extra.len(), msg.extra.len());
}

// ======================== MessageFormat additional tests ========================

#[rstest]
fn test_message_format_equality() {
    assert_eq!(MessageFormat::Text, MessageFormat::Text);
    assert_eq!(MessageFormat::Markdown, MessageFormat::Markdown);
    assert_eq!(MessageFormat::Html, MessageFormat::Html);
    assert_ne!(MessageFormat::Text, MessageFormat::Markdown);
    assert_ne!(MessageFormat::Text, MessageFormat::Html);
    assert_ne!(MessageFormat::Markdown, MessageFormat::Html);
}

#[rstest]
fn test_message_format_debug() {
    let debug_text = format!("{:?}", MessageFormat::Text);
    assert_eq!(debug_text, "Text");
    let debug_md = format!("{:?}", MessageFormat::Markdown);
    assert_eq!(debug_md, "Markdown");
    let debug_html = format!("{:?}", MessageFormat::Html);
    assert_eq!(debug_html, "Html");
}

// ======================== ProviderConfig additional tests ========================

#[rstest]
fn test_provider_config_many_values() {
    let mut config = ProviderConfig::new();
    for i in 0..100 {
        config = config.set(format!("key_{i}"), format!("value_{i}"));
    }
    assert_eq!(config.values.len(), 100);
    for i in 0..100 {
        assert_eq!(
            config.get(&format!("key_{i}")),
            Some(format!("value_{i}")).as_deref()
        );
    }
}

#[rstest]
fn test_provider_config_empty_key_and_value() {
    let config = ProviderConfig::new().set("", "").set("normal", "");
    assert_eq!(config.get(""), Some(""));
    assert_eq!(config.get("normal"), Some(""));
}

#[rstest]
fn test_provider_config_require_error_message() {
    let config = ProviderConfig::new();
    let err = config.require("api_key", "slack").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("api_key"), "error should mention the missing key");
    assert!(msg.contains("slack"), "error should mention the provider name");
}

#[rstest]
fn test_provider_config_clone() {
    let config = ProviderConfig::new().set("key", "value");
    let cloned = config.clone();
    assert_eq!(cloned.get("key"), Some("value"));
}

// ======================== ParamDef additional tests ========================

#[rstest]
fn test_param_def_debug() {
    let param = ParamDef::required("key", "desc");
    let debug = format!("{:?}", param);
    assert!(debug.contains("key"));
    assert!(debug.contains("desc"));
}

#[rstest]
fn test_param_def_clone() {
    let param = ParamDef::required("key", "desc").with_example("example");
    let cloned = param.clone();
    assert_eq!(cloned.name, param.name);
    assert_eq!(cloned.description, param.description);
    assert_eq!(cloned.required, param.required);
    assert_eq!(cloned.example, param.example);
}

#[rstest]
fn test_param_def_optional_with_example() {
    let param = ParamDef::optional("channel", "Channel name").with_example("#general");
    assert!(!param.required);
    assert_eq!(param.example, Some("#general".to_string()));
}

// ======================== SendResponse additional tests ========================

#[rstest]
fn test_send_response_debug() {
    let resp = SendResponse::success("test", "ok");
    let debug = format!("{:?}", resp);
    assert!(debug.contains("test"));
    assert!(debug.contains("ok"));
}

#[rstest]
fn test_send_response_clone() {
    let resp = SendResponse::success("test", "ok")
        .with_status_code(200)
        .with_raw_response(serde_json::json!({"ok": true}));
    let cloned = resp.clone();
    assert_eq!(cloned.success, resp.success);
    assert_eq!(cloned.provider, resp.provider);
    assert_eq!(cloned.status_code, resp.status_code);
    assert_eq!(cloned.raw_response, resp.raw_response);
}

#[rstest]
fn test_send_response_serde_complete() {
    let resp = SendResponse::success("test", "message sent")
        .with_status_code(200)
        .with_raw_response(serde_json::json!({"result": "ok", "code": 0}));
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SendResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.success, true);
    assert_eq!(parsed.provider, "test");
    assert_eq!(parsed.message, "message sent");
    assert_eq!(parsed.status_code, Some(200));
    assert!(parsed.raw_response.is_some());
}

// ======================== Profile additional tests ========================

#[rstest]
fn test_profile_debug() {
    let profile = Profile {
        provider: "wecom".to_string(),
        config: ProviderConfig::new().set("key", "val"),
    };
    let debug = format!("{:?}", profile);
    assert!(debug.contains("wecom"));
}

#[rstest]
fn test_profile_clone() {
    let profile = Profile {
        provider: "slack".to_string(),
        config: ProviderConfig::new().set("webhook_url", "https://..."),
    };
    let cloned = profile.clone();
    assert_eq!(cloned.provider, profile.provider);
    assert_eq!(cloned.config.get("webhook_url"), profile.config.get("webhook_url"));
}

// ======================== AppConfig additional tests ========================

#[rstest]
fn test_app_config_debug() {
    let config = AppConfig::default();
    let debug = format!("{:?}", config);
    assert!(debug.contains("AppConfig"));
}

#[rstest]
fn test_app_config_clone() {
    let mut config = AppConfig::default();
    config.set_profile(
        "test",
        Profile {
            provider: "slack".into(),
            config: ProviderConfig::new(),
        },
    );
    let cloned = config.clone();
    assert!(cloned.get_profile("test").is_some());
}

#[rstest]
fn test_app_config_serde_roundtrip() {
    let mut config = AppConfig::default();
    config.set_profile(
        "work",
        Profile {
            provider: "wecom".into(),
            config: ProviderConfig::new().set("key", "abc123"),
        },
    );
    config.set_profile(
        "personal",
        Profile {
            provider: "telegram".into(),
            config: ProviderConfig::new()
                .set("bot_token", "123:ABC")
                .set("chat_id", "-100"),
        },
    );

    let toml_str = toml::to_string_pretty(&config).unwrap();
    let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.profiles.len(), 2);
    assert!(parsed.get_profile("work").is_some());
    assert!(parsed.get_profile("personal").is_some());
}

// ======================== ParsedUrl tests ========================

#[rstest]
fn test_parsed_url_debug_and_clone() {
    use noti_core::parse_notification_url;
    let parsed = parse_notification_url("wecom://test-key").unwrap();
    let debug = format!("{:?}", parsed);
    assert!(debug.contains("wecom"));

    let cloned = parsed.clone();
    assert_eq!(cloned.scheme, parsed.scheme);
}
