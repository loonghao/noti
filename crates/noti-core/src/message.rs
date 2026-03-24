use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported message formats.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageFormat {
    #[default]
    Text,
    Markdown,
    Html,
}

impl std::fmt::Display for MessageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Markdown => write!(f, "markdown"),
            Self::Html => write!(f, "html"),
        }
    }
}

impl std::str::FromStr for MessageFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "markdown" | "md" => Ok(Self::Markdown),
            "html" => Ok(Self::Html),
            _ => Err(format!("unknown message format: {s}")),
        }
    }
}

/// A unified message to be sent through any notification provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The message body text.
    pub text: String,

    /// Optional title / subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Message format (text, markdown, html).
    #[serde(default)]
    pub format: MessageFormat,

    /// Extra provider-specific key-value pairs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Message {
    /// Create a new plain-text message.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            title: None,
            format: MessageFormat::Text,
            extra: HashMap::new(),
        }
    }

    /// Create a new markdown message.
    pub fn markdown(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            title: None,
            format: MessageFormat::Markdown,
            extra: HashMap::new(),
        }
    }

    /// Set the title.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the format.
    pub fn with_format(mut self, format: MessageFormat) -> Self {
        self.format = format;
        self
    }

    /// Add an extra key-value pair.
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}
