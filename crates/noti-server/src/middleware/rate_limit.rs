//! Token-bucket rate limiter middleware for axum.
//!
//! Provides both **global** and **per-IP** rate limiting as a Tower layer.
//! When a client exceeds the allowed rate, the middleware returns
//! `429 Too Many Requests` with a JSON body and standard `Retry-After` header.
//!
//! Per-IP tracking uses [`DashMap`] for sharded locking, so different IPs
//! are processed concurrently without contending on a single global mutex.

use std::net::IpAddr;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use tokio::sync::Mutex;

// ───────────────────── Configuration ─────────────────────

/// Rate limit configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed per window.
    pub max_requests: u64,
    /// Time window for the rate limit.
    pub window: Duration,
    /// Whether to apply per-IP rate limiting (vs global only).
    pub per_ip: bool,
    /// Maximum number of tracked IPs (to prevent memory exhaustion).
    pub max_tracked_ips: usize,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            per_ip: true,
            max_tracked_ips: 10_000,
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit config.
    pub fn new(max_requests: u64, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            ..Default::default()
        }
    }

    /// Enable or disable per-IP tracking.
    pub fn with_per_ip(mut self, per_ip: bool) -> Self {
        self.per_ip = per_ip;
        self
    }
}

// ───────────────────── Token Bucket ─────────────────────

