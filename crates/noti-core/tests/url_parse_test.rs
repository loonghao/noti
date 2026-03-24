use noti_core::parse_notification_url;
use rstest::rstest;

#[rstest]
#[case("wecom://abc123", "wecom", "key", "abc123")]
#[case("feishu://hook-uuid-here", "feishu", "hook_id", "hook-uuid-here")]
#[case("lark://hook-uuid-here", "feishu", "hook_id", "hook-uuid-here")]
fn test_simple_url_parse(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] expected_key: &str,
    #[case] expected_value: &str,
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    assert_eq!(parsed.config.get(expected_key), Some(expected_value));
}

#[rstest]
#[case(
    "tg://bot123:token/chat456",
    "tg",
    "bot_token",
    "bot123:token",
    "chat_id",
    "chat456"
)]
#[case(
    "discord://webhook_id/webhook_token",
    "discord",
    "webhook_id",
    "webhook_id",
    "webhook_token",
    "webhook_token"
)]
fn test_two_part_url_parse(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] key1: &str,
    #[case] value1: &str,
    #[case] key2: &str,
    #[case] value2: &str,
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    assert_eq!(parsed.config.get(key1), Some(value1));
    assert_eq!(parsed.config.get(key2), Some(value2));
}

#[rstest]
#[case("slack://T123/B456/xxx789")]
fn test_slack_url_parse(#[case] url: &str) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, "slack");
    assert_eq!(
        parsed.config.get("webhook_url"),
        Some("https://hooks.slack.com/services/T123/B456/xxx789")
    );
}

#[rstest]
#[case(
    "feishu://hook123?secret=mysecret",
    "feishu",
    "hook_id",
    "hook123",
    "secret",
    "mysecret"
)]
fn test_url_with_query_params(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] key1: &str,
    #[case] value1: &str,
    #[case] key2: &str,
    #[case] value2: &str,
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    assert_eq!(parsed.config.get(key1), Some(value1));
    assert_eq!(parsed.config.get(key2), Some(value2));
}

#[rstest]
#[case("smtp://user:pass@smtp.gmail.com:587?to=recipient@example.com")]
fn test_smtp_url_parse(#[case] url: &str) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, "smtp");
    assert_eq!(parsed.config.get("username"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("smtp.gmail.com"));
    assert_eq!(parsed.config.get("port"), Some("587"));
    assert_eq!(parsed.config.get("to"), Some("recipient@example.com"));
}

#[rstest]
#[case("webhook://example.com/api/notify")]
fn test_webhook_url_parse(#[case] url: &str) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, "webhook");
    assert_eq!(
        parsed.config.get("url"),
        Some("https://example.com/api/notify")
    );
}

#[rstest]
#[case("unknown://something")]
fn test_unknown_scheme_fails(#[case] url: &str) {
    let result = parse_notification_url(url);
    assert!(result.is_err());
}

#[rstest]
#[case("not-a-url")]
fn test_missing_scheme_fails(#[case] url: &str) {
    let result = parse_notification_url(url);
    assert!(result.is_err());
}
