use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

/// Health check response with dependency status.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Overall service status: `"ok"` or `"degraded"`.
    pub status: String,
    /// Server version.
    pub version: &'static str,
    /// Uptime in seconds.
    pub uptime_seconds: u64,
    /// Dependency health details.
    pub dependencies: DependencyHealth,
}

/// Per-dependency health status.
#[derive(Debug, Serialize, ToSchema)]
pub struct DependencyHealth {
    /// Queue backend health.
    pub queue: ComponentHealth,
    /// Provider registry health.
    pub providers: ComponentHealth,
}

/// Health status for a single component.
#[derive(Debug, Serialize, ToSchema)]
pub struct ComponentHealth {
    /// `"up"` or `"down"`.
    pub status: String,
    /// Optional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Health check endpoint with dependency status.
///
/// Returns `200 OK` when the service is healthy, including queue and provider
/// readiness. If any dependency is unhealthy the overall status becomes
/// `"degraded"` but the endpoint still returns `200` so load-balancers can
/// distinguish between "process alive" and "fully down".
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "Service health status", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let uptime = state
        .started_at
        .elapsed()
        .unwrap_or_default()
        .as_secs();

    // Check queue health by requesting stats.
    let queue_health = match state.queue.stats().await {
        Ok(stats) => ComponentHealth {
            status: "up".to_string(),
            detail: Some(format!(
                "queued={} processing={} completed={}",
                stats.queued, stats.processing, stats.completed,
            )),
        },
        Err(e) => ComponentHealth {
            status: "down".to_string(),
            detail: Some(e.to_string()),
        },
    };

    // Check provider registry health.
    let provider_count = state.registry.all_providers().len();
    let providers_health = ComponentHealth {
        status: if provider_count > 0 { "up" } else { "down" }.to_string(),
        detail: Some(format!("{} registered", provider_count)),
    };

    let all_up = queue_health.status == "up" && providers_health.status == "up";

    Json(HealthResponse {
        status: if all_up { "ok" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: uptime,
        dependencies: DependencyHealth {
            queue: queue_health,
            providers: providers_health,
        },
    })
}
