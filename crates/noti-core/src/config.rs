use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::NotiError;
use crate::provider::ProviderConfig;

/// A named profile that stores provider + config for quick reuse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// The provider name (e.g. "wecom", "slack").
    pub provider: String,
    /// Provider-specific configuration values.
    #[serde(flatten)]
    pub config: ProviderConfig,
}

/// Top-level application configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Named profiles.
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

impl AppConfig {
    /// Load configuration from the default path or `NOTI_CONFIG` env var.
    pub fn load() -> Result<Self, NotiError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| NotiError::Config(format!("failed to read {}: {e}", path.display())))?;
        toml::from_str(&content)
            .map_err(|e| NotiError::Config(format!("failed to parse {}: {e}", path.display())))
    }

    /// Save configuration to the default path.
    pub fn save(&self) -> Result<(), NotiError> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| NotiError::Config(format!("failed to create config dir: {e}")))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| NotiError::Config(format!("failed to serialize config: {e}")))?;
        std::fs::write(&path, content)
            .map_err(|e| NotiError::Config(format!("failed to write {}: {e}", path.display())))
    }

    /// Get a profile by name.
    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// Set (create or update) a profile.
    pub fn set_profile(&mut self, name: impl Into<String>, profile: Profile) {
        self.profiles.insert(name.into(), profile);
    }

    /// Remove a profile by name. Returns true if it existed.
    pub fn remove_profile(&mut self, name: &str) -> bool {
        self.profiles.remove(name).is_some()
    }

    /// Resolve the configuration file path.
    ///
    /// Priority: `NOTI_CONFIG` env var > `~/.config/noti/config.toml`
    pub fn config_path() -> Result<PathBuf, NotiError> {
        if let Ok(env_path) = std::env::var("NOTI_CONFIG") {
            return Ok(PathBuf::from(env_path));
        }
        let config_dir = dirs::config_dir()
            .ok_or_else(|| NotiError::Config("could not determine config directory".into()))?;
        Ok(config_dir.join("noti").join("config.toml"))
    }
}
