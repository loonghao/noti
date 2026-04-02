use axum::Router;
use axum::routing::{get, post};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers;
use crate::openapi::ApiDoc;
use crate::state::AppState;

/// Available API versions.
pub const API_VERSIONS: &[ApiVersion] = &[ApiVersion {
    version: "v1",
    status: "stable",
    deprecated: false,
}];

/// Metadata describing a single API version.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ApiVersion {
    /// Version identifier (e.g. "v1").
    pub version: &'static str,
    /// Lifecycle status: "stable", "beta", or "deprecated".
    pub status: &'static str,
    /// Whether this version is deprecated and scheduled for removal.
    pub deprecated: bool,
}

/// Response for `GET /api/versions`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ApiVersionsResponse {
    /// List of available API versions.
    pub versions: Vec<ApiVersion>,
    /// The recommended version for new integrations.
    pub latest: String,
}

/// List available API versions.
#[utoipa::path(
    get,
    path = "/api/versions",
    tag = "Meta",
    responses(
        (status = 200, description = "Available API versions", body = ApiVersionsResponse),
    )
)]
pub async fn list_api_versions() -> axum::Json<ApiVersionsResponse> {
    axum::Json(ApiVersionsResponse {
        versions: API_VERSIONS.to_vec(),
        latest: "v1".to_string(),
    })
}

/// Build the v1 API routes (without the `/api/v1` prefix — that is applied via `nest`).
fn build_v1_routes() -> Router<AppState> {
    Router::new()
        // Metrics endpoint
        .route("/metrics", get(handlers::metrics::get_metrics))
        // Synchronous notification endpoints
        .route("/send", post(handlers::send::send_notification))
        .route("/send/batch", post(handlers::send::send_batch))
        // Async queue-based notification
        .route("/send/async", post(handlers::queue::send_async))
        .route("/send/async/batch", post(handlers::queue::send_async_batch))
        // Status endpoints
        .route(
            "/status/{notification_id}",
            get(handlers::status::get_status),
        )
        .route("/status", get(handlers::status::get_all_statuses))
        .route("/status/purge", post(handlers::status::purge_statuses))
        // Template endpoints
        .route("/templates", post(handlers::templates::create_template))
        .route("/templates", get(handlers::templates::list_templates))
        .route(
            "/templates/{name}",
            get(handlers::templates::get_template)
                .put(handlers::templates::update_template)
                .delete(handlers::templates::delete_template),
        )
        .route(
            "/templates/{name}/render",
            post(handlers::templates::render_template),
        )
        // Provider info endpoints
        .route("/providers", get(handlers::providers::list_providers))
        .route(
            "/providers/{name}",
            get(handlers::providers::get_provider),
        )
        // Queue management endpoints
        .route("/queue/stats", get(handlers::queue::get_stats))
        .route("/queue/tasks", get(handlers::queue::list_tasks))
        .route("/queue/tasks/{task_id}", get(handlers::queue::get_task))
        .route(
            "/queue/tasks/{task_id}/cancel",
            post(handlers::queue::cancel_task),
        )
        .route("/queue/purge", post(handlers::queue::purge_tasks))
}

/// Build the application router with all API routes and Swagger UI.
///
/// The versioned API is mounted under `/api/v1` using [`Router::nest`],
/// making it straightforward to add `/api/v2` in the future without
/// touching existing v1 handlers.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Health check (version-independent)
        .route("/health", get(handlers::health::health_check))
        // API version discovery (version-independent)
        .route("/api/versions", get(list_api_versions))
        // Mount v1 API under /api/v1
        .nest("/api/v1", build_v1_routes())
        .with_state(state)
        // Swagger UI and OpenAPI spec (stateless, merged after with_state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}
