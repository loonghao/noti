use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::Instrument;

use noti_core::{CircuitBreakerRegistry, ProviderRegistry};

use crate::callback::fire_callback;
use crate::queue::QueueBackend;

/// Worker pool statistics for health monitoring.
/// Wrapped in Arc for cheap cloning across worker tasks.
pub struct WorkerStats {
    /// Total number of workers in the pool.
    pub total: u32,
    /// Number of workers currently processing a task.
    active: AtomicU32,
    /// Number of workers currently idle (waiting for tasks).
    idle: AtomicU32,
}

impl Default for WorkerStats {
    fn default() -> Self {
        Self {
            total: 0,
            active: AtomicU32::new(0),
            idle: AtomicU32::new(0),
        }
    }
}

impl std::fmt::Debug for WorkerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerStats")
            .field("total", &self.total)
            .field("active", &self.active.load(Ordering::SeqCst))
            .field("idle", &self.idle.load(Ordering::SeqCst))
            .finish()
    }
}

impl WorkerStats {
    /// Increment active count and decrement idle count.
    pub fn mark_active(&self) {
        self.active.fetch_add(1, Ordering::SeqCst);
        self.idle.fetch_sub(1, Ordering::SeqCst);
    }

    /// Increment idle count and decrement active count.
    pub fn mark_idle(&self) {
        self.idle.fetch_add(1, Ordering::SeqCst);
        self.active.fetch_sub(1, Ordering::SeqCst);
    }

    /// Get current snapshot of worker stats.
    pub fn snapshot(&self) -> WorkerStatsSnapshot {
        WorkerStatsSnapshot {
            total: self.total,
            active: self.active.load(Ordering::SeqCst),
            idle: self.idle.load(Ordering::SeqCst),
        }
    }
}

/// Immutable snapshot of worker statistics.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct WorkerStatsSnapshot {
    pub total: u32,
    pub active: u32,
    pub idle: u32,
}

/// Handle to worker pool statistics. Cloneable so it can be stored in AppState.
/// Use this to query worker stats without needing the full WorkerHandle.
#[derive(Clone)]
pub struct WorkerStatsHandle {
    stats: Arc<WorkerStats>,
}

impl WorkerStatsHandle {
    /// Get a snapshot of current worker statistics.
    pub fn stats(&self) -> WorkerStatsSnapshot {
        self.stats.snapshot()
    }
}

/// Configuration for the worker pool.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of concurrent worker tasks.
    pub concurrency: usize,
    /// Maximum time to wait for a new task before checking again.
    pub poll_interval: Duration,
    /// Whether to respect the task's retry policy on nack.
    pub enable_retries: bool,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            poll_interval: Duration::from_secs(1),
            enable_retries: true,
        }
    }
}

impl WorkerConfig {
    /// Set the number of concurrent workers.
    pub fn with_concurrency(mut self, n: usize) -> Self {
        self.concurrency = n;
        self
    }

    /// Set the poll interval.
    pub fn with_poll_interval(mut self, d: Duration) -> Self {
        self.poll_interval = d;
        self
    }
}

/// Handle to a running worker pool. Drop to trigger shutdown.
pub struct WorkerHandle {
    handles: Vec<JoinHandle<()>>,
    shutdown_flag: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
    /// Shared worker statistics for health monitoring.
    stats: Arc<WorkerStats>,
}

impl WorkerHandle {
    /// Signal all workers to shut down gracefully.
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
    }

    /// Wait for all workers to finish.
    pub async fn join(self) {
        for handle in self.handles {
            let _ = handle.await;
        }
    }

    /// Shut down and wait for completion.
    pub async fn shutdown_and_join(self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_waiters();
        for handle in self.handles {
            let _ = handle.await;
        }
    }

    /// Get a snapshot of current worker statistics.
    pub fn stats(&self) -> WorkerStatsSnapshot {
        self.stats.snapshot()
    }
}

/// A pool of workers that consume tasks from a queue and send notifications.
pub struct WorkerPool;

