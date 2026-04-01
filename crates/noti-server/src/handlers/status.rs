use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use utoipa::ToSchema;

use noti_core::{DeliveryRecord, StatusSummary};

use crate::handlers::error::{ApiError, codes};
use crate::state::AppState;

/// Response for a single notification's delivery records.
#[derive(Debug, Serialize, ToSchema)]
pub struct StatusResponse {
    pub notification_id: String,
    pub records: Vec<DeliveryRecord>,
}

/// Response for listing all tracked statuses.
#[derive(Debug, Serialize, ToSchema)]
pub struct AllStatusesResponse {
    pub summary: StatusSummary,
    pub notification_ids: Vec<String>,
    pub total: usize,
}

/// Get delivery status for a specific notification.
#[utoipa::path(
    get,
    path = "/api/v1/status/{notification_id}",
    tag = "Status",
    params(
        ("notification_id" = String, Path, description = "Notification ID to look up")
    ),
    responses(
        (status = 200, description = "Delivery status found", body = StatusResponse),
        (status = 404, description = "Notification not found", body = ApiError),
    )
)]
pub async fn get_status(
    State(state): State<AppState>,
    Path(notification_id): Path<String>,
) -> Result<Json<StatusResponse>, ApiError> {
    let records = state.status_tracker.get_records(&notification_id).await;

    if records.is_empty() {
        return Err(
            ApiError::not_found(format!("notification '{}' not found", notification_id))
                .with_code(codes::NOTIFICATION_NOT_FOUND),
        );
    }

    Ok(Json(StatusResponse {
        notification_id,
        records,
    }))
}

/// Get summary of all tracked delivery statuses.
#[utoipa::path(
    get,
    path = "/api/v1/status",
    tag = "Status",
    responses(
        (status = 200, description = "All statuses summary", body = AllStatusesResponse)
    )
)]
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
