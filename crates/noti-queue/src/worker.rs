use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::task::JoinHandle;

use noti_core::ProviderRegistry;

use crate::callback::fire_callback;
use crate::queue::QueueBackend;

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
}

/// A pool of workers that consume tasks from a queue and send notifications.
pub struct WorkerPool;

impl WorkerPool {
    /// Start a worker pool that processes tasks from the given queue.
    ///
    /// Each worker loops: dequeue → send via provider → ack/nack.
    pub fn start(
        queue: Arc<dyn QueueBackend>,
        registry: Arc<ProviderRegistry>,
        config: WorkerConfig,
        task_notify: Arc<Notify>,
    ) -> WorkerHandle {
        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let shutdown_notify = Arc::new(Notify::new());
        let mut handles = Vec::with_capacity(config.concurrency);

        for worker_id in 0..config.concurrency {
            let queue = queue.clone();
            let registry = registry.clone();
            let config = config.clone();
            let shutdown_flag = shutdown_flag.clone();
            let shutdown_notify = shutdown_notify.clone();
            let task_notify = task_notify.clone();

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
                            let task_id = task.id.clone();
                            let provider_name = task.provider.clone();
                            let has_callback = task.callback_url.is_some();

                            tracing::debug!(
                                worker_id,
                                task_id = %task_id,
                                provider = %provider_name,
                                attempt = task.attempts,
                                "processing task"
                            );

                            // Look up the provider
                            let provider = match registry.get_by_name(&provider_name) {
                                Some(p) => p,
                                None => {
                                    let err =
                                        format!("provider '{}' not found", provider_name);
                                    tracing::error!(worker_id, task_id = %task_id, %err);
                                    let _ = queue.nack(&task_id, &err).await;
                                    // Fire callback if task reached terminal state
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
                                    continue;
                                }
                            };

                            // Send the notification
                            match provider.send(&task.message, &task.config).await {
                                Ok(resp) if resp.success => {
                                    tracing::info!(
                                        worker_id,
                                        task_id = %task_id,
                                        provider = %provider_name,
                                        "task completed successfully"
                                    );
                                    let _ = queue.ack(&task_id).await;
                                    // Fire callback on success
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
                                }
                                Ok(resp) => {
                                    tracing::warn!(
                                        worker_id,
                                        task_id = %task_id,
                                        provider = %provider_name,
                                        message = %resp.message,
                                        "provider returned failure response"
                                    );
                                    let _ = queue.nack(&task_id, &resp.message).await;
                                    // Fire callback only if task reached terminal state (not retrying)
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        worker_id,
                                        task_id = %task_id,
                                        provider = %provider_name,
                                        error = %e,
                                        "task send failed"
                                    );
                                    let _ = queue.nack(&task_id, &e.to_string()).await;
                                    // Fire callback only if task reached terminal state (not retrying)
                                    if has_callback {
                                        if let Ok(Some(updated)) = queue.get_task(&task_id).await {
                                            fire_callback(&updated).await;
                                        }
                                    }
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

        WorkerHandle {
            handles,
            shutdown_flag,
            shutdown_notify,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::InMemoryQueue;
    use crate::task::{NotificationTask, TaskStatus};

    use async_trait::async_trait;
    use noti_core::{
        Message, NotiError, NotifyProvider, ParamDef, Priority, ProviderConfig, SendResponse,
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

        let handle = WorkerPool::start(queue.clone(), registry, config, task_notify);

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

        let handle = WorkerPool::start(queue.clone(), registry, config, task_notify);

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

        let handle = WorkerPool::start(queue, registry, config, task_notify);

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

        let handle = WorkerPool::start(queue.clone(), registry, config, task_notify);

        tokio::time::sleep(Duration::from_millis(200)).await;

        let task = queue.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);

        handle.shutdown_and_join().await;
    }
}
