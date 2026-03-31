use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use noti_core::{DeliveryRecord, StatusSummary};

use crate::handlers::error::ApiError;
use crate::state::AppState;

/// Response for a single notification's delivery records.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub notification_id: String,
    pub records: Vec<DeliveryRecord>,
}

/// Response for listing all tracked statuses.
#[derive(Debug, Serialize)]
pub struct AllStatusesResponse {
    pub summary: StatusSummary,
    pub notification_ids: Vec<String>,
    pub total: usize,
}

/// GET /api/v1/status/:notification_id — Get delivery status for a notification.
pub async fn get_status(
    State(state): State<AppState>,
    Path(notification_id): Path<String>,
) -> Result<Json<StatusResponse>, ApiError> {
    let records = state.status_tracker.get_records(&notification_id).await;

    if records.is_empty() {
        return Err(ApiError::not_found(format!(
            "notification '{}' not found",
            notification_id
        )));
    }

    Ok(Json(StatusResponse {
        notification_id,
        records,
    }))
}

/// GET /api/v1/status — Get summary of all tracked statuses.
pub async fn get_all_statuses(State(state): State<AppState>) -> Json<AllStatusesResponse> {
    let summary = state.status_tracker.summary().await;
    let ids = state.status_tracker.list_ids().await;
    let total = ids.len();

    Json(AllStatusesResponse {
        summary,
        notification_ids: ids,
        total,
    })
}
