use std::sync::Arc;
use std::time::SystemTime;
use std::path::PathBuf;

use noti_core::{CircuitBreakerRegistry, ProviderRegistry, StatusTracker, TemplateRegistry};
use noti_queue::{
    InMemoryQueue, QueueBackend, QueueError, SqliteQueue, WorkerConfig, WorkerHandle,
    WorkerPool, WorkerStatsHandle,
};
use tokio::sync::{Notify, RwLock};

use crate::config::QueueBackendType;
use crate::middleware::rate_limit::RateLimiterState;

/// Shared application state for all request handlers.
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<ProviderRegistry>,
    pub circuit_breakers: Arc<CircuitBreakerRegistry>,
    pub status_tracker: StatusTracker,
    pub template_registry: Arc<RwLock<TemplateRegistry>>,
    pub queue: Arc<dyn QueueBackend>,
    pub task_notify: Arc<Notify>,
    pub started_at: SystemTime,
    /// Root directory for file storage (uploaded files and thumbnails).
    pub storage_root: PathBuf,
    /// Optional worker handle for accessing worker statistics.
    /// None when workers are not started (e.g., read-only mode or tests).
    /// Stored as Arc so that AppState can remain Clone while allowing
    /// stats to be shared across clones.
    pub worker_stats_handle: Option<Arc<WorkerStatsHandle>>,
    /// Optional rate limiter state for accessing rate limiting metrics.
    /// None when rate limiting is not enabled or when in read-only mode/tests.
    /// This is set by `main.rs` after creating the rate limiter so that
    /// the prometheus handler can access rate limit metrics.
    pub rate_limiter: Option<RateLimiterState>,
}

impl AppState {
    /// Create state with the default in-memory queue backend.
    pub fn new(registry: ProviderRegistry) -> Self {
        let queue = Arc::new(InMemoryQueue::new());
        let task_notify = queue.notifier();

        Self {
            registry: Arc::new(registry),
            circuit_breakers: Arc::new(CircuitBreakerRegistry::new()),
            status_tracker: StatusTracker::new(),
            template_registry: Arc::new(RwLock::new(TemplateRegistry::new())),
            queue,
            task_notify,
            started_at: SystemTime::now(),
            storage_root: PathBuf::from("storage"),
            worker_stats_handle: None,
            rate_limiter: None,
        }
    }

    /// Create state with a specific queue backend.
    ///
    /// For persistent backends (SQLite), this also recovers any tasks that
    /// were left in `Processing` state after an unclean shutdown.
    ///
    /// # Errors
    ///
    /// Returns [`QueueError`] if the SQLite database cannot be opened.
    pub async fn with_queue_backend(
        registry: ProviderRegistry,
        backend: &QueueBackendType,
        db_path: &str,
    ) -> Result<Self, QueueError> {
        let (queue, task_notify): (Arc<dyn QueueBackend>, Arc<Notify>) = match backend {
            QueueBackendType::Sqlite => {
                let q = SqliteQueue::open(db_path)?;
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

        Ok(Self {
            registry: Arc::new(registry),
            circuit_breakers: Arc::new(CircuitBreakerRegistry::new()),
            status_tracker: StatusTracker::new(),
            template_registry: Arc::new(RwLock::new(TemplateRegistry::new())),
            queue,
            task_notify,
            started_at: SystemTime::now(),
            storage_root: PathBuf::from("storage"),
            worker_stats_handle: None,
            rate_limiter: None,
        })
    }

    /// Create state with a caller-provided queue backend and notifier.
    ///
    /// Useful in tests where you want full control over the queue implementation
    /// (e.g. an in-memory SQLite queue for isolation without file I/O).
    pub fn with_custom_queue(
        registry: ProviderRegistry,
        queue: Arc<dyn QueueBackend>,
        task_notify: Arc<Notify>,
    ) -> Self {
        Self {
            registry: Arc::new(registry),
            circuit_breakers: Arc::new(CircuitBreakerRegistry::new()),
            status_tracker: StatusTracker::new(),
            template_registry: Arc::new(RwLock::new(TemplateRegistry::new())),
            queue,
            task_notify,
            started_at: SystemTime::now(),
            storage_root: PathBuf::from("storage"),
            worker_stats_handle: None,
            rate_limiter: None,
        }
    }

    /// Start background worker pool for async task processing.
    ///
    /// Returns `(WorkerHandle, WorkerStatsHandle)`. Use `WorkerHandle` for shutdown
    /// and `WorkerStatsHandle` for querying worker statistics.
    pub fn start_workers(&self, config: WorkerConfig) -> (WorkerHandle, WorkerStatsHandle) {
        WorkerPool::start(
            self.queue.clone(),
            self.registry.clone(),
            self.circuit_breakers.clone(),
            config,
            self.task_notify.clone(),
        )
    }

    /// Return a new AppState clone with the worker handle set.
    /// This is needed because AppState is Clone but we need to set the
    /// worker_stats_handle after starting workers.
    pub fn with_worker_handle(self, worker_stats_handle: Arc<WorkerStatsHandle>) -> Self {
        let mut this = self;
        this.worker_stats_handle = Some(worker_stats_handle);
        this
    }

    /// Return a new AppState clone with the rate limiter state set.
    /// This is needed because AppState is Clone but we need to set the
    /// rate_limiter after creating it in main.rs so that prometheus
    /// handler can access rate limit metrics.
    pub fn with_rate_limiter(self, rate_limiter: RateLimiterState) -> Self {
        let mut this = self;
        this.rate_limiter = Some(rate_limiter);
        this
    }

    /// Return a new AppState clone with a custom storage root directory.
    pub fn with_storage_root(self, storage_root: PathBuf) -> Self {
        let mut this = self;
        this.storage_root = storage_root;
        this
    }

    /// Returns the directory for uploaded files.
    pub fn storage_dir(&self) -> PathBuf {
        self.storage_root.join("uploads")
    }

    /// Returns the directory for generated thumbnails.
    pub fn thumbnails_dir(&self) -> PathBuf {
        self.storage_root.join("thumbnails")
    }
}
