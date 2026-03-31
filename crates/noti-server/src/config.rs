//! Server configuration loaded from environment variables.
//!
//! All settings can be overridden via environment variables prefixed with `NOTI_`.
//! Unset variables fall back to sensible defaults.
//!
//! | Variable | Default | Description |
//! |---|---|---|
//! | `NOTI_HOST` | `0.0.0.0` | Bind address |
//! | `NOTI_PORT` | `3000` | Listen port |
//! | `NOTI_API_KEYS` | *(empty)* | Comma-separated API keys; empty = auth disabled |
//! | `NOTI_AUTH_EXCLUDED_PATHS` | `/health` | Comma-separated paths that bypass auth |
//! | `NOTI_RATE_LIMIT_MAX` | `100` | Max requests per window |
//! | `NOTI_RATE_LIMIT_WINDOW_SECS` | `60` | Rate-limit window in seconds |
//! | `NOTI_RATE_LIMIT_PER_IP` | `true` | Per-IP rate limiting |
//! | `NOTI_WORKER_COUNT` | `4` | Number of background queue workers |
//! | `NOTI_LOG_LEVEL` | `info` | Tracing log level filter |
//! | `NOTI_MAX_BODY_SIZE` | `2097152` | Max request body size in bytes (default 2 MiB) |

use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use crate::middleware::auth::AuthConfig;
use crate::middleware::rate_limit::RateLimitConfig;

/// Centralized server configuration, populated from environment variables.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind address.
    pub host: IpAddr,
    /// Listen port.
    pub port: u16,
    /// Auth configuration derived from env.
    pub auth: AuthConfig,
    /// Rate-limit configuration derived from env.
    pub rate_limit: RateLimitConfig,
    /// Number of background queue workers.
    pub worker_count: usize,
    /// Log-level filter string.
    pub log_level: String,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
}

/// Default max body size: 2 MiB.
const DEFAULT_MAX_BODY_SIZE: usize = 2 * 1024 * 1024;

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            port: 3000,
            auth: AuthConfig::default(),
            rate_limit: RateLimitConfig::new(100, Duration::from_secs(60)).with_per_ip(true),
            worker_count: 4,
            log_level: "info".to_string(),
            max_body_size: DEFAULT_MAX_BODY_SIZE,
        }
    }
}

impl ServerConfig {
    /// Load configuration from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        let host = env::var("NOTI_HOST")
            .ok()
            .and_then(|v| v.parse::<IpAddr>().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));

        let port = env::var("NOTI_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(3000);

        let auth = Self::build_auth_config();

        let rate_limit_max = env::var("NOTI_RATE_LIMIT_MAX")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(100);

        let rate_limit_window = env::var("NOTI_RATE_LIMIT_WINDOW_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60);

        let rate_limit_per_ip = env::var("NOTI_RATE_LIMIT_PER_IP")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        let rate_limit =
            RateLimitConfig::new(rate_limit_max, Duration::from_secs(rate_limit_window))
                .with_per_ip(rate_limit_per_ip);

        let worker_count = env::var("NOTI_WORKER_COUNT")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(4);

        let log_level = env::var("NOTI_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());

        let max_body_size = env::var("NOTI_MAX_BODY_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_BODY_SIZE);

        Self {
            host,
            port,
            auth,
            rate_limit,
            worker_count,
            log_level,
            max_body_size,
        }
    }

    /// Socket address derived from host + port.
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    /// Build [`AuthConfig`] from `NOTI_API_KEYS` and `NOTI_AUTH_EXCLUDED_PATHS`.
    fn build_auth_config() -> AuthConfig {
        let keys: Vec<String> = env::var("NOTI_API_KEYS")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let mut config = AuthConfig::new(keys);

        if let Ok(paths) = env::var("NOTI_AUTH_EXCLUDED_PATHS") {
            let extra: Vec<&str> = paths.split(',').map(|s| s.trim()).collect();
            config = config.with_excluded_paths(&extra);
        }

        config
    }
}

// ───────────────────── Tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.worker_count, 4);
        assert_eq!(cfg.log_level, "info");
        assert!(!cfg.auth.enabled);
    }

    #[test]
    fn test_socket_addr() {
        let cfg = ServerConfig {
            port: 8080,
            ..Default::default()
        };
        let addr = cfg.socket_addr();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_from_env_defaults() {
        // With no env vars set, should return defaults
        // (other env vars from CI might interfere, so we only assert non-env-dependent behavior)
        let cfg = ServerConfig::from_env();
        assert!(cfg.port > 0);
        assert!(cfg.worker_count > 0);
    }
}
