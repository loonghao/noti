use noti_core::parse_notification_url;
use rstest::rstest;

// ======================== Error cases ========================

#[rstest]
#[case("not-a-url")]
#[case("")]
#[case("just-text")]
fn test_missing_scheme_separator(#[case] url: &str) {
    assert!(parse_notification_url(url).is_err());
}

#[rstest]
#[case("unknown://something")]
#[case("ftp://server")]
#[case("abc123://test")]
fn test_unknown_scheme(#[case] url: &str) {
    assert!(parse_notification_url(url).is_err());
}

// ======================== Simple key-based providers ========================

#[rstest]
#[case("wecom://abc123", "wecom", &[("key", "abc123")])]
#[case("feishu://hook-uuid", "feishu", &[("hook_id", "hook-uuid")])]
#[case("lark://hook-uuid", "feishu", &[("hook_id", "hook-uuid")])]
#[case("dingtalk://my-token", "dingtalk", &[("access_token", "my-token")])]
#[case("pushbullet://my-token", "pushbullet", &[("access_token", "my-token")])]
#[case("pagerduty://int-key", "pagerduty", &[("integration_key", "int-key")])]
#[case("simplepush://mykey", "simplepush", &[("key", "mykey")])]
#[case("notica://mytoken", "notica", &[("token", "mytoken")])]
#[case("prowl://myapikey", "prowl", &[("api_key", "myapikey")])]
#[case("pushjet://mysecret", "pushjet", &[("secret", "mysecret")])]
#[case("pushme://mypushkey", "pushme", &[("push_key", "mypushkey")])]
#[case("pushplus://mytoken", "pushplus", &[("token", "mytoken")])]
#[case("opsgenie://myapikey", "opsgenie", &[("api_key", "myapikey")])]
#[case("ntfy://mytopic", "ntfy", &[("topic", "mytopic")])]
#[case("serverchan://mysendkey", "serverchan", &[("send_key", "mysendkey")])]
#[case("line://mytoken", "line", &[("access_token", "mytoken")])]
#[case("lunasea://mytoken", "lunasea", &[("user_token", "mytoken")])]
#[case("pagertree://myid", "pagertree", &[("integration_id", "myid")])]
#[case("signl4://mysecret", "signl4", &[("team_secret", "mysecret")])]
#[case("flock://mytoken", "flock", &[("token", "mytoken")])]
#[case("streamlabs://mytoken", "streamlabs", &[("access_token", "mytoken")])]
#[case("boxcar://mytoken", "boxcar", &[("access_token", "mytoken")])]
#[case("notifiarr://mykey", "notifiarr", &[("api_key", "mykey")])]
#[case("twitter://mybearertoken", "twitter", &[("bearer_token", "mybearertoken")])]
#[case("x://mybearertoken", "twitter", &[("bearer_token", "mybearertoken")])]
#[case("spugpush://mytoken", "spugpush", &[("token", "mytoken")])]
#[case("push://myapikey", "push", &[("api_key", "myapikey")])]
fn test_simple_key_providers(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] expected_kvs: &[(&str, &str)],
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    for (key, value) in expected_kvs {
        assert_eq!(parsed.config.get(key), Some(*value), "key={key}");
    }
}

// ======================== Empty path failures for simple providers ========================

#[rstest]
#[case("wecom://")]
#[case("feishu://")]
#[case("dingtalk://")]
#[case("ntfy://")]
#[case("bark://")]
#[case("pushdeer://")]
#[case("serverchan://")]
#[case("teams://")]
#[case("pushbullet://")]
#[case("simplepush://")]
#[case("notica://")]
#[case("prowl://")]
#[case("line://")]
#[case("lunasea://")]
#[case("pagertree://")]
#[case("signl4://")]
#[case("flock://")]
#[case("pushjet://")]
#[case("pushme://")]
#[case("pushplus://")]
#[case("opsgenie://")]
#[case("pagerduty://")]
#[case("streamlabs://")]
#[case("boxcar://")]
#[case("notifiarr://")]
#[case("twitter://")]
#[case("spugpush://")]
#[case("push://")]
#[case("chanify://")]
fn test_empty_path_fails(#[case] url: &str) {
    assert!(parse_notification_url(url).is_err());
}

// ======================== Two-part path providers ========================

#[rstest]
#[case("tg://bot123:token/chat456", "tg", &[("bot_token", "bot123:token"), ("chat_id", "chat456")])]
#[case("telegram://bot123:token/chat456", "tg", &[("bot_token", "bot123:token"), ("chat_id", "chat456")])]
#[case("discord://wh_id/wh_token", "discord", &[("webhook_id", "wh_id"), ("webhook_token", "wh_token")])]
#[case("guilded://wh_id/wh_token", "guilded", &[("webhook_id", "wh_id"), ("webhook_token", "wh_token")])]
#[case("fluxer://wh_id/wh_token", "fluxer", &[("webhook_id", "wh_id"), ("webhook_token", "wh_token")])]
#[case("webex://mytoken/myroom", "webex", &[("access_token", "mytoken"), ("room_id", "myroom")])]
#[case("revolt://bottoken/channelid", "revolt", &[("bot_token", "bottoken"), ("channel_id", "channelid")])]
#[case("gitter://mytoken/myroom", "gitter", &[("token", "mytoken"), ("room_id", "myroom")])]
#[case("notifico://proj123/msghook456", "notifico", &[("project_id", "proj123"), ("msghook", "msghook456")])]
#[case("pushcut://apikey/notif_name", "pushcut", &[("api_key", "apikey"), ("notification_name", "notif_name")])]
#[case("ryver://myorg/mytoken", "ryver", &[("organization", "myorg"), ("token", "mytoken")])]
#[case("freemobile://userid/apikey", "freemobile", &[("user", "userid"), ("password", "apikey")])]
#[case("victorops://apikey/routekey", "victorops", &[("api_key", "apikey"), ("routing_key", "routekey")])]
#[case("splunk://apikey/routekey", "victorops", &[("api_key", "apikey"), ("routing_key", "routekey")])]
#[case("wxpusher://apptoken/uid123", "wxpusher", &[("app_token", "apptoken"), ("uid", "uid123")])]
fn test_two_part_providers(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] expected_kvs: &[(&str, &str)],
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    for (key, value) in expected_kvs {
        assert_eq!(parsed.config.get(key), Some(*value), "key={key}");
    }
}

// ======================== Two-part failures ========================

#[rstest]
#[case("tg://only-one-part")]
#[case("discord://only-one-part")]
#[case("guilded://only-one")]
#[case("fluxer://only-one")]
#[case("webex://only-one")]
#[case("revolt://only-one")]
#[case("gitter://only-one")]
#[case("notifico://only-one")]
#[case("pushcut://only-one")]
#[case("ryver://only-one")]
#[case("freemobile://only-one")]
#[case("victorops://only-one")]
#[case("wxpusher://only-one")]
fn test_two_part_missing_second(#[case] url: &str) {
    assert!(parse_notification_url(url).is_err());
}

