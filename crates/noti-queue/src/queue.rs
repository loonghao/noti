use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::QueueError;
use crate::task::{DlqEntry, NotificationTask, TaskId, TaskStatus};

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

/// Statistics about the dead letter queue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DlqStats {
    /// Number of entries currently in the DLQ.
    pub dlq_size: usize,
}

/// Abstract queue backend trait for pluggable implementations.
///
/// Implementations can range from simple in-memory queues to distributed
/// message brokers like RabbitMQ or Kafka.
#[async_trait]
pub trait QueueBackend: Send + Sync {
    /// Enqueue a notification task for asynchronous processing.
    ///
    /// Returns the task ID on success.
    async fn enqueue(&self, task: NotificationTask) -> Result<TaskId, QueueError>;

    /// Dequeue the next highest-priority task for processing.
    ///
    /// Returns `None` if the queue is empty.
    async fn dequeue(&self) -> Result<Option<NotificationTask>, QueueError>;

    /// Acknowledge that a task has been completed successfully.
    async fn ack(&self, task_id: &str) -> Result<(), QueueError>;

    /// Negative-acknowledge: mark a task as failed.
    ///
    /// If the task is eligible for retry, it will be re-queued.
    /// Otherwise, it will be moved to the failed state.
    async fn nack(&self, task_id: &str, error: &str) -> Result<(), QueueError>;

    /// Get the current status of a task.
    async fn get_task(&self, task_id: &str) -> Result<Option<NotificationTask>, QueueError>;

    /// Cancel a queued (not yet processing) task.
    async fn cancel(&self, task_id: &str) -> Result<bool, QueueError>;

    /// Get current queue statistics.
    async fn stats(&self) -> Result<QueueStats, QueueError>;

    /// Get tasks filtered by status.
    async fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: usize,
    ) -> Result<Vec<NotificationTask>, QueueError>;

    /// Purge all completed/failed/cancelled tasks from the queue.
    async fn purge_completed(&self) -> Result<usize, QueueError>;

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
    async fn recover_stale_tasks(&self) -> Result<usize, QueueError> {
        Ok(0)
    }

    /// Move a task to the dead letter queue.
    ///
    /// Called by the worker when all retry attempts have been exhausted.
    /// The task is removed from the main queue and stored separately
    /// for later inspection or manual replay.
    async fn move_to_dlq(&self, task_id: &str, reason: &str) -> Result<(), QueueError>;

    /// List entries in the dead letter queue.
    ///
    /// Returns up to `limit` DLQ entries, ordered by when they were moved
    /// to the DLQ (newest first).
    async fn list_dlq(&self, limit: usize) -> Result<Vec<DlqEntry>, QueueError>;

    /// Get DLQ statistics.
    async fn dlq_stats(&self) -> Result<DlqStats, QueueError>;

    /// Requeue a task from the DLQ back into the main queue.
    ///
    /// The task's attempt counter is reset so it gets a fresh retry budget
    /// according to its retry policy.
    async fn requeue_from_dlq(&self, task_id: &str) -> Result<(), QueueError>;

    /// Permanently delete a task from the DLQ.
    async fn delete_from_dlq(&self, task_id: &str) -> Result<(), QueueError>;
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

    #[test]
    fn test_dlq_stats_default() {
        let stats = DlqStats::default();
        assert_eq!(stats.dlq_size, 0);
    }

    #[test]
    fn test_dlq_stats_serde() {
        let stats = DlqStats { dlq_size: 42 };
        let json = serde_json::to_string(&stats).unwrap();
        let parsed: DlqStats = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.dlq_size, 42);
    }
}