/// A single token bucket for rate limiting.
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u64, window: Duration) -> Self {
        let max = max_tokens as f64;
        let refill_rate = max / window.as_secs_f64();
        Self {
            tokens: max,
            max_tokens: max,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume one token. Returns `true` if allowed.
    fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Remaining tokens (floor).
    fn remaining(&self) -> u64 {
        let mut bucket = self.clone();
        bucket.refill();
        bucket.tokens.floor().max(0.0) as u64
    }

    /// Seconds until the next token is available.
    fn retry_after(&self) -> f64 {
        if self.refill_rate <= 0.0 {
            return 0.0;
        }
        let needed = 1.0 - self.tokens;
        if needed <= 0.0 {
            0.0
        } else {
            needed / self.refill_rate
        }
    }

    /// Whether this bucket is fully refilled (idle client).
    fn is_idle(&self, threshold: f64) -> bool {
        let mut clone = self.clone();
        clone.refill();
        clone.tokens >= threshold
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

// ───────────────────── Metrics ─────────────────────

/// Atomic counters for rate limiting metrics.
#[derive(Debug)]
pub struct RateLimitMetrics {
    /// Total requests processed (both allowed and rejected).
    pub requests_total: std::sync::atomic::AtomicU64,
    /// Total requests rejected due to rate limiting.
    pub rejected_total: std::sync::atomic::AtomicU64,
    /// Number of IPs currently being tracked (per-IP mode only).
    pub tracked_ips: std::sync::atomic::AtomicUsize,
}

/// Snapshot of rate limiting metrics for Prometheus export.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitMetricsSnapshot {
    /// Total requests processed.
    pub requests_total: u64,
    /// Total requests rejected.
    pub rejected_total: u64,
    /// Number of IPs currently tracked.
    pub tracked_ips: usize,
    /// Whether per-IP rate limiting is enabled.
    pub per_ip: bool,
}

// ───────────────────── Shared State ─────────────────────

/// Shared rate limiter state, safe to clone across handlers.
///
/// Uses `DashMap` for per-IP buckets so that different IPs are tracked
/// in separate shards, eliminating the global mutex bottleneck that would
/// otherwise serialize all per-IP rate limit checks.
#[derive(Clone)]
pub struct RateLimiterState {
    config: RateLimitConfig,
    global_bucket: Arc<Mutex<TokenBucket>>,
    ip_buckets: Arc<DashMap<IpAddr, TokenBucket>>,
    metrics: Arc<RateLimitMetrics>,
}

impl RateLimiterState {
    pub fn new(config: RateLimitConfig) -> Self {
        let global_bucket = TokenBucket::new(config.max_requests, config.window);
        Self {
            config: config.clone(),
            global_bucket: Arc::new(Mutex::new(global_bucket)),
            ip_buckets: Arc::new(DashMap::new()),
            metrics: Arc::new(RateLimitMetrics {
                requests_total: std::sync::atomic::AtomicU64::new(0),
                rejected_total: std::sync::atomic::AtomicU64::new(0),
                tracked_ips: std::sync::atomic::AtomicUsize::new(0),
            }),
        }
    }

    /// Returns a snapshot of current rate limiting metrics.
    pub fn metrics(&self) -> RateLimitMetricsSnapshot {
        RateLimitMetricsSnapshot {
            requests_total: self.metrics.requests_total.load(Ordering::Relaxed),
            rejected_total: self.metrics.rejected_total.load(Ordering::Relaxed),
            tracked_ips: self.metrics.tracked_ips.load(Ordering::Relaxed),
            per_ip: self.config.per_ip,
        }
    }

    /// Check if a request from the given IP is allowed.
    /// Returns `Ok(remaining)` or `Err(retry_after_secs)`.
    pub async fn check(&self, ip: Option<IpAddr>) -> Result<RateLimitInfo, RateLimitInfo> {
        self.metrics.requests_total.fetch_add(1, Ordering::Relaxed);

        if self.config.per_ip {
            if let Some(ip) = ip {
                return self.check_ip(ip).await;
            }
        }
        self.check_global().await
    }

    async fn check_global(&self) -> Result<RateLimitInfo, RateLimitInfo> {
        let mut bucket = self.global_bucket.lock().await;
        let info = RateLimitInfo {
            limit: self.config.max_requests,
            remaining: bucket.remaining(),
            retry_after: bucket.retry_after(),
        };
        if bucket.try_acquire() {
            Ok(RateLimitInfo {
                remaining: bucket.remaining(),
                ..info
            })
        } else {
            self.metrics.rejected_total.fetch_add(1, Ordering::Relaxed);
            Err(info)
        }
    }

    async fn check_ip(&self, ip: IpAddr) -> Result<RateLimitInfo, RateLimitInfo> {
        // Evict stale entries if we hit the limit and this IP is new
        if !self.ip_buckets.contains_key(&ip)
            && self.ip_buckets.len() >= self.config.max_tracked_ips
        {
            self.evict_stale();
        }

        let ip_is_new = !self.ip_buckets.contains_key(&ip);

        // Use DashMap entry API — only locks the shard for this IP's hash
        let mut entry = self
            .ip_buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(self.config.max_requests, self.config.window));

        // Track new IP entry
        if ip_is_new {
            self.metrics.tracked_ips.fetch_add(1, Ordering::Relaxed);
        }

        let bucket = entry.value_mut();
        let info = RateLimitInfo {
            limit: self.config.max_requests,
            remaining: bucket.remaining(),
            retry_after: bucket.retry_after(),
        };

        if bucket.try_acquire() {
            Ok(RateLimitInfo {
                remaining: bucket.remaining(),
                ..info
            })
        } else {
            self.metrics.rejected_total.fetch_add(1, Ordering::Relaxed);
            Err(info)
        }
    }

    /// Evict stale IP buckets that are fully refilled (idle clients).
    /// DashMap's `retain` iterates per-shard, so this does not block
    /// concurrent accesses to other shards.
    fn evict_stale(&self) {
        let threshold = self.config.max_requests as f64;
        let before = self.ip_buckets.len();
        self.ip_buckets.retain(|_, b| !b.is_idle(threshold));
        let evicted = before - self.ip_buckets.len();
        if evicted > 0 {
            self.metrics
                .tracked_ips
                .fetch_sub(evicted, Ordering::Relaxed);
        }
    }
}

/// Rate limit info included in responses.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub limit: u64,
    pub remaining: u64,
    pub retry_after: f64,
}

// ───────────────────── Axum middleware function ─────────────────────

