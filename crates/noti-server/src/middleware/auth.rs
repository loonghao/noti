//! API key authentication middleware for axum.
//!
//! Validates requests against a set of configured API keys.
//! Keys can be provided via `Authorization: Bearer <key>` header
//! or `X-API-Key: <key>` header.
//!
//! Certain paths (like `/health`) can be excluded from authentication.

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};

// ───────────────────── Configuration ─────────────────────

/// Authentication configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Set of valid API keys.
    api_keys: HashSet<String>,
    /// Paths that skip authentication (e.g. health checks).
    pub excluded_paths: HashSet<String>,
    /// Whether authentication is enabled.
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_keys: HashSet::new(),
            excluded_paths: HashSet::from(["/health".to_string()]),
            enabled: false,
        }
    }
}

impl AuthConfig {
    /// Create a new auth config with the given API keys.
    pub fn new(keys: Vec<String>) -> Self {
        let enabled = !keys.is_empty();
        Self {
            api_keys: keys.into_iter().collect(),
            excluded_paths: HashSet::from(["/health".to_string()]),
            enabled,
        }
    }

    /// Add a path that should skip authentication.
    pub fn with_excluded_path(mut self, path: &str) -> Self {
        self.excluded_paths.insert(path.to_string());
        self
    }

    /// Add multiple excluded paths.
    pub fn with_excluded_paths(mut self, paths: &[&str]) -> Self {
        for path in paths {
            self.excluded_paths.insert((*path).to_string());
        }
        self
    }

    /// Check if a given API key is valid.
    pub fn is_valid_key(&self, key: &str) -> bool {
        self.api_keys.contains(key)
    }

    /// Check if a path is excluded from authentication.
    pub fn is_excluded(&self, path: &str) -> bool {
        self.excluded_paths.contains(path)
    }

    /// Return the number of configured API keys.
    pub fn key_count(&self) -> usize {
        self.api_keys.len()
    }
}

// ───────────────────── Shared State ─────────────────────

/// Shared authentication state, safe to clone across handlers.
#[derive(Clone)]
pub struct AuthState {
    config: Arc<AuthConfig>,
}

impl AuthState {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Check if authentication is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

// ───────────────────── Key extraction ─────────────────────

/// Extract API key from request headers.
///
/// Checks in order:
/// 1. `Authorization: Bearer <key>`
/// 2. `X-API-Key: <key>`
fn extract_api_key<B>(request: &Request<B>) -> Option<String> {
    // Try Authorization: Bearer <key>
    if let Some(auth) = request.headers().get("authorization") {
        if let Ok(val) = auth.to_str() {
            let val = val.trim();
            if let Some(key) = val.strip_prefix("Bearer ") {
                let key = key.trim();
                if !key.is_empty() {
                    return Some(key.to_string());
                }
            }
        }
    }

    // Try X-API-Key header
    if let Some(api_key) = request.headers().get("x-api-key") {
        if let Ok(val) = api_key.to_str() {
            let key = val.trim();
            if !key.is_empty() {
                return Some(key.to_string());
            }
        }
    }

    None
}

// ───────────────────── Middleware ─────────────────────

/// Axum middleware that enforces API key authentication.
///
/// When `AuthConfig.enabled` is `false`, all requests pass through.
/// When enabled, requests must include a valid API key in
/// `Authorization: Bearer <key>` or `X-API-Key: <key>`.
///
/// Paths listed in `AuthConfig.excluded_paths` bypass authentication.
pub async fn auth_middleware(
    State(auth): State<AuthState>,
    request: Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    // Skip if auth is disabled
    if !auth.config.enabled {
        return next.run(request).await;
    }

    // Skip excluded paths
    let path = request.uri().path().to_string();
    if auth.config.is_excluded(&path) {
        return next.run(request).await;
    }

    // Extract and validate API key
    match extract_api_key(&request) {
        Some(key) if auth.config.is_valid_key(&key) => next.run(request).await,
        Some(_) => unauthorized_response("invalid API key"),
        None => unauthorized_response("missing API key — provide via Authorization: Bearer <key> or X-API-Key header"),
    }
}

fn unauthorized_response(message: &str) -> Response {
    let body = serde_json::json!({
        "error": "unauthorized",
        "message": message,
    });
    (StatusCode::UNAUTHORIZED, axum::Json(body)).into_response()
}

// ───────────────────── Tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::http::header::{HeaderName, HeaderValue};
    use axum::routing::get;
    use axum_test::TestServer;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn build_test_app(config: AuthConfig) -> Router {
        let auth = AuthState::new(config);
        Router::new()
            .route("/test", get(ok_handler))
            .route("/health", get(ok_handler))
            .layer(axum::middleware::from_fn_with_state(
                auth.clone(),
                auth_middleware,
            ))
            .with_state(auth)
    }

