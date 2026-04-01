use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify};

use crate::error::QueueError;
use crate::queue::{QueueBackend, QueueStats};
use crate::task::{NotificationTask, TaskId, TaskStatus};

/// Wrapper for priority-based ordering in the binary heap.
///
/// Higher priority tasks are dequeued first. Within the same priority,
/// earlier-created tasks are dequeued first (FIFO within priority level).
struct PriorityEntry {
    task: NotificationTask,
}

impl PartialEq for PriorityEntry {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

impl Eq for PriorityEntry {}

impl PartialOrd for PriorityEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        let pri_cmp = self
            .task
            .priority()
            .as_numeric()
            .cmp(&other.task.priority().as_numeric());
        match pri_cmp {
            Ordering::Equal => {
                // Earlier created first (reverse: older = higher priority in heap)
                other.task.created_at.cmp(&self.task.created_at)
            }
            other_ord => other_ord,
        }
    }
}

/// In-memory queue backend using a priority heap.
///
/// Suitable for single-process deployments and testing.
/// Tasks are lost if the process crashes (no persistence).
pub struct InMemoryQueue {
    /// Priority-ordered queue of pending tasks.
    heap: Mutex<BinaryHeap<PriorityEntry>>,
    /// All tasks indexed by ID (for lookup, ack, nack).
    tasks: Mutex<HashMap<TaskId, NotificationTask>>,
    /// Notifier for waking blocked dequeue calls.
    notify: Arc<Notify>,
    /// Maximum queue capacity (0 = unlimited).
    capacity: usize,
    /// Aggregate counters.
    counters: Mutex<QueueStats>,
}

impl InMemoryQueue {
    /// Create a new in-memory queue with unlimited capacity.
    pub fn new() -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            tasks: Mutex::new(HashMap::new()),
            notify: Arc::new(Notify::new()),
            capacity: 0,
            counters: Mutex::new(QueueStats::default()),
        }
    }

    /// Create a new in-memory queue with a maximum capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            tasks: Mutex::new(HashMap::new()),
            notify: Arc::new(Notify::new()),
            capacity,
            counters: Mutex::new(QueueStats::default()),
        }
    }

    /// Get a clone of the notifier for external use (e.g. workers).
    pub fn notifier(&self) -> Arc<Notify> {
        self.notify.clone()
    }
}

impl Default for InMemoryQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QueueBackend for InMemoryQueue {
    async fn enqueue(&self, task: NotificationTask) -> Result<TaskId, QueueError> {
        let id = task.id.clone();

        if self.capacity > 0 {
            let heap = self.heap.lock().await;
            if heap.len() >= self.capacity {
                return Err(QueueError::QueueFull {
                    capacity: self.capacity,
                    current: heap.len(),
                });
            }
            drop(heap);
        }

        let mut tasks = self.tasks.lock().await;
        tasks.insert(id.clone(), task.clone());
        drop(tasks);

        let mut heap = self.heap.lock().await;
        heap.push(PriorityEntry { task });
        drop(heap);

        let mut counters = self.counters.lock().await;
        counters.queued += 1;
        drop(counters);

        // Wake any blocked dequeue call
        self.notify.notify_one();

        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<NotificationTask>, QueueError> {
        let mut heap = self.heap.lock().await;
        // Skip cancelled (or otherwise non-queued) tasks that remain in the heap
        // after being cancelled via the `cancel()` method.
        while let Some(entry) = heap.pop() {
            let task_id = &entry.task.id;

            let tasks = self.tasks.lock().await;
            let current_status = tasks.get(task_id).map(|t| t.status.clone());
            drop(tasks);

            if current_status != Some(TaskStatus::Queued) {
                // Task was cancelled (or otherwise modified) after enqueue — skip it.
                continue;
            }

            let mut task = entry.task;
            task.mark_processing();

            let mut tasks = self.tasks.lock().await;
            tasks.insert(task.id.clone(), task.clone());
            drop(tasks);

            let mut counters = self.counters.lock().await;
            counters.queued = counters.queued.saturating_sub(1);
            counters.processing += 1;
            drop(counters);

            return Ok(Some(task));
        }
        Ok(None)
    }

    async fn ack(&self, task_id: &str) -> Result<(), QueueError> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| QueueError::NotFound(task_id.to_string()))?;

