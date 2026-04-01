//! SQLite-backed persistent queue backend.
//!
//! Unlike [`crate::memory::InMemoryQueue`], tasks survive process restarts.
//! Priority ordering uses SQL `ORDER BY` on numeric priority (desc) and
//! creation time (asc = FIFO within the same priority level).

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rusqlite::{Connection, params};
use tokio::sync::{Mutex, Notify};

use crate::error::QueueError;
use crate::queue::{QueueBackend, QueueStats};
use crate::task::{NotificationTask, TaskId, TaskStatus};

// ───────────────────── error conversion helpers ─────────────────────

/// Extension trait to convert `rusqlite::Error` → `QueueError::Backend` via `?`.
trait SqliteResultExt<T> {
    fn backend_err(self) -> Result<T, QueueError>;
}

impl<T> SqliteResultExt<T> for Result<T, rusqlite::Error> {
    fn backend_err(self) -> Result<T, QueueError> {
        self.map_err(|e| QueueError::Backend(e.to_string()))
    }
}

/// Extension trait to convert `serde_json::Error` → `QueueError::Serialization` via `?`.
trait SerdeResultExt<T> {
    fn serde_err(self) -> Result<T, QueueError>;
}

impl<T> SerdeResultExt<T> for Result<T, serde_json::Error> {
    fn serde_err(self) -> Result<T, QueueError> {
        self.map_err(|e| QueueError::Serialization(e.to_string()))
    }
}

// ───────────────────── helpers ─────────────────────

fn system_time_to_epoch_ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as i64
}

fn epoch_ms_to_system_time(ms: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(ms.max(0) as u64)
}

fn status_to_str(s: &TaskStatus) -> &'static str {
    match s {
        TaskStatus::Queued => "queued",
        TaskStatus::Processing => "processing",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn str_to_status(s: &str) -> TaskStatus {
    match s {
        "queued" => TaskStatus::Queued,
        "processing" => TaskStatus::Processing,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "cancelled" => TaskStatus::Cancelled,
        other => {
            tracing::warn!(
                status = other,
                "unknown task status in database, defaulting to Queued"
            );
            TaskStatus::Queued
        }
    }
}

// ───────────────────── SqliteQueue ─────────────────────

/// SQLite-backed persistent queue.
pub struct SqliteQueue {
    conn: Mutex<Connection>,
    notify: Arc<Notify>,
    capacity: usize,
}

impl SqliteQueue {
    /// Open (or create) a SQLite queue backed by a file on disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, QueueError> {
        let conn = Connection::open(path).backend_err()?;
        Self::from_connection(conn, 0)
    }

    /// Create an in-memory SQLite queue (useful for testing).
    pub fn in_memory() -> Result<Self, QueueError> {
        let conn = Connection::open_in_memory().backend_err()?;
        Self::from_connection(conn, 0)
    }

    /// Open with a capacity limit.
    pub fn open_with_capacity(path: impl AsRef<Path>, capacity: usize) -> Result<Self, QueueError> {
        let conn = Connection::open(path).backend_err()?;
        Self::from_connection(conn, capacity)
    }

    fn from_connection(conn: Connection, capacity: usize) -> Result<Self, QueueError> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;",
        )
        .backend_err()?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                config_json TEXT NOT NULL,
                message_json TEXT NOT NULL,
                retry_policy_json TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'queued',
                attempts INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                metadata_json TEXT NOT NULL DEFAULT '{}',
                callback_url TEXT,
                priority INTEGER NOT NULL DEFAULT 1,
                available_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_status_priority
                ON tasks(status, priority DESC, created_at ASC);",
        )
        .backend_err()?;

        // Migration: add available_at column if upgrading from an older schema.
        let _ = conn.execute_batch("ALTER TABLE tasks ADD COLUMN available_at INTEGER;");

        Ok(Self {
            conn: Mutex::new(conn),
            notify: Arc::new(Notify::new()),
            capacity,
        })
    }

    /// Get a clone of the notifier (for worker integration).
    pub fn notifier(&self) -> Arc<Notify> {
        self.notify.clone()
    }

    fn serialize_task(task: &NotificationTask) -> Result<TaskRow, QueueError> {
        let config_json = serde_json::to_string(&task.config).serde_err()?;
        let message_json = serde_json::to_string(&task.message).serde_err()?;
        let retry_json = serde_json::to_string(&task.retry_policy).serde_err()?;
        let metadata_json = serde_json::to_string(&task.metadata).serde_err()?;

        Ok(TaskRow {
            id: task.id.clone(),
            provider: task.provider.clone(),
            config_json,
            message_json,
            retry_policy_json: retry_json,
            status: status_to_str(&task.status).to_string(),
            attempts: task.attempts as i64,
            last_error: task.last_error.clone(),
            created_at: system_time_to_epoch_ms(task.created_at),
            updated_at: system_time_to_epoch_ms(task.updated_at),
            metadata_json,
            callback_url: task.callback_url.clone(),
            priority: task.priority().as_numeric() as i64,
            available_at: task.available_at.map(system_time_to_epoch_ms),
        })
    }

    fn deserialize_task(row: &TaskRow) -> Result<NotificationTask, QueueError> {
        let config = serde_json::from_str(&row.config_json).serde_err()?;
        let message = serde_json::from_str(&row.message_json).serde_err()?;
        let retry_policy = serde_json::from_str(&row.retry_policy_json).serde_err()?;
        let metadata = serde_json::from_str(&row.metadata_json).serde_err()?;

        Ok(NotificationTask {
            id: row.id.clone(),
            provider: row.provider.clone(),
            config,
            message,
            retry_policy,
            status: str_to_status(&row.status),
            attempts: row.attempts as u32,
            last_error: row.last_error.clone(),
            created_at: epoch_ms_to_system_time(row.created_at),
            updated_at: epoch_ms_to_system_time(row.updated_at),
            metadata,
            callback_url: row.callback_url.clone(),
            available_at: row.available_at.map(epoch_ms_to_system_time),
        })
    }
}

