//! Custom JSON extractor with automatic request body validation.
//!
//! [`ValidatedJson`] wraps axum's [`Json`] extractor and runs
//! [`validator::Validate`] on the deserialized body, returning a
//! 422 Unprocessable Entity with structured field-level errors on failure.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::FromRequest;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use validator::Validate;

/// An axum extractor that deserializes JSON and validates the body.
///
/// Drop-in replacement for `Json<T>` when `T: Validate`.
///
/// On validation failure, returns 422 with a JSON body listing field errors.
///
/// # Example
/// ```ignore
/// use validator::Validate;
///
/// #[derive(Deserialize, Validate)]
/// struct CreateUser {
///     #[validate(length(min = 1, message = "name must not be empty"))]
///     name: String,
/// }
///
/// async fn handler(ValidatedJson(body): ValidatedJson<CreateUser>) -> impl IntoResponse {
///     // body is guaranteed valid here
/// }
/// ```
pub struct ValidatedJson<T>(pub T);

/// Rejection type for validation failures.
pub enum ValidatedJsonRejection {
    /// JSON deserialization failed.
    JsonError(JsonRejection),
    /// Validation failed.
    ValidationError(validator::ValidationErrors),
}

impl IntoResponse for ValidatedJsonRejection {
    fn into_response(self) -> Response {
        match self {
            Self::JsonError(rejection) => {
                let body = serde_json::json!({
                    "error": "invalid_json",
                    "message": rejection.body_text(),
                });
                (StatusCode::BAD_REQUEST, Json(body)).into_response()
            }
            Self::ValidationError(errors) => {
                let field_errors = format_validation_errors(&errors);
                let body = serde_json::json!({
                    "error": "validation_failed",
                    "message": "Request body validation failed",
                    "fields": field_errors,
                });
                (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
            }
        }
    }
}

/// Format validation errors into a JSON-friendly structure.
///
/// Returns a map of field name → list of error messages.
fn format_validation_errors(
    errors: &validator::ValidationErrors,
) -> serde_json::Value {
    let mut fields = serde_json::Map::new();

    for (field, field_errors) in errors.field_errors() {
        let messages: Vec<String> = field_errors
            .iter()
            .map(|e| {
                e.message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| format!("validation failed: {}", e.code))
            })
            .collect();
        fields.insert(
            field.to_string(),
            serde_json::Value::Array(
                messages
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }

    serde_json::Value::Object(fields)
}

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ValidatedJsonRejection;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(ValidatedJsonRejection::JsonError)?;

        value
            .validate()
            .map_err(ValidatedJsonRejection::ValidationError)?;

        Ok(ValidatedJson(value))
    }
}

// ───────────────────── Tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::post;
    use axum_test::TestServer;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, Validate)]
    struct TestBody {
        #[validate(length(min = 1, message = "name must not be empty"))]
        name: String,
        #[validate(range(min = 1, max = 100, message = "age must be between 1 and 100"))]
        age: u32,
    }

    async fn validated_handler(
        ValidatedJson(body): ValidatedJson<TestBody>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "name": body.name,
            "age": body.age,
        }))
    }

    fn build_test_app() -> Router {
        Router::new().route("/test", post(validated_handler))
    }

    #[tokio::test]
    async fn test_valid_body_passes() {
        let server = TestServer::new(build_test_app());
        let resp = server
            .post("/test")
            .json(&serde_json::json!({"name": "Alice", "age": 30}))
            .await;
        resp.assert_status_ok();

        let body: serde_json::Value = resp.json();
        assert_eq!(body["name"], "Alice");
        assert_eq!(body["age"], 30);
    }

    #[tokio::test]
    async fn test_empty_name_rejected() {
        let server = TestServer::new(build_test_app());
        let resp = server
            .post("/test")
            .json(&serde_json::json!({"name": "", "age": 30}))
            .await;
        resp.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "validation_failed");
        assert!(body["fields"]["name"].is_array());
    }

    #[tokio::test]
    async fn test_invalid_age_rejected() {
        let server = TestServer::new(build_test_app());
        let resp = server
            .post("/test")
            .json(&serde_json::json!({"name": "Bob", "age": 0}))
            .await;
        resp.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "validation_failed");
        assert!(body["fields"]["age"].is_array());
    }

    #[tokio::test]
    async fn test_invalid_json_body() {
        let server = TestServer::new(build_test_app());
        let resp = server
            .post("/test")
            .content_type("application/json")
            .bytes(b"not json".to_vec().into())
            .await;
        resp.assert_status(StatusCode::BAD_REQUEST);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "invalid_json");
    }

    #[tokio::test]
    async fn test_multiple_validation_errors() {
        let server = TestServer::new(build_test_app());
        let resp = server
            .post("/test")
            .json(&serde_json::json!({"name": "", "age": 200}))
            .await;
        resp.assert_status(StatusCode::UNPROCESSABLE_ENTITY);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "validation_failed");
        // Both fields should have errors
        assert!(body["fields"]["name"].is_array());
        assert!(body["fields"]["age"].is_array());
    }
}
