//! Message queue abstraction for asynchronous notification processing.
//!
//! Provides a trait-based queue system that decouples notification submission
//! from delivery. Supports priority-based ordering, configurable workers,
//! and pluggable backends (in-memory, SQLite, Redis, RabbitMQ, Kafka, etc.).

pub mod callback;
pub mod error;
pub mod memory;
pub mod queue;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod task;
pub mod worker;

#[cfg(test)]
pub(crate) mod test_utils;

pub use callback::{CallbackPayload, fire_callback};
pub use error::QueueError;
pub use memory::InMemoryQueue;
pub use queue::{QueueBackend, QueueStats};
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteQueue;
pub use task::{NotificationTask, TaskId, TaskStatus};
pub use worker::{WorkerConfig, WorkerHandle, WorkerPool, WorkerStats, WorkerStatsSnapshot, WorkerStatsHandle};