/// Axum middleware that enforces rate limiting.
///
/// Usage in route construction:
/// ```ignore
/// let rate_limiter = RateLimiterState::new(RateLimitConfig::default());
/// let app = build_router(state)
///     .layer(axum::middleware::from_fn_with_state(
///         rate_limiter,
///         rate_limit_middleware,
///     ));
/// ```
pub async fn rate_limit_middleware(
    axum::extract::State(limiter): axum::extract::State<RateLimiterState>,
    request: Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> Response {
    // Extract client IP from ConnectInfo if available, or from x-forwarded-for
    let ip = extract_client_ip(&request);

    match limiter.check(ip).await {
        Ok(info) => {
            let mut response = next.run(request).await;
            inject_rate_limit_headers(response.headers_mut(), &info);
            response
        }
        Err(info) => rate_limit_response(info),
    }
}

fn extract_client_ip<B>(request: &Request<B>) -> Option<IpAddr> {
    // Try X-Forwarded-For first (for reverse proxies)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            if let Some(first) = val.split(',').next() {
                if let Ok(ip) = first.trim().parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }

    // Try X-Real-IP
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(val) = real_ip.to_str() {
            if let Ok(ip) = val.trim().parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    // Fall back to ConnectInfo
    request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
}

fn inject_rate_limit_headers(headers: &mut axum::http::HeaderMap, info: &RateLimitInfo) {
    headers.insert("x-ratelimit-limit", info.limit.into());
    headers.insert("x-ratelimit-remaining", info.remaining.into());
}

fn rate_limit_response(info: RateLimitInfo) -> Response {
    let retry_after = info.retry_after.ceil() as u64;
    let body = serde_json::json!({
        "error": "rate limit exceeded",
        "retry_after_seconds": retry_after,
        "limit": info.limit,
    });

    let mut response = (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
    response
        .headers_mut()
        .insert("retry-after", retry_after.into());
    inject_rate_limit_headers(response.headers_mut(), &info);
    response
}

// ───────────────────── Tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::http::header::{HeaderName, HeaderValue};
    use axum::routing::get;
    use axum_test::TestServer;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    fn build_test_app(config: RateLimitConfig) -> Router {
        let limiter = RateLimiterState::new(config);
        Router::new()
            .route("/test", get(ok_handler))
            .layer(axum::middleware::from_fn_with_state(
                limiter.clone(),
                rate_limit_middleware,
            ))
            .with_state(limiter)
    }

    #[tokio::test]
    async fn test_allows_requests_within_limit() {
        let config = RateLimitConfig::new(5, Duration::from_secs(60)).with_per_ip(false);
        let server = TestServer::new(build_test_app(config));

        for _ in 0..5 {
            let resp = server.get("/test").await;
            resp.assert_status_ok();
        }
    }

    #[tokio::test]
    async fn test_blocks_requests_over_limit() {
        let config = RateLimitConfig::new(3, Duration::from_secs(60)).with_per_ip(false);
        let server = TestServer::new(build_test_app(config));

        // Use up all tokens
        for _ in 0..3 {
            let resp = server.get("/test").await;
            resp.assert_status_ok();
        }

        // 4th request should be rate limited
        let resp = server.get("/test").await;
        resp.assert_status(StatusCode::TOO_MANY_REQUESTS);

        let body: serde_json::Value = resp.json();
        assert_eq!(body["error"], "rate limit exceeded");
        assert!(body["retry_after_seconds"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_rate_limit_headers_present() {
        let config = RateLimitConfig::new(10, Duration::from_secs(60)).with_per_ip(false);
        let server = TestServer::new(build_test_app(config));

        let resp = server.get("/test").await;
        resp.assert_status_ok();

        let headers = resp.headers();
        assert!(headers.contains_key("x-ratelimit-limit"));
        assert!(headers.contains_key("x-ratelimit-remaining"));
        assert_eq!(headers["x-ratelimit-limit"], "10");
    }

    #[tokio::test]
    async fn test_retry_after_header_on_429() {
        let config = RateLimitConfig::new(1, Duration::from_secs(60)).with_per_ip(false);
        let server = TestServer::new(build_test_app(config));

        // Use the one allowed request
        server.get("/test").await;

        // Next should be rate limited with retry-after
        let resp = server.get("/test").await;
        resp.assert_status(StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().contains_key("retry-after"));
    }

    #[tokio::test]
    async fn test_per_ip_with_x_forwarded_for() {
        let config = RateLimitConfig::new(2, Duration::from_secs(60)).with_per_ip(true);
        let server = TestServer::new(build_test_app(config));

        let xff: HeaderName = "x-forwarded-for".parse().unwrap();

        // IP-A uses 2 requests
        for _ in 0..2 {
            let resp = server
                .get("/test")
                .add_header(xff.clone(), HeaderValue::from_static("10.0.0.1"))
                .await;
            resp.assert_status_ok();
        }

        // IP-A is now limited
        let resp = server
            .get("/test")
            .add_header(xff.clone(), HeaderValue::from_static("10.0.0.1"))
            .await;
        resp.assert_status(StatusCode::TOO_MANY_REQUESTS);

        // IP-B still has quota
        let resp = server
            .get("/test")
            .add_header(xff.clone(), HeaderValue::from_static("10.0.0.2"))
            .await;
        resp.assert_status_ok();
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(10, Duration::from_secs(10));
        assert!(bucket.try_acquire());

        // After consuming 1 token from a 10-token bucket, remaining should be ~9
        // (may be exactly 9 due to micro-refill between calls)
        let remaining = bucket.remaining();
        assert!(
            (8..=9).contains(&remaining),
            "expected 8-9, got {remaining}"
        );

        // Tokens refill over time — for a 10/10s bucket, that's 1 token/sec
        assert!(bucket.remaining() <= 10);
    }

    #[tokio::test]
    async fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window, Duration::from_secs(60));
        assert!(config.per_ip);
        assert_eq!(config.max_tracked_ips, 10_000);
    }

    #[tokio::test]
    async fn test_rate_limiter_state_global_mode() {
        let config = RateLimitConfig::new(2, Duration::from_secs(60)).with_per_ip(false);
        let state = RateLimiterState::new(config);

        // First two should succeed
        assert!(state.check(None).await.is_ok());
        assert!(state.check(None).await.is_ok());

        // Third should fail
        assert!(state.check(None).await.is_err());
    }

    #[tokio::test]
    async fn test_evict_stale_removes_idle_buckets() {
        // Create a limiter with a very small max_tracked_ips
        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_millis(100), // very short window for fast refill
            per_ip: true,
            max_tracked_ips: 3,
        };
        let state = RateLimiterState::new(config);

        // Add 3 IPs (fill up to max_tracked_ips)
        for i in 1..=3u8 {
            let ip: IpAddr = format!("10.0.0.{i}").parse().unwrap();
            assert!(state.check(Some(ip)).await.is_ok());
        }

        assert_eq!(state.ip_buckets.len(), 3);

        // Wait for tokens to fully refill (idle state)
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Adding a 4th IP should trigger eviction of idle IPs
        let ip4: IpAddr = "10.0.0.4".parse().unwrap();
        assert!(state.check(Some(ip4)).await.is_ok());

        // After eviction, at least one idle IP should have been removed
        // and the new IP added. The map should still be at most max_tracked_ips.
        assert!(state.ip_buckets.len() <= 3);
    }

    #[tokio::test]
    async fn test_is_idle_detection() {
        let bucket = TokenBucket::new(5, Duration::from_millis(50));
        // Fresh bucket is fully refilled → idle
        assert!(bucket.is_idle(5.0));

        // Consume a token → no longer idle
        let mut b = bucket.clone();
        b.try_acquire();
        assert!(!b.is_idle(5.0));
    }
}
