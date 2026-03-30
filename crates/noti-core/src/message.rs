use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

/// The kind of attachment being sent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentKind {
    /// A generic file attachment.
    File,
    /// An image (may be displayed inline by some providers).
    Image,
    /// An audio clip.
    Audio,
    /// A video file.
    Video,
}

impl Default for AttachmentKind {
    fn default() -> Self {
        Self::File
    }
}

/// Infer `AttachmentKind` from a MIME type string.
fn kind_from_mime(mime: &str) -> AttachmentKind {
    if mime.starts_with("image/") {
        AttachmentKind::Image
    } else if mime.starts_with("audio/") {
        AttachmentKind::Audio
    } else if mime.starts_with("video/") {
        AttachmentKind::Video
    } else {
        AttachmentKind::File
    }
}

/// A file attachment to include with a notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    /// Local file path (resolved at send time).
    pub path: PathBuf,

    /// MIME type (auto-detected from extension if not set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Logical kind of attachment.
    #[serde(default)]
    pub kind: AttachmentKind,

    /// File name override (defaults to the file name from `path`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

impl Attachment {
    /// Create an attachment from a file path, auto-detecting MIME and kind.
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let mime = mime_guess::from_path(&path)
            .first()
            .map(|m| m.to_string());
        let kind = mime
            .as_deref()
            .map(kind_from_mime)
            .unwrap_or(AttachmentKind::File);
        Self {
            path,
            mime_type: mime,
            kind,
            file_name: None,
        }
    }

    /// Override the MIME type.
    pub fn with_mime(mut self, mime: impl Into<String>) -> Self {
        let m = mime.into();
        self.kind = kind_from_mime(&m);
        self.mime_type = Some(m);
        self
    }

    /// Override the attachment kind.
    pub fn with_kind(mut self, kind: AttachmentKind) -> Self {
        self.kind = kind;
        self
    }

    /// Override the file name sent to the provider.
    pub fn with_file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// Return the effective file name (override or stem from path).
    pub fn effective_file_name(&self) -> String {
        self.file_name.clone().unwrap_or_else(|| {
            self.path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "attachment".to_string())
        })
    }

    /// Return the effective MIME type string.
    pub fn effective_mime(&self) -> String {
        self.mime_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string())
    }

    /// Read the full file contents into memory.
    pub async fn read_bytes(&self) -> std::io::Result<Vec<u8>> {
        tokio::fs::read(&self.path).await
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

    /// File attachments (images, documents, etc.).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,

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
            attachments: Vec::new(),
            extra: HashMap::new(),
        }
    }

    /// Create a new markdown message.
    pub fn markdown(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            title: None,
            format: MessageFormat::Markdown,
            attachments: Vec::new(),
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

    /// Add a single attachment.
    pub fn with_attachment(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Add a file attachment by path (auto-detects MIME type and kind).
    pub fn with_file(self, path: impl AsRef<Path>) -> Self {
        self.with_attachment(Attachment::from_path(path))
    }

    /// Whether this message has any attachments.
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    /// Get the first image attachment, if any.
    pub fn first_image(&self) -> Option<&Attachment> {
        self.attachments
            .iter()
            .find(|a| a.kind == AttachmentKind::Image)
    }

    /// Add an extra key-value pair.
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}
