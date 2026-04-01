//! Standardized API error response type.
//!
//! All error responses returned by the noti-server API use this format,
//! ensuring clients always receive a predictable JSON structure.
//!
//! # Error code hierarchy
//!
//! Each error has two levels of classification:
//! - `error`: HTTP-level category (`"bad_request"`, `"not_found"`, etc.)
//! - `code`: Granular business-level error code (`"PROVIDER_NOT_FOUND"`,
//!   `"CONFIG_VALIDATION_FAILED"`, etc.)
//!
//! The `code` field is optional — when absent it is omitted from the JSON body,
//! preserving backward compatibility with existing clients.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

// ───────────────────── Well-known error codes ─────────────────────

/// Well-known granular error codes for API responses.
///
/// These constants provide a single source of truth so both production code
/// and tests can reference the same strings without risk of typos.
pub mod codes {
    // 400 — Bad Request
    /// Provider-specific configuration failed validation.
    pub const CONFIG_VALIDATION_FAILED: &str = "CONFIG_VALIDATION_FAILED";
    /// A query-parameter or path-parameter value is invalid.
    pub const INVALID_PARAMETER: &str = "INVALID_PARAMETER";
    /// Template variable validation failed (missing required variables).
    pub const TEMPLATE_VARIABLE_MISSING: &str = "TEMPLATE_VARIABLE_MISSING";

    // 404 — Not Found
    /// The requested notification provider does not exist.
    pub const PROVIDER_NOT_FOUND: &str = "PROVIDER_NOT_FOUND";
    /// The requested message template does not exist.
    pub const TEMPLATE_NOT_FOUND: &str = "TEMPLATE_NOT_FOUND";
    /// The requested notification (delivery tracking) does not exist.
    pub const NOTIFICATION_NOT_FOUND: &str = "NOTIFICATION_NOT_FOUND";
    /// The requested queue task does not exist.
    pub const TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";

    // 500 — Internal
    /// An internal queue backend error occurred.
    pub const QUEUE_BACKEND_ERROR: &str = "QUEUE_BACKEND_ERROR";
    /// A serialization/deserialization error in the queue layer.
    pub const QUEUE_SERIALIZATION_ERROR: &str = "QUEUE_SERIALIZATION_ERROR";
    /// The queue has been shut down.
    pub const QUEUE_SHUT_DOWN: &str = "QUEUE_SHUT_DOWN";
    /// A notification-level send error propagated from the core layer.
    pub const NOTIFICATION_SEND_ERROR: &str = "NOTIFICATION_SEND_ERROR";

    // 503 — Service Unavailable
    /// The queue is at capacity and cannot accept more tasks.
    pub const QUEUE_FULL: &str = "QUEUE_FULL";
}

// ───────────────────── ApiError ─────────────────────

/// Standardized error response for all API endpoints.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiError {
    /// Machine-readable error category (e.g. `"not_found"`, `"bad_request"`).
    pub error: String,
    /// Human-readable error message.
    pub message: String,
    /// Granular machine-readable error code (e.g. `"PROVIDER_NOT_FOUND"`).
    ///
    /// Omitted from the JSON body when `None`, preserving backward compatibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// HTTP status code (not serialized to JSON body, used for response status).
    #[serde(skip)]
    #[schema(ignore)]
    pub status: StatusCode,
}

impl ApiError {
    /// Attach a granular error `code` (builder pattern).
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Create a 400 Bad Request error.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            error: "bad_request".to_string(),
            message: message.into(),
            code: None,
            status: StatusCode::BAD_REQUEST,
        }
    }

    /// Create a 404 Not Found error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            error: "not_found".to_string(),
            message: message.into(),
            code: None,
            status: StatusCode::NOT_FOUND,
        }
    }

    /// Create a 409 Conflict error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            error: "conflict".to_string(),
            message: message.into(),
            code: None,
            status: StatusCode::CONFLICT,
        }
    }

    /// Create a 500 Internal Server Error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            error: "internal_error".to_string(),
            message: message.into(),
            code: None,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Create a 422 Unprocessable Entity error.
    pub fn unprocessable(message: impl Into<String>) -> Self {
        Self {
            error: "unprocessable_entity".to_string(),
            message: message.into(),
            code: None,
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    /// Create a 503 Service Unavailable error.
    pub fn service_unavailable(error_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error_code.into(),
            message: message.into(),
            code: None,
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut body = serde_json::json!({
            "error": self.error,
            "message": self.message,
        });
        if let Some(code) = &self.code {
            body["code"] = serde_json::Value::String(code.clone());
        }
        (self.status, Json(body)).into_response()
    }
}

// Allow using ApiError as the error type in Result<T, ApiError>
impl From<ApiError> for (StatusCode, Json<serde_json::Value>) {
    fn from(err: ApiError) -> Self {
        let mut body = serde_json::json!({
            "error": err.error,
            "message": err.message,
        });
        if let Some(code) = &err.code {
            body["code"] = serde_json::Value::String(code.clone());
        }
        (err.status, Json(body))
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
        assert!(err.code.is_none());
    }

    #[test]
    fn test_not_found() {
        let err = ApiError::not_found("provider not found");
        assert_eq!(err.error, "not_found");
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert!(err.code.is_none());
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
    fn test_serialization_without_code() {
        let err = ApiError::bad_request("test");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "bad_request");
        assert_eq!(json["message"], "test");
        // status should be skipped
        assert!(json.get("status").is_none());
        // code should be absent when None
        assert!(json.get("code").is_none());
    }

    #[test]
    fn test_serialization_with_code() {
        let err =
            ApiError::not_found("provider 'x' not found").with_code(codes::PROVIDER_NOT_FOUND);
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"], "not_found");
        assert_eq!(json["message"], "provider 'x' not found");
        assert_eq!(json["code"], "PROVIDER_NOT_FOUND");
        assert!(json.get("status").is_none());
    }

    #[test]
    fn test_with_code_builder() {
        let err = ApiError::bad_request("bad config").with_code(codes::CONFIG_VALIDATION_FAILED);
        assert_eq!(err.code.as_deref(), Some("CONFIG_VALIDATION_FAILED"));
        assert_eq!(err.error, "bad_request");
    }

    #[test]
    fn test_all_code_constants_are_uppercase() {
        let all_codes = [
            codes::CONFIG_VALIDATION_FAILED,
            codes::INVALID_PARAMETER,
            codes::TEMPLATE_VARIABLE_MISSING,
            codes::PROVIDER_NOT_FOUND,
            codes::TEMPLATE_NOT_FOUND,
            codes::NOTIFICATION_NOT_FOUND,
            codes::TASK_NOT_FOUND,
            codes::QUEUE_BACKEND_ERROR,
            codes::QUEUE_SERIALIZATION_ERROR,
            codes::QUEUE_SHUT_DOWN,
            codes::NOTIFICATION_SEND_ERROR,
            codes::QUEUE_FULL,
        ];
        for code in all_codes {
            assert_eq!(
                code,
                code.to_uppercase(),
                "error code {code} must be UPPER_SNAKE_CASE"
            );
        }
    }
}