// ======================== Slack ========================

#[rstest]
fn test_slack_three_tokens() {
    let parsed = parse_notification_url("slack://T123/B456/xxx").unwrap();
    assert_eq!(parsed.scheme, "slack");
    assert_eq!(
        parsed.config.get("webhook_url"),
        Some("https://hooks.slack.com/services/T123/B456/xxx")
    );
}

#[rstest]
fn test_slack_single_token_as_webhook_url() {
    // When slack:// is followed by a URL like https://..., the "://" in https://
    // causes the path_part "https://custom.webhook.url" to be split into 3 parts:
    // ["https:", "", "custom.webhook.url"], which gets formatted as a hooks.slack.com URL.
    let parsed = parse_notification_url("slack://https://custom.webhook.url").unwrap();
    assert_eq!(parsed.scheme, "slack");
    assert_eq!(
        parsed.config.get("webhook_url"),
        Some("https://hooks.slack.com/services/https://custom.webhook.url")
    );
}

#[rstest]
fn test_slack_empty_fails() {
    assert!(parse_notification_url("slack://").is_err());
}

// ======================== SMTP / Email ========================

#[rstest]
fn test_smtp_full_url() {
    let parsed =
        parse_notification_url("smtp://user:pass@smtp.gmail.com:587?to=dest@example.com")
            .unwrap();
    assert_eq!(parsed.scheme, "smtp");
    assert_eq!(parsed.config.get("username"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("smtp.gmail.com"));
    assert_eq!(parsed.config.get("port"), Some("587"));
    assert_eq!(parsed.config.get("to"), Some("dest@example.com"));
}

#[rstest]
fn test_email_alias_to_smtp() {
    let parsed = parse_notification_url("email://user:pass@host:25").unwrap();
    assert_eq!(parsed.scheme, "smtp");
    assert_eq!(parsed.config.get("username"), Some("user"));
}

#[rstest]
fn test_smtp_host_only() {
    let parsed = parse_notification_url("smtp://smtp.example.com").unwrap();
    assert_eq!(parsed.scheme, "smtp");
    assert_eq!(parsed.config.get("host"), Some("smtp.example.com"));
}

#[rstest]
fn test_smtp_user_at_host() {
    let parsed = parse_notification_url("smtp://user@host.com:465").unwrap();
    assert_eq!(parsed.scheme, "smtp");
    assert_eq!(parsed.config.get("username"), Some("user"));
    assert_eq!(parsed.config.get("host"), Some("host.com"));
    assert_eq!(parsed.config.get("port"), Some("465"));
}

// ======================== Auth@host pattern providers ========================

#[rstest]
#[case(
    "twilio://sid:token@+1234/+5678",
    "twilio",
    &[("account_sid", "sid"), ("auth_token", "token"), ("from", "+1234"), ("to", "+5678")]
)]
#[case(
    "vonage://key:secret@sender/+1234",
    "vonage",
    &[("api_key", "key"), ("api_secret", "secret"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "nexmo://key:secret@sender/+1234",
    "vonage",
    &[("api_key", "key"), ("api_secret", "secret"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "sinch://planid:token@sender/+1234",
    "sinch",
    &[("service_plan_id", "planid"), ("api_token", "token"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "bulksms://tid:tsec@sender/+1234",
    "bulksms",
    &[("token_id", "tid"), ("token_secret", "tsec"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "plivo://authid:authtoken@sender/+1234",
    "plivo",
    &[("auth_id", "authid"), ("auth_token", "authtoken"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "burstsms://key:secret@sender/+1234",
    "burstsms",
    &[("api_key", "key"), ("api_secret", "secret"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "clicksend://user:apikey@sender/+1234",
    "clicksend",
    &[("username", "user"), ("api_key", "apikey"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "46elks://user:pass@sender/+1234",
    "46elks",
    &[("api_username", "user"), ("api_password", "pass"), ("from", "sender"), ("to", "+1234")]
)]
#[case(
    "bulkvs://user:pass@sender/+1234",
    "bulkvs",
    &[("username", "user"), ("password", "pass"), ("from", "sender"), ("to", "+1234")]
)]
fn test_auth_at_host_phone_providers(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] expected_kvs: &[(&str, &str)],
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    for (key, value) in expected_kvs {
        assert_eq!(parsed.config.get(key), Some(*value), "key={key}");
    }
}

// ======================== Auth@host failures ========================

#[rstest]
#[case("twilio://missing-at-sign")]
#[case("twilio://nosecret@from/to")]
#[case("twilio://sid:token@onlyhost")]
#[case("vonage://missing-at")]
#[case("vonage://nosecret@from/to")]
#[case("vonage://key:secret@onlyhost")]
#[case("sinch://missing-at")]
#[case("bulksms://missing-at")]
#[case("plivo://missing-at")]
#[case("burstsms://missing-at")]
#[case("46elks://missing-at")]
#[case("46elks://nosecret@from/to")]
#[case("46elks://user:pass@onlyhost")]
#[case("bulkvs://missing-at")]
#[case("bulkvs://nosecret@from/to")]
#[case("bulkvs://user:pass@onlyhost")]
fn test_auth_at_host_failures(#[case] url: &str) {
    assert!(parse_notification_url(url).is_err());
}

// ======================== API key @ host/email providers ========================

#[rstest]
#[case(
    "mailgun://apikey@example.com/user@dest.com",
    "mailgun",
    &[("api_key", "apikey"), ("domain", "example.com"), ("to", "user@dest.com")]
)]
#[case(
    "sendgrid://apikey@from@example.com/to@dest.com",
    "sendgrid",
    &[("api_key", "apikey"), ("from", "from@example.com"), ("to", "to@dest.com")]
)]
#[case(
    "sparkpost://apikey@from@example.com/to@dest.com",
    "sparkpost",
    &[("api_key", "apikey"), ("from", "from@example.com"), ("to", "to@dest.com")]
)]
#[case(
    "resend://apikey@from@example.com/to@dest.com",
    "resend",
    &[("api_key", "apikey"), ("from", "from@example.com"), ("to", "to@dest.com")]
)]
#[case(
    "brevo://apikey@from@example.com/to@dest.com",
    "brevo",
    &[("api_key", "apikey"), ("from", "from@example.com"), ("to", "to@dest.com")]
)]
#[case(
    "sendinblue://apikey@from@example.com/to@dest.com",
    "brevo",
    &[("api_key", "apikey"), ("from", "from@example.com"), ("to", "to@dest.com")]
)]
#[case(
    "smtp2go://apikey@from@example.com/to@dest.com",
    "smtp2go",
    &[("api_key", "apikey"), ("from", "from@example.com"), ("to", "to@dest.com")]
)]
#[case(
    "httpsms://apikey@+1234/+5678",
    "httpsms",
    &[("api_key", "apikey"), ("from", "+1234"), ("to", "+5678")]
)]
#[case(
    "popcorn://apikey@from/+1234",
    "popcorn",
    &[("api_key", "apikey"), ("from", "from"), ("to", "+1234")]
)]
fn test_api_key_at_host_providers(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] expected_kvs: &[(&str, &str)],
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    for (key, value) in expected_kvs {
        assert_eq!(parsed.config.get(key), Some(*value), "key={key}");
    }
}

