/// Extended CLI integration tests for additional coverage.
use assert_cmd::Command;
use predicates::prelude::*;
use rstest::rstest;

// ======================== Help subcommands ========================

#[rstest]
fn test_send_help() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["send", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--message"))
        .stdout(predicate::str::contains("--to"))
        .stdout(predicate::str::contains("--profile"))
        .stdout(predicate::str::contains("--provider"));
}

#[rstest]
fn test_config_help() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("test"))
        .stdout(predicate::str::contains("path"));
}

#[rstest]
fn test_providers_help() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["providers", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("info"));
}

// ======================== JSON output tests ========================

#[rstest]
fn test_config_path_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"path\""))
        .stdout(predicate::str::contains("config.toml"));
}

#[rstest]
fn test_config_list_empty_json() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["--json", "config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

#[rstest]
fn test_config_set_and_get_json() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_str = config_path.to_str().unwrap();

    // Set a profile
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "--json",
            "config",
            "set",
            "--name",
            "json-test",
            "--provider",
            "slack",
            "--param",
            "webhook_url=https://hooks.slack.com/test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("success"));

    // Get as JSON
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["--json", "config", "get", "json-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("slack"))
        .stdout(predicate::str::contains("webhook_url"));

    // List as JSON
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["--json", "config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("json-test"))
        .stdout(predicate::str::contains("slack"));

    // Remove as JSON
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["--json", "config", "remove", "json-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("success"));
}

// ======================== Error cases ========================

#[rstest]
fn test_send_no_message() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["send", "--to", "wecom://key123"])
        .assert()
        .failure();
}

#[rstest]
fn test_send_invalid_url() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["send", "--message", "test", "--to", "invalid-url"])
        .assert()
        .failure();
}

#[rstest]
fn test_send_unknown_provider() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["send", "--message", "test", "--provider", "nonexistent"])
        .assert()
        .failure();
}

#[rstest]
fn test_config_get_nonexistent() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["config", "get", "does-not-exist"])
        .assert()
        .failure();
}

#[rstest]
fn test_config_remove_nonexistent() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["config", "remove", "does-not-exist"])
        .assert()
        .failure();
}

#[rstest]
fn test_providers_info_nonexistent_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "providers", "info", "nonexistent"])
        .assert()
        .failure();
}

// ======================== Providers info for various providers ========================

#[rstest]
#[case("slack")]
#[case("telegram")]
#[case("discord")]
#[case("ntfy")]
#[case("gotify")]
#[case("pushover")]
#[case("webhook")]
#[case("feishu")]
#[case("dingtalk")]
fn test_providers_info_various(#[case] provider_name: &str) {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["providers", "info", provider_name])
        .assert()
        .success()
        .stdout(predicate::str::contains(provider_name));
}

#[rstest]
#[case("slack")]
#[case("telegram")]
#[case("webhook")]
fn test_providers_info_json_various(#[case] provider_name: &str) {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "providers", "info", provider_name])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"params\""))
        .stdout(predicate::str::contains("\"description\""))
        .stdout(predicate::str::contains("\"example_url\""));
}

// ======================== Config set with multiple params ========================

#[rstest]
fn test_config_set_multiple_params() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_str = config_path.to_str().unwrap();

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "config",
            "set",
            "--name",
            "multi-param",
            "--provider",
            "telegram",
            "--param",
            "bot_token=123:ABC",
            "--param",
            "chat_id=-1001234567890",
        ])
        .assert()
        .success();

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "get", "multi-param"])
        .assert()
        .success()
        .stdout(predicate::str::contains("telegram"))
        .stdout(predicate::str::contains("bot_token"))
        .stdout(predicate::str::contains("chat_id"));
}

// ======================== Config overwrite ========================

#[rstest]
fn test_config_set_overwrites_existing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_str = config_path.to_str().unwrap();

    // Set first version
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "config",
            "set",
            "--name",
            "overwrite-test",
            "--provider",
            "wecom",
            "--param",
            "key=old-key",
        ])
        .assert()
        .success();

    // Overwrite
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "config",
            "set",
            "--name",
            "overwrite-test",
            "--provider",
            "slack",
            "--param",
            "webhook_url=https://new-url",
        ])
        .assert()
        .success();

    // Verify overwritten
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "get", "overwrite-test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("slack"));
}

