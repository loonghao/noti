use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::AppState;

/// Prometheus-compatible metrics response (in JSON format).
///
/// Provides key operational metrics for monitoring dashboards.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MetricsResponse {
    /// Queue statistics.
    pub queue: QueueMetrics,
    /// Provider statistics.
    pub providers: ProviderMetrics,
    /// Server uptime in seconds.
    pub uptime_seconds: u64,
    /// Server version.
    pub version: String,
}

/// Queue-related metrics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueueMetrics {
    pub queued: usize,
    pub processing: usize,
    pub completed: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub total: usize,
}

/// Provider-related metrics.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProviderMetrics {
    /// Total registered providers.
    pub total_registered: usize,
    /// Number of providers that support attachments.
    pub with_attachment_support: usize,
}

/// Get operational metrics for monitoring.
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    tag = "Monitoring",
    responses(
        (status = 200, description = "Server metrics", body = MetricsResponse)
    )
)]
pub async fn get_metrics(State(state): State<AppState>) -> Json<MetricsResponse> {
    let queue_stats = state.queue.stats().await.unwrap_or_default();
    let all_providers = state.registry.all_providers();
    let attachment_count = all_providers
        .iter()
        .filter(|p| p.supports_attachments())
        .count();

    let uptime = state
        .started_at
        .elapsed()
        .unwrap_or_default()
        .as_secs();

    Json(MetricsResponse {
        queue: QueueMetrics {
            queued: queue_stats.queued,
            processing: queue_stats.processing,
            completed: queue_stats.completed,
            failed: queue_stats.failed,
            cancelled: queue_stats.cancelled,
            total: queue_stats.total(),
        },
        providers: ProviderMetrics {
            total_registered: all_providers.len(),
            with_attachment_support: attachment_count,
        },
        uptime_seconds: uptime,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use axum_test::TestServer;
    use noti_core::ProviderRegistry;

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let state = AppState::new(ProviderRegistry::new());
        let app = Router::new()
            .route("/api/v1/metrics", get(get_metrics))
            .with_state(state);
        let server = TestServer::new(app);

        let resp = server.get("/api/v1/metrics").await;
        resp.assert_status_ok();

        let body: MetricsResponse = resp.json();
        assert_eq!(body.queue.total, 0);
        assert_eq!(body.providers.total_registered, 0);
        assert!(!body.version.is_empty());
    }

    #[tokio::test]
    async fn test_metrics_with_providers() {
        let mut registry = ProviderRegistry::new();
        noti_providers::register_all_providers(&mut registry);
        let state = AppState::new(registry);

        let app = Router::new()
            .route("/api/v1/metrics", get(get_metrics))
            .with_state(state);
        let server = TestServer::new(app);

        let resp = server.get("/api/v1/metrics").await;
        resp.assert_status_ok();

        let body: MetricsResponse = resp.json();
        assert!(body.providers.total_registered > 100);
        assert!(body.providers.with_attachment_support > 0);
        assert!(body.uptime_seconds < 5); // just started
    }
}
