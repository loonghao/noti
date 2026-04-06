/// Tests for register_all_providers and provider metadata consistency.
use noti_core::ProviderRegistry;
use rstest::rstest;

#[rstest]
fn test_register_all_providers_count() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let all = registry.all_providers();
    assert_eq!(all.len(), 126, "expected 126 providers, got {}", all.len());
}

#[rstest]
fn test_all_providers_have_names() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    for provider in registry.all_providers() {
        assert!(
            !provider.name().is_empty(),
            "provider name should not be empty"
        );
    }
}

#[rstest]
fn test_all_providers_have_schemes() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    for provider in registry.all_providers() {
        assert!(
            !provider.url_scheme().is_empty(),
            "provider '{}' should have a non-empty URL scheme",
            provider.name()
        );
    }
}

#[rstest]
fn test_all_providers_have_descriptions() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    for provider in registry.all_providers() {
        assert!(
            !provider.description().is_empty(),
            "provider '{}' should have a non-empty description",
            provider.name()
        );
    }
}

#[rstest]
fn test_all_providers_have_example_urls() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    for provider in registry.all_providers() {
        assert!(
            !provider.example_url().is_empty(),
            "provider '{}' should have a non-empty example URL",
            provider.name()
        );
    }
}

#[rstest]
fn test_all_providers_have_params() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    for provider in registry.all_providers() {
        let params = provider.params();
        assert!(
            !params.is_empty(),
            "provider '{}' should have at least one parameter",
            provider.name()
        );
        // Verify all params have names and descriptions
        for param in &params {
            assert!(
                !param.name.is_empty(),
                "provider '{}' has a param with empty name",
                provider.name()
            );
            assert!(
                !param.description.is_empty(),
                "provider '{}' param '{}' has empty description",
                provider.name(),
                param.name
            );
        }
    }
}

#[rstest]
fn test_all_providers_have_at_least_one_required_param() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    for provider in registry.all_providers() {
        let params = provider.params();
        let has_required = params.iter().any(|p| p.required);
        assert!(
            has_required,
            "provider '{}' should have at least one required parameter",
            provider.name()
        );
    }
}

#[rstest]
fn test_provider_names_sorted() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let names = registry.provider_names();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "provider_names() should return sorted names");
}

#[rstest]
fn test_get_by_name_all_providers() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let names = registry.provider_names();
    for name in &names {
        let provider = registry.get_by_name(name);
        assert!(
            provider.is_some(),
            "provider '{name}' should be retrievable by name"
        );
        assert_eq!(provider.unwrap().name(), *name);
    }
}

#[rstest]
fn test_get_by_scheme_all_providers() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    for provider in registry.all_providers() {
        let scheme = provider.url_scheme();
        let by_scheme = registry.get_by_scheme(scheme);
        assert!(
            by_scheme.is_some(),
            "provider '{}' should be retrievable by scheme '{}'",
            provider.name(),
            scheme
        );
    }
}

// ======================== Specific provider existence tests ========================

#[rstest]
#[case("wecom")]
#[case("feishu")]
#[case("dingtalk")]
#[case("slack")]
#[case("telegram")]
#[case("discord")]
#[case("teams")]
#[case("googlechat")]
#[case("mattermost")]
#[case("rocketchat")]
#[case("matrix")]
#[case("zulip")]
#[case("webex")]
#[case("line")]
#[case("revolt")]
#[case("mastodon")]
#[case("ryver")]
#[case("twist")]
#[case("flock")]
#[case("gitter")]
#[case("guilded")]
#[case("misskey")]
#[case("bluesky")]
#[case("pushover")]
#[case("ntfy")]
#[case("gotify")]
#[case("bark")]
#[case("pushdeer")]
#[case("serverchan")]
#[case("pushbullet")]
#[case("simplepush")]
#[case("notica")]
#[case("prowl")]
#[case("join")]
#[case("pushsafer")]
#[case("onesignal")]
#[case("techulus")]
#[case("pushy")]
#[case("chanify")]
#[case("pushplus")]
#[case("wxpusher")]
#[case("fcm")]
#[case("pushjet")]
#[case("ifttt")]
#[case("pagerduty")]
#[case("opsgenie")]
#[case("pagertree")]
#[case("signl4")]
#[case("victorops")]
#[case("spike")]
#[case("twilio")]
#[case("vonage")]
#[case("d7sms")]
#[case("sinch")]
#[case("clickatell")]
#[case("bulksms")]
#[case("kavenegar")]
#[case("messagebird")]
#[case("plivo")]
#[case("burstsms")]
#[case("popcorn")]
#[case("clicksend")]
#[case("seven")]
#[case("smseagle")]
#[case("httpsms")]
#[case("msg91")]
#[case("freemobile")]
#[case("email")]
#[case("mailgun")]
#[case("sendgrid")]
#[case("sparkpost")]
#[case("ses")]
#[case("resend")]
#[case("brevo")]
#[case("smtp2go")]
#[case("sns")]
#[case("webhook")]
#[case("json")]
#[case("form")]
#[case("xml")]
#[case("homeassistant")]
#[case("lametric")]
#[case("lunasea")]
#[case("nextcloud")]
#[case("threema")]
#[case("signal")]
#[case("reddit")]
#[case("apprise")]
#[case("webpush")]
#[case("whatsapp")]
#[case("kodi")]
#[case("notifico")]
#[case("46elks")]
#[case("bulkvs")]
#[case("jira")]
#[case("pushme")]
#[case("sendpulse")]
#[case("streamlabs")]
#[case("synology")]
#[case("africastalking")]
#[case("o365")]
#[case("nctalk")]
#[case("emby")]
#[case("jellyfin")]
#[case("pushcut")]
#[case("mqtt")]
#[case("voipms")]
#[case("sfr")]
#[case("pushed")]
#[case("growl")]
#[case("kumulos")]
#[case("parse")]
#[case("rsyslog")]
#[case("smsmanager")]
#[case("twitter")]
#[case("boxcar")]
#[case("dapnet")]
#[case("enigma2")]
#[case("notifiarr")]
#[case("statuspage")]
#[case("dot")]
#[case("fluxer")]
#[case("workflows")]
#[case("notificationapi")]
#[case("spugpush")]
#[case("apns")]
fn test_provider_exists(#[case] name: &str) {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);
    assert!(
        registry.get_by_name(name).is_some(),
        "provider '{name}' should be registered"
    );
}

// ======================== Validate config with empty config for all providers ========================

#[rstest]
fn test_all_providers_reject_empty_config() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let empty_config = noti_core::ProviderConfig::new();

    for provider in registry.all_providers() {
        let result = provider.validate_config(&empty_config);
        assert!(
            result.is_err(),
            "provider '{}' should reject empty config (has required params)",
            provider.name()
        );
    }
}

// ======================== Alias scheme tests ========================

#[rstest]
fn test_alias_schemes() {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    // Telegram is registered under scheme "tg"
    let tg = registry.get_by_scheme("tg");
    assert!(tg.is_some());
    assert_eq!(tg.unwrap().name(), "telegram");
}
