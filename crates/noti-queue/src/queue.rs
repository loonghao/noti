use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::QueueError;
use crate::task::{NotificationTask, TaskId, TaskStatus};

/// Statistics about the current queue state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct QueueStats {
    /// Number of tasks waiting to be processed.
    pub queued: usize,
    /// Number of tasks currently being processed.
    pub processing: usize,
    /// Total tasks completed since the queue was created.
    pub completed: usize,
    /// Total tasks failed since the queue was created.
    pub failed: usize,
    /// Total tasks cancelled since the queue was created.
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