impl WorkerPool {
    /// Start a worker pool that processes tasks from the given queue.
    ///
    /// Each worker loops: dequeue → check circuit breaker → send via provider → ack/nack.
    ///
    /// Returns `(WorkerHandle, WorkerStatsHandle)`. Use `WorkerHandle` for shutdown
    /// and `WorkerStatsHandle` for querying worker statistics.
    pub fn start(
        queue: Arc<dyn QueueBackend>,
        registry: Arc<ProviderRegistry>,
        circuit_breakers: Arc<CircuitBreakerRegistry>,
        config: WorkerConfig,
        task_notify: Arc<Notify>,
    ) -> (WorkerHandle, WorkerStatsHandle) {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_notify = Arc::new(Notify::new());
        let stats = Arc::new(WorkerStats {
            total: config.concurrency as u32,
            ..Default::default()
        });
        let mut handles = Vec::with_capacity(config.concurrency);

        // Initialize all workers as idle
        for _ in 0..config.concurrency {
            stats.idle.fetch_add(1, Ordering::SeqCst);
        }

        for worker_id in 0..config.concurrency {
            let queue = queue.clone();
            let registry = registry.clone();
            let circuit_breakers = circuit_breakers.clone();
            let config = config.clone();
            let shutdown_flag = shutdown_flag.clone();
            let shutdown_notify = shutdown_notify.clone();
            let task_notify = task_notify.clone();
            let stats = stats.clone();

            let handle = tokio::spawn(async move {
                tracing::info!(worker_id, "queue worker started");

                loop {
                    // Check shutdown flag first
                    if shutdown_flag.load(Ordering::SeqCst) {
                        tracing::info!(worker_id, "queue worker shutting down");
                        break;
                    }

                    match queue.dequeue().await {
                        Ok(Some(task)) => {
                            stats.mark_active();
                            let task_id = task.id.clone();
                            let provider_name = task.provider.clone();
                            let has_callback = task.callback_url.is_some();

                            let span = tracing::info_span!(
                                "process_task",
                                worker_id,
                                task_id = %task_id,
                                provider = %provider_name,
                                attempt = task.attempts
                            );
                            let _guard = span.enter();

                            tracing::debug!("processing task");

                            // Look up the provider
                            let provider = match registry.get_by_name(&provider_name) {
                                Some(p) => p,
                                None => {
                                    let err = format!("provider '{}' not found", provider_name);
                                    tracing::error!(worker_id, task_id = %task_id, %err);
                                    if let Err(nack_err) = queue.nack(&task_id, &err).await {
                                        tracing::error!(
                                            worker_id,
                                            task_id = %task_id,
                                            error = %nack_err,
                                            "failed to nack task after provider-not-found"
                                        );
                                    }
                                    // Fire callback if task reached terminal state
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
                                    continue;
                                }
                            };

                            // Get circuit breaker for this provider
                            let circuit = circuit_breakers.get_or_create(&provider_name);

                            // Check circuit breaker before sending
                            if circuit.is_open() {
                                let err = format!(
                                    "circuit breaker open for provider '{}' (fast-fail)",
                                    provider_name
                                );
                                tracing::warn!(
                                    worker_id,
                                    task_id = %task_id,
                                    provider = %provider_name,
                                    "circuit breaker open, failing fast"
                                );
                                if let Err(nack_err) = queue.nack(&task_id, &err).await {
                                    tracing::error!(
                                        worker_id,
                                        task_id = %task_id,
                                        error = %nack_err,
                                        "failed to nack task after circuit breaker open"
                                    );
                                }
                                if has_callback {
                                    if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                        fire_callback(&updated).await;
                                    }
                                }
                                continue;
                            }

                            // Send the notification
                            let send_span = tracing::info_span!(
                                "provider.send",
                                provider = %provider_name,
                                task_id = %task_id
                            );
                            let send_result = async {
                                provider.send(&task.message, &task.config).await
                            }
                            .instrument(send_span)
                            .await;
                            match send_result {
                                Ok(resp) if resp.success => {
                                    circuit.record_success();
                                    tracing::info!(
                                        worker_id,
                                        task_id = %task_id,
                                        provider = %provider_name,
                                        "task completed successfully"
                                    );
                                    if let Err(ack_err) = queue.ack(&task_id).await {
                                        tracing::error!(
                                            worker_id,
                                            task_id = %task_id,
                                            error = %ack_err,
                                            "failed to ack completed task"
                                        );
                                    }
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
                                }
                                Ok(resp) => {
                                    circuit.record_failure();
                                    tracing::warn!(
                                        worker_id,
                                        task_id = %task_id,
                                        provider = %provider_name,
                                        message = %resp.message,
                                        "provider returned failure response"
                                    );
                                    if let Err(nack_err) = queue.nack(&task_id, &resp.message).await {
                                        tracing::error!(
                                            worker_id,
                                            task_id = %task_id,
                                            error = %nack_err,
                                            "failed to nack task after provider failure"
                                        );
                                    }
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    circuit.record_failure();
                                    tracing::warn!(
                                        worker_id,
                                        task_id = %task_id,
                                        provider = %provider_name,
                                        error = %e,
                                        "task send failed"
                                    );
                                    if let Err(nack_err) =
                                        queue.nack(&task_id, &e.to_string()).await
                                    {
                                        tracing::error!(
                                            worker_id,
                                            task_id = %task_id,
                                            error = %nack_err,
                                            "failed to nack task after send error"
                                        );
                                    }
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
                                    // Mark worker as idle after completing task
                                    stats.mark_idle();
                                }
                            }
                        }
                        Ok(None) => {
                            // Queue empty — wait for new task or shutdown
                            tokio::select! {
                                biased;
                                _ = shutdown_notify.notified() => {
                                    // Shutdown signalled
                                }
                                _ = task_notify.notified() => {
                                    // New task may be available
                                }
                                _ = tokio::time::sleep(config.poll_interval) => {
                                    // Periodic re-check
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(worker_id, error = %e, "queue dequeue error");
                            tokio::time::sleep(config.poll_interval).await;
                        }
                    }
                }

                tracing::info!(worker_id, "queue worker stopped");
            });

            handles.push(handle);
        }

        let worker_handle = WorkerHandle {
            handles,
            shutdown_flag,
            shutdown_notify,
            stats: stats.clone(),
        };
        let stats_handle = WorkerStatsHandle { stats };
        (worker_handle, stats_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::InMemoryQueue;
    use crate::task::{NotificationTask, TaskStatus};

    use async_trait::async_trait;
    use noti_core::{
        CircuitBreakerRegistry, CircuitState, Message, NotiError, NotifyProvider, ParamDef,
        Priority, ProviderConfig, SendResponse,
    };
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

    struct MockOkProvider {
        call_count: AtomicU32,
    }

    impl MockOkProvider {
        fn new() -> Self {
            Self {
                call_count: AtomicU32::new(0),
            }
        }

        fn calls(&self) -> u32 {
            self.call_count.load(AtomicOrdering::SeqCst)
        }
    }

    #[async_trait]
    impl NotifyProvider for MockOkProvider {
        fn name(&self) -> &str {
            "mock-ok"
        }
        fn url_scheme(&self) -> &str {
            "mock-ok"
        }
        fn params(&self) -> Vec<ParamDef> {
            vec![]
        }
        fn description(&self) -> &str {
            "always succeeds"
        }
        fn example_url(&self) -> &str {
            "mock-ok://test"
        }

        async fn send(
            &self,
            _message: &Message,
            _config: &ProviderConfig,
        ) -> Result<SendResponse, NotiError> {
            self.call_count.fetch_add(1, AtomicOrdering::SeqCst);
            Ok(SendResponse::success("mock-ok", "ok"))
        }
    }

    struct MockFailProvider;

    #[async_trait]
    impl NotifyProvider for MockFailProvider {
        fn name(&self) -> &str {
            "mock-fail"
        }
        fn url_scheme(&self) -> &str {
            "mock-fail"
        }
        fn params(&self) -> Vec<ParamDef> {
            vec![]
        }
        fn description(&self) -> &str {
            "always fails"
        }
        fn example_url(&self) -> &str {
            "mock-fail://test"
        }

        async fn send(
            &self,
            _message: &Message,
            _config: &ProviderConfig,
        ) -> Result<SendResponse, NotiError> {
            Err(NotiError::Network("simulated failure".into()))
        }
    }

    #[tokio::test]
    async fn test_worker_processes_task() {
        let queue = Arc::new(InMemoryQueue::new());
        let task_notify = queue.notifier();

        let provider = Arc::new(MockOkProvider::new());
        let mut registry = ProviderRegistry::new();
        registry.register(provider.clone());
        let registry = Arc::new(registry);

        let msg = Message::text("hello").with_priority(Priority::Normal);
        let task = NotificationTask::new("mock-ok", ProviderConfig::new(), msg);
        let task_id = task.id.clone();

        queue.enqueue(task).await.unwrap();

        let config = WorkerConfig::default()
            .with_concurrency(1)
            .with_poll_interval(Duration::from_millis(50));

        let (handle, _stats_handle) = WorkerPool::start(
            queue.clone(),
            registry,
            Arc::new(CircuitBreakerRegistry::new()),
            config,
            task_notify,
        );

        // Wait for the task to be processed
        tokio::time::sleep(Duration::from_millis(200)).await;

        let task = queue.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(provider.calls() >= 1);

        handle.shutdown_and_join().await;
    }

    #[tokio::test]
    async fn test_worker_nacks_failed_task() {
        let queue = Arc::new(InMemoryQueue::new());
        let task_notify = queue.notifier();

        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockFailProvider));
        let registry = Arc::new(registry);

        let msg = Message::text("hello");
        let task = NotificationTask::new("mock-fail", ProviderConfig::new(), msg)
            .with_retry_policy(noti_core::RetryPolicy::none());
        let task_id = task.id.clone();

        queue.enqueue(task).await.unwrap();

        let config = WorkerConfig::default()
            .with_concurrency(1)
            .with_poll_interval(Duration::from_millis(50));

        let (handle, _stats_handle) = WorkerPool::start(
            queue.clone(),
            registry,
            Arc::new(CircuitBreakerRegistry::new()),
            config,
            task_notify,
        );

        tokio::time::sleep(Duration::from_millis(200)).await;

        let task = queue.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);

        handle.shutdown_and_join().await;
    }

