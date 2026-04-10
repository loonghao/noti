use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Unique identifier for a tracked notification.
pub type NotificationId = String;

/// Current delivery status of a notification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    /// Queued for sending but not yet dispatched.
    Pending,
    /// Currently being sent (in-flight).
    Sending,
    /// Successfully delivered to the provider.
    Delivered,
    /// Delivery failed after all retry attempts.
    Failed,
    /// Delivery was cancelled before completion.
    Cancelled,
    /// Provider confirmed the message was read/received by the end user.
    Read,
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Sending => write!(f, "sending"),
            Self::Delivered => write!(f, "delivered"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Read => write!(f, "read"),
        }
    }
}

/// A timestamped event in the delivery lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StatusEvent {
    /// The status at this point.
    pub status: DeliveryStatus,
    /// When this event occurred.
    #[cfg_attr(feature = "openapi", schema(value_type = f64))]
    pub timestamp: SystemTime,
    /// Optional detail message (e.g. error reason).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Full tracking record for a single notification delivery to one provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeliveryRecord {
    /// Unique notification identifier.
    pub notification_id: NotificationId,
    /// Provider name that handled this delivery.
    pub provider: String,
    /// Current status.
    pub current_status: DeliveryStatus,
    /// Ordered history of status transitions.
    pub events: Vec<StatusEvent>,
    /// Number of send attempts made.
    pub attempts: u32,
    /// Total time from first attempt to final status.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "openapi", schema(value_type = Option<f64>))]
    pub total_duration: Option<Duration>,
    /// When the record was created.
    #[cfg_attr(feature = "openapi", schema(value_type = f64))]
    pub created_at: SystemTime,
    /// When the record was last updated.
    #[cfg_attr(feature = "openapi", schema(value_type = f64))]
    pub updated_at: SystemTime,
}

impl DeliveryRecord {
    /// Create a new delivery record in Pending state.
    pub fn new(notification_id: impl Into<String>, provider: impl Into<String>) -> Self {
        let now = SystemTime::now();
        Self {
            notification_id: notification_id.into(),
            provider: provider.into(),
            current_status: DeliveryStatus::Pending,
            events: vec![StatusEvent {
                status: DeliveryStatus::Pending,
                timestamp: now,
                detail: None,
            }],
            attempts: 0,
            total_duration: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Transition to a new status, recording the event.
    pub fn transition(&mut self, status: DeliveryStatus, detail: Option<String>) {
        let now = SystemTime::now();
        self.current_status = status.clone();
        self.events.push(StatusEvent {
            status,
            timestamp: now,
            detail,
        });
        self.updated_at = now;
    }

    /// Mark as sending (in-flight).
    pub fn mark_sending(&mut self) {
        self.transition(DeliveryStatus::Sending, None);
    }

    /// Mark as delivered with optional detail.
    pub fn mark_delivered(&mut self, detail: Option<String>) {
        self.transition(DeliveryStatus::Delivered, detail);
    }

    /// Mark as failed with an error reason.
    pub fn mark_failed(&mut self, reason: impl Into<String>) {
        self.transition(DeliveryStatus::Failed, Some(reason.into()));
    }

    /// Mark as cancelled.
    pub fn mark_cancelled(&mut self, reason: Option<String>) {
        self.transition(DeliveryStatus::Cancelled, reason);
    }

    /// Mark as read (receipt confirmed by end user).
    pub fn mark_read(&mut self) {
        self.transition(DeliveryStatus::Read, None);
    }

    /// Record an additional attempt.
    pub fn increment_attempts(&mut self) {
        self.attempts += 1;
    }

    /// Set the total duration.
    pub fn set_duration(&mut self, duration: Duration) {
        self.total_duration = Some(duration);
    }

    /// Whether the delivery has reached a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.current_status,
            DeliveryStatus::Delivered
                | DeliveryStatus::Failed
                | DeliveryStatus::Cancelled
                | DeliveryStatus::Read
        )
    }
}

/// In-memory store for tracking notification delivery statuses.
///
/// Thread-safe via `Arc<RwLock<...>>` — can be shared across async tasks.
#[derive(Debug, Clone, Default)]
pub struct StatusTracker {
    records: Arc<RwLock<HashMap<NotificationId, Vec<DeliveryRecord>>>>,
}

