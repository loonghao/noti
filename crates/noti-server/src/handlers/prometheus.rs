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
/// - `noti_server_version` - Server version
/// - `noti_workers_total` - Total number of workers in the pool
/// - `noti_workers_active` - Number of workers actively processing tasks
/// - `noti_workers_idle` - Number of workers idle and available
/// - `noti_ratelimit_requests_total` - Total requests processed by rate limiter (counter)
/// - `noti_ratelimit_rejected_total` - Requests rejected due to rate limiting (counter)
/// - `noti_ratelimit_tracked_ips` - Number of IPs tracked in per-IP mode (gauge)
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

    let worker_stats = state.worker_stats_handle.as_ref().map(|h| h.stats());
    let rate_limit_metrics = state.rate_limiter.as_ref().map(|r| r.metrics());

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

    // Worker pool metrics (only when workers are started)
    if let Some(workers) = worker_stats {
        output.push_str("# HELP noti_workers_total Total number of workers in the pool\n");
        output.push_str("# TYPE noti_workers_total gauge\n");
        output.push_str(&format!("noti_workers_total {}\n", workers.total));

        output.push_str("# HELP noti_workers_active Number of workers actively processing tasks\n");
        output.push_str("# TYPE noti_workers_active gauge\n");
        output.push_str(&format!("noti_workers_active {}\n", workers.active));

        output.push_str("# HELP noti_workers_idle Number of workers idle and available\n");
        output.push_str("# TYPE noti_workers_idle gauge\n");
        output.push_str(&format!("noti_workers_idle {}\n", workers.idle));
    }

    // Rate limiting metrics (only when rate limiter is enabled)
    if let Some(rl) = rate_limit_metrics {
        output.push_str("# HELP noti_ratelimit_requests_total Total requests processed by rate limiter\n");
        output.push_str("# TYPE noti_ratelimit_requests_total counter\n");
        output.push_str(&format!("noti_ratelimit_requests_total{{per_ip=\"{}\"}} {}\n", rl.per_ip, rl.requests_total));

        output.push_str("# HELP noti_ratelimit_rejected_total Requests rejected due to rate limiting\n");
        output.push_str("# TYPE noti_ratelimit_rejected_total counter\n");
        output.push_str(&format!("noti_ratelimit_rejected_total{{per_ip=\"{}\"}} {}\n", rl.per_ip, rl.rejected_total));

        output.push_str("# HELP noti_ratelimit_tracked_ips Number of IPs currently tracked (per-IP mode)\n");
        output.push_str("# TYPE noti_ratelimit_tracked_ips gauge\n");
        output.push_str(&format!("noti_ratelimit_tracked_ips {}\n", rl.tracked_ips));
    }

    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(output)
        .expect("static headers are always valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use axum_test::TestServer;
    use noti_core::ProviderRegistry;
    use noti_queue::WorkerConfig;

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

    #[tokio::test]
    async fn test_prometheus_metrics_with_worker_stats() {
        let mut state = AppState::new(ProviderRegistry::new());
        let worker_config = WorkerConfig::default().with_concurrency(2);
        let (_worker_handle, worker_stats_handle) = state.start_workers(worker_config);
        state = state.with_worker_handle(std::sync::Arc::new(worker_stats_handle));

        let app = Router::new()
            .route("/metrics", get(prometheus_metrics))
            .with_state(state);
        let server = TestServer::new(app);

        let resp = server.get("/metrics").await;
        resp.assert_status_ok();

        let body = resp.text();
        // Worker metrics should be present when workers are started
        assert!(body.contains("noti_workers_total"), "missing noti_workers_total");
        assert!(body.contains("noti_workers_active"), "missing noti_workers_active");
        assert!(body.contains("noti_workers_idle"), "missing noti_workers_idle");
        // Should show the 2 workers from our config
        assert!(body.contains("noti_workers_total 2"), "expected 2 total workers");
    }

    #[tokio::test]
    async fn test_prometheus_metrics_with_rate_limit_stats() {
        use std::time::Duration;
        use crate::middleware::rate_limit::{RateLimitConfig, RateLimiterState};

        let config = RateLimitConfig::new(100, Duration::from_secs(60));
        let rate_limiter = RateLimiterState::new(config);

        let state = AppState::new(ProviderRegistry::new())
            .with_rate_limiter(rate_limiter);

        let app = Router::new()
            .route("/metrics", get(prometheus_metrics))
            .with_state(state);
        let server = TestServer::new(app);

        let resp = server.get("/metrics").await;
        resp.assert_status_ok();

        let body = resp.text();
        // Rate limit metrics should be present
        assert!(body.contains("noti_ratelimit_requests_total"), "missing noti_ratelimit_requests_total");
        assert!(body.contains("noti_ratelimit_rejected_total"), "missing noti_ratelimit_rejected_total");
        assert!(body.contains("noti_ratelimit_tracked_ips"), "missing noti_ratelimit_tracked_ips");
        // Should show per_ip="true" for the config we used
        assert!(body.contains("per_ip=\"true\""), "expected per_ip=\"true\"");
    }
}