struct TaskRow {
    id: String,
    provider: String,
    config_json: String,
    message_json: String,
    retry_policy_json: String,
    status: String,
    attempts: i64,
    last_error: Option<String>,
    created_at: i64,
    updated_at: i64,
    metadata_json: String,
    callback_url: Option<String>,
    priority: i64,
    available_at: Option<i64>,
}

impl TaskRow {
    fn from_rusqlite_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            provider: row.get("provider")?,
            config_json: row.get("config_json")?,
            message_json: row.get("message_json")?,
            retry_policy_json: row.get("retry_policy_json")?,
            status: row.get("status")?,
            attempts: row.get("attempts")?,
            last_error: row.get("last_error")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            metadata_json: row.get("metadata_json")?,
            callback_url: row.get("callback_url")?,
            priority: row.get("priority")?,
            available_at: row.get("available_at")?,
        })
    }
}

#[async_trait]
impl QueueBackend for SqliteQueue {
    async fn enqueue(&self, task: NotificationTask) -> Result<TaskId, QueueError> {
        let row = Self::serialize_task(&task)?;
        let id = row.id.clone();
        let conn = self.conn.lock().await;

        if self.capacity > 0 {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM tasks WHERE status = 'queued'",
                    [],
                    |r| r.get(0),
                )
                .backend_err()?;
            if count as usize >= self.capacity {
                return Err(QueueError::QueueFull {
                    capacity: self.capacity,
                    current: count as usize,
                });
            }
        }

