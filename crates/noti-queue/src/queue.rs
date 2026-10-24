use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

use crate::error::QueueError;
use crate::task::{NotificationTask, TaskId, TaskStatus};

/// Statistics about the current queue state.
///
/// Counts reflect the number of tasks **currently tracked** in each state.
/// After [`QueueBackend::purge_completed`] is called, terminal-state counters
/// (completed, failed, cancelled) are reset to zero in all backends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct QueueStats {
    /// Number of tasks waiting to be processed.
    pub queued: usize,
    /// Number of tasks currently being processed.
    pub processing: usize,
    /// Number of completed tasks currently tracked.
    pub completed: usize,
    /// Number of failed tasks currently tracked.
    pub failed: usize,
    /// Number of cancelled tasks currently tracked.
    pub cancelled: usize,
}

impl QueueStats {
    /// Total tasks across all states.
    pub fn total(&self) -> usize {
        self.queued + self.processing + self.completed + self.failed + self.cancelled
    }
}

/// Abstract queue backend trait for pluggable implementations.
///
/// Implementations can range from simple in-memory queues to distributed
/// message brokers like RabbitMQ or Kafka.
///
/// Signatures preserve async-trait's boxed, Send, object-safe future ABI.
/// Implementations may keep using `#[async_trait]` with `async fn`; spelling out
/// declarations avoids a redundant generated `must_use` on current Clippy.
pub trait QueueBackend: Send + Sync {
    /// Enqueue a notification task for asynchronous processing.
    ///
    /// Returns the task ID on success.
    fn enqueue<'life0, 'async_trait>(
        &'life0 self,
        task: NotificationTask,
    ) -> Pin<Box<dyn Future<Output = Result<TaskId, QueueError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Dequeue the next highest-priority task for processing.
    ///
    /// Returns `None` if the queue is empty.
    fn dequeue<'life0, 'async_trait>(
        &'life0 self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<NotificationTask>, QueueError>> + Send + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Acknowledge that a task has been completed successfully.
    fn ack<'life0, 'life1, 'async_trait>(
        &'life0 self,
        task_id: &'life1 str,
    ) -> Pin<Box<dyn Future<Output = Result<(), QueueError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait;

    /// Negative-acknowledge: mark a task as failed.
    ///
    /// If the task is eligible for retry, it will be re-queued.
    /// Otherwise, it will be moved to the failed state.
    fn nack<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        task_id: &'life1 str,
        error: &'life2 str,
    ) -> Pin<Box<dyn Future<Output = Result<(), QueueError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        Self: 'async_trait;

    /// Get the current status of a task.
    fn get_task<'life0, 'life1, 'async_trait>(
        &'life0 self,
        task_id: &'life1 str,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Option<NotificationTask>, QueueError>> + Send + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait;

    /// Cancel a queued (not yet processing) task.
    fn cancel<'life0, 'life1, 'async_trait>(
        &'life0 self,
        task_id: &'life1 str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, QueueError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait;

    /// Get current queue statistics.
    fn stats<'life0, 'async_trait>(
        &'life0 self,
    ) -> Pin<Box<dyn Future<Output = Result<QueueStats, QueueError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Get tasks filtered by status.
    fn list_tasks<'life0, 'async_trait>(
        &'life0 self,
        status: Option<TaskStatus>,
        limit: usize,
    ) -> Pin<
        Box<dyn Future<Output = Result<Vec<NotificationTask>, QueueError>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Purge all completed/failed/cancelled tasks from the queue.
    fn purge_completed<'life0, 'async_trait>(
        &'life0 self,
    ) -> Pin<Box<dyn Future<Output = Result<usize, QueueError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait;

    /// Recover stale tasks that were left in `Processing` state.
    ///
    /// After an unclean shutdown, persistent backends may have tasks stuck
    /// in the `Processing` state. This method resets them back to `Queued`
    /// so they can be retried by workers.
    ///
    /// Returns the number of recovered tasks.
    ///
    /// The default implementation is a no-op (returns 0), which is correct
    /// for in-memory backends where all state is lost on restart.
    fn recover_stale_tasks<'life0, 'async_trait>(
        &'life0 self,
    ) -> Pin<Box<dyn Future<Output = Result<usize, QueueError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { Ok(0) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_stats_total() {
        let stats = QueueStats {
            queued: 5,
            processing: 2,
            completed: 10,
            failed: 3,
            cancelled: 1,
        };
        assert_eq!(stats.total(), 21);
    }

    #[test]
    fn test_queue_stats_default() {
        let stats = QueueStats::default();
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_queue_stats_serde() {
        let stats = QueueStats {
            queued: 1,
            processing: 2,
            completed: 3,
            failed: 4,
            cancelled: 5,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let parsed: QueueStats = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total(), stats.total());
    }
}
