use noti_core::ProviderRegistry;
use noti_queue::WorkerConfig;
use noti_server::config::ServerConfig;
use noti_server::middleware::auth::{AuthState, auth_middleware};
use noti_server::middleware::rate_limit::{RateLimiterState, rate_limit_middleware};
use noti_server::state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Load configuration from environment variables
    let config = ServerConfig::from_env();

    // Initialize tracing with configured log level
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .init();

    tracing::info!("loaded configuration from environment");

    // Build provider registry
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = AppState::new(registry);

    // Start background worker pool
    let worker_config = WorkerConfig {
        concurrency: config.worker_count,
        ..Default::default()
    };
    let worker_handle = state.start_workers(worker_config);
    tracing::info!(workers = config.worker_count, "queue worker pool started");

    // Auth middleware
    let auth_state = AuthState::new(config.auth.clone());
    if auth_state.is_enabled() {
        tracing::info!(
            keys = config.auth.key_count(),
            "API key authentication enabled"
        );
    } else {
        tracing::info!("API key authentication disabled (no NOTI_API_KEYS set)");
    }

    // Rate limiter
    let rate_limiter = RateLimiterState::new(config.rate_limit.clone());
    tracing::info!("rate limiter enabled");

    // Build application with middleware stack (outermost first)
    // Order: CORS → Trace → Auth → Rate-limit → Router
    let app = noti_server::routes::build_router(state)
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = config.socket_addr();
    tracing::info!(%addr, "noti-server listening");

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
