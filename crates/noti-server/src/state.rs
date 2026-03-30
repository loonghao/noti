use std::sync::Arc;

use noti_core::{ProviderRegistry, StatusTracker, TemplateRegistry};
use tokio::sync::RwLock;

/// Shared application state for all request handlers.
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<ProviderRegistry>,
    pub status_tracker: StatusTracker,
    pub template_registry: Arc<RwLock<TemplateRegistry>>,
}

impl AppState {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
            status_tracker: StatusTracker::new(),
            template_registry: Arc::new(RwLock::new(TemplateRegistry::new())),
        }
    }
}