    #[tokio::test]
    async fn test_auth_disabled_allows_all() {
        let config = AuthConfig::default(); // disabled by default
        let server = TestServer::new(build_test_app(config));

        let resp = server.get("/test").await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn test_auth_enabled_rejects_no_key() {
        let config = AuthConfig::new(vec!["secret-key-123".to_string()]);
        let server = TestServer::new(build_test_app(config));

        let resp = server.get("/test").await;
        resp.assert_status(StatusCode::UNAUTHORIZED);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "unauthorized");
        assert!(body["message"].as_str().unwrap().contains("missing API key"));
    }

    #[tokio::test]
    async fn test_auth_enabled_rejects_invalid_key() {
        let config = AuthConfig::new(vec!["secret-key-123".to_string()]);
        let server = TestServer::new(build_test_app(config));

        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("authorization"),
                HeaderValue::from_static("Bearer wrong-key"),
            )
            .await;
        resp.assert_status(StatusCode::UNAUTHORIZED);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "unauthorized");
        assert!(body["message"].as_str().unwrap().contains("invalid API key"));
    }

    #[tokio::test]
    async fn test_auth_accepts_valid_bearer_token() {
        let config = AuthConfig::new(vec!["secret-key-123".to_string()]);
        let server = TestServer::new(build_test_app(config));

        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("authorization"),
                HeaderValue::from_static("Bearer secret-key-123"),
            )
            .await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn test_auth_accepts_x_api_key_header() {
        let config = AuthConfig::new(vec!["my-api-key".to_string()]);
        let server = TestServer::new(build_test_app(config));

        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_static("my-api-key"),
            )
            .await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn test_excluded_path_bypasses_auth() {
        let config = AuthConfig::new(vec!["secret-key-123".to_string()]);
        let server = TestServer::new(build_test_app(config));

        // /health is excluded by default
        let resp = server.get("/health").await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn test_multiple_api_keys() {
        let config = AuthConfig::new(vec![
            "key-alpha".to_string(),
            "key-beta".to_string(),
        ]);
        let server = TestServer::new(build_test_app(config));

        // Both keys should work
        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_static("key-alpha"),
            )
            .await;
        resp.assert_status_ok();

        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_static("key-beta"),
            )
            .await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn test_custom_excluded_paths() {
        let config = AuthConfig::new(vec!["secret".to_string()])
            .with_excluded_path("/api/v1/metrics");
        let server = TestServer::new(
            Router::new()
                .route("/api/v1/metrics", get(ok_handler))
                .route("/api/v1/send", get(ok_handler))
                .layer(axum::middleware::from_fn_with_state(
                    AuthState::new(config.clone()),
                    auth_middleware,
                ))
                .with_state(AuthState::new(config)),
        );

        // Excluded path works without auth
        let resp = server.get("/api/v1/metrics").await;
        resp.assert_status_ok();

        // Non-excluded path requires auth
        let resp = server.get("/api/v1/send").await;
        resp.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_bearer_prefix_case_sensitivity() {
        let config = AuthConfig::new(vec!["my-key".to_string()]);
        let server = TestServer::new(build_test_app(config));

        // "Bearer" with correct casing
        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("authorization"),
                HeaderValue::from_static("Bearer my-key"),
            )
            .await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn test_auth_config_key_count() {
        let config = AuthConfig::new(vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ]);
        assert_eq!(config.key_count(), 3);
        assert!(config.is_valid_key("a"));
        assert!(!config.is_valid_key("d"));
    }

    #[tokio::test]
    async fn test_empty_bearer_token_rejected() {
        let config = AuthConfig::new(vec!["valid".to_string()]);
        let server = TestServer::new(build_test_app(config));

        let resp = server
            .get("/test")
            .add_header(
                HeaderName::from_static("authorization"),
                HeaderValue::from_static("Bearer "),
            )
            .await;
        resp.assert_status(StatusCode::UNAUTHORIZED);
    }
}