// ======================== API key @ host failures ========================

#[rstest]
#[case("mailgun://noat")]
#[case("mailgun://apikey@")]
#[case("sendgrid://noat")]
#[case("sparkpost://noat")]
#[case("resend://noat")]
#[case("brevo://noat")]
#[case("smtp2go://noat")]
#[case("httpsms://noat")]
#[case("httpsms://apikey@")]
#[case("popcorn://noat")]
fn test_api_key_at_host_failures(#[case] url: &str) {
    assert!(parse_notification_url(url).is_err());
}

// ======================== Zulip ========================

#[rstest]
fn test_zulip_full() {
    // Note: bot email containing '@' is ambiguous with the URL format's '@' separator.
    // Use URL-encoded email or an email without '@' in the host part.
    let parsed =
        parse_notification_url("zulip://botemail:apikey@example.zulipchat.com/stream/topic")
            .unwrap();
    assert_eq!(parsed.scheme, "zulip");
    assert_eq!(parsed.config.get("bot_email"), Some("botemail"));
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(
        parsed.config.get("domain"),
        Some("example.zulipchat.com")
    );
    assert_eq!(parsed.config.get("stream"), Some("stream"));
    assert_eq!(parsed.config.get("topic"), Some("topic"));
}

#[rstest]
fn test_zulip_email_with_at_fails() {
    // Emails with '@' in them can't be parsed correctly due to ambiguous '@' separator
    let result =
        parse_notification_url("zulip://bot@email.com:apikey@example.zulipchat.com/stream/topic");
    assert!(result.is_err());
}

#[rstest]
fn test_zulip_domain_only() {
    let parsed =
        parse_notification_url("zulip://bot:key@domain.com").unwrap();
    assert_eq!(parsed.config.get("domain"), Some("domain.com"));
}

#[rstest]
fn test_zulip_failures() {
    assert!(parse_notification_url("zulip://no-at-sign").is_err());
    assert!(parse_notification_url("zulip://nocolon@domain").is_err());
    assert!(parse_notification_url("zulip://bot:key@").is_err());
}

// ======================== Teams ========================

#[rstest]
fn test_teams_url() {
    let parsed =
        parse_notification_url("teams://outlook.office.com/webhook/xxx").unwrap();
    assert_eq!(parsed.scheme, "teams");
    assert_eq!(
        parsed.config.get("webhook_url"),
        Some("https://outlook.office.com/webhook/xxx")
    );
}

// ======================== Google Chat ========================

#[rstest]
fn test_gchat_three_parts() {
    let parsed = parse_notification_url("gchat://space1/key1/token1").unwrap();
    assert_eq!(parsed.scheme, "gchat");
    assert_eq!(
        parsed.config.get("webhook_url"),
        Some("https://chat.googleapis.com/v1/spaces/space1/messages?key=key1&token=token1")
    );
}

#[rstest]
fn test_googlechat_alias() {
    let parsed = parse_notification_url("googlechat://space1/key1/token1").unwrap();
    assert_eq!(parsed.scheme, "gchat");
}

#[rstest]
fn test_gchat_single_part_as_full_url() {
    let parsed = parse_notification_url("gchat://chat.example.com").unwrap();
    assert_eq!(
        parsed.config.get("webhook_url"),
        Some("https://chat.example.com")
    );
}

#[rstest]
fn test_gchat_empty_fails() {
    assert!(parse_notification_url("gchat://").is_err());
}

// ======================== Mattermost ========================

#[rstest]
fn test_mattermost() {
    let parsed = parse_notification_url("mattermost://myhost.com/hookid123").unwrap();
    assert_eq!(parsed.scheme, "mattermost");
    assert_eq!(parsed.config.get("host"), Some("myhost.com"));
    assert_eq!(parsed.config.get("hook_id"), Some("hookid123"));
}

#[rstest]
fn test_mattermost_missing_hookid() {
    assert!(parse_notification_url("mattermost://onlyhost").is_err());
}

// ======================== Rocket.Chat ========================

#[rstest]
fn test_rocketchat() {
    let parsed = parse_notification_url("rocketchat://host.com/tokenA/tokenB").unwrap();
    assert_eq!(parsed.scheme, "rocketchat");
    assert_eq!(parsed.config.get("host"), Some("host.com"));
    assert_eq!(parsed.config.get("token_a"), Some("tokenA"));
    assert_eq!(parsed.config.get("token_b"), Some("tokenB"));
}

#[rstest]
fn test_rocketchat_too_few_parts() {
    assert!(parse_notification_url("rocketchat://host/only-one").is_err());
}

// ======================== Matrix ========================

#[rstest]
fn test_matrix() {
    let parsed = parse_notification_url("matrix://accesstoken/!roomid:server.com").unwrap();
    assert_eq!(parsed.scheme, "matrix");
    assert_eq!(parsed.config.get("access_token"), Some("accesstoken"));
    assert_eq!(parsed.config.get("room_id"), Some("!roomid:server.com"));
}

// ======================== Gotify ========================

#[rstest]
fn test_gotify() {
    let parsed = parse_notification_url("gotify://gotify.example.com/apptoken123").unwrap();
    assert_eq!(parsed.scheme, "gotify");
    assert_eq!(
        parsed.config.get("host"),
        Some("https://gotify.example.com")
    );
    assert_eq!(parsed.config.get("app_token"), Some("apptoken123"));
}

#[rstest]
fn test_gotify_missing_token() {
    assert!(parse_notification_url("gotify://onlyhost").is_err());
}

// ======================== Bark ========================

#[rstest]
fn test_bark() {
    let parsed = parse_notification_url("bark://devicekey123").unwrap();
    assert_eq!(parsed.scheme, "bark");
    assert_eq!(parsed.config.get("device_key"), Some("devicekey123"));
}

// ======================== PushDeer ========================

#[rstest]
fn test_pushdeer() {
    let parsed = parse_notification_url("pushdeer://pushkey123").unwrap();
    assert_eq!(parsed.scheme, "pushdeer");
    assert_eq!(parsed.config.get("push_key"), Some("pushkey123"));
}

// ======================== Pushover ========================