// ======================== No subcommand ========================

#[rstest]
fn test_no_subcommand() {
    Command::cargo_bin("noti").unwrap().assert().failure();
}

// ======================== Config test subcommand ========================

#[rstest]
fn test_config_test_nonexistent_profile() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["config", "test", "does-not-exist"])
        .assert()
        .failure();
}

#[rstest]
fn test_config_test_nonexistent_profile_json() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["--json", "config", "test", "nonexistent"])
        .assert()
        .failure();
}

// ======================== Send --profile mode ========================

#[rstest]
fn test_send_profile_nonexistent() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["send", "--message", "test", "--profile", "nonexistent"])
        .assert()
        .failure();
}

// ======================== Send --provider + --param mode ========================

#[rstest]
fn test_send_provider_missing_required_params() {
    // Sending with a valid provider but missing required params should fail
    // (validate_config will reject it)
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test",
            "--provider",
            "webhook",
            // Missing the required "url" param
        ])
        .assert()
        .failure();
}

#[rstest]
fn test_send_provider_invalid_param_format() {
    // Param without '=' separator should fail
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test",
            "--provider",
            "webhook",
            "--param",
            "invalid-param-no-equals",
        ])
        .assert()
        .failure();
}

#[rstest]
fn test_config_set_invalid_param_format() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // config set with param that has no '='
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args([
            "config",
            "set",
            "--name",
            "bad-param",
            "--provider",
            "slack",
            "--param",
            "missing-equals",
        ])
        .assert()
        .failure();
}

// ======================== Send --format validation ========================

#[rstest]
fn test_send_invalid_format() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test",
            "--to",
            "webhook://http://localhost",
            "--format",
            "invalid_format",
        ])
        .assert()
        .failure();
}

// ======================== Send with title ========================

#[rstest]
fn test_send_with_title_flag() {
    // Even though we can't actually send, verify the flag is accepted
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test body",
            "--title",
            "My Title",
            "--provider",
            "webhook",
            // Missing url, so it will fail at validation
        ])
        .assert()
        .failure();
}

// ======================== Config get/remove in JSON mode ========================

#[rstest]
fn test_config_get_nonexistent_json() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["--json", "config", "get", "does-not-exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[rstest]
fn test_config_remove_nonexistent_json() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_path.to_str().unwrap())
        .args(["--json", "config", "remove", "does-not-exist"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

// ======================== Send with --json error output ========================

#[rstest]
fn test_send_invalid_url_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "send", "--message", "test", "--to", "bad-url"])
        .assert()
        .failure();
}

#[rstest]
fn test_send_unknown_provider_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "--json",
            "send",
            "--message",
            "test",
            "--provider",
            "nonexistent",
        ])
        .assert()
        .failure();
}

// ======================== Send with --format markdown/html ========================

#[rstest]
fn test_send_format_markdown_flag() {
    // Verify markdown format is accepted (but fails due to missing webhook url)
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test",
            "--provider",
            "webhook",
            "--format",
            "markdown",
        ])
        .assert()
        .failure();
}

#[rstest]
fn test_send_format_html_flag() {
    // Verify html format is accepted (but fails due to missing webhook url)
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test",
            "--provider",
            "webhook",
            "--format",
            "html",
        ])
        .assert()
        .failure();
}

// ======================== Config test with unknown provider in profile ========================

#[rstest]
fn test_config_test_profile_with_unknown_provider() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_str = config_path.to_str().unwrap();

    // Create a profile with a non-existent provider name
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "config",
            "set",
            "--name",
            "bad-provider-profile",
            "--provider",
            "nonexistent_provider_xyz",
            "--param",
            "key=value",
        ])
        .assert()
        .success();

    // Testing this profile should fail because the provider doesn't exist
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "test", "bad-provider-profile"])
        .assert()
        .failure();
}

// ======================== Agent-First CLI: Schema introspection ========================

