use std::net::SocketAddr;
use std::time::Duration;

use noti_core::ProviderRegistry;
use noti_queue::WorkerConfig;
use noti_server::middleware::rate_limit::{RateLimitConfig, RateLimiterState, rate_limit_middleware};
use noti_server::state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    // Build provider registry
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = AppState::new(registry);

    // Start background worker pool (4 concurrent workers by default)
    let worker_handle = state.start_workers(WorkerConfig::default());
    tracing::info!("queue worker pool started");

    // Rate limiter: 100 requests per minute per IP
    let rate_limiter = RateLimiterState::new(
        RateLimitConfig::new(100, Duration::from_secs(60)).with_per_ip(true),
    );
    tracing::info!("rate limiter enabled (100 req/min per IP)");

    let app = noti_server::routes::build_router(state)
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("noti-server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    // Graceful shutdown of workers (reached after shutdown signal)
    tracing::info!("shutting down worker pool...");
    worker_handle.shutdown_and_join().await;
    tracing::info!("worker pool stopped, server exiting");
}

/// Wait for a shutdown signal (Ctrl+C or SIGTERM on Unix).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received Ctrl+C, starting graceful shutdown"),
        () = terminate => tracing::info!("received SIGTERM, starting graceful shutdown"),
    }
}
