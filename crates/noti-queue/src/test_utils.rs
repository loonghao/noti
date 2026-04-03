//! Shared test utilities for noti-queue tests.
//!
//! This module is only compiled when running tests (`#[cfg(test)]`).

use crate::task::NotificationTask;
use noti_core::{Message, Priority, ProviderConfig};

/// Create a minimal [`NotificationTask`] for testing.
///
/// The task has a fixed `"test"` message body and the given provider name
/// and priority. Useful for queue backend unit tests that only care about
/// enqueue/dequeue/ordering behavior.
pub fn make_task(provider: &str, priority: Priority) -> NotificationTask {
    let msg = Message::text("test").with_priority(priority);
    NotificationTask::new(provider, ProviderConfig::new(), msg)
}
