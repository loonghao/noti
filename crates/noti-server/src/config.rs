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
//! | `NOTI_LOG_FORMAT` | `text` | Log output format: `text` or `json` |
//! | `NOTI_MAX_BODY_SIZE` | `2097152` | Max request body size in bytes (default 2 MiB) |
//! | `NOTI_QUEUE_BACKEND` | `memory` | Queue backend: `memory` or `sqlite` |
//! | `NOTI_QUEUE_DB_PATH` | `noti-queue.db` | SQLite database path (when backend=sqlite) |
//! | `NOTI_CORS_ALLOWED_ORIGINS` | `*` | Comma-separated allowed origins; `*` = permissive |
//! | `NOTI_OTEL_ENDPOINT` | *(empty)* | OTLP collector gRPC endpoint (e.g. `http://localhost:4317`). When empty, OpenTelemetry is disabled. |
//! | `NOTI_OTEL_SERVICE_NAME` | `noti-server` | Service name used in OTEL resource and span names. |

use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use crate::middleware::auth::AuthConfig;
use crate::middleware::rate_limit::RateLimitConfig;

/// Queue backend type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueBackendType {
    /// In-memory queue (default). Tasks are lost on restart.
    Memory,
    /// SQLite-backed persistent queue. Tasks survive restarts.
    Sqlite,
}

impl QueueBackendType {
    /// Parse from string, defaulting to [`QueueBackendType::Memory`].
    ///
    /// Recognised values (case-insensitive):
    /// - Memory: `"memory"`, `"mem"`, `"in-memory"`
    /// - Sqlite: `"sqlite"`, `"sql"`, `"db"`
    ///
    /// Any other value falls back to `Memory` with a warning log.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "memory" | "mem" | "in-memory" => Self::Memory,
            "sqlite" | "sql" | "db" => Self::Sqlite,
            other => {
                tracing::warn!(
                    input = other,
                    "unrecognised NOTI_QUEUE_BACKEND value, defaulting to memory"
                );
                Self::Memory
            }
        }
    }
}

impl std::fmt::Display for QueueBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory => write!(f, "memory"),
            Self::Sqlite => write!(f, "sqlite"),
        }
    }
}

/// Error returned when a string cannot be parsed into a [`QueueBackendType`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseQueueBackendError {
    /// The invalid input string.
    pub input: String,
}

impl std::fmt::Display for ParseQueueBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown queue backend type '{}' (expected: memory, sqlite, sql, db)",
            self.input
        )
    }
}

impl std::error::Error for ParseQueueBackendError {}

impl TryFrom<&str> for QueueBackendType {
    type Error = ParseQueueBackendError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_ascii_lowercase().as_str() {
            "memory" | "mem" | "in-memory" => Ok(Self::Memory),
            "sqlite" | "sql" | "db" => Ok(Self::Sqlite),
            _ => Err(ParseQueueBackendError {
                input: s.to_string(),
            }),
        }
    }
}

/// Log output format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable text output (default).
    Text,
    /// Structured JSON output suitable for log aggregation pipelines.
    Json,
}