    #[tokio::test]
    async fn test_worker_shutdown() {
        let queue = Arc::new(InMemoryQueue::new());
        let task_notify = queue.notifier();

        let registry = Arc::new(ProviderRegistry::new());

        let config = WorkerConfig::default()
            .with_concurrency(2)
            .with_poll_interval(Duration::from_millis(50));

        let (handle, _stats_handle) = WorkerPool::start(
            queue,
            registry,
            Arc::new(CircuitBreakerRegistry::new()),
            config,
            task_notify,
        );

        // Should shut down cleanly within reasonable time
        handle.shutdown_and_join().await;
    }

    #[tokio::test]
    async fn test_worker_unknown_provider() {
        let queue = Arc::new(InMemoryQueue::new());
        let task_notify = queue.notifier();

        let registry = Arc::new(ProviderRegistry::new()); // empty registry

        let msg = Message::text("hello");
        let task = NotificationTask::new("nonexistent", ProviderConfig::new(), msg)
            .with_retry_policy(noti_core::RetryPolicy::none());
        let task_id = task.id.clone();

        queue.enqueue(task).await.unwrap();

        let config = WorkerConfig::default()
            .with_concurrency(1)
            .with_poll_interval(Duration::from_millis(50));

        let (handle, _stats_handle) = WorkerPool::start(
            queue.clone(),
            registry,
            Arc::new(CircuitBreakerRegistry::new()),
            config,
            task_notify,
        );

        tokio::time::sleep(Duration::from_millis(200)).await;

        let task = queue.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);

        handle.shutdown_and_join().await;
    }

    #[tokio::test]
    async fn test_worker_circuit_breaker_open() {
        use std::time::Duration;

        let queue = Arc::new(InMemoryQueue::new());
        let task_notify = queue.notifier();

        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(MockFailProvider));
        let registry = Arc::new(registry);

        // Create a circuit breaker registry with a very low threshold
        let circuit_registry = Arc::new(CircuitBreakerRegistry::new());
        // Pre-open the circuit for mock-fail
        let cb = circuit_registry.get_or_create("mock-fail");
        cb.force_state(CircuitState::Open);

        let msg = Message::text("hello");
        let task = NotificationTask::new("mock-fail", ProviderConfig::new(), msg)
            .with_retry_policy(noti_core::RetryPolicy::none());
        let task_id = task.id.clone();

        queue.enqueue(task).await.unwrap();

        let config = WorkerConfig::default()
            .with_concurrency(1)
            .with_poll_interval(Duration::from_millis(50));

        let (handle, _stats_handle) = WorkerPool::start(
            queue.clone(),
            registry,
            circuit_registry,
            config,
            task_notify,
        );

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Task should have been nacked due to open circuit breaker
        let task = queue.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);

        handle.shutdown_and_join().await;
    }
}
