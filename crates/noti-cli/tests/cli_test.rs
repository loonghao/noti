use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_output() {
    Command::cargo_bin("noti")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("multi-channel notification CLI"));
}

#[test]
fn test_version_output() {
    Command::cargo_bin("noti")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("noti"));
}

#[test]
fn test_providers_list() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["providers", "list"])
        .assert()
        .success()
        // Chat / IM providers (20)
        .stdout(predicate::str::contains("wecom"))
        .stdout(predicate::str::contains("slack"))
        .stdout(predicate::str::contains("telegram"))
        .stdout(predicate::str::contains("discord"))
        .stdout(predicate::str::contains("feishu"))
        .stdout(predicate::str::contains("email"))
        .stdout(predicate::str::contains("webhook"))
        .stdout(predicate::str::contains("dingtalk"))
        .stdout(predicate::str::contains("teams"))
        .stdout(predicate::str::contains("googlechat"))
        .stdout(predicate::str::contains("mattermost"))
        .stdout(predicate::str::contains("rocketchat"))
        .stdout(predicate::str::contains("matrix"))
        .stdout(predicate::str::contains("zulip"))
        .stdout(predicate::str::contains("webex"))
        .stdout(predicate::str::contains("line"))
        .stdout(predicate::str::contains("revolt"))
        .stdout(predicate::str::contains("mastodon"))
        .stdout(predicate::str::contains("ryver"))
        .stdout(predicate::str::contains("twist"))
        .stdout(predicate::str::contains("flock"))
        .stdout(predicate::str::contains("gitter"))
        // Gaming / community (2)
        .stdout(predicate::str::contains("guilded"))
        .stdout(predicate::str::contains("misskey"))
        // Social networks (1)
        .stdout(predicate::str::contains("bluesky"))
        // Push notification providers (20)
        .stdout(predicate::str::contains("pushover"))
        .stdout(predicate::str::contains("ntfy"))
        .stdout(predicate::str::contains("gotify"))
        .stdout(predicate::str::contains("bark"))
        .stdout(predicate::str::contains("pushdeer"))
        .stdout(predicate::str::contains("serverchan"))
        .stdout(predicate::str::contains("pushbullet"))
        .stdout(predicate::str::contains("simplepush"))
        .stdout(predicate::str::contains("notica"))
        .stdout(predicate::str::contains("prowl"))
        .stdout(predicate::str::contains("join"))
        .stdout(predicate::str::contains("pushsafer"))
        .stdout(predicate::str::contains("onesignal"))
        .stdout(predicate::str::contains("techulus"))
        .stdout(predicate::str::contains("pushy"))
        .stdout(predicate::str::contains("chanify"))
        .stdout(predicate::str::contains("pushplus"))
        .stdout(predicate::str::contains("wxpusher"))
        .stdout(predicate::str::contains("fcm"))
        .stdout(predicate::str::contains("pushjet"))
        // Automation & incident (7)
        .stdout(predicate::str::contains("ifttt"))
        .stdout(predicate::str::contains("pagerduty"))
        .stdout(predicate::str::contains("opsgenie"))
        .stdout(predicate::str::contains("pagertree"))
        .stdout(predicate::str::contains("signl4"))
        .stdout(predicate::str::contains("victorops"))
        .stdout(predicate::str::contains("spike"))
        // SMS providers (17)
        .stdout(predicate::str::contains("twilio"))
        .stdout(predicate::str::contains("vonage"))
        .stdout(predicate::str::contains("d7sms"))
        .stdout(predicate::str::contains("sinch"))
        .stdout(predicate::str::contains("clickatell"))
        .stdout(predicate::str::contains("bulksms"))
        .stdout(predicate::str::contains("kavenegar"))
        .stdout(predicate::str::contains("messagebird"))
        .stdout(predicate::str::contains("plivo"))
        .stdout(predicate::str::contains("burstsms"))
        .stdout(predicate::str::contains("popcorn"))
        .stdout(predicate::str::contains("clicksend"))
        .stdout(predicate::str::contains("seven"))
        .stdout(predicate::str::contains("smseagle"))
        .stdout(predicate::str::contains("httpsms"))
        .stdout(predicate::str::contains("msg91"))
        .stdout(predicate::str::contains("freemobile"))
        // Email providers (8)
        .stdout(predicate::str::contains("mailgun"))
        .stdout(predicate::str::contains("sendgrid"))
        .stdout(predicate::str::contains("sparkpost"))
        .stdout(predicate::str::contains("ses"))
        .stdout(predicate::str::contains("resend"))
        .stdout(predicate::str::contains("brevo"))
        .stdout(predicate::str::contains("smtp2go"))
        // AWS cloud (1)
        .stdout(predicate::str::contains("sns"))
        // Generic webhooks (4)
        .stdout(predicate::str::contains("json"))
        .stdout(predicate::str::contains("form"))
        .stdout(predicate::str::contains("xml"))
        // Home automation & IoT (2)
        .stdout(predicate::str::contains("homeassistant"))
        .stdout(predicate::str::contains("lametric"))
        // Self-hosted media / cloud (2)
        .stdout(predicate::str::contains("lunasea"))
        .stdout(predicate::str::contains("nextcloud"))
        // Secure messaging (2)
        .stdout(predicate::str::contains("threema"))
        .stdout(predicate::str::contains("signal"))
        // Messaging (1)
        .stdout(predicate::str::contains("reddit"))
        // Relay / aggregation (1)
        .stdout(predicate::str::contains("apprise"))
        // Browser push (1)
        .stdout(predicate::str::contains("webpush"))
        // Iteration 7: new providers (13)
        .stdout(predicate::str::contains("whatsapp"))
        .stdout(predicate::str::contains("kodi"))
        .stdout(predicate::str::contains("notifico"))
        .stdout(predicate::str::contains("46elks"))
        .stdout(predicate::str::contains("bulkvs"))
        .stdout(predicate::str::contains("jira"))
        .stdout(predicate::str::contains("pushme"))
        .stdout(predicate::str::contains("sendpulse"))
        .stdout(predicate::str::contains("streamlabs"))
        .stdout(predicate::str::contains("synology"))
        .stdout(predicate::str::contains("africastalking"))
        .stdout(predicate::str::contains("o365"))
        .stdout(predicate::str::contains("nctalk"))
        // Iteration 8: new providers (13)
        .stdout(predicate::str::contains("emby"))
        .stdout(predicate::str::contains("jellyfin"))
        .stdout(predicate::str::contains("pushcut"))
        .stdout(predicate::str::contains("mqtt"))
        .stdout(predicate::str::contains("voipms"))
        .stdout(predicate::str::contains("sfr"))
        .stdout(predicate::str::contains("pushed"))
        .stdout(predicate::str::contains("growl"))
        .stdout(predicate::str::contains("kumulos"))
        .stdout(predicate::str::contains("parse"))
        .stdout(predicate::str::contains("rsyslog"))
        .stdout(predicate::str::contains("smsmanager"))
        .stdout(predicate::str::contains("twitter"))
        // Iteration 9: new providers (5)
        .stdout(predicate::str::contains("boxcar"))
        .stdout(predicate::str::contains("dapnet"))
        .stdout(predicate::str::contains("enigma2"))
        .stdout(predicate::str::contains("notifiarr"))
        .stdout(predicate::str::contains("statuspage"))
        // Iteration 10: new providers (5)
        .stdout(predicate::str::contains("dot"))
        .stdout(predicate::str::contains("fluxer"))
        .stdout(predicate::str::contains("workflows"))
        .stdout(predicate::str::contains("notificationapi"))
        .stdout(predicate::str::contains("spugpush"));
}

#[test]
fn test_providers_list_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "providers", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"scheme\""));
}

#[test]
fn test_providers_info() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["providers", "info", "wecom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wecom"))
        .stdout(predicate::str::contains("key"));
}

#[test]
fn test_providers_info_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "providers", "info", "wecom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"wecom\""))
        .stdout(predicate::str::contains("\"params\""));
}

#[test]
fn test_providers_info_unknown() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["providers", "info", "nonexistent"])
        .assert()
        .failure();
}

#[test]
fn test_send_missing_target() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["send", "--message", "hello"])
        .assert()
        .failure();
}

#[test]
fn test_config_path() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("config.toml"));
}

#[test]
fn test_config_list_empty() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles"));
}

#[test]
fn test_config_set_and_get() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_str = config_path.to_str().unwrap();

    // Set a profile
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "config",
            "set",
            "--name",
            "my-wecom",
            "--provider",
            "wecom",
            "--param",
            "key=test-key-123",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved"));

    // Get the profile
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "get", "my-wecom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wecom"));

    // List profiles
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-wecom"));

    // Remove profile
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "remove", "my-wecom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed"));
}