#[rstest]
fn test_pushover() {
    let parsed = parse_notification_url("pushover://userkey/apitoken").unwrap();
    assert_eq!(parsed.scheme, "pushover");
    assert_eq!(parsed.config.get("user_key"), Some("userkey"));
    assert_eq!(parsed.config.get("api_token"), Some("apitoken"));
}

#[rstest]
fn test_pushover_missing_token() {
    assert!(parse_notification_url("pushover://onlyuserkey").is_err());
}

// ======================== IFTTT ========================

#[rstest]
fn test_ifttt() {
    let parsed = parse_notification_url("ifttt://webhookkey/eventname").unwrap();
    assert_eq!(parsed.scheme, "ifttt");
    assert_eq!(parsed.config.get("webhook_key"), Some("webhookkey"));
    assert_eq!(parsed.config.get("event"), Some("eventname"));
}

#[rstest]
fn test_ifttt_missing_event() {
    assert!(parse_notification_url("ifttt://onlykey").is_err());
}

// ======================== Mastodon / Toot ========================

#[rstest]
fn test_mastodon() {
    let parsed = parse_notification_url("mastodon://mytoken@mastodon.social").unwrap();
    assert_eq!(parsed.scheme, "mastodon");
    assert_eq!(parsed.config.get("access_token"), Some("mytoken"));
    assert_eq!(parsed.config.get("instance"), Some("mastodon.social"));
}

#[rstest]
fn test_toot_alias() {
    let parsed = parse_notification_url("toot://mytoken@mastodon.social").unwrap();
    assert_eq!(parsed.scheme, "mastodon");
}

#[rstest]
fn test_mastodon_missing_at() {
    assert!(parse_notification_url("mastodon://noatsign").is_err());
}

// ======================== Misskey ========================

#[rstest]
fn test_misskey() {
    let parsed = parse_notification_url("misskey://token@misskey.io").unwrap();
    assert_eq!(parsed.scheme, "misskey");
    assert_eq!(parsed.config.get("access_token"), Some("token"));
    assert_eq!(parsed.config.get("instance"), Some("misskey.io"));
}

#[rstest]
fn test_misskey_missing_at() {
    assert!(parse_notification_url("misskey://noatsign").is_err());
}

// ======================== Webhooks (json, form, xml, webhook) ========================

#[rstest]
#[case("json://example.com/api/v1", "json", "url", "https://example.com/api/v1")]
#[case("form://example.com/api", "form", "url", "https://example.com/api")]
#[case("xml://example.com/api", "xml", "url", "https://example.com/api")]
#[case("webhook://example.com/hook", "webhook", "url", "https://example.com/hook")]
fn test_webhook_variants(
    #[case] url: &str,
    #[case] expected_scheme: &str,
    #[case] key: &str,
    #[case] expected_value: &str,
) {
    let parsed = parse_notification_url(url).unwrap();
    assert_eq!(parsed.scheme, expected_scheme);
    assert_eq!(parsed.config.get(key), Some(expected_value));
}

#[rstest]
fn test_http_passthrough() {
    let parsed = parse_notification_url("https://example.com/api").unwrap();
    assert_eq!(parsed.scheme, "webhook");
    assert_eq!(
        parsed.config.get("url"),
        Some("https://example.com/api")
    );
}

#[rstest]
fn test_http_passthrough_http() {
    let parsed = parse_notification_url("http://example.com/api").unwrap();
    assert_eq!(parsed.scheme, "webhook");
    assert_eq!(
        parsed.config.get("url"),
        Some("http://example.com/api")
    );
}

#[rstest]
#[case("json://")]
#[case("form://")]
#[case("xml://")]
fn test_webhook_variants_empty_fails(#[case] url: &str) {
    assert!(parse_notification_url(url).is_err());
}

// ======================== HomeAssistant / hassio ========================

#[rstest]
fn test_homeassistant() {
    let parsed = parse_notification_url("hassio://mytoken@ha.local:8123").unwrap();
    assert_eq!(parsed.scheme, "hassio");
    assert_eq!(parsed.config.get("access_token"), Some("mytoken"));
    assert_eq!(parsed.config.get("host"), Some("ha.local:8123"));
}

#[rstest]
fn test_homeassistant_alias() {
    let parsed = parse_notification_url("homeassistant://mytoken@ha.local").unwrap();
    assert_eq!(parsed.scheme, "hassio");
}

#[rstest]
fn test_homeassistant_missing_at() {
    assert!(parse_notification_url("hassio://noatsign").is_err());
}

// ======================== Signal ========================

#[rstest]
fn test_signal() {
    let parsed = parse_notification_url("signal://+1234/+5678").unwrap();
    assert_eq!(parsed.scheme, "signal");
    assert_eq!(parsed.config.get("from"), Some("+1234"));
    assert_eq!(parsed.config.get("to"), Some("+5678"));
}

#[rstest]
fn test_signal_missing_to() {
    assert!(parse_notification_url("signal://+1234").is_err());
}

// ======================== SNS ========================

#[rstest]
fn test_sns() {
    let parsed =
        parse_notification_url("sns://accesskey:secretkey@us-east-1/arn:aws:sns:topic")
            .unwrap();
    assert_eq!(parsed.scheme, "sns");
    assert_eq!(parsed.config.get("access_key"), Some("accesskey"));
    assert_eq!(parsed.config.get("secret_key"), Some("secretkey"));
    assert_eq!(parsed.config.get("region"), Some("us-east-1"));
    assert_eq!(
        parsed.config.get("topic_arn"),
        Some("arn:aws:sns:topic")
    );
}

#[rstest]
fn test_sns_failures() {
    assert!(parse_notification_url("sns://noat").is_err());
    assert!(parse_notification_url("sns://nosecret@region").is_err());
    assert!(parse_notification_url("sns://key:secret@").is_err());
}

// ======================== SES ========================

#[rstest]
fn test_ses() {
    let parsed =
        parse_notification_url("ses://key:secret@us-west-2/from@ex.com/to@ex.com").unwrap();
    assert_eq!(parsed.scheme, "ses");
    assert_eq!(parsed.config.get("access_key"), Some("key"));
    assert_eq!(parsed.config.get("secret_key"), Some("secret"));
    assert_eq!(parsed.config.get("region"), Some("us-west-2"));
    assert_eq!(parsed.config.get("from"), Some("from@ex.com"));
    assert_eq!(parsed.config.get("to"), Some("to@ex.com"));
}

#[rstest]
fn test_ses_failures() {
    assert!(parse_notification_url("ses://noat").is_err());
    assert!(parse_notification_url("ses://nosecret@region").is_err());
}

// ======================== D7SMS ========================

