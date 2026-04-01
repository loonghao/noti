//! Shared test helpers for noti-server e2e tests.
//!
//! Provides reusable server spawn helpers, mock providers, callback infrastructure,
//! and polling utilities so that individual test files stay focused on assertions.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::post as axum_post;
use noti_core::ProviderRegistry;
use noti_server::middleware::auth::{AuthConfig, AuthState, auth_middleware};
use noti_server::middleware::rate_limit::{
    RateLimitConfig, RateLimiterState, rate_limit_middleware,
};
use noti_server::middleware::request_id::request_id_middleware;
use reqwest::StatusCode;
use serde_json::Value;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

// ───────────────────── Core bind-and-serve helper ─────────────────────

/// Bind to a random port, spawn the server, and return the base URL.
async fn bind_and_serve(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

/// Bind to a random port using `into_make_service_with_connect_info` (needed for per-IP rate limiting).
async fn bind_and_serve_with_connect_info(app: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind to random port");
    let addr: SocketAddr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    format!("http://{addr}")
}

/// Create a default `AppState` with all real providers registered.
fn default_app_state() -> noti_server::state::AppState {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);
    noti_server::state::AppState::new(registry)
}

// ───────────────────── Server spawn helpers ─────────────────────

/// Start a real HTTP server on a random port and return the base URL.
pub async fn spawn_server() -> String {
    let state = default_app_state();
    let app = noti_server::routes::build_router(state);
    bind_and_serve(app).await
}

/// Start a server with auth middleware enabled.
/// Returns `(base_url, valid_api_keys)`.
pub async fn spawn_server_with_auth(api_keys: Vec<String>) -> (String, Vec<String>) {
    let state = default_app_state();
    let auth_config = AuthConfig::new(api_keys.clone());
    let auth_state = AuthState::new(auth_config);

    let app = noti_server::routes::build_router(state).layer(axum::middleware::from_fn_with_state(
        auth_state,
        auth_middleware,
    ));

    let base = bind_and_serve(app).await;
    (base, api_keys)
}

/// Start a server with rate limit middleware enabled (global mode).
/// Returns `(base_url, max_requests)`.
pub async fn spawn_server_with_rate_limit(max_requests: u64, window_secs: u64) -> (String, u64) {
    let state = default_app_state();
    let rate_config =
        RateLimitConfig::new(max_requests, Duration::from_secs(window_secs)).with_per_ip(false);
    let rate_state = RateLimiterState::new(rate_config);

    let app = noti_server::routes::build_router(state).layer(axum::middleware::from_fn_with_state(
        rate_state,
        rate_limit_middleware,
    ));

    let base = bind_and_serve(app).await;
    (base, max_requests)
}

/// Start a server with per-IP rate limiting enabled.
/// Uses `into_make_service_with_connect_info` so `ConnectInfo` is populated.
/// Returns `(base_url, max_requests_per_ip)`.
pub async fn spawn_server_with_rate_limit_per_ip(
    max_requests: u64,
    window_secs: u64,
) -> (String, u64) {
    let state = default_app_state();
    let rate_config =
        RateLimitConfig::new(max_requests, Duration::from_secs(window_secs)).with_per_ip(true);
    let rate_state = RateLimiterState::new(rate_config);

    let app = noti_server::routes::build_router(state).layer(axum::middleware::from_fn_with_state(
        rate_state,
        rate_limit_middleware,
    ));

    let base = bind_and_serve_with_connect_info(app).await;
    (base, max_requests)
}

/// Start a server with both auth and rate limit middleware.
/// Returns `(base_url, valid_api_keys)`.
pub async fn spawn_server_with_full_middleware(
    api_keys: Vec<String>,
    max_requests: u64,
    window_secs: u64,
) -> (String, Vec<String>) {
    let state = default_app_state();
    let auth_config = AuthConfig::new(api_keys.clone());
    let auth_state = AuthState::new(auth_config);
    let rate_config =
        RateLimitConfig::new(max_requests, Duration::from_secs(window_secs)).with_per_ip(false);
    let rate_state = RateLimiterState::new(rate_config);

    let app = noti_server::routes::build_router(state)
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            rate_state,
            rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            auth_middleware,
        ));

    let base = bind_and_serve(app).await;
    (base, api_keys)
}

/// Start a server with a custom body size limit.
/// Returns `(base_url, max_bytes)`.
pub async fn spawn_server_with_body_limit(max_bytes: usize) -> (String, usize) {
    let state = default_app_state();
    let app = noti_server::routes::build_router(state).layer(DefaultBodyLimit::max(max_bytes));

    let base = bind_and_serve(app).await;
    (base, max_bytes)
}