        conn.execute(
            "INSERT INTO tasks (id, provider, config_json, message_json, retry_policy_json,
             status, attempts, last_error, created_at, updated_at, metadata_json, callback_url, priority, available_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                row.id, row.provider, row.config_json, row.message_json,
                row.retry_policy_json, row.status, row.attempts, row.last_error,
                row.created_at, row.updated_at, row.metadata_json, row.callback_url, row.priority,
                row.available_at
            ],
        )
        .backend_err()?;

        drop(conn);
        self.notify.notify_one();
        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<NotificationTask>, QueueError> {
        let conn = self.conn.lock().await;
        let now = system_time_to_epoch_ms(SystemTime::now());

        let result = conn.query_row(
            "SELECT * FROM tasks WHERE status = 'queued'
             AND (available_at IS NULL OR available_at <= ?1)
             ORDER BY priority DESC, created_at ASC LIMIT 1",
            params![now],
            TaskRow::from_rusqlite_row,
        );

        match result {
            Ok(row) => {
                conn.execute(
                    "UPDATE tasks SET status = 'processing', attempts = attempts + 1, updated_at = ?1
                     WHERE id = ?2",
                    params![now, row.id],
                )
                .backend_err()?;

                let mut task = Self::deserialize_task(&row)?;
                task.mark_processing();
                Ok(Some(task))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(QueueError::Backend(e.to_string())),
        }
    }

    async fn ack(&self, task_id: &str) -> Result<(), QueueError> {
        let conn = self.conn.lock().await;
        let now = system_time_to_epoch_ms(SystemTime::now());

        let updated = conn
            .execute(
                "UPDATE tasks SET status = 'completed', updated_at = ?1 WHERE id = ?2",
                params![now, task_id],
            )
            .backend_err()?;

        if updated == 0 {
            return Err(QueueError::NotFound(task_id.to_string()));
        }
        Ok(())
    }

    async fn nack(&self, task_id: &str, error: &str) -> Result<(), QueueError> {
        let conn = self.conn.lock().await;
        let now = system_time_to_epoch_ms(SystemTime::now());

        // Read the task to check retry eligibility
        let row = conn
            .query_row(
                "SELECT * FROM tasks WHERE id = ?1",
                params![task_id],
                TaskRow::from_rusqlite_row,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => QueueError::NotFound(task_id.to_string()),
                _ => QueueError::Backend(e.to_string()),
            })?;

        let task = Self::deserialize_task(&row)?;

        if task.should_retry() {
            let delay = task.retry_delay();
            let available_at: Option<i64> = if delay.is_zero() {
                None
            } else {
                Some(now + delay.as_millis() as i64)
            };

            conn.execute(
                "UPDATE tasks SET status = 'queued', last_error = ?1, updated_at = ?2, available_at = ?3 WHERE id = ?4",
                params![error, now, available_at, task_id],
            )
            .backend_err()?;

            drop(conn);
            self.notify.notify_one();
        } else {
            conn.execute(
                "UPDATE tasks SET status = 'failed', last_error = ?1, updated_at = ?2 WHERE id = ?3",
                params![error, now, task_id],
            )
            .backend_err()?;
        }

        Ok(())
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<NotificationTask>, QueueError> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT * FROM tasks WHERE id = ?1",
            params![task_id],
            TaskRow::from_rusqlite_row,
        );

        match result {
            Ok(row) => Ok(Some(Self::deserialize_task(&row)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(QueueError::Backend(e.to_string())),
        }
    }

    async fn cancel(&self, task_id: &str) -> Result<bool, QueueError> {
        let conn = self.conn.lock().await;
        let now = system_time_to_epoch_ms(SystemTime::now());

        let updated = conn
            .execute(
                "UPDATE tasks SET status = 'cancelled', updated_at = ?1
                 WHERE id = ?2 AND status = 'queued'",
                params![now, task_id],
            )
            .backend_err()?;

        Ok(updated > 0)
    }

    async fn stats(&self) -> Result<QueueStats, QueueError> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT status, COUNT(*) FROM tasks GROUP BY status")
            .backend_err()?;

        let mut stats = QueueStats::default();
        let rows = stmt
            .query_map([], |row| {
                let status: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((status, count as usize))
            })
            .backend_err()?;

        for row in rows {
            let (status, count) = row.backend_err()?;
            match status.as_str() {
                "queued" => stats.queued = count,
                "processing" => stats.processing = count,
                "completed" => stats.completed = count,
                "failed" => stats.failed = count,
                "cancelled" => stats.cancelled = count,
                _ => {}
            }
        }

        Ok(stats)
    }

    async fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        limit: usize,
    ) -> Result<Vec<NotificationTask>, QueueError> {
        let conn = self.conn.lock().await;
        let limit_i64 = limit as i64;

        let mut stmt;
        let rows: Vec<TaskRow> = if let Some(ref s) = status {
            let status_str = status_to_str(s);
            stmt = conn
                .prepare("SELECT * FROM tasks WHERE status = ?1 ORDER BY created_at ASC LIMIT ?2")
                .backend_err()?;
            stmt.query_map(params![status_str, limit_i64], TaskRow::from_rusqlite_row)
                .backend_err()?
                .collect::<Result<Vec<_>, _>>()
                .backend_err()?
        } else {
            stmt = conn
                .prepare("SELECT * FROM tasks ORDER BY created_at ASC LIMIT ?1")
                .backend_err()?;
            stmt.query_map(params![limit_i64], TaskRow::from_rusqlite_row)
                .backend_err()?
                .collect::<Result<Vec<_>, _>>()
                .backend_err()?
        };

        rows.iter().map(Self::deserialize_task).collect()
    }

    async fn purge_completed(&self) -> Result<usize, QueueError> {
        let conn = self.conn.lock().await;

        let deleted = conn
            .execute(
                "DELETE FROM tasks WHERE status IN ('completed', 'failed', 'cancelled')",
                [],
            )
            .backend_err()?;

        Ok(deleted)
    }

    async fn recover_stale_tasks(&self) -> Result<usize, QueueError> {
        let conn = self.conn.lock().await;
        let now = system_time_to_epoch_ms(SystemTime::now());

        let recovered = conn
            .execute(
                "UPDATE tasks SET status = 'queued', updated_at = ?1 WHERE status = 'processing'",
                params![now],
            )
            .backend_err()?;

        if recovered > 0 {
            drop(conn);
            self.notify.notify_waiters();
        }

        Ok(recovered)
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
    async fn test_sqlite_enqueue_dequeue() {
        let queue = SqliteQueue::in_memory().unwrap();
        let task = make_task("slack", Priority::Normal);
        let id = queue.enqueue(task).await.unwrap();

        let dequeued = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(dequeued.id, id);
        assert_eq!(dequeued.status, TaskStatus::Processing);
        assert_eq!(dequeued.attempts, 1);
    }

    #[tokio::test]
    async fn test_sqlite_priority_ordering() {
        let queue = SqliteQueue::in_memory().unwrap();

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
    async fn test_sqlite_ack() {
        let queue = SqliteQueue::in_memory().unwrap();
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
    async fn test_sqlite_nack_with_retry() {
        let queue = SqliteQueue::in_memory().unwrap();
        let task = make_task("slack", Priority::Normal)
            .with_retry_policy(RetryPolicy::fixed(3, Duration::from_millis(1)));
        let id = queue.enqueue(task).await.unwrap();

        queue.dequeue().await.unwrap();
        queue.nack(&id, "timeout").await.unwrap();

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.queued, 1);
        assert_eq!(stats.processing, 0);

        // Wait for retry backoff delay to elapse
        tokio::time::sleep(Duration::from_millis(5)).await;

        let task = queue.dequeue().await.unwrap().unwrap();
        assert_eq!(task.attempts, 2);
    }

    #[tokio::test]
    async fn test_sqlite_nack_exhausted_retries() {
        let queue = SqliteQueue::in_memory().unwrap();
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
    async fn test_sqlite_cancel() {
        let queue = SqliteQueue::in_memory().unwrap();
        let task = make_task("slack", Priority::Normal);
        let id = queue.enqueue(task).await.unwrap();

        let cancelled = queue.cancel(&id).await.unwrap();
        assert!(cancelled);

        let task = queue.get_task(&id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_sqlite_cancel_processing_task() {
        let queue = SqliteQueue::in_memory().unwrap();
        let task = make_task("slack", Priority::Normal);
        let id = queue.enqueue(task).await.unwrap();

        queue.dequeue().await.unwrap();
        let cancelled = queue.cancel(&id).await.unwrap();
        assert!(!cancelled);
    }

    #[tokio::test]
    async fn test_sqlite_capacity_limit() {
        let conn = Connection::open_in_memory().unwrap();
        let queue = SqliteQueue::from_connection(conn, 2).unwrap();

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
    async fn test_sqlite_dequeue_empty() {
        let queue = SqliteQueue::in_memory().unwrap();
        let result = queue.dequeue().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_list_tasks() {
        let queue = SqliteQueue::in_memory().unwrap();

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
    async fn test_sqlite_purge_completed() {
        let queue = SqliteQueue::in_memory().unwrap();

        let id1 = queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        queue
            .enqueue(make_task("b", Priority::Normal))
            .await
            .unwrap();

        queue.dequeue().await.unwrap();
        queue.ack(&id1).await.unwrap();

        let purged = queue.purge_completed().await.unwrap();
        assert_eq!(purged, 1);
    }

    #[tokio::test]
    async fn test_sqlite_purge_completed_resets_stats_counters() {
        let queue = SqliteQueue::in_memory().unwrap();

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
    async fn test_sqlite_stats() {
        let queue = SqliteQueue::in_memory().unwrap();

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
    async fn test_sqlite_get_nonexistent_task() {
        let queue = SqliteQueue::in_memory().unwrap();
        let result = queue.get_task("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_sqlite_persistence_roundtrip() {
        let queue = SqliteQueue::in_memory().unwrap();

        let msg = Message::text("hello world").with_priority(Priority::High);
        let config = ProviderConfig::new().set("webhook_url", "https://example.com");
        let task = NotificationTask::new("webhook", config, msg)
            .with_metadata("correlation_id", "abc-123")
            .with_callback_url("https://example.com/callback");
        let id = task.id.clone();

        queue.enqueue(task).await.unwrap();

        let loaded = queue.get_task(&id).await.unwrap().unwrap();
        assert_eq!(loaded.provider, "webhook");
        assert_eq!(loaded.priority(), Priority::High);
        assert_eq!(loaded.metadata.get("correlation_id").unwrap(), "abc-123");
        assert_eq!(
            loaded.callback_url.as_deref(),
            Some("https://example.com/callback")
        );
    }

    #[tokio::test]
    async fn test_sqlite_recover_stale_tasks() {
        let queue = SqliteQueue::in_memory().unwrap();

        // Enqueue and dequeue two tasks (they become "processing")
        let id1 = queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();
        let id2 = queue.enqueue(make_task("b", Priority::High)).await.unwrap();
        queue.enqueue(make_task("c", Priority::Low)).await.unwrap();

        queue.dequeue().await.unwrap(); // b (high priority)
        queue.dequeue().await.unwrap(); // a (normal priority)

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.processing, 2);
        assert_eq!(stats.queued, 1);

        // Simulate server restart — recover stale "processing" tasks
        let recovered = queue.recover_stale_tasks().await.unwrap();
        assert_eq!(recovered, 2);

        // All should now be queued
        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.processing, 0);
        assert_eq!(stats.queued, 3);

        // Verify recovered tasks are dequeue-able again
        let t = queue.dequeue().await.unwrap().unwrap();
        assert!(t.id == id2 || t.id == id1);
    }

    #[tokio::test]
    async fn test_sqlite_recover_stale_no_processing() {
        let queue = SqliteQueue::in_memory().unwrap();

        queue
            .enqueue(make_task("a", Priority::Normal))
            .await
            .unwrap();

        let recovered = queue.recover_stale_tasks().await.unwrap();
        assert_eq!(recovered, 0);

        let stats = queue.stats().await.unwrap();
        assert_eq!(stats.queued, 1);
    }
}
