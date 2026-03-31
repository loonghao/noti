//! Request ID middleware for axum.
//!
//! Assigns a unique request ID to every incoming request, propagates it
//! in the response via the `X-Request-Id` header, and injects it into
//! a [`tracing::Span`] so that all downstream log entries automatically
//! include the `request_id` field.

use axum::http::{Request, header::HeaderName, header::HeaderValue};
use axum::response::Response;
use tracing::Instrument;
use uuid::Uuid;

static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Axum middleware that assigns a unique request ID.
///
/// If the request already carries an `X-Request-Id` header, it is preserved.
/// Otherwise a new UUID v4 is generated. The ID is always echoed back in the
/// response and attached to a [`tracing::info_span`] that wraps the
/// downstream handler, enabling automatic log correlation.
pub async fn request_id_middleware(
    mut request: Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    // Use existing ID or generate a new one
    let request_id = request
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Insert into request headers (overwrite if already present to normalize)
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        request.headers_mut().insert(X_REQUEST_ID.clone(), val);
    }

    // Create a span that carries the request_id for all downstream logging
    let span = tracing::info_span!(
        "request",
        request_id = %request_id,
        method = %request.method(),
        path = %request.uri().path(),
    );

    let mut response = next.run(request).instrument(span).await;

    // Echo the request ID in the response
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(X_REQUEST_ID.clone(), val);
    }

    response
}

// ───────────────────── Tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use axum_test::TestServer;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn build_test_app() -> Router {
        Router::new()
            .route("/test", get(ok_handler))
            .layer(axum::middleware::from_fn(request_id_middleware))
    }

    #[tokio::test]
    async fn test_generates_request_id() {
        let server = TestServer::new(build_test_app());
        let resp = server.get("/test").await;
        resp.assert_status_ok();

        let id = resp
            .headers()
            .get("x-request-id")
            .expect("should have x-request-id");
        let id_str = id.to_str().unwrap();
        // Should be a valid UUID v4
        assert!(
            Uuid::parse_str(id_str).is_ok(),
            "not a valid UUID: {id_str}"
        );
    }

    #[tokio::test]
    async fn test_preserves_existing_request_id() {
        let server = TestServer::new(build_test_app());
        let custom_id = "my-custom-id-123";

        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_static("my-custom-id-123"),
            )
            .await;
        resp.assert_status_ok();

        let id = resp
            .headers()
            .get("x-request-id")
            .expect("should have x-request-id");
        assert_eq!(id.to_str().unwrap(), custom_id);
    }
}
