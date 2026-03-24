use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::NotiError;
use crate::message::Message;

/// Describes a single parameter accepted by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    /// Parameter name (used as key in config / CLI flag).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Whether this parameter is required.
    pub required: bool,
    /// Optional example value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

impl ParamDef {
    /// Create a required parameter definition.
    pub fn required(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: true,
            example: None,
        }
    }

    /// Create an optional parameter definition.
    pub fn optional(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: false,
            example: None,
        }
    }

    /// Set an example value.
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }
}

/// Provider-specific configuration (a flat key-value map).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Configuration values (webhook key, token, URL, etc.).
    #[serde(flatten)]
    pub values: HashMap<String, String>,
}

impl ProviderConfig {
    /// Create an empty provider config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a key-value pair.
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// Get a required value or return error.
    pub fn require(&self, key: &str, provider: &str) -> Result<&str, NotiError> {
        self.get(key).ok_or_else(|| {
            NotiError::Validation(format!(
                "missing required parameter '{key}' for provider '{provider}'"
            ))
        })
    }
}

/// The result of a send operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResponse {
    /// Whether the send was successful.
    pub success: bool,
    /// Name of the provider that handled the send.
    pub provider: String,
    /// HTTP status code (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Human-readable result message.
    pub message: String,
    /// Raw response body from the provider API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<serde_json::Value>,
}

impl SendResponse {
    /// Create a success response.
    pub fn success(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: true,
            provider: provider.into(),
            status_code: None,
            message: message.into(),
            raw_response: None,
        }
    }

    /// Create a failure response.
    pub fn failure(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            provider: provider.into(),
            status_code: None,
            message: message.into(),
            raw_response: None,
        }
    }

    /// Set the HTTP status code.
    pub fn with_status_code(mut self, code: u16) -> Self {
        self.status_code = Some(code);
        self
    }

    /// Set the raw response body.
    pub fn with_raw_response(mut self, raw: serde_json::Value) -> Self {
        self.raw_response = Some(raw);
        self
    }
}

/// The core trait every notification provider must implement.
#[async_trait]
pub trait NotifyProvider: Send + Sync {
    /// Unique name of the provider (e.g. "wecom", "slack").
    fn name(&self) -> &str;

    /// URL scheme prefix (e.g. "wecom", "slack", "tg").
    fn url_scheme(&self) -> &str;

    /// List of parameters this provider accepts.
    fn params(&self) -> Vec<ParamDef>;

    /// A one-line description of the provider.
    fn description(&self) -> &str;

    /// Example URL scheme usage.
    fn example_url(&self) -> &str;

    /// Validate that the given config has all required parameters.
    fn validate_config(&self, config: &ProviderConfig) -> Result<(), NotiError> {
        for param in self.params() {
            if param.required && config.get(&param.name).is_none() {
                return Err(NotiError::Validation(format!(
                    "missing required parameter '{}' for provider '{}'",
                    param.name,
                    self.name()
                )));
            }
        }
        Ok(())
    }

    /// Send a message using this provider.
    async fn send(
        &self,
        message: &Message,
        config: &ProviderConfig,
    ) -> Result<SendResponse, NotiError>;
}