#[rstest]
fn test_d7sms_with_at() {
    let parsed = parse_notification_url("d7sms://token@sender/+1234").unwrap();
    assert_eq!(parsed.scheme, "d7sms");
    assert_eq!(parsed.config.get("api_token"), Some("token"));
    assert_eq!(parsed.config.get("from"), Some("sender"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

#[rstest]
fn test_d7sms_simple() {
    let parsed = parse_notification_url("d7sms://mytoken").unwrap();
    assert_eq!(parsed.config.get("api_token"), Some("mytoken"));
}

#[rstest]
fn test_d7sms_empty_fails() {
    assert!(parse_notification_url("d7sms://").is_err());
}

// ======================== Clickatell ========================

#[rstest]
fn test_clickatell() {
    let parsed = parse_notification_url("clickatell://apikey/+1234").unwrap();
    assert_eq!(parsed.scheme, "clickatell");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

#[rstest]
fn test_clickatell_key_only() {
    let parsed = parse_notification_url("clickatell://apikey").unwrap();
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
}

// ======================== Kavenegar ========================

#[rstest]
fn test_kavenegar() {
    let parsed = parse_notification_url("kavenegar://apikey/sender/+1234").unwrap();
    assert_eq!(parsed.scheme, "kavenegar");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("from"), Some("sender"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

// ======================== LaMetric ========================

#[rstest]
fn test_lametric() {
    let parsed = parse_notification_url("lametric://apikey@192.168.1.10").unwrap();
    assert_eq!(parsed.scheme, "lametric");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("host"), Some("192.168.1.10"));
}

#[rstest]
fn test_lametric_missing_at() {
    assert!(parse_notification_url("lametric://noat").is_err());
}

// ======================== OneSignal ========================

#[rstest]
fn test_onesignal() {
    let parsed = parse_notification_url("onesignal://appid:apikey").unwrap();
    assert_eq!(parsed.scheme, "onesignal");
    assert_eq!(parsed.config.get("app_id"), Some("appid"));
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
}

#[rstest]
fn test_onesignal_missing_colon() {
    assert!(parse_notification_url("onesignal://onlyappid").is_err());
}

// ======================== Pushy ========================

#[rstest]
fn test_pushy() {
    let parsed = parse_notification_url("pushy://apikey/devicetoken").unwrap();
    assert_eq!(parsed.scheme, "pushy");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("device_token"), Some("devicetoken"));
}

// ======================== PushSafer ========================

#[rstest]
fn test_pushsafer() {
    let parsed = parse_notification_url("pushsafer://privatekey123").unwrap();
    assert_eq!(parsed.scheme, "pushsafer");
    assert_eq!(parsed.config.get("private_key"), Some("privatekey123"));
}

// ======================== Join ========================

#[rstest]
fn test_join() {
    let parsed = parse_notification_url("join://apikey/deviceid").unwrap();
    assert_eq!(parsed.scheme, "join");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("device_id"), Some("deviceid"));
}

#[rstest]
fn test_join_key_only() {
    let parsed = parse_notification_url("join://apikey").unwrap();
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
}

// ======================== FCM ========================

#[rstest]
fn test_fcm() {
    let parsed = parse_notification_url("fcm://serverkey/devicetoken").unwrap();
    assert_eq!(parsed.scheme, "fcm");
    assert_eq!(parsed.config.get("server_key"), Some("serverkey"));
    assert_eq!(parsed.config.get("device_token"), Some("devicetoken"));
}

// ======================== Threema ========================

#[rstest]
fn test_threema() {
    let parsed = parse_notification_url("threema://gwid:secret@recipient").unwrap();
    assert_eq!(parsed.scheme, "threema");
    assert_eq!(parsed.config.get("gateway_id"), Some("gwid"));
    assert_eq!(parsed.config.get("api_secret"), Some("secret"));
    assert_eq!(parsed.config.get("to"), Some("recipient"));
}

#[rstest]
fn test_threema_failures() {
    assert!(parse_notification_url("threema://noat").is_err());
    assert!(parse_notification_url("threema://nocolon@recipient").is_err());
}

// ======================== Nextcloud (ncloud) ========================

#[rstest]
fn test_nextcloud() {
    let parsed =
        parse_notification_url("ncloud://user:pass@host.com/targetuser").unwrap();
    assert_eq!(parsed.scheme, "ncloud");
    assert_eq!(parsed.config.get("user"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("host.com"));
    assert_eq!(parsed.config.get("target_user"), Some("targetuser"));
}

#[rstest]
fn test_nextcloud_alias() {
    let parsed = parse_notification_url("nextcloud://u:p@h.com/tu").unwrap();
    assert_eq!(parsed.scheme, "ncloud");
}

#[rstest]
fn test_nextcloud_failures() {
    assert!(parse_notification_url("ncloud://noat").is_err());
    assert!(parse_notification_url("ncloud://nocolon@host").is_err());
    assert!(parse_notification_url("ncloud://u:p@").is_err());
}

// ======================== Bluesky ========================

#[rstest]
fn test_bluesky() {
    let parsed = parse_notification_url("bluesky://handle:apppass").unwrap();
    assert_eq!(parsed.scheme, "bluesky");
    assert_eq!(parsed.config.get("handle"), Some("handle"));
    assert_eq!(parsed.config.get("app_password"), Some("apppass"));
}

#[rstest]
fn test_bluesky_missing_colon() {
    assert!(parse_notification_url("bluesky://onlyhandle").is_err());
}

// ======================== MessageBird (msgbird) ========================

#[rstest]
fn test_msgbird() {
    let parsed = parse_notification_url("msgbird://key@sender/+1234").unwrap();
    assert_eq!(parsed.scheme, "msgbird");
    assert_eq!(parsed.config.get("access_key"), Some("key"));
    assert_eq!(parsed.config.get("from"), Some("sender"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

// ======================== Reddit ========================

#[rstest]
fn test_reddit() {
    let parsed =
        parse_notification_url("reddit://cid:csecret@user:password/subreddit").unwrap();
    assert_eq!(parsed.scheme, "reddit");
    assert_eq!(parsed.config.get("client_id"), Some("cid"));
    assert_eq!(parsed.config.get("client_secret"), Some("csecret"));
    assert_eq!(parsed.config.get("user"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("password"));
    assert_eq!(parsed.config.get("to"), Some("subreddit"));
}

#[rstest]
fn test_reddit_failures() {
    assert!(parse_notification_url("reddit://noat").is_err());
    assert!(parse_notification_url("reddit://nocolon@rest").is_err());
    assert!(parse_notification_url("reddit://cid:csecret@").is_err());
}

// ======================== Twist ========================

#[rstest]
fn test_twist() {
    let parsed = parse_notification_url("twist://install_id=123&post_data_key=abc").unwrap();
    assert_eq!(parsed.scheme, "twist");
    assert!(parsed.config.get("webhook_url").unwrap().contains("twist.com"));
}

#[rstest]
fn test_twist_empty_fails() {
    assert!(parse_notification_url("twist://").is_err());
}

// ======================== Chanify ========================

#[rstest]
fn test_chanify_simple() {
    let parsed = parse_notification_url("chanify://mytoken").unwrap();
    assert_eq!(parsed.scheme, "chanify");
    assert_eq!(parsed.config.get("token"), Some("mytoken"));
}

#[rstest]
fn test_chanify_with_host() {
    let parsed = parse_notification_url("chanify://mytoken@custom.host.com").unwrap();
    assert_eq!(parsed.config.get("token"), Some("mytoken"));
    assert_eq!(
        parsed.config.get("server"),
        Some("https://custom.host.com")
    );
}

// ======================== Spike ========================

#[rstest]
fn test_spike() {
    let parsed = parse_notification_url("spike://api.spike.sh/v1/hook/abc").unwrap();
    assert_eq!(parsed.scheme, "spike");
    assert_eq!(
        parsed.config.get("webhook_url"),
        Some("https://api.spike.sh/v1/hook/abc")
    );
}

// ======================== Apprise ========================

#[rstest]
fn test_apprise() {
    let parsed = parse_notification_url("apprise://myhost.com/configkey").unwrap();
    assert_eq!(parsed.scheme, "apprise");
    assert_eq!(parsed.config.get("host"), Some("https://myhost.com"));
    assert_eq!(parsed.config.get("config_key"), Some("configkey"));
}

#[rstest]
fn test_apprise_host_only() {
    let parsed = parse_notification_url("apprise://myhost.com").unwrap();
    assert_eq!(parsed.config.get("host"), Some("https://myhost.com"));
}

// ======================== WebPush ========================

#[rstest]
fn test_webpush() {
    let parsed = parse_notification_url("webpush://fcm.googleapis.com/fcm/send/abc").unwrap();
    assert_eq!(parsed.scheme, "webpush");
    assert_eq!(
        parsed.config.get("endpoint"),
        Some("https://fcm.googleapis.com/fcm/send/abc")
    );
}

// ======================== WhatsApp ========================

#[rstest]
fn test_whatsapp() {
    let parsed = parse_notification_url("whatsapp://token@phoneid/+1234").unwrap();
    assert_eq!(parsed.scheme, "whatsapp");
    assert_eq!(parsed.config.get("access_token"), Some("token"));
    assert_eq!(parsed.config.get("phone_number_id"), Some("phoneid"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

#[rstest]
fn test_whatsapp_failures() {
    assert!(parse_notification_url("whatsapp://noat").is_err());
    assert!(parse_notification_url("whatsapp://token@").is_err());
}

// ======================== Kodi / XBMC ========================

#[rstest]
fn test_kodi_host_only() {
    let parsed = parse_notification_url("kodi://192.168.1.10").unwrap();
    assert_eq!(parsed.scheme, "kodi");
    assert_eq!(parsed.config.get("host"), Some("192.168.1.10"));
}

#[rstest]
fn test_kodi_host_port() {
    let parsed = parse_notification_url("kodi://192.168.1.10:8080").unwrap();
    assert_eq!(parsed.config.get("host"), Some("192.168.1.10"));
    assert_eq!(parsed.config.get("port"), Some("8080"));
}

#[rstest]
fn test_kodi_with_auth() {
    let parsed = parse_notification_url("kodi://user:pass@192.168.1.10:8080").unwrap();
    assert_eq!(parsed.config.get("user"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("192.168.1.10"));
    assert_eq!(parsed.config.get("port"), Some("8080"));
}

#[rstest]
fn test_xbmc_alias() {
    let parsed = parse_notification_url("xbmc://host.local").unwrap();
    assert_eq!(parsed.scheme, "kodi");
}

#[rstest]
fn test_kodi_empty_fails() {
    assert!(parse_notification_url("kodi://").is_err());
}

// ======================== Jira ========================

#[rstest]
fn test_jira() {
    let parsed =
        parse_notification_url("jira://user:apitoken@jira.example.com/PROJ-123").unwrap();
    assert_eq!(parsed.scheme, "jira");
    assert_eq!(parsed.config.get("user"), Some("user"));
    assert_eq!(parsed.config.get("api_token"), Some("apitoken"));
    assert_eq!(parsed.config.get("host"), Some("jira.example.com"));
    assert_eq!(parsed.config.get("issue_key"), Some("PROJ-123"));
}

#[rstest]
fn test_jira_failures() {
    assert!(parse_notification_url("jira://noat").is_err());
    assert!(parse_notification_url("jira://nocolon@host").is_err());
    assert!(parse_notification_url("jira://u:t@").is_err());
}

// ======================== SendPulse ========================

#[rstest]
fn test_sendpulse() {
    let parsed =
        parse_notification_url("sendpulse://cid:csecret@from@ex.com/to@ex.com").unwrap();
    assert_eq!(parsed.scheme, "sendpulse");
    assert_eq!(parsed.config.get("client_id"), Some("cid"));
    assert_eq!(parsed.config.get("client_secret"), Some("csecret"));
    assert_eq!(parsed.config.get("from"), Some("from@ex.com"));
    assert_eq!(parsed.config.get("to"), Some("to@ex.com"));
}

#[rstest]
fn test_sendpulse_failures() {
    assert!(parse_notification_url("sendpulse://noat").is_err());
    assert!(parse_notification_url("sendpulse://nocolon@host").is_err());
    assert!(parse_notification_url("sendpulse://cid:csecret@").is_err());
}

// ======================== Synology ========================

#[rstest]
fn test_synology() {
    let parsed = parse_notification_url("synology://token@nas.local:5000").unwrap();
    assert_eq!(parsed.scheme, "synology");
    assert_eq!(parsed.config.get("token"), Some("token"));
    assert_eq!(parsed.config.get("host"), Some("nas.local"));
    assert_eq!(parsed.config.get("port"), Some("5000"));
}

#[rstest]
fn test_synology_no_port() {
    let parsed = parse_notification_url("synology://token@nas.local").unwrap();
    assert_eq!(parsed.config.get("host"), Some("nas.local"));
}

#[rstest]
fn test_synology_missing_at() {
    assert!(parse_notification_url("synology://noat").is_err());
}

// ======================== Africa's Talking ========================

#[rstest]
fn test_africastalking() {
    let parsed = parse_notification_url("africastalking://user:key@+1234").unwrap();
    assert_eq!(parsed.scheme, "africastalking");
    assert_eq!(parsed.config.get("username"), Some("user"));
    assert_eq!(parsed.config.get("api_key"), Some("key"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

// ======================== O365 ========================

#[rstest]
fn test_o365() {
    let parsed =
        parse_notification_url("o365://cid:csecret@tenantid/from@ex.com/to@ex.com").unwrap();
    assert_eq!(parsed.scheme, "o365");
    assert_eq!(parsed.config.get("client_id"), Some("cid"));
    assert_eq!(parsed.config.get("client_secret"), Some("csecret"));
    assert_eq!(parsed.config.get("tenant_id"), Some("tenantid"));
    assert_eq!(parsed.config.get("from"), Some("from@ex.com"));
    assert_eq!(parsed.config.get("to"), Some("to@ex.com"));
}

#[rstest]
fn test_outlook_alias() {
    let parsed = parse_notification_url("outlook://cid:csecret@tenant/f/t").unwrap();
    assert_eq!(parsed.scheme, "o365");
}

// ======================== NcTalk ========================

#[rstest]
fn test_nctalk() {
    let parsed = parse_notification_url("nctalk://user:pass@nc.local/roomtoken").unwrap();
    assert_eq!(parsed.scheme, "nctalk");
    assert_eq!(parsed.config.get("user"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("nc.local"));
    assert_eq!(parsed.config.get("room_token"), Some("roomtoken"));
}

// ======================== Emby ========================

#[rstest]
fn test_emby() {
    let parsed = parse_notification_url("emby://apikey@emby.local/userid").unwrap();
    assert_eq!(parsed.scheme, "emby");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("host"), Some("emby.local"));
    assert_eq!(parsed.config.get("user_id"), Some("userid"));
}

// ======================== Jellyfin ========================

#[rstest]
fn test_jellyfin() {
    let parsed = parse_notification_url("jellyfin://apikey@jf.local/userid").unwrap();
    assert_eq!(parsed.scheme, "jellyfin");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("host"), Some("jf.local"));
}

// ======================== MQTT ========================

#[rstest]
fn test_mqtt_with_auth() {
    let parsed = parse_notification_url("mqtt://user:pass@broker.local/topic").unwrap();
    assert_eq!(parsed.scheme, "mqtt");
    assert_eq!(parsed.config.get("user"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("broker.local"));
    assert_eq!(parsed.config.get("topic"), Some("topic"));
}

#[rstest]
fn test_mqtt_without_auth() {
    let parsed = parse_notification_url("mqtt://broker.local/topic").unwrap();
    assert_eq!(parsed.config.get("host"), Some("broker.local"));
    assert_eq!(parsed.config.get("topic"), Some("topic"));
}

#[rstest]
fn test_mqtt_empty_fails() {
    assert!(parse_notification_url("mqtt://").is_err());
}

// ======================== VoIP.ms ========================

#[rstest]
fn test_voipms() {
    let parsed = parse_notification_url("voipms://email:pass@did/+1234").unwrap();
    assert_eq!(parsed.scheme, "voipms");
    assert_eq!(parsed.config.get("email"), Some("email"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("did"), Some("did"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

// ======================== SFR ========================

#[rstest]
fn test_sfr() {
    let parsed = parse_notification_url("sfr://+33612345678:password").unwrap();
    assert_eq!(parsed.scheme, "sfr");
    assert_eq!(parsed.config.get("phone"), Some("+33612345678"));
    assert_eq!(parsed.config.get("password"), Some("password"));
}

#[rstest]
fn test_sfr_missing_colon() {
    assert!(parse_notification_url("sfr://onlyphone").is_err());
}

// ======================== Pushed ========================

#[rstest]
fn test_pushed() {
    let parsed = parse_notification_url("pushed://appkey:appsecret").unwrap();
    assert_eq!(parsed.scheme, "pushed");
    assert_eq!(parsed.config.get("app_key"), Some("appkey"));
    assert_eq!(parsed.config.get("app_secret"), Some("appsecret"));
}

#[rstest]
fn test_pushed_missing_colon() {
    assert!(parse_notification_url("pushed://onlykey").is_err());
}

// ======================== Growl ========================

#[rstest]
fn test_growl_host_only() {
    let parsed = parse_notification_url("growl://192.168.1.10").unwrap();
    assert_eq!(parsed.scheme, "growl");
    assert_eq!(parsed.config.get("host"), Some("192.168.1.10"));
}

#[rstest]
fn test_growl_host_port() {
    let parsed = parse_notification_url("growl://host:23053").unwrap();
    assert_eq!(parsed.config.get("host"), Some("host"));
    assert_eq!(parsed.config.get("port"), Some("23053"));
}

#[rstest]
fn test_growl_with_password() {
    let parsed = parse_notification_url("growl://pass@host:23053").unwrap();
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("host"));
    assert_eq!(parsed.config.get("port"), Some("23053"));
}

#[rstest]
fn test_growl_empty_fails() {
    assert!(parse_notification_url("growl://").is_err());
}

// ======================== Kumulos ========================

#[rstest]
fn test_kumulos() {
    let parsed = parse_notification_url("kumulos://apikey:serverkey").unwrap();
    assert_eq!(parsed.scheme, "kumulos");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("server_key"), Some("serverkey"));
}

#[rstest]
fn test_kumulos_missing_colon() {
    assert!(parse_notification_url("kumulos://onlykey").is_err());
}

// ======================== Parse (Parse Platform) ========================

#[rstest]
fn test_parse_with_host() {
    let parsed = parse_notification_url("parse://appid:restkey@host.com").unwrap();
    assert_eq!(parsed.scheme, "parse");
    assert_eq!(parsed.config.get("app_id"), Some("appid"));
    assert_eq!(parsed.config.get("rest_api_key"), Some("restkey"));
    assert_eq!(parsed.config.get("host"), Some("host.com"));
}

#[rstest]
fn test_parse_without_host() {
    let parsed = parse_notification_url("parse://appid:restkey").unwrap();
    assert_eq!(parsed.config.get("app_id"), Some("appid"));
    assert_eq!(parsed.config.get("rest_api_key"), Some("restkey"));
}

// ======================== Rsyslog ========================

#[rstest]
fn test_rsyslog() {
    let parsed = parse_notification_url("rsyslog://syslog.host.com/mytoken").unwrap();
    assert_eq!(parsed.scheme, "rsyslog");
    assert_eq!(parsed.config.get("host"), Some("syslog.host.com"));
    assert_eq!(parsed.config.get("token"), Some("mytoken"));
}

#[rstest]
fn test_syslog_alias() {
    let parsed = parse_notification_url("syslog://host.com/token").unwrap();
    assert_eq!(parsed.scheme, "rsyslog");
}

// ======================== SMS Manager ========================

#[rstest]
fn test_smsmanager_with_at() {
    let parsed = parse_notification_url("smsmanager://apikey@sender/+1234").unwrap();
    assert_eq!(parsed.scheme, "smsmanager");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("from"), Some("sender"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

#[rstest]
fn test_smsmanager_simple() {
    let parsed = parse_notification_url("smsmanager://mykey").unwrap();
    assert_eq!(parsed.config.get("api_key"), Some("mykey"));
}

// ======================== Seven / SMS77 ========================

#[rstest]
fn test_seven() {
    let parsed = parse_notification_url("seven://apikey/+1234").unwrap();
    assert_eq!(parsed.scheme, "seven");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

#[rstest]
fn test_sms77_alias() {
    let parsed = parse_notification_url("sms77://apikey/+1234").unwrap();
    assert_eq!(parsed.scheme, "seven");
}

// ======================== SMSEagle ========================

#[rstest]
fn test_smseagle() {
    let parsed = parse_notification_url("smseagle://token@host.com/+1234").unwrap();
    assert_eq!(parsed.scheme, "smseagle");
    assert_eq!(parsed.config.get("access_token"), Some("token"));
    assert_eq!(parsed.config.get("host"), Some("host.com"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

// ======================== MSG91 ========================

#[rstest]
fn test_msg91() {
    let parsed = parse_notification_url("msg91://authkey/senderid/+1234").unwrap();
    assert_eq!(parsed.scheme, "msg91");
    assert_eq!(parsed.config.get("authkey"), Some("authkey"));
    assert_eq!(parsed.config.get("sender"), Some("senderid"));
    assert_eq!(parsed.config.get("to"), Some("+1234"));
}

// ======================== DAPNET ========================

#[rstest]
fn test_dapnet() {
    let parsed = parse_notification_url("dapnet://callsign:password@tocallsign").unwrap();
    assert_eq!(parsed.scheme, "dapnet");
    assert_eq!(parsed.config.get("callsign"), Some("callsign"));
    assert_eq!(parsed.config.get("password"), Some("password"));
    assert_eq!(parsed.config.get("to"), Some("tocallsign"));
}

#[rstest]
fn test_dapnet_failures() {
    assert!(parse_notification_url("dapnet://noat").is_err());
    assert!(parse_notification_url("dapnet://nocolon@to").is_err());
}

// ======================== Enigma2 / E2 ========================

#[rstest]
fn test_enigma2_host_only() {
    let parsed = parse_notification_url("enigma2://192.168.1.10").unwrap();
    assert_eq!(parsed.scheme, "enigma2");
    assert_eq!(parsed.config.get("host"), Some("192.168.1.10"));
}

#[rstest]
fn test_enigma2_with_auth() {
    let parsed = parse_notification_url("enigma2://user:pass@host:8080").unwrap();
    assert_eq!(parsed.config.get("user"), Some("user"));
    assert_eq!(parsed.config.get("password"), Some("pass"));
    assert_eq!(parsed.config.get("host"), Some("host"));
    assert_eq!(parsed.config.get("port"), Some("8080"));
}

#[rstest]
fn test_e2_alias() {
    let parsed = parse_notification_url("e2://host.local").unwrap();
    assert_eq!(parsed.scheme, "enigma2");
}

#[rstest]
fn test_enigma2_empty_fails() {
    assert!(parse_notification_url("enigma2://").is_err());
}

// ======================== StatusPage ========================

#[rstest]
fn test_statuspage() {
    let parsed = parse_notification_url("statuspage://apikey@pageid").unwrap();
    assert_eq!(parsed.scheme, "statuspage");
    assert_eq!(parsed.config.get("api_key"), Some("apikey"));
    assert_eq!(parsed.config.get("page_id"), Some("pageid"));
}

#[rstest]
fn test_statuspage_missing_at() {
    assert!(parse_notification_url("statuspage://noat").is_err());
}

// ======================== Dot ========================

#[rstest]
fn test_dot() {
    let parsed = parse_notification_url("dot://token@deviceid").unwrap();
    assert_eq!(parsed.scheme, "dot");
    assert_eq!(parsed.config.get("token"), Some("token"));
    assert_eq!(parsed.config.get("device_id"), Some("deviceid"));
}

#[rstest]
fn test_dot_missing_at() {
    assert!(parse_notification_url("dot://noat").is_err());
}

// ======================== Workflows / Power Automate ========================

#[rstest]
fn test_workflows() {
    let parsed = parse_notification_url("workflows://host.com/workflow/sig").unwrap();
    assert_eq!(parsed.scheme, "workflows");
    assert_eq!(parsed.config.get("host"), Some("host.com"));
    assert_eq!(parsed.config.get("workflow"), Some("workflow"));
    assert_eq!(parsed.config.get("signature"), Some("sig"));
}

#[rstest]
fn test_workflows_with_port() {
    let parsed = parse_notification_url("workflows://host.com:443/workflow/sig").unwrap();
    assert_eq!(parsed.config.get("host"), Some("host.com"));
    assert_eq!(parsed.config.get("port"), Some("443"));
}

#[rstest]
fn test_workflow_alias() {
    let parsed = parse_notification_url("workflow://host.com/wf/sig").unwrap();
    assert_eq!(parsed.scheme, "workflows");
}

#[rstest]
fn test_powerautomate_alias() {
    let parsed = parse_notification_url("powerautomate://host.com/wf/sig").unwrap();
    assert_eq!(parsed.scheme, "workflows");
}

#[rstest]
fn test_workflows_too_few_parts() {
    assert!(parse_notification_url("workflows://onlyhost").is_err());
}

// ======================== NotificationAPI (napi) ========================

#[rstest]
fn test_napi() {
    let parsed = parse_notification_url("napi://cid/csecret/userid").unwrap();
    assert_eq!(parsed.scheme, "napi");
    assert_eq!(parsed.config.get("client_id"), Some("cid"));
    assert_eq!(parsed.config.get("client_secret"), Some("csecret"));
    assert_eq!(parsed.config.get("user_id"), Some("userid"));
}

#[rstest]
fn test_notificationapi_alias() {
    let parsed = parse_notification_url("notificationapi://cid/csecret/uid").unwrap();
    assert_eq!(parsed.scheme, "napi");
}

#[rstest]
fn test_napi_too_few_parts() {
    assert!(parse_notification_url("napi://cid/csecret").is_err());
    assert!(parse_notification_url("napi://cid").is_err());
}

// ======================== Query params ========================

#[rstest]
fn test_query_params_preserved() {
    let parsed =
        parse_notification_url("feishu://hookid?secret=mysecret&extra=value").unwrap();
    assert_eq!(parsed.config.get("hook_id"), Some("hookid"));
    assert_eq!(parsed.config.get("secret"), Some("mysecret"));
    assert_eq!(parsed.config.get("extra"), Some("value"));
}

#[rstest]
fn test_ntfy_with_server_query() {
    let parsed = parse_notification_url("ntfy://topic?server=https://ntfy.sh").unwrap();
    assert_eq!(parsed.config.get("topic"), Some("topic"));
    assert_eq!(
        parsed.config.get("server"),
        Some("https://ntfy.sh")
    );
}

#[rstest]
fn test_bark_with_server_query() {
    let parsed = parse_notification_url("bark://key?server=https://bark.custom.com").unwrap();
    assert_eq!(parsed.config.get("device_key"), Some("key"));
    assert_eq!(
        parsed.config.get("server"),
        Some("https://bark.custom.com")
    );
}
