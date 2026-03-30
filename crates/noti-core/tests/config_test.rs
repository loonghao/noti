use noti_core::{AppConfig, Profile, ProviderConfig};
use rstest::rstest;
use std::io::Write;
use std::sync::Mutex;

/// Global mutex to serialize tests that touch the `NOTI_CONFIG` environment variable.
/// Without this, parallel tests overwrite each other's env var values and cause flaky failures.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[rstest]
fn test_app_config_default_empty_profiles() {
    let config = AppConfig::default();
    assert!(config.profiles.is_empty());
}

#[rstest]
fn test_app_config_set_and_get_profile() {
    let mut config = AppConfig::default();
    let profile = Profile {
        provider: "wecom".to_string(),
        config: ProviderConfig::new().set("key", "test-key"),
    };
    config.set_profile("my-wecom", profile);

    let retrieved = config.get_profile("my-wecom");
    assert!(retrieved.is_some());
    let p = retrieved.unwrap();
    assert_eq!(p.provider, "wecom");
    assert_eq!(p.config.get("key"), Some("test-key"));
}

#[rstest]
fn test_app_config_get_profile_not_found() {
    let config = AppConfig::default();
    assert!(config.get_profile("nonexistent").is_none());
}

#[rstest]
fn test_app_config_set_profile_overwrites() {
    let mut config = AppConfig::default();
    let profile1 = Profile {
        provider: "wecom".to_string(),
        config: ProviderConfig::new().set("key", "first"),
    };
    config.set_profile("test", profile1);

    let profile2 = Profile {
        provider: "slack".to_string(),
        config: ProviderConfig::new().set("webhook_url", "https://..."),
    };
    config.set_profile("test", profile2);

    let retrieved = config.get_profile("test").unwrap();
    assert_eq!(retrieved.provider, "slack");
}

#[rstest]
fn test_app_config_remove_profile_existing() {
    let mut config = AppConfig::default();
    let profile = Profile {
        provider: "wecom".to_string(),
        config: ProviderConfig::new(),
    };
    config.set_profile("my-wecom", profile);

    assert!(config.remove_profile("my-wecom"));
    assert!(config.get_profile("my-wecom").is_none());
}

#[rstest]
fn test_app_config_remove_profile_not_existing() {
    let mut config = AppConfig::default();
    assert!(!config.remove_profile("nonexistent"));
}

#[rstest]
fn test_app_config_load_nonexistent_returns_default() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("nonexistent").join("config.toml");
    unsafe { std::env::set_var("NOTI_CONFIG", config_path.to_str().unwrap()) };
    let config = AppConfig::load().unwrap();
    assert!(config.profiles.is_empty());
    unsafe { std::env::remove_var("NOTI_CONFIG") };
}

#[rstest]
fn test_app_config_save_and_load_roundtrip() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    unsafe { std::env::set_var("NOTI_CONFIG", config_path.to_str().unwrap()) };

    let mut config = AppConfig::default();
    let profile = Profile {
        provider: "wecom".to_string(),
        config: ProviderConfig::new().set("key", "abc123"),
    };
    config.set_profile("work-bot", profile);
    config.save().unwrap();

    // Reload from disk
    let loaded = AppConfig::load().unwrap();
    let p = loaded.get_profile("work-bot").unwrap();
    assert_eq!(p.provider, "wecom");
    assert_eq!(p.config.get("key"), Some("abc123"));

    unsafe { std::env::remove_var("NOTI_CONFIG") };
}

#[rstest]
fn test_app_config_config_path_env_override() {
    let _guard = ENV_MUTEX.lock().unwrap();
    unsafe { std::env::set_var("NOTI_CONFIG", "/custom/path/config.toml") };
    let path = AppConfig::config_path().unwrap();
    assert_eq!(
        path.to_str().unwrap().replace('\\', "/"),
        "/custom/path/config.toml"
    );
    unsafe { std::env::remove_var("NOTI_CONFIG") };
}

#[rstest]
fn test_app_config_config_path_default() {
    let _guard = ENV_MUTEX.lock().unwrap();
    unsafe { std::env::remove_var("NOTI_CONFIG") };
    let path = AppConfig::config_path().unwrap();
    let path_str = path.to_string_lossy();
    assert!(path_str.contains("config.toml"));
    assert!(path_str.contains("noti"));
}

#[rstest]
fn test_app_config_load_invalid_toml() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    {
        let mut f = std::fs::File::create(&config_path).unwrap();
        f.write_all(b"this is not valid toml [[[").unwrap();
    }
    unsafe { std::env::set_var("NOTI_CONFIG", config_path.to_str().unwrap()) };
    let result = AppConfig::load();
    assert!(result.is_err());
    unsafe { std::env::remove_var("NOTI_CONFIG") };
}

#[rstest]
fn test_app_config_multiple_profiles() {
    let mut config = AppConfig::default();
    for i in 0..5 {
        let profile = Profile {
            provider: format!("provider_{i}"),
            config: ProviderConfig::new().set("key", format!("value_{i}")),
        };
        config.set_profile(format!("profile_{i}"), profile);
    }
    assert_eq!(config.profiles.len(), 5);
    for i in 0..5 {
        let p = config.get_profile(&format!("profile_{i}")).unwrap();
        assert_eq!(p.provider, format!("provider_{i}"));
    }
}

#[rstest]
fn test_profile_serde_roundtrip() {
    let profile = Profile {
        provider: "wecom".to_string(),
        config: ProviderConfig::new().set("key", "abc"),
    };
    let json = serde_json::to_string(&profile).unwrap();
    let parsed: Profile = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.provider, "wecom");
    assert_eq!(parsed.config.get("key"), Some("abc"));
}

#[rstest]
fn test_app_config_save_creates_parent_dirs() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let nested_path = temp_dir
        .path()
        .join("deep")
        .join("nested")
        .join("config.toml");
    unsafe { std::env::set_var("NOTI_CONFIG", nested_path.to_str().unwrap()) };

    let config = AppConfig::default();
    let result = config.save();
    assert!(result.is_ok());
    assert!(nested_path.exists());

    unsafe { std::env::remove_var("NOTI_CONFIG") };
}

#[rstest]
fn test_app_config_load_empty_file() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(&config_path, "").unwrap();

    unsafe { std::env::set_var("NOTI_CONFIG", config_path.to_str().unwrap()) };
    let config = AppConfig::load().unwrap();
    assert!(config.profiles.is_empty());
    unsafe { std::env::remove_var("NOTI_CONFIG") };
}

#[rstest]
fn test_app_config_load_valid_toml_with_profiles() {
    let _guard = ENV_MUTEX.lock().unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let toml_content = r#"
[profiles.work]
provider = "wecom"
key = "abc123"

[profiles.personal]
provider = "telegram"
bot_token = "123:ABC"
chat_id = "-100"
"#;
    std::fs::write(&config_path, toml_content).unwrap();

    unsafe { std::env::set_var("NOTI_CONFIG", config_path.to_str().unwrap()) };
    let config = AppConfig::load().unwrap();
    assert_eq!(config.profiles.len(), 2);

    let work = config.get_profile("work").unwrap();
    assert_eq!(work.provider, "wecom");
    assert_eq!(work.config.get("key"), Some("abc123"));

    let personal = config.get_profile("personal").unwrap();
    assert_eq!(personal.provider, "telegram");
    assert_eq!(personal.config.get("bot_token"), Some("123:ABC"));
    assert_eq!(personal.config.get("chat_id"), Some("-100"));

    unsafe { std::env::remove_var("NOTI_CONFIG") };
}