impl StatusTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new delivery record and return its notification ID.
    pub async fn track(
        &self,
        notification_id: impl Into<NotificationId>,
        provider: impl Into<String>,
    ) -> NotificationId {
        let id = notification_id.into();
        let record = DeliveryRecord::new(&id, provider);
        let mut store = self.records.write().await;
        store.entry(id.clone()).or_default().push(record);
        id
    }

    /// Update the status of a specific provider delivery for a notification.
    ///
    /// Enforces valid state transitions:
    /// - From any non-terminal state, any forward transition is allowed.
    /// - Terminal states (Delivered, Failed, Cancelled, Read) cannot be left.
    /// - Read can only be reached from Delivered.
    pub async fn update_status(
        &self,
        notification_id: &str,
        provider: &str,
        status: DeliveryStatus,
        detail: Option<String>,
    ) -> bool {
        let mut store = self.records.write().await;
        if let Some(records) = store.get_mut(notification_id) {
            if let Some(record) = records.iter_mut().find(|r| r.provider == provider) {
                // Enforce state machine: terminal states cannot be left
                if record.is_terminal() {
                    return false;
                }
                // Enforce: Read can only come after Delivered
                if status == DeliveryStatus::Read && record.current_status != DeliveryStatus::Delivered {
                    return false;
                }
                record.transition(status, detail);
                return true;
            }
        }
        false
    }

    /// Get all delivery records for a notification.
    pub async fn get_records(&self, notification_id: &str) -> Vec<DeliveryRecord> {
        let store = self.records.read().await;
        store.get(notification_id).cloned().unwrap_or_default()
    }

    /// Get the delivery record for a specific notification + provider combination.
    pub async fn get_record(
        &self,
        notification_id: &str,
        provider: &str,
    ) -> Option<DeliveryRecord> {
        let store = self.records.read().await;
        store
            .get(notification_id)
            .and_then(|records| records.iter().find(|r| r.provider == provider).cloned())
    }

    /// Get all tracked notification IDs.
    pub async fn list_ids(&self) -> Vec<NotificationId> {
        let store = self.records.read().await;
        store.keys().cloned().collect()
    }

    /// Remove all records for a notification.
    pub async fn remove(&self, notification_id: &str) -> bool {
        let mut store = self.records.write().await;
        store.remove(notification_id).is_some()
    }

    /// Count total tracked notifications.
    pub async fn count(&self) -> usize {
        let store = self.records.read().await;
        store.len()
    }

    /// Remove all notifications where **every** delivery record has reached a
    /// terminal state (Delivered, Failed, Cancelled, or Read).
    ///
    /// Returns the number of notifications purged.
    pub async fn purge_terminal(&self) -> usize {
        let mut store = self.records.write().await;
        let before = store.len();
        store.retain(|_, records| !records.iter().all(|r| r.is_terminal()));
        before - store.len()
    }

    /// Remove all notifications whose **most recent update** is older than
    /// `max_age`, but only if every delivery record in that notification has
    /// reached a terminal state.
    ///
    /// This prevents unbounded memory growth in long-running servers while
    /// preserving records that are still in-flight or recently updated.
    ///
    /// Returns the number of notifications purged.
    pub async fn purge_older_than(&self, max_age: Duration) -> usize {
        let cutoff = SystemTime::now()
            .checked_sub(max_age)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut store = self.records.write().await;
        let before = store.len();
        store.retain(|_, records| {
            // Keep if any record is non-terminal
            if !records.iter().all(|r| r.is_terminal()) {
                return true;
            }
            // Keep if any record was updated after the cutoff
            records.iter().any(|r| r.updated_at > cutoff)
        });
        before - store.len()
    }

    /// Get a summary of all delivery statuses across all tracked notifications.
    pub async fn summary(&self) -> StatusSummary {
        let store = self.records.read().await;
        let mut summary = StatusSummary::default();
        for records in store.values() {
            for record in records {
                match record.current_status {
                    DeliveryStatus::Pending => summary.pending += 1,
                    DeliveryStatus::Sending => summary.sending += 1,
                    DeliveryStatus::Delivered => summary.delivered += 1,
                    DeliveryStatus::Failed => summary.failed += 1,
                    DeliveryStatus::Cancelled => summary.cancelled += 1,
                    DeliveryStatus::Read => summary.read += 1,
                }
            }
        }
        summary
    }
}

/// Aggregate counts of delivery statuses.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct StatusSummary {
    pub pending: usize,
    pub sending: usize,
    pub delivered: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub read: usize,
}

