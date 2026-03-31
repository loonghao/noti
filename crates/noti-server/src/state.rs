use std::sync::Arc;
use std::time::SystemTime;

use noti_core::{ProviderRegistry, StatusTracker, TemplateRegistry};
use noti_queue::{InMemoryQueue, QueueBackend, SqliteQueue, WorkerConfig, WorkerHandle, WorkerPool};
use tokio::sync::{Notify, RwLock};

use crate::config::QueueBackendType;

/// Shared application state for all request handlers.
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<ProviderRegistry>,
    pub status_tracker: StatusTracker,
    pub template_registry: Arc<RwLock<TemplateRegistry>>,
    pub queue: Arc<dyn QueueBackend>,
    pub task_notify: Arc<Notify>,
    pub started_at: SystemTime,
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
            started_at: SystemTime::now(),
        }
    }

    /// Create state with a specific queue backend.
    ///
    /// For persistent backends (SQLite), this also recovers any tasks that
    /// were left in `Processing` state after an unclean shutdown.
    pub async fn with_queue_backend(
        registry: ProviderRegistry,
        backend: &QueueBackendType,
        db_path: &str,
    ) -> Self {
        let (queue, task_notify): (Arc<dyn QueueBackend>, Arc<Notify>) = match backend {
            QueueBackendType::Sqlite => {
                let q = SqliteQueue::open(db_path)
                    .expect("failed to open SQLite queue database");
                let notify = q.notifier();
                (Arc::new(q), notify)
            }
            QueueBackendType::Memory => {
                let q = InMemoryQueue::new();
                let notify = q.notifier();
                (Arc::new(q), notify)
            }
        };

        // Recover stale tasks left in "processing" state from a previous crash
        match queue.recover_stale_tasks().await {
            Ok(0) => {}
            Ok(n) => tracing::info!(recovered = n, "recovered stale processing tasks"),
            Err(e) => tracing::warn!(error = %e, "failed to recover stale tasks"),
        }

        Self {
            registry: Arc::new(registry),
            status_tracker: StatusTracker::new(),
            template_registry: Arc::new(RwLock::new(TemplateRegistry::new())),
            queue,
            task_notify,
            started_at: SystemTime::now(),
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
