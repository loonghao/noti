//! Standardized API error response type.
//!
//! All error responses returned by the noti-server API use this format,
//! ensuring clients always receive a predictable JSON structure.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Standardized error response for all API endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiError {
    /// Machine-readable error code (e.g. "not_found", "bad_request").
    pub error: String,
    /// Human-readable error message.
    pub message: String,
    /// HTTP status code (not serialized to JSON body, used for response status).
    #[serde(skip)]
    #[schema(ignore)]
    pub status: StatusCode,
}

impl ApiError {
    /// Create a 400 Bad Request error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: "bad_request".to_string(),
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    /// Create a 404 Not Found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: "not_found".to_string(),
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    /// Create a 409 Conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            error: "conflict".to_string(),
            message: message.into(),
            status: StatusCode::CONFLICT,
        }
    }

    /// Create a 500 Internal Server Error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: message.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Create a 422 Unprocessable Entity error.
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            error: "unprocessable_entity".to_string(),
            message: message.into(),
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    /// Create a 503 Service Unavailable error.
    pub fn service_unavailable(error_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error_code.into(),
            message: message.into(),
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": self.error,
                "message": self.message,
            })),
        )
            .into_response()
    }
}

// Allow using ApiError as the error type in Result<T, ApiError>
impl From<ApiError> for (StatusCode, Json<serde_json::Value>) {
    fn from(err: ApiError) -> Self {
        (
            err.status,
            Json(serde_json::json!({
                "error": err.error,
                "message": err.message,
            })),
        )
    }
}

// ───────────────────── Tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_request() {
        let err = ApiError::bad_request("invalid field");
        assert_eq!(err.error, "bad_request");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.message, "invalid field");
    }

    #[test]
    fn test_not_found() {
        let err = ApiError::not_found("provider not found");
        assert_eq!(err.error, "not_found");
        assert_eq!(err.status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_conflict() {
        let err = ApiError::conflict("already exists");
        assert_eq!(err.error, "conflict");
        assert_eq!(err.status, StatusCode::CONFLICT);
    }

    #[test]
    fn test_internal() {
        let err = ApiError::internal("something broke");
        assert_eq!(err.error, "internal_error");
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_unprocessable() {
        let err = ApiError::unprocessable("bad data");
        assert_eq!(err.error, "unprocessable_entity");
        assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn test_service_unavailable() {
        let err = ApiError::service_unavailable("queue_full", "queue is full");
        assert_eq!(err.error, "queue_full");
        assert_eq!(err.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.message, "queue is full");
    }

    #[test]
    fn test_serialization() {
        let err = ApiError::bad_request("test");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "bad_request");
        assert_eq!(json["message"], "test");
        // status should be skipped
        assert!(json.get("status").is_none());
    }
}