impl LogFormat {
    /// Parse from string, defaulting to [`LogFormat::Text`] for unrecognised values.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "json" => Self::Json,
            _ => Self::Text,
        }
    }
}

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
    /// Log output format (`text` or `json`).
    pub log_format: LogFormat,
    /// Maximum request body size in bytes.
    pub max_body_size: usize,
    /// Queue backend type.
    pub queue_backend: QueueBackendType,
    /// SQLite database path (used when `queue_backend` is `Sqlite`).
    pub queue_db_path: String,
    /// CORS allowed origins. Empty vec = permissive (allow all).
    /// Populated from `NOTI_CORS_ALLOWED_ORIGINS`.
    pub cors_allowed_origins: Vec<String>,
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
            log_format: LogFormat::Text,
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            queue_backend: QueueBackendType::Memory,
            queue_db_path: "noti-queue.db".to_string(),
            cors_allowed_origins: Vec::new(),
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

        let log_format = env::var("NOTI_LOG_FORMAT")
            .map(|v| LogFormat::from_str_lossy(&v))
            .unwrap_or(LogFormat::Text);

        let max_body_size = env::var("NOTI_MAX_BODY_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MAX_BODY_SIZE);

        let queue_backend = env::var("NOTI_QUEUE_BACKEND")
            .map(|v| QueueBackendType::from_str_lossy(&v))
            .unwrap_or(QueueBackendType::Memory);

        let queue_db_path =
            env::var("NOTI_QUEUE_DB_PATH").unwrap_or_else(|_| "noti-queue.db".to_string());

        let cors_allowed_origins: Vec<String> = env::var("NOTI_CORS_ALLOWED_ORIGINS")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            host,
            port,
            auth,
            rate_limit,
            worker_count,
            log_level,
            log_format,
            max_body_size,
            queue_backend,
            queue_db_path,
            cors_allowed_origins,
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
        assert_eq!(cfg.log_format, LogFormat::Text);
        assert!(!cfg.auth.enabled);
        assert!(cfg.cors_allowed_origins.is_empty());
    }

    #[test]
    fn test_log_format_parsing() {
        assert_eq!(LogFormat::from_str_lossy("json"), LogFormat::Json);
        assert_eq!(LogFormat::from_str_lossy("JSON"), LogFormat::Json);
        assert_eq!(LogFormat::from_str_lossy("Json"), LogFormat::Json);
        assert_eq!(LogFormat::from_str_lossy("text"), LogFormat::Text);
        assert_eq!(LogFormat::from_str_lossy("TEXT"), LogFormat::Text);
        assert_eq!(LogFormat::from_str_lossy("unknown"), LogFormat::Text);
        assert_eq!(LogFormat::from_str_lossy(""), LogFormat::Text);
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

    #[test]
    fn test_queue_backend_type_try_from_valid() {
        assert_eq!(
            QueueBackendType::try_from("memory").unwrap(),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::try_from("mem").unwrap(),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::try_from("in-memory").unwrap(),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::try_from("sqlite").unwrap(),
            QueueBackendType::Sqlite
        );
        assert_eq!(
            QueueBackendType::try_from("SQL").unwrap(),
            QueueBackendType::Sqlite
        );
        assert_eq!(
            QueueBackendType::try_from("DB").unwrap(),
            QueueBackendType::Sqlite
        );
    }

    #[test]
    fn test_queue_backend_type_try_from_invalid() {
        let err = QueueBackendType::try_from("redis").unwrap_err();
        assert_eq!(err.input, "redis");
        assert!(err.to_string().contains("redis"));
        assert!(err.to_string().contains("expected"));
    }

    #[test]
    fn test_queue_backend_type_from_str_lossy() {
        // Explicit memory variants
        assert_eq!(
            QueueBackendType::from_str_lossy("memory"),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::from_str_lossy("mem"),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::from_str_lossy("in-memory"),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::from_str_lossy("MEMORY"),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::from_str_lossy("In-Memory"),
            QueueBackendType::Memory
        );

        // SQLite variants
        assert_eq!(
            QueueBackendType::from_str_lossy("sqlite"),
            QueueBackendType::Sqlite
        );
        assert_eq!(
            QueueBackendType::from_str_lossy("sql"),
            QueueBackendType::Sqlite
        );
        assert_eq!(
            QueueBackendType::from_str_lossy("db"),
            QueueBackendType::Sqlite
        );
        assert_eq!(
            QueueBackendType::from_str_lossy("SQLITE"),
            QueueBackendType::Sqlite
        );

        // Unknown values default to Memory (with warning)
        assert_eq!(
            QueueBackendType::from_str_lossy("redis"),
            QueueBackendType::Memory
        );
        assert_eq!(
            QueueBackendType::from_str_lossy(""),
            QueueBackendType::Memory
        );
    }

    #[test]
    fn test_queue_backend_type_display() {
        assert_eq!(QueueBackendType::Memory.to_string(), "memory");
        assert_eq!(QueueBackendType::Sqlite.to_string(), "sqlite");
    }
}