        task.mark_completed();

        let mut counters = self.counters.lock().await;
        counters.processing = counters.processing.saturating_sub(1);
        counters.completed += 1;

        Ok(())
    }

    async fn nack(&self, task_id: &str, error: &str) -> Result<(), QueueError> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| QueueError::NotFound(task_id.to_string()))?;

        if task.should_retry() {
            // Re-queue for retry
            task.status = TaskStatus::Queued;
            task.last_error = Some(error.to_string());
            task.updated_at = std::time::SystemTime::now();

            let requeue_task = task.clone();
            drop(tasks);

            let mut heap = self.heap.lock().await;
            heap.push(PriorityEntry { task: requeue_task });
            drop(heap);

            let mut counters = self.counters.lock().await;
            counters.processing = counters.processing.saturating_sub(1);
            counters.queued += 1;
            drop(counters);

            // Wake workers for retry
            self.notify.notify_one();
        } else {
            task.mark_failed(error);
            drop(tasks);

            let mut counters = self.counters.lock().await;
            counters.processing = counters.processing.saturating_sub(1);
            counters.failed += 1;
        }

        Ok(())
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<NotificationTask>, QueueError> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.get(task_id).cloned())
    }

    async fn cancel(&self, task_id: &str) -> Result<bool, QueueError> {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.get_mut(task_id) {
            if task.status == TaskStatus::Queued {
                task.mark_cancelled();

                let mut counters = self.counters.lock().await;
                counters.queued = counters.queued.saturating_sub(1);
                counters.cancelled += 1;

                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn stats(&self) -> Result<QueueStats, QueueError> {
        let counters = self.counters.lock().await;
        Ok(counters.clone())
    }

    async fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: usize,
    ) -> Result<Vec<NotificationTask>, QueueError> {
        let tasks = self.tasks.lock().await;
        let iter = tasks
            .values()
            .filter(|t| status.as_ref().is_none_or(|s| t.status == *s));

        let mut result: Vec<_> = iter.cloned().collect();
        result.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        result.truncate(limit);
        Ok(result)
    }

    async fn purge_completed(&self) -> Result<usize, QueueError> {
        let mut tasks = self.tasks.lock().await;
        let before = tasks.len();
        tasks.retain(|_, t| !t.is_terminal());
        let purged = before - tasks.len();
        drop(tasks);

        // Decrement counters so stats() reflects actual remaining tasks,
        // matching SqliteQueue semantics where purge DELETEs rows.
        if purged > 0 {
            let mut counters = self.counters.lock().await;
            counters.completed = 0;
            counters.failed = 0;
            counters.cancelled = 0;
        }

        Ok(purged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noti_core::{Message, Priority, ProviderConfig, RetryPolicy};
    use std::time::Duration;

    fn make_task(provider: &str, priority: Priority) -> NotificationTask {
        let msg = Message::text("test").with_priority(priority);
        NotificationTask::new(provider, ProviderConfig::new(), msg)
    }

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let queue = InMemoryQueue::new();

        let task = make_task("slack", Priority::Normal);
        let id = queue.enqueue(task).await.unwrap();

        let dequeued = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(dequeued.id, id);
        assert_eq!(dequeued.status, TaskStatus::Processing);
        assert_eq!(dequeued.attempts, 1);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = InMemoryQueue::new();

        let low = make_task("low", Priority::Low);
        let urgent = make_task("urgent", Priority::Urgent);
        let normal = make_task("normal", Priority::Normal);
        let high = make_task("high", Priority::High);

        queue.enqueue(low).await.unwrap();
        queue.enqueue(urgent).await.unwrap();
        queue.enqueue(normal).await.unwrap();
        queue.enqueue(high).await.unwrap();

        let t1 = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(t1.provider, "urgent");

        let t2 = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(t2.provider, "high");

        let t3 = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(t3.provider, "normal");

        let t4 = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(t4.provider, "low");
    }

    #[tokio::test]
    async fn test_ack() {
        let queue = InMemoryQueue::new();
        let task = make_task("slack", Priority::Normal);
        let id = queue.enqueue(task).await.unwrap();

        queue.dequeue().await.unwrap();
        queue.ack(&id).await.unwrap();

        let task = queue.get_task(&id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Completed);

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.processing, 0);
    }

    #[tokio::test]
    async fn test_nack_with_retry() {
        let queue = InMemoryQueue::new();
        let task = make_task("slack", Priority::Normal)
            .with_retry_policy(RetryPolicy::fixed(3, Duration::from_millis(1)));
        let id = queue.enqueue(task).await.unwrap();

        // First dequeue + nack
        queue.dequeue().await.unwrap();
        queue.nack(&id, "timeout").await.unwrap();

        // Task should be re-queued
        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.queued, 1);
        assert_eq!(stats.processing, 0);

        // Dequeue again
        let task = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(task.attempts, 2);
    }

    #[tokio::test]
    async fn test_nack_exhausted_retries() {
        let queue = InMemoryQueue::new();
        let task = make_task("slack", Priority::Normal).with_retry_policy(RetryPolicy::none());
        let id = queue.enqueue(task).await.unwrap();

        queue.dequeue().await.unwrap();
        queue.nack(&id, "permanent failure").await.unwrap();

        let task = queue.get_task(&id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Failed);

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.failed, 1);
    }

    #[tokio::test]
    async fn test_cancel() {
        let queue = InMemoryQueue::new();
        let task = make_task("slack", Priority::Normal);
        let id = queue.enqueue(task).await.unwrap();

        let cancelled = queue.cancel(&id).await.unwrap();
        assert!(cancelled);

        let task = queue.get_task(&id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.cancelled, 1);
    }

    #[tokio::test]
    async fn test_cancel_processing_task() {
        let queue = InMemoryQueue::new();
        let task = make_task("slack", Priority::Normal);
        let id = queue.enqueue(task).await.unwrap();

        queue.dequeue().await.unwrap(); // now Processing

        let cancelled = queue.cancel(&id).await.unwrap();
        assert!(!cancelled); // cannot cancel a processing task
    }

    #[tokio::test]
    async fn test_capacity_limit() {
        let queue = InMemoryQueue::with_capacity(2);

        queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        queue
            .enqueue(make_task("b", Priority::Normal))
            .await
            .unwrap();

        let result = queue.enqueue(make_task("c", Priority::Normal)).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueueError::QueueFull { .. }));
    }

    #[tokio::test]
    async fn test_dequeue_empty() {
        let queue = InMemoryQueue::new();
        let result = queue.dequeue().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let queue = InMemoryQueue::new();

        queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        queue.enqueue(make_task("b", Priority::High)).await.unwrap();

        let all = queue.list_tasks(None, 100).await.unwrap();
        assert_eq!(all.len(), 2);

        let queued = queue
            .list_tasks(Some(TaskStatus::Queued), 100)
            .await
            .unwrap();
        assert_eq!(queued.len(), 2);
    }

    #[tokio::test]
    async fn test_purge_completed() {
        let queue = InMemoryQueue::new();

        let id1 = queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        let id2 = queue
            .enqueue(make_task("b", Priority::Normal))
            .await
            .unwrap();

        queue.dequeue().await.unwrap(); // dequeue a
        queue.ack(&id1).await.unwrap();
        // b is still queued but gets dequeued too by the heap
        queue.dequeue().await.unwrap();
        queue.nack(&id2, "fail").await.unwrap(); // b fails with no retry (default has retries)

        // Only completed/failed/cancelled are purged
        let purged = queue.purge_completed().await.unwrap();
        assert!(purged >= 1); // at least id1 (completed)
    }

    #[tokio::test]
    async fn test_purge_completed_resets_stats_counters() {
        let queue = InMemoryQueue::new();

        // Create three tasks: one completed, one failed, one cancelled
        let id_a = queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        let id_b = queue
            .enqueue(make_task("b", Priority::Normal).with_retry_policy(RetryPolicy::none()))
            .await
            .unwrap();
        let id_c = queue
            .enqueue(make_task("c", Priority::Normal))
            .await
            .unwrap();
        queue
            .enqueue(make_task("d", Priority::Normal))
            .await
            .unwrap();

        // a → completed
        queue.dequeue().await.unwrap();
        queue.ack(&id_a).await.unwrap();

        // b → failed (no retry)
        queue.dequeue().await.unwrap();
        queue.nack(&id_b, "error").await.unwrap();

        // c → cancelled
        queue.cancel(&id_c).await.unwrap();

        // Before purge: terminal counters should be non-zero
        let stats_before = queue.stats().await.unwrap();
        assert_eq!(stats_before.completed, 1);
        assert_eq!(stats_before.failed, 1);
        assert_eq!(stats_before.cancelled, 1);
        assert_eq!(stats_before.queued, 1); // d still queued

        // Purge terminal tasks
        let purged = queue.purge_completed().await.unwrap();
        assert_eq!(purged, 3);

        // After purge: terminal counters reset to 0, non-terminal unchanged
        let stats_after = queue.stats().await.unwrap();
        assert_eq!(stats_after.completed, 0);
        assert_eq!(stats_after.failed, 0);
        assert_eq!(stats_after.cancelled, 0);
        assert_eq!(stats_after.queued, 1); // d still queued
        assert_eq!(stats_after.processing, 0);
    }

    #[tokio::test]
    async fn test_stats() {
        let queue = InMemoryQueue::new();

        queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        queue.enqueue(make_task("b", Priority::High)).await.unwrap();

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.queued, 2);
        assert_eq!(stats.processing, 0);
    }

    #[tokio::test]
    async fn test_get_nonexistent_task() {
        let queue = InMemoryQueue::new();
        let result = queue.get_task("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_dequeue_skips_cancelled_tasks() {
        let queue = InMemoryQueue::new();

        let id_a = queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        let _id_b = queue
            .enqueue(make_task("b", Priority::Normal))
            .await
            .unwrap();

        // Cancel task a while it is still in the heap
        let cancelled = queue.cancel(&id_a).await.unwrap();
        assert!(cancelled);

        // Dequeue should skip 'a' (cancelled) and return 'b'
        let task = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(task.provider, "b");
        assert_eq!(task.status, TaskStatus::Processing);

        // Stats: a was cancelled (queued->cancelled), b was dequeued (queued->processing)
        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.queued, 0);
        assert_eq!(stats.processing, 1);
        assert_eq!(stats.cancelled, 1);
    }

    #[tokio::test]
    async fn test_dequeue_all_cancelled_returns_none() {
        let queue = InMemoryQueue::new();

        let id = queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        queue.cancel(&id).await.unwrap();

        // Heap still has the entry, but dequeue should skip it and return None
        let result = queue.dequeue().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_dequeue_skips_cancelled_preserves_priority_order() {
        let queue = InMemoryQueue::new();

        // Enqueue: low, urgent, normal
        let id_low = queue
            .enqueue(make_task("low", Priority::Low))
            .await
            .unwrap();
        let id_urgent = queue
            .enqueue(make_task("urgent", Priority::Urgent))
            .await
            .unwrap();
        let _id_normal = queue
            .enqueue(make_task("normal", Priority::Normal))
            .await
            .unwrap();

        // Cancel the urgent task — dequeue should skip it
        queue.cancel(&id_urgent).await.unwrap();

        // First dequeue should return 'normal' (highest non-cancelled priority)
        let t1 = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(t1.provider, "normal");

        // Cancel low too
        queue.cancel(&id_low).await.unwrap();

        // Next dequeue should skip 'low' (cancelled) and return None
        let t2 = queue.dequeue().await.unwrap();
        assert!(t2.is_none());
    }
}
