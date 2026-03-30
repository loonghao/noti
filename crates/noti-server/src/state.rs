use std::sync::Arc;

use noti_core::{ProviderRegistry, StatusTracker, TemplateRegistry};
use noti_queue::{InMemoryQueue, QueueBackend, WorkerConfig, WorkerHandle, WorkerPool};
use tokio::sync::{Notify, RwLock};

/// Shared application state for all request handlers.
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<ProviderRegistry>,
    pub status_tracker: StatusTracker,
    pub template_registry: Arc<RwLock<TemplateRegistry>>,
    pub queue: Arc<dyn QueueBackend>,
    pub task_notify: Arc<Notify>,
}

impl AppState {
    pub fn new(registry: ProviderRegistry) -> Self {
        let queue = Arc::new(InMemoryQueue::new());
        let task_notify = queue.notifier();

        Self {
            registry: Arc::new(registry),
            status_tracker: StatusTracker::new(),
            template_registry: Arc::new(RwLock::new(TemplateRegistry::new())),
            queue,
            task_notify,
        }
    }

    /// Start background worker pool for async task processing.
    ///
    /// Returns a handle that must be kept alive; dropping it does not shut down
    /// workers, but calling `shutdown_and_join()` on it will.
    pub fn start_workers(&self, config: WorkerConfig) -> WorkerHandle {
        WorkerPool::start(
            self.queue.clone(),
            self.registry.clone(),
            config,
            self.task_notify.clone(),
        )
    }
}
