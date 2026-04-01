use thiserror::Error;

/// Errors that can occur in the queue system.
#[derive(Error, Debug)]
pub enum QueueError {
    /// The queue is full and cannot accept more tasks.
    #[error("queue full: capacity {capacity}, current size {current}")]
    QueueFull { capacity: usize, current: usize },

    /// The requested task was not found.
    #[error("task not found: {0}")]
    NotFound(String),

    /// The queue has been shut down and is no longer accepting tasks.
    #[error("queue shut down")]
    ShutDown,

    /// Serialization/deserialization error (for persistent backends).
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Backend-specific error.
    #[error("backend error: {0}")]
    Backend(String),

    /// Core notification error propagated from send operations.
    #[error("notification error: {0}")]
    Notification(#[from] noti_core::NotiError),
}
