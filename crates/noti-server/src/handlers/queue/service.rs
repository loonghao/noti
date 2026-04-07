use noti_queue::{DlqEntry, NotificationTask, TaskStatus};

use super::dto::{DlqEntryInfo, TaskInfo};
use crate::handlers::error::{ApiError, codes};

// ───────────────────── Mapping helpers ─────────────────────

/// Convert a [`NotificationTask`] to the API response type [`TaskInfo`].
pub fn task_to_info(task: &NotificationTask) -> TaskInfo {
    let scheduled_at = task
        .available_at
        .map(|at| humantime::format_rfc3339(at).to_string());

    TaskInfo {
        id: task.id.clone(),
        provider: task.provider.clone(),
        status: task.status.to_string(),
        attempts: task.attempts,
        last_error: task.last_error.clone(),
        priority: format!("{:?}", task.priority()),
        metadata: task.metadata.clone(),
        scheduled_at,
    }
}

/// Convert a [`DlqEntry`] to the API response type [`DlqEntryInfo`].
pub fn dlq_entry_to_info(entry: &DlqEntry) -> DlqEntryInfo {
    DlqEntryInfo {
        task_id: entry.task.id.clone(),
        provider: entry.task.provider.clone(),
        status: entry.task.status.to_string(),
        attempts: entry.task.attempts,
        last_error: entry.task.last_error.clone(),
        reason: entry.reason.clone(),
        moved_at: humantime::format_rfc3339(entry.moved_at).to_string(),
        priority: format!("{:?}", entry.task.priority()),
        metadata: entry.task.metadata.clone(),
    }
}

/// Convert a failed [`NotificationTask`] to the API response type [`DlqEntryInfo`].
pub fn task_to_dlq_entry(task: &NotificationTask) -> DlqEntryInfo {
    DlqEntryInfo {
        task_id: task.id.clone(),
        provider: task.provider.clone(),
        status: task.status.to_string(),
        attempts: task.attempts,
        last_error: task.last_error.clone(),
        reason: task.last_error.clone().unwrap_or_default(),
        moved_at: humantime::format_rfc3339(task.updated_at).to_string(),
        priority: format!("{:?}", task.priority()),
        metadata: task.metadata.clone(),
    }
}

/// Parse an optional status string into a [`TaskStatus`].
///
/// Returns `None` for unrecognised values.
pub fn parse_task_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().as_str() {
        "queued" => Some(TaskStatus::Queued),
        "processing" => Some(TaskStatus::Processing),
        "completed" => Some(TaskStatus::Completed),
        "failed" => Some(TaskStatus::Failed),
        "cancelled" => Some(TaskStatus::Cancelled),
        _ => None,
    }
}

/// Parse a schedule specification from the API request into a `SystemTime`.
///
/// Supports two mutually exclusive options:
/// - `delay_seconds`: relative delay from now
/// - `scheduled_at`: absolute RFC 3339 timestamp
///
/// Returns `None` if neither is provided.
pub fn parse_scheduled_time(
    delay_seconds: Option<u64>,
    scheduled_at: Option<&str>,
) -> Result<Option<std::time::SystemTime>, ApiError> {
    match (delay_seconds, scheduled_at) {
        (Some(_), Some(_)) => Err(ApiError::bad_request(
            "delay_seconds and scheduled_at are mutually exclusive; provide only one".to_string(),
        )
        .with_code(codes::INVALID_PARAMETER)),
        (Some(secs), None) => {
            if secs == 0 {
                Ok(None)
            } else {
                let at = std::time::SystemTime::now() + std::time::Duration::from_secs(secs);
                Ok(Some(at))
            }
        }
        (None, Some(ts)) => {
            let dt = humantime::parse_rfc3339(ts).map_err(|e| {
                ApiError::bad_request(format!(
                    "invalid scheduled_at timestamp (expected RFC 3339 / ISO 8601): {e}"
                ))
                .with_code(codes::INVALID_PARAMETER)
            })?;
            Ok(Some(dt))
        }
        (None, None) => Ok(None),
    }
}

/// Map a [`noti_queue::QueueError`] to an [`ApiError`] with an appropriate code.
pub fn queue_error(e: noti_queue::QueueError) -> ApiError {
    match &e {
        noti_queue::QueueError::QueueFull { .. } => {
            ApiError::service_unavailable(e.to_string()).with_code(codes::QUEUE_FULL)
        }
        noti_queue::QueueError::NotFound(_) => {
            ApiError::not_found(e.to_string()).with_code(codes::TASK_NOT_FOUND)
        }
        noti_queue::QueueError::ShutDown => {
            ApiError::internal(e.to_string()).with_code(codes::QUEUE_SHUT_DOWN)
        }
        noti_queue::QueueError::Serialization(_) => {
            ApiError::internal(e.to_string()).with_code(codes::QUEUE_SERIALIZATION_ERROR)
        }
        noti_queue::QueueError::Backend(_) => {
            ApiError::internal(e.to_string()).with_code(codes::QUEUE_BACKEND_ERROR)
        }
        noti_queue::QueueError::Notification(_) => {
            ApiError::internal(e.to_string()).with_code(codes::NOTIFICATION_SEND_ERROR)
        }
    }
}

// ───────────────────── Tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scheduled_time_none() {
        let result = parse_scheduled_time(None, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_scheduled_time_delay_seconds() {
        let result = parse_scheduled_time(Some(60), None).unwrap();
        assert!(result.is_some());
        let at = result.unwrap();
        let now = std::time::SystemTime::now();
        let diff = at.duration_since(now).unwrap();
        // Should be roughly 60 seconds from now (within 2s tolerance)
        assert!(diff.as_secs() >= 58 && diff.as_secs() <= 62);
    }

    #[test]
    fn test_parse_scheduled_time_delay_zero() {
        let result = parse_scheduled_time(Some(0), None).unwrap();
        assert!(
            result.is_none(),
            "delay_seconds=0 should be treated as immediate"
        );
    }

    #[test]
    fn test_parse_scheduled_time_rfc3339() {
        let result = parse_scheduled_time(None, Some("2030-01-15T10:30:00Z")).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_scheduled_time_invalid_rfc3339() {
        let result = parse_scheduled_time(None, Some("not-a-timestamp"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_scheduled_time_mutually_exclusive() {
        let result = parse_scheduled_time(Some(60), Some("2030-01-15T10:30:00Z"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("mutually exclusive"));
    }
}
