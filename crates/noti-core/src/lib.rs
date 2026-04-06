pub mod circuit_breaker;
pub mod config;
pub mod error;
pub mod message;
pub mod priority;
pub mod provider;
pub mod registry;
pub mod retry;
pub mod sender;
pub mod status;
pub mod template;
pub mod url;

// Re-export commonly used types at crate root.
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry, CircuitState, SharedCircuitBreaker};
pub use config::{AppConfig, Profile};
pub use error::NotiError;
pub use message::{Attachment, AttachmentKind, Message, MessageFormat};
pub use priority::Priority;
pub use provider::{NotifyProvider, ParamDef, ProviderConfig, SendResponse};
pub use registry::ProviderRegistry;
pub use retry::{RetryPolicy, send_with_retry};
pub use sender::{BatchResult, SendTarget, TargetResult, send_batch, send_failover};
pub use status::{DeliveryRecord, DeliveryStatus, StatusEvent, StatusSummary, StatusTracker};
pub use template::{MessageTemplate, TemplateRegistry};
pub use url::{ParsedUrl, parse_notification_url};
