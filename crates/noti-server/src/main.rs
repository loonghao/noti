use std::net::SocketAddr;

use noti_core::ProviderRegistry;
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

    let app = noti_server::routes::build_router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("noti-server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