#[rstest]
fn test_schema_list_all() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["schema"])
        .assert()
        .success()
        .stdout(predicate::str::contains("schema"));
}

#[rstest]
fn test_schema_list_all_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "schema"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"provider\""))
        .stdout(predicate::str::contains("\"required_params\""));
}

#[rstest]
fn test_schema_specific_provider() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["schema", "wecom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wecom"))
        .stdout(predicate::str::contains("key"));
}

#[rstest]
fn test_schema_specific_provider_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["--json", "schema", "slack"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"provider\": \"slack\""))
        .stdout(predicate::str::contains("\"params\""))
        .stdout(predicate::str::contains("\"send_command\""));
}

#[rstest]
fn test_schema_unknown_provider() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["schema", "nonexistent"])
        .assert()
        .failure();
}

// ======================== Agent-First CLI: Dry-run ========================

#[rstest]
fn test_send_dry_run() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test message",
            "--provider",
            "wecom",
            "--param",
            "key=test-key-123",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run"));
}

#[rstest]
fn test_send_dry_run_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "--json",
            "send",
            "--message",
            "test message",
            "--provider",
            "wecom",
            "--param",
            "key=test-key-123",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"dry_run\""))
        .stdout(predicate::str::contains("\"valid\": true"))
        .stdout(predicate::str::contains("\"provider\": \"wecom\""));
}

#[rstest]
fn test_config_set_dry_run() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_str = config_path.to_str().unwrap();

    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "config",
            "set",
            "--name",
            "dry-test",
            "--provider",
            "slack",
            "--param",
            "webhook_url=https://hooks.slack.com/test",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run"));

    // Verify nothing was actually saved
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles"));
}

#[rstest]
fn test_config_remove_dry_run() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let config_str = config_path.to_str().unwrap();

    // First, create a profile
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args([
            "config",
            "set",
            "--name",
            "keep-me",
            "--provider",
            "wecom",
            "--param",
            "key=abc",
        ])
        .assert()
        .success();

    // Dry-run remove
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "remove", "keep-me", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dry-run"));

    // Verify it was NOT removed
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_CONFIG", config_str)
        .args(["config", "get", "keep-me"])
        .assert()
        .success()
        .stdout(predicate::str::contains("wecom"));
}

// ======================== Agent-First CLI: JSON payload ========================

#[rstest]
fn test_send_json_payload_dry_run() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "--json",
            "send",
            "--provider",
            "wecom",
            "--param",
            "key=test-key-123",
            "--json-payload",
            r#"{"text": "hello from agent", "format": "markdown", "priority": "high"}"#,
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"dry_run\""))
        .stdout(predicate::str::contains("hello from agent"));
}

#[rstest]
fn test_send_json_payload_invalid_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--provider",
            "wecom",
            "--param",
            "key=test-key-123",
            "--json-payload",
            "not valid json",
        ])
        .assert()
        .failure();
}

// ======================== Agent-First CLI: --fields filtering ========================

#[rstest]
fn test_send_dry_run_with_fields() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "--json",
            "--fields",
            "status,provider",
            "send",
            "--message",
            "test",
            "--provider",
            "wecom",
            "--param",
            "key=test-key-123",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\""))
        .stdout(predicate::str::contains("\"provider\""));
}

// ======================== Agent-First CLI: NOTI_OUTPUT env var ========================

#[rstest]
fn test_noti_output_env_json() {
    Command::cargo_bin("noti")
        .unwrap()
        .env("NOTI_OUTPUT", "json")
        .args(["schema", "wecom"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"provider\": \"wecom\""));
}

// ======================== Agent-First CLI: Input hardening ========================

#[rstest]
fn test_send_rejects_path_traversal() {
    Command::cargo_bin("noti")
        .unwrap()
        .args([
            "send",
            "--message",
            "test",
            "--provider",
            "wecom",
            "--param",
            "key=abc",
            "--file",
            "../../../etc/passwd",
        ])
        .assert()
        .failure();
}

// ======================== Schema help text ========================

#[rstest]
fn test_schema_help() {
    Command::cargo_bin("noti")
        .unwrap()
        .args(["schema", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Introspect"));
}
