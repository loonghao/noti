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
        .stdout(predicate::str::contains("wecom"))
        .stdout(predicate::str::contains("slack"))
        .stdout(predicate::str::contains("telegram"))
        .stdout(predicate::str::contains("discord"))
        .stdout(predicate::str::contains("feishu"))
        .stdout(predicate::str::contains("email"))
        .stdout(predicate::str::contains("webhook"));
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
