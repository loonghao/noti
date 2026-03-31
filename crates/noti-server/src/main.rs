use axum::extract::DefaultBodyLimit;

use noti_core::ProviderRegistry;
use noti_queue::WorkerConfig;
use noti_server::config::{LogFormat, ServerConfig};
use noti_server::middleware::auth::{AuthState, auth_middleware};
use noti_server::middleware::rate_limit::{RateLimiterState, rate_limit_middleware};
use noti_server::middleware::request_id::request_id_middleware;
use noti_server::state::AppState;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Load configuration from environment variables
    let config = ServerConfig::from_env();

    // Initialize tracing with configured log level and format
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| config.log_level.clone().into());

    match config.log_format {
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(env_filter)
                .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
                .flatten_event(true)
                .with_current_span(true)
                .init();
        }
        LogFormat::Text => {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .init();
        }
    }

    tracing::info!("loaded configuration from environment");

    // Build provider registry
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);

    let state = AppState::with_queue_backend(registry, &config.queue_backend, &config.queue_db_path)
        .await
        .expect("failed to initialize queue backend");
    tracing::info!(backend = ?config.queue_backend, "queue backend initialized");

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

    // CORS configuration
    let cors_layer = if config.cors_allowed_origins.is_empty()
        || config.cors_allowed_origins.iter().any(|o| o == "*")
    {
        tracing::info!("CORS: permissive (allow all origins)");
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = config
            .cors_allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        tracing::info!(origins = ?config.cors_allowed_origins, "CORS: restricted origins");
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Build application with middleware stack (outermost first)
    // Order: CORS → Trace → RequestId → Auth → Rate-limit → BodyLimit → Router
    let app = noti_server::routes::build_router(state)
        .layer(DefaultBodyLimit::max(config.max_body_size))
        .layer(axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth_middleware,
        ))
        .layer(axum::middleware::from_fn(request_id_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer);

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