/// Start a server with request-id middleware.
pub async fn spawn_server_with_request_id() -> String {
    let state = default_app_state();
    let app = noti_server::routes::build_router(state)
        .layer(axum::middleware::from_fn(request_id_middleware));

    bind_and_serve(app).await
}

/// Start a server with permissive CORS.
pub async fn spawn_server_with_cors_permissive() -> String {
    let state = default_app_state();
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = noti_server::routes::build_router(state).layer(cors_layer);
    bind_and_serve(app).await
}

/// Start a server with restricted CORS (specific origins only).
pub async fn spawn_server_with_cors_restricted(allowed_origins: Vec<String>) -> String {
    let state = default_app_state();
    let origins: Vec<axum::http::HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any);

    let app = noti_server::routes::build_router(state).layer(cors_layer);
    bind_and_serve(app).await
}

/// Start a noti server with mock providers and background workers enabled.
/// Workers will actually process queued tasks.
/// Returns `(base_url, worker_handle)`.
pub async fn spawn_server_with_workers() -> (String, noti_queue::WorkerHandle) {
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));
    registry.register(Arc::new(MockFailProvider));

    let state = noti_server::state::AppState::new(registry);
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(2)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let app = noti_server::routes::build_router(state);
    let base = bind_and_serve(app).await;
    (base, worker_handle)
}

/// Start a noti server with a single worker (serial processing) and all mock providers.
/// The single worker ensures tasks are dequeued in strict priority order.
/// Returns `(base_url, worker_handle)`.
pub async fn spawn_server_with_workers_serial(
    extra_providers: Vec<Arc<dyn noti_core::NotifyProvider>>,
) -> (String, noti_queue::WorkerHandle) {
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));
    registry.register(Arc::new(MockFailProvider));
    for p in extra_providers {
        registry.register(p);
    }

    let state = noti_server::state::AppState::new(registry);
    // Single worker ensures sequential processing in priority order.
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let app = noti_server::routes::build_router(state);
    let base = bind_and_serve(app).await;
    (base, worker_handle)
}

// ───────────────────── SQLite queue backend helpers ─────────────────────

/// Create an `AppState` backed by an in-memory SQLite queue (no file I/O).
fn sqlite_app_state_with_registry(registry: ProviderRegistry) -> noti_server::state::AppState {
    let queue = Arc::new(
        noti_queue::SqliteQueue::in_memory().expect("failed to create in-memory SQLite queue"),
    );
    let task_notify = queue.notifier();
    noti_server::state::AppState::with_custom_queue(registry, queue, task_notify)
}

/// Start a real HTTP server backed by an in-memory SQLite queue.
/// Returns the base URL.
pub async fn spawn_server_sqlite() -> String {
    let mut registry = ProviderRegistry::new();
    noti_providers::register_all_providers(&mut registry);
    let state = sqlite_app_state_with_registry(registry);
    let app = noti_server::routes::build_router(state);
    bind_and_serve(app).await
}

/// Start a server with in-memory SQLite queue and mock providers + workers.
/// Returns `(base_url, worker_handle)`.
pub async fn spawn_server_sqlite_with_workers() -> (String, noti_queue::WorkerHandle) {
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));
    registry.register(Arc::new(MockFailProvider));

    let state = sqlite_app_state_with_registry(registry);
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(2)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let app = noti_server::routes::build_router(state);
    let base = bind_and_serve(app).await;
    (base, worker_handle)
}

/// Start a server with in-memory SQLite queue, mock providers, single worker (serial).
/// Returns `(base_url, worker_handle)`.
pub async fn spawn_server_sqlite_with_workers_serial(
    extra_providers: Vec<Arc<dyn noti_core::NotifyProvider>>,
) -> (String, noti_queue::WorkerHandle) {
    let mut registry = noti_core::ProviderRegistry::new();
    registry.register(Arc::new(MockOkProvider));
    registry.register(Arc::new(MockFailProvider));
    for p in extra_providers {
        registry.register(p);
    }

    let state = sqlite_app_state_with_registry(registry);
    let worker_config = noti_queue::WorkerConfig::default()
        .with_concurrency(1)
        .with_poll_interval(Duration::from_millis(50));
    let worker_handle = state.start_workers(worker_config);

    let app = noti_server::routes::build_router(state);
    let base = bind_and_serve(app).await;
    (base, worker_handle)
}

// ───────────────────── Mock providers ─────────────────────

/// A mock provider that always succeeds.
pub struct MockOkProvider;

