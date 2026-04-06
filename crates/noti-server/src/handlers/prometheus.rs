//! Prometheus-compatible metrics endpoint.
//!
//! Returns metrics in Prometheus text format for direct scraping by Prometheus servers.

use axum::extract::State;
use axum::response::IntoResponse;

use crate::state::AppState;

/// Prometheus text-format metrics endpoint.
///
/// Returns operational metrics in Prometheus exposition format.
/// This endpoint is designed to be scraped directly by Prometheus servers.
///
/// # Metrics Exposed
/// - `noti_queue_total` - Total tasks by status (queued, processing, completed, failed, cancelled)
/// - `noti_queue_priority_total` - Tasks broken down by priority level
/// - `noti_providers_registered` - Number of registered notification providers
/// - `noti_providers_with_attachments` - Number of providers supporting attachments
/// - `noti_server_uptime_seconds` - Server uptime in seconds
///
/// # Example
/// ```text
/// # HELP noti_queue_total Total tasks in queue by status
/// # TYPE noti_queue_total gauge
/// noti_queue_total{status="queued"} 10
/// noti_queue_total{status="processing"} 2
/// noti_queue_total{status="completed"} 150
/// ```
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Monitoring",
    responses(
        (status = 200, description = "Prometheus metrics in text format")
    )
)]
pub async fn prometheus_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let queue_stats = state.queue.stats().await.unwrap_or_default();
    let all_providers = state.registry.all_providers();
    let attachment_count = all_providers.iter().filter(|p| p.supports_attachments()).count();
    let uptime = state.started_at.elapsed().unwrap_or_default().as_secs();

    let mut output = String::new();

    // Queue metrics
    output.push_str("# HELP noti_queue_total Total tasks in queue by status\n");
    output.push_str("# TYPE noti_queue_total gauge\n");
    output.push_str(&format!("noti_queue_total{{status=\"queued\"}} {}\n", queue_stats.queued));
    output.push_str(&format!("noti_queue_total{{status=\"processing\"}} {}\n", queue_stats.processing));
    output.push_str(&format!("noti_queue_total{{status=\"completed\"}} {}\n", queue_stats.completed));
    output.push_str(&format!("noti_queue_total{{status=\"failed\"}} {}\n", queue_stats.failed));
    output.push_str(&format!("noti_queue_total{{status=\"cancelled\"}} {}\n", queue_stats.cancelled));

    // Provider metrics
    output.push_str("# HELP noti_providers_registered Number of registered providers\n");
    output.push_str("# TYPE noti_providers_registered gauge\n");
    output.push_str(&format!("noti_providers_registered {}\n", all_providers.len()));

    output.push_str("# HELP noti_providers_with_attachments Number of providers supporting attachments\n");
    output.push_str("# TYPE noti_providers_with_attachments gauge\n");
    output.push_str(&format!("noti_providers_with_attachments {}\n", attachment_count));

    // Uptime
    output.push_str("# HELP noti_server_uptime_seconds Server uptime in seconds\n");
    output.push_str("# TYPE noti_server_uptime_seconds gauge\n");
    output.push_str(&format!("noti_server_uptime_seconds {}\n", uptime));

    // Version
    output.push_str("# HELP noti_server_version Server version\n");
    output.push_str("# TYPE noti_server_version gauge\n");
    output.push_str(&format!("noti_server_version{{version=\"{}\"}} 1\n", env!("CARGO_PKG_VERSION")));

    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(output)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use axum_test::TestServer;
    use noti_core::ProviderRegistry;

    #[tokio::test]
    async fn test_prometheus_metrics_endpoint() {
        let state = AppState::new(ProviderRegistry::new());
        let app = Router::new()
            .route("/metrics", get(prometheus_metrics))
            .with_state(state);
        let server = TestServer::new(app);

        let resp = server.get("/metrics").await;
        resp.assert_status_ok();
        assert_eq!(
            resp.content_type(),
            "text/plain; version=0.0.4; charset=utf-8"
        );

        let body = resp.text();
        assert!(body.contains("noti_queue_total"));
        assert!(body.contains("noti_providers_registered"));
        assert!(body.contains("noti_server_uptime_seconds"));
        assert!(body.contains("noti_server_version"));
    }
}
