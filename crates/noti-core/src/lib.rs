pub mod config;
pub mod error;
pub mod message;
pub mod priority;
pub mod provider;
pub mod registry;
pub mod retry;
pub mod template;
pub mod url;

// Re-export commonly used types at crate root.
pub use config::{AppConfig, Profile};
pub use error::NotiError;
pub use message::{Attachment, AttachmentKind, Message, MessageFormat};
pub use priority::Priority;
pub use provider::{NotifyProvider, ParamDef, ProviderConfig, SendResponse};
pub use registry::ProviderRegistry;
pub use retry::RetryPolicy;
pub use template::{MessageTemplate, TemplateRegistry};
pub use url::{ParsedUrl, parse_notification_url};