impl StatusSummary {
    /// Total number of records across all statuses.
    pub fn total(&self) -> usize {
        self.pending + self.sending + self.delivered + self.failed + self.cancelled + self.read
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_status_display() {
        assert_eq!(DeliveryStatus::Pending.to_string(), "pending");
        assert_eq!(DeliveryStatus::Sending.to_string(), "sending");
        assert_eq!(DeliveryStatus::Delivered.to_string(), "delivered");
        assert_eq!(DeliveryStatus::Failed.to_string(), "failed");
        assert_eq!(DeliveryStatus::Cancelled.to_string(), "cancelled");
        assert_eq!(DeliveryStatus::Read.to_string(), "read");
    }

    #[test]
    fn test_delivery_record_new() {
        let record = DeliveryRecord::new("notif-1", "slack");
        assert_eq!(record.notification_id, "notif-1");
        assert_eq!(record.provider, "slack");
        assert_eq!(record.current_status, DeliveryStatus::Pending);
        assert_eq!(record.events.len(), 1);
        assert_eq!(record.attempts, 0);
        assert!(!record.is_terminal());
    }

    #[test]
    fn test_delivery_record_transitions() {
        let mut record = DeliveryRecord::new("notif-1", "slack");

        record.mark_sending();
        assert_eq!(record.current_status, DeliveryStatus::Sending);
        assert!(!record.is_terminal());

        record.increment_attempts();
        assert_eq!(record.attempts, 1);

        record.mark_delivered(Some("200 OK".to_string()));
        assert_eq!(record.current_status, DeliveryStatus::Delivered);
        assert!(record.is_terminal());
        assert_eq!(record.events.len(), 3);
    }

    #[test]
    fn test_delivery_record_failed() {
        let mut record = DeliveryRecord::new("notif-2", "email");
        record.mark_sending();
        record.mark_failed("connection timeout");
        assert_eq!(record.current_status, DeliveryStatus::Failed);
        assert!(record.is_terminal());
        assert_eq!(
            record.events.last().unwrap().detail.as_deref(),
            Some("connection timeout")
        );
    }

    #[test]
    fn test_delivery_record_cancelled() {
        let mut record = DeliveryRecord::new("notif-3", "teams");
        record.mark_cancelled(Some("user cancelled".to_string()));
        assert_eq!(record.current_status, DeliveryStatus::Cancelled);
        assert!(record.is_terminal());
    }

    #[test]
    fn test_delivery_record_read() {
        let mut record = DeliveryRecord::new("notif-4", "telegram");
        record.mark_sending();
        record.mark_delivered(None);
        record.mark_read();
        assert_eq!(record.current_status, DeliveryStatus::Read);
        assert!(record.is_terminal());
        assert_eq!(record.events.len(), 4);
    }

    #[test]
    fn test_delivery_record_duration() {
        let mut record = DeliveryRecord::new("notif-5", "slack");
        assert!(record.total_duration.is_none());
        record.set_duration(Duration::from_millis(250));
        assert_eq!(record.total_duration, Some(Duration::from_millis(250)));
    }

    #[test]
    fn test_delivery_status_serde() {
        let status = DeliveryStatus::Delivered;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"delivered\"");
        let parsed: DeliveryStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[tokio::test]
    async fn test_tracker_track_and_get() {
        let tracker = StatusTracker::new();
        let id = tracker.track("n1", "slack").await;
        assert_eq!(id, "n1");

        let records = tracker.get_records("n1").await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].provider, "slack");
        assert_eq!(records[0].current_status, DeliveryStatus::Pending);
    }

    #[tokio::test]
    async fn test_tracker_multiple_providers() {
        let tracker = StatusTracker::new();
        tracker.track("n1", "slack").await;
        tracker.track("n1", "email").await;

        let records = tracker.get_records("n1").await;
        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn test_tracker_update_status() {
        let tracker = StatusTracker::new();
        tracker.track("n1", "slack").await;

        let updated = tracker
            .update_status("n1", "slack", DeliveryStatus::Delivered, None)
            .await;
        assert!(updated);

        let record = tracker.get_record("n1", "slack").await.unwrap();
        assert_eq!(record.current_status, DeliveryStatus::Delivered);
    }

    #[tokio::test]
    async fn test_tracker_update_nonexistent() {
        let tracker = StatusTracker::new();
        let updated = tracker
            .update_status("missing", "slack", DeliveryStatus::Failed, None)
            .await;
        assert!(!updated);
    }

    #[tokio::test]
    async fn test_tracker_list_ids() {
        let tracker = StatusTracker::new();
        tracker.track("n1", "slack").await;
        tracker.track("n2", "email").await;

        let ids = tracker.list_ids().await;
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"n1".to_string()));
        assert!(ids.contains(&"n2".to_string()));
    }

    #[tokio::test]
    async fn test_tracker_remove() {
        let tracker = StatusTracker::new();
        tracker.track("n1", "slack").await;
        assert_eq!(tracker.count().await, 1);

        let removed = tracker.remove("n1").await;
        assert!(removed);
        assert_eq!(tracker.count().await, 0);
    }

    #[tokio::test]
    async fn test_tracker_remove_nonexistent() {
        let tracker = StatusTracker::new();
        let removed = tracker.remove("missing").await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_tracker_summary() {
        let tracker = StatusTracker::new();
        tracker.track("n1", "slack").await;
        tracker.track("n1", "email").await;
        tracker.track("n2", "teams").await;

        tracker
            .update_status("n1", "slack", DeliveryStatus::Delivered, None)
            .await;
        tracker
            .update_status(
                "n1",
                "email",
                DeliveryStatus::Failed,
                Some("timeout".into()),
            )
            .await;

        let summary = tracker.summary().await;
        assert_eq!(summary.pending, 1); // n2/teams
        assert_eq!(summary.delivered, 1); // n1/slack
        assert_eq!(summary.failed, 1); // n1/email
        assert_eq!(summary.total(), 3);
    }

    #[tokio::test]
    async fn test_tracker_get_record_specific() {
        let tracker = StatusTracker::new();
        tracker.track("n1", "slack").await;
        tracker.track("n1", "email").await;

        let record = tracker.get_record("n1", "email").await;
        assert!(record.is_some());
        assert_eq!(record.unwrap().provider, "email");

        let missing = tracker.get_record("n1", "teams").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_tracker_purge_terminal() {
        let tracker = StatusTracker::new();
        // n1: terminal (all records delivered/failed)
        tracker.track("n1", "slack").await;
        tracker.track("n1", "email").await;
        tracker
            .update_status("n1", "slack", DeliveryStatus::Delivered, None)
            .await;
        tracker
            .update_status("n1", "email", DeliveryStatus::Failed, Some("err".into()))
            .await;

        // n2: non-terminal (still pending)
        tracker.track("n2", "teams").await;

        // n3: partially terminal (one delivered, one pending)
        tracker.track("n3", "slack").await;
        tracker.track("n3", "email").await;
        tracker
            .update_status("n3", "slack", DeliveryStatus::Delivered, None)
            .await;

        assert_eq!(tracker.count().await, 3);
        let purged = tracker.purge_terminal().await;
        assert_eq!(purged, 1); // only n1 (fully terminal)
        assert_eq!(tracker.count().await, 2);

        // n2 and n3 should remain
        assert!(!tracker.get_records("n2").await.is_empty());
        assert!(!tracker.get_records("n3").await.is_empty());
        assert!(tracker.get_records("n1").await.is_empty());
    }

    #[tokio::test]
    async fn test_tracker_purge_terminal_empty() {
        let tracker = StatusTracker::new();
        assert_eq!(tracker.purge_terminal().await, 0);
    }

    #[tokio::test]
    async fn test_tracker_purge_older_than() {
        let tracker = StatusTracker::new();

        // Create a record and manually backdate it
        {
            let mut store = tracker.records.write().await;
            let old_time = SystemTime::now() - Duration::from_secs(3600);
            let mut record = DeliveryRecord::new("old-1", "slack");
            record.current_status = DeliveryStatus::Delivered;
            record.updated_at = old_time;
            store.entry("old-1".to_string()).or_default().push(record);
        }

        // Create a recent terminal record
        tracker.track("new-1", "slack").await;
        tracker
            .update_status("new-1", "slack", DeliveryStatus::Delivered, None)
            .await;

        // Create a non-terminal record
        tracker.track("pending-1", "email").await;

        assert_eq!(tracker.count().await, 3);

        // Purge records older than 30 minutes
        let purged = tracker.purge_older_than(Duration::from_secs(1800)).await;
        assert_eq!(purged, 1); // only old-1
        assert_eq!(tracker.count().await, 2);

        // new-1 and pending-1 remain
        assert!(!tracker.get_records("new-1").await.is_empty());
        assert!(!tracker.get_records("pending-1").await.is_empty());
    }

    #[tokio::test]
    async fn test_tracker_purge_older_than_keeps_non_terminal() {
        let tracker = StatusTracker::new();

        // Create an old non-terminal record — should NOT be purged
        {
            let mut store = tracker.records.write().await;
            let old_time = SystemTime::now() - Duration::from_secs(7200);
            let mut record = DeliveryRecord::new("old-pending", "slack");
            record.updated_at = old_time;
            // status remains Pending (non-terminal)
            store
                .entry("old-pending".to_string())
                .or_default()
                .push(record);
        }

        let purged = tracker.purge_older_than(Duration::from_secs(60)).await;
        assert_eq!(purged, 0);
        assert_eq!(tracker.count().await, 1);
    }

    #[test]
    fn test_status_summary_total() {
        let summary = StatusSummary {
            pending: 2,
            sending: 1,
            delivered: 5,
            failed: 3,
            cancelled: 0,
            read: 1,
        };
        assert_eq!(summary.total(), 12);
    }

    #[test]
    fn test_status_summary_default() {
        let summary = StatusSummary::default();
        assert_eq!(summary.total(), 0);
    }
}
