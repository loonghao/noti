//! Prometheus-compatible metrics endpoint.
//!
//! Returns metrics in Prometheus text format for direct scraping by Prometheus servers.

use axum::extract::State;
use axum::response::IntoResponse;
use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;

use crate::state::AppState;

/// Label set for queue status metrics (e.g. queued, completed).
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct QueueStatusLabel {
    status: String,
}

impl QueueStatusLabel {
    fn new(status: &str) -> Self {
        QueueStatusLabel {
            status: status.to_string(),
        }
    }
}

/// Label set for per-IP rate limiting metrics.
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct PerIpLabel {
    per_ip: String,
}

impl PerIpLabel {
    fn new(per_ip: bool) -> Self {
        PerIpLabel {
            per_ip: if per_ip { "true".to_string() } else { "false".to_string() },
        }
    }
}

/// Prometheus text-format metrics endpoint.
///
/// Returns operational metrics in Prometheus exposition format.
/// This endpoint is designed to be scraped directly by Prometheus servers.
///
/// # Metrics Exposed
/// - `noti_queue_total` - Total tasks by status (queued, processing, completed, failed, cancelled)
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

    let mut registry = Registry::default();

    // Queue metrics — Family with status label
    let queue_total: Family<QueueStatusLabel, Gauge> = Family::default();
    registry.register(
        "noti_queue_total",
        "Total tasks in queue by status",
        queue_total.clone(),
    );
    queue_total
        .get_or_create(&QueueStatusLabel::new("queued"))
        .set(queue_stats.queued as i64);
    queue_total
        .get_or_create(&QueueStatusLabel::new("processing"))
        .set(queue_stats.processing as i64);
    queue_total
        .get_or_create(&QueueStatusLabel::new("completed"))
        .set(queue_stats.completed as i64);
    queue_total
        .get_or_create(&QueueStatusLabel::new("failed"))
        .set(queue_stats.failed as i64);
    queue_total
        .get_or_create(&QueueStatusLabel::new("cancelled"))
        .set(queue_stats.cancelled as i64);

    // Provider metrics — plain gauges
    let providers_registered: Gauge = Gauge::default();
    registry.register(
        "noti_providers_registered",
        "Number of registered providers",
        providers_registered.clone(),
    );
    providers_registered.set(all_providers.len() as i64);

    let providers_with_attachments: Gauge = Gauge::default();
    registry.register(
        "noti_providers_with_attachments",
        "Number of providers supporting attachments",
        providers_with_attachments.clone(),
    );
    providers_with_attachments.set(attachment_count as i64);

    // Uptime gauge
    let uptime_gauge: Gauge = Gauge::default();
    registry.register(
        "noti_server_uptime_seconds",
        "Server uptime in seconds",
        uptime_gauge.clone(),
    );
    uptime_gauge.set(uptime as i64);

    // Version gauge
    let version_gauge: Gauge = Gauge::default();
    registry.register(
        "noti_server_version",
        "Server version",
        version_gauge.clone(),
    );
    version_gauge.set(1);

    // Worker pool metrics (only when workers are started)
    if let Some(workers) = worker_stats {
        let workers_total: Gauge = Gauge::default();
        registry.register(
            "noti_workers_total",
            "Total number of workers in the pool",
            workers_total.clone(),
        );
        workers_total.set(workers.total as i64);

        let workers_active: Gauge = Gauge::default();
        registry.register(
            "noti_workers_active",
            "Number of workers actively processing tasks",
            workers_active.clone(),
        );
        workers_active.set(workers.active as i64);

        let workers_idle: Gauge = Gauge::default();
        registry.register(
            "noti_workers_idle",
            "Number of workers idle and available",
            workers_idle.clone(),
        );
        workers_idle.set(workers.idle as i64);
    }

    // Rate limiting metrics (only when rate limiter is enabled)
    if let Some(rl) = rate_limit_metrics {
        let rl_requests_total: Family<PerIpLabel, Gauge> = Family::default();
        registry.register(
            "noti_ratelimit_requests_total",
            "Total requests processed by rate limiter",
            rl_requests_total.clone(),
        );
        rl_requests_total
            .get_or_create(&PerIpLabel::new(rl.per_ip))
            .set(rl.requests_total as i64);

        let rl_rejected_total: Family<PerIpLabel, Gauge> = Family::default();
        registry.register(
            "noti_ratelimit_rejected_total",
            "Requests rejected due to rate limiting",
            rl_rejected_total.clone(),
        );
        rl_rejected_total
            .get_or_create(&PerIpLabel::new(rl.per_ip))
            .set(rl.rejected_total as i64);

        let rl_tracked_ips: Gauge = Gauge::default();
        registry.register(
            "noti_ratelimit_tracked_ips",
            "Number of IPs currently tracked (per-IP mode)",
            rl_tracked_ips.clone(),
        );
        rl_tracked_ips.set(rl.tracked_ips as i64);
    }

    // Encode to Prometheus text format
    let mut output = String::new();
    encode(&mut output, &registry).expect("prometheus-client encoding should not fail");

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
        use crate::middleware::rate_limit::{RateLimitConfig, RateLimiterState};
        use std::time::Duration;

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
        assert!(
            body.contains("noti_ratelimit_requests_total"),
            "missing noti_ratelimit_requests_total"
        );
        assert!(
            body.contains("noti_ratelimit_rejected_total"),
            "missing noti_ratelimit_rejected_total"
        );
        assert!(
            body.contains("noti_ratelimit_tracked_ips"),
            "missing noti_ratelimit_tracked_ips"
        );
    }
}