#[async_trait]
impl noti_core::NotifyProvider for MockOkProvider {
    fn name(&self) -> &str {
        "mock-ok"
    }
    fn url_scheme(&self) -> &str {
        "mock-ok"
    }
    fn params(&self) -> Vec<noti_core::ParamDef> {
        vec![]
    }
    fn description(&self) -> &str {
        "always succeeds"
    }
    fn example_url(&self) -> &str {
        "mock-ok://test"
    }
    async fn send(
        &self,
        _message: &noti_core::Message,
        _config: &noti_core::ProviderConfig,
    ) -> Result<noti_core::SendResponse, noti_core::NotiError> {
        Ok(noti_core::SendResponse::success("mock-ok", "ok"))
    }
}

/// A mock provider that always fails (returns an error).
pub struct MockFailProvider;

#[async_trait]
impl noti_core::NotifyProvider for MockFailProvider {
    fn name(&self) -> &str {
        "mock-fail"
    }
    fn url_scheme(&self) -> &str {
        "mock-fail"
    }
    fn params(&self) -> Vec<noti_core::ParamDef> {
        vec![]
    }
    fn description(&self) -> &str {
        "always fails"
    }
    fn example_url(&self) -> &str {
        "mock-fail://test"
    }
    async fn send(
        &self,
        _message: &noti_core::Message,
        _config: &noti_core::ProviderConfig,
    ) -> Result<noti_core::SendResponse, noti_core::NotiError> {
        Err(noti_core::NotiError::Network("simulated failure".into()))
    }
}

/// A mock provider that fails the first N calls then succeeds.
pub struct MockFlakyProvider {
    fail_count: u32,
    call_counter: AtomicU32,
}

impl MockFlakyProvider {
    pub fn new(fail_count: u32) -> Self {
        Self {
            fail_count,
            call_counter: AtomicU32::new(0),
        }
    }
}

#[async_trait]
impl noti_core::NotifyProvider for MockFlakyProvider {
    fn name(&self) -> &str {
        "mock-flaky"
    }
    fn url_scheme(&self) -> &str {
        "mock-flaky"
    }
    fn params(&self) -> Vec<noti_core::ParamDef> {
        vec![]
    }
    fn description(&self) -> &str {
        "fails first N calls then succeeds"
    }
    fn example_url(&self) -> &str {
        "mock-flaky://test"
    }
    async fn send(
        &self,
        _message: &noti_core::Message,
        _config: &noti_core::ProviderConfig,
    ) -> Result<noti_core::SendResponse, noti_core::NotiError> {
        let call = self.call_counter.fetch_add(1, AtomicOrdering::SeqCst);
        if call < self.fail_count {
            Err(noti_core::NotiError::Network(format!(
                "flaky failure #{}",
                call + 1
            )))
        } else {
            Ok(noti_core::SendResponse::success(
                "mock-flaky",
                "ok after retries",
            ))
        }
    }
}

// ───────────────────── Callback server infrastructure ─────────────────────

/// Shared state for the mock callback receiver.
#[derive(Clone)]
pub struct CallbackReceiverState {
    pub payloads: Arc<Mutex<Vec<Value>>>,
}

/// Handler that records incoming callback payloads.
async fn callback_handler(
    axum::extract::State(state): axum::extract::State<CallbackReceiverState>,
    axum::Json(payload): axum::Json<Value>,
) -> StatusCode {
    state.payloads.lock().unwrap().push(payload);
    StatusCode::OK
}

/// Start a mock HTTP server that records POST payloads at `/callback`.
/// Returns `(base_url, shared_payloads)`.
pub async fn spawn_callback_server() -> (String, Arc<Mutex<Vec<Value>>>) {
    let payloads: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let state = CallbackReceiverState {
        payloads: payloads.clone(),
    };

    let app = Router::new()
        .route("/callback", axum_post(callback_handler))
        .with_state(state);

    let base = bind_and_serve(app).await;
    (base, payloads)
}

// ───────────────────── Polling utilities ─────────────────────

/// Poll a task until it reaches a terminal state (`completed`, `failed`, or `cancelled`).
/// Panics if the task does not reach a terminal state within 5 seconds.
pub async fn wait_for_terminal_status(
    client: &reqwest::Client,
    base: &str,
    task_id: &str,
) -> Value {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(5);

    loop {
        let resp = client
            .get(format!("{base}/api/v1/queue/tasks/{task_id}"))
            .send()
            .await
            .unwrap();
        let body: Value = resp.json().await.unwrap();
        let status = body["status"].as_str().unwrap_or("");

        if matches!(status, "completed" | "failed" | "cancelled") {
            return body;
        }

        if start.elapsed() > timeout {
            panic!(
                "task {task_id} did not reach terminal state within {timeout:?}, last status: {status}"
            );
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
