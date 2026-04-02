# Logging & Observability

noti-server provides structured logging powered by the Rust [tracing](https://docs.rs/tracing) ecosystem, with automatic request ID correlation, configurable output formats, and fine-grained log level filtering — all driven by environment variables.

## Quick Start

```bash
# Human-readable text logs (default)
export NOTI_LOG_FORMAT=text
export NOTI_LOG_LEVEL=info

# Structured JSON logs for production (ELK, Loki, Datadog, etc.)
export NOTI_LOG_FORMAT=json
export NOTI_LOG_LEVEL=info
```

Start the server and every HTTP request, queue event, and internal operation is automatically logged with contextual metadata.

## Configuration

### Environment Variables

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_LOG_LEVEL` | `info` | Tracing filter level (`trace`, `debug`, `info`, `warn`, `error`) |
| `NOTI_LOG_FORMAT` | `text` | Output format: `text` (human-readable) or `json` (structured) |
| `RUST_LOG` | *(unset)* | Standard `tracing_subscriber::EnvFilter` variable — **overrides** `NOTI_LOG_LEVEL` when set |

### Log Levels

| Level | Use Case | Example |
|:------|:---------|:--------|
| `error` | Unrecoverable failures | Queue backend crash, bind failure |
| `warn` | Recoverable issues | Stale task recovery failure, metrics fetch error |
| `info` | Operational events (recommended for production) | Server started, request processed, worker started |
| `debug` | Development diagnostics | Middleware execution details, config parsing |
| `trace` | Maximum verbosity | Individual span enter/exit events |

### Priority Order

The server determines the active log filter in this priority:

1. **`RUST_LOG`** (if set) — takes full precedence; supports per-module filtering
2. **`NOTI_LOG_LEVEL`** (fallback) — simple single-level filter

::: tip Fine-Grained Filtering with RUST_LOG
`RUST_LOG` supports the full `tracing_subscriber::EnvFilter` syntax, allowing per-crate filtering:

```bash
# Show debug for noti-server, info for everything else
export RUST_LOG="noti_server=debug,tower_http=info,info"

# Trace-level for the queue module only
export RUST_LOG="noti_queue=trace,info"

# Suppress noisy tower_http logs
export RUST_LOG="tower_http=warn,info"
```

When `RUST_LOG` is set, `NOTI_LOG_LEVEL` is completely ignored.
:::

## Output Formats

### Text Format (Default)

Human-readable output suitable for development and local debugging:

```
2025-08-15T09:30:00.123Z  INFO noti_server: loaded configuration from environment
2025-08-15T09:30:00.124Z  INFO noti_server: queue backend initialized backend=Memory
2025-08-15T09:30:00.125Z  INFO noti_server: queue worker pool started workers=4
2025-08-15T09:30:00.126Z  INFO noti_server: noti-server listening addr=0.0.0.0:3000
2025-08-15T09:30:05.200Z  INFO request{request_id=a1b2c3d4 method=POST path=/api/v1/send}: noti_server: notification sent
```

### JSON Format

Structured JSON output for log aggregation pipelines (ELK Stack, Grafana Loki, Datadog, Splunk, etc.):

```bash
export NOTI_LOG_FORMAT=json
```

Each log line is a single JSON object:

```json
{
  "timestamp": "2025-08-15T09:30:05.200Z",
  "level": "INFO",
  "fields": {
    "message": "notification sent"
  },
  "target": "noti_server::handlers::send",
  "span": {
    "request_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "method": "POST",
    "path": "/api/v1/send",
    "name": "request"
  }
}
```

#### JSON Features

| Feature | Description |
|:--------|:------------|
| **Flattened events** | Event fields are merged into the top-level JSON (`.flatten_event(true)`) |
| **Span events** | Span close events are emitted (`.with_span_events(FmtSpan::CLOSE)`), showing request duration |
| **Current span** | Each log line includes the enclosing span's fields (`.with_current_span(true)`) |
| **Request ID correlation** | Every log within an HTTP request includes `request_id` from the Request ID middleware |

## Request ID Correlation

Every HTTP request is automatically assigned a unique identifier for end-to-end log tracing.

### How It Works

1. If the client sends an `X-Request-Id` header, the server **preserves** it
2. Otherwise, the server generates a new **UUID v4**
3. The ID is injected into a `tracing::info_span!("request", request_id, method, path)`, so **all downstream log entries** automatically include the `request_id` field
4. The same `X-Request-Id` is **echoed back** in the response headers

### Client-Provided ID

Useful for distributed tracing across multiple services:

```bash
curl -H "X-Request-Id: my-trace-123" http://localhost:3000/api/v1/send \
  -H "Content-Type: application/json" \
  -d '{"provider": "slack", "config": {"webhook": "..."}, "text": "hello"}'

# Response header: X-Request-Id: my-trace-123
# All server logs for this request contain: request_id=my-trace-123
```

### Server-Generated ID

When no `X-Request-Id` is provided, the server auto-generates one:

```bash
curl -v http://localhost:3000/health
# < X-Request-Id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

### Correlating Logs

In JSON mode, search your log aggregation system by `request_id` to see the full request lifecycle:

```json
{"timestamp":"...","level":"INFO","span":{"request_id":"a1b2c3d4-...","method":"POST","path":"/api/v1/send"},"fields":{"message":"notification sent"}}
{"timestamp":"...","level":"INFO","span":{"request_id":"a1b2c3d4-...","method":"POST","path":"/api/v1/send"},"fields":{"message":"request completed","latency":"12ms"}}
```

In text mode, the request span context appears inline:

```
INFO request{request_id=a1b2c3d4 method=POST path=/api/v1/send}: notification sent
```

## Middleware Stack & Tracing

The noti-server middleware stack is layered so that tracing captures the full request lifecycle:

```
Request → CORS → TraceLayer → RequestID → Auth → RateLimit → BodyLimit → Router
```

| Layer | Tracing Behavior |
|:------|:-----------------|
| **CORS** | Handles preflight `OPTIONS` before any logging |
| **TraceLayer** | `tower_http::trace::TraceLayer` — records HTTP request start/end with status code and duration |
| **RequestID** | Creates `info_span!("request", request_id, method, path)` — all downstream logs inherit these fields |
| **Auth** | Logs `401 Unauthorized` events within the request span |
| **RateLimit** | Logs `429 Too Many Requests` events within the request span |
| **BodyLimit** | Rejects oversized payloads (logged by TraceLayer) |
| **Router** | Handler-level `tracing::info!`/`warn!`/`error!` calls automatically include request context |

## Startup Logs

When the server starts, it emits a sequence of operational logs that are useful for verifying configuration:

```
INFO noti_server: loaded configuration from environment
INFO noti_server: queue backend initialized backend=Memory
INFO noti_server: queue worker pool started workers=4
INFO noti_server: API key authentication disabled (no NOTI_API_KEYS set)
INFO noti_server: rate limiter enabled
INFO noti_server: CORS: permissive (allow all origins)
INFO noti_server: noti-server listening addr=0.0.0.0:3000
```

With authentication enabled:

```
INFO noti_server: API key authentication enabled keys=2
```

With SQLite backend and stale task recovery:

```
INFO noti_server: queue backend initialized backend=Sqlite
INFO noti_server: recovered stale processing tasks recovered=3
```

## Shutdown Logs

Graceful shutdown is logged when the server receives `Ctrl+C` or `SIGTERM`:

```
INFO noti_server: received Ctrl+C, starting graceful shutdown
INFO noti_server: shutting down worker pool...
INFO noti_server: worker pool stopped, server exiting
```

## Log Aggregation Patterns

### ELK Stack (Elasticsearch + Logstash + Kibana)

Configure the server with JSON output and pipe to Logstash:

```bash
export NOTI_LOG_FORMAT=json
export NOTI_LOG_LEVEL=info
```

Logstash input configuration:

```ruby
input {
  file {
    path => "/var/log/noti-server.log"
    codec => json
  }
}

filter {
  # request_id is already in the JSON — no extra parsing needed
  date {
    match => ["timestamp", "ISO8601"]
    target => "@timestamp"
  }
}

output {
  elasticsearch {
    hosts => ["http://elasticsearch:9200"]
    index => "noti-server-%{+YYYY.MM.dd}"
  }
}
```

### Grafana Loki

Use Docker log driver or Promtail to ship JSON logs:

```yaml
# docker-compose.yml — production profile
services:
  noti:
    image: ghcr.io/loonghao/noti-server:latest
    environment:
      NOTI_LOG_FORMAT: "json"
      NOTI_LOG_LEVEL: "info"
    logging:
      driver: loki
      options:
        loki-url: "http://loki:3100/loki/api/v1/push"
        loki-external-labels: "service=noti-server,env=production"
```

Loki automatically indexes JSON fields, so you can query by `request_id`:

```logql
{service="noti-server"} | json | request_id = "a1b2c3d4-..."
```

### Datadog

Ship JSON logs via the Datadog Agent:

```yaml
# /etc/datadog-agent/conf.d/noti.d/conf.yaml
logs:
  - type: file
    path: /var/log/noti-server.log
    service: noti-server
    source: rust
    sourcecategory: server
```

Or use Docker labels:

```yaml
services:
  noti:
    image: ghcr.io/loonghao/noti-server:latest
    labels:
      com.datadoghq.ad.logs: '[{"source": "rust", "service": "noti-server"}]'
    environment:
      NOTI_LOG_FORMAT: "json"
```

### Splunk

Use the Splunk HEC (HTTP Event Collector) with a sidecar or direct file monitoring:

```bash
# Forward JSON logs to Splunk HEC
noti-server 2>&1 | curl -s -k \
  -H "Authorization: Splunk YOUR_HEC_TOKEN" \
  -d @- \
  https://splunk-hec:8088/services/collector/raw
```

## Docker Configuration

### Production

```dockerfile
# Default in the official Dockerfile
ENV NOTI_LOG_FORMAT=json
```

```yaml
# docker-compose.yml — production profile
services:
  noti:
    environment:
      NOTI_LOG_FORMAT: "json"
      NOTI_LOG_LEVEL: "${NOTI_LOG_LEVEL:-info}"
```

### Development

```yaml
# docker-compose.yml — dev profile
services:
  noti-dev:
    environment:
      NOTI_LOG_FORMAT: "text"
      NOTI_LOG_LEVEL: "debug"
```

## Troubleshooting

### Common Issues

| Symptom | Cause | Fix |
|:--------|:------|:----|
| No logs appear | Log level too restrictive | Set `NOTI_LOG_LEVEL=debug` or `RUST_LOG=debug` |
| Too many logs | `trace` or `debug` in production | Use `NOTI_LOG_LEVEL=info` |
| `RUST_LOG` has no effect | Syntax error in filter string | Validate syntax: `RUST_LOG="info"` (quotes important in shell) |
| Request ID missing in logs | Not using JSON format | Set `NOTI_LOG_FORMAT=json` — text format shows it in span prefix |
| Logs not appearing in aggregator | Wrong format or path | Verify `NOTI_LOG_FORMAT=json` and check file/stdout routing |

### Debugging Tips

```bash
# Maximum verbosity for debugging a specific issue
export RUST_LOG="noti_server=trace,noti_queue=trace,tower_http=debug,info"

# See only warnings and errors (minimal noise)
export NOTI_LOG_LEVEL=warn

# Debug middleware execution order
export RUST_LOG="noti_server::middleware=debug,info"

# Debug queue worker processing
export RUST_LOG="noti_queue::worker=debug,info"
```

## Implementation Details

### Technology Stack

| Component | Crate | Purpose |
|:----------|:------|:--------|
| Tracing facade | `tracing` | Structured, span-based logging API |
| Subscriber | `tracing-subscriber` | Log formatting and filtering |
| HTTP tracing | `tower-http::TraceLayer` | Automatic HTTP request/response logging |
| Filter | `tracing-subscriber::EnvFilter` | `RUST_LOG`-compatible dynamic filtering |

### Initialization

The tracing subscriber is initialized once at server startup in `main.rs`:

```rust
// Load config from environment
let config = ServerConfig::from_env();

// Prefer RUST_LOG if set, otherwise use NOTI_LOG_LEVEL
let env_filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| config.log_level.clone().into());

match config.log_format {
    LogFormat::Json => {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_span_events(FmtSpan::CLOSE)
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
```

### Request ID Middleware

The Request ID middleware creates a tracing span per request:

```rust
let span = tracing::info_span!(
    "request",
    request_id = %request_id,
    method = %request.method(),
    path = %request.uri().path(),
);

let response = next.run(request).instrument(span).await;
```

All `tracing::info!()`, `tracing::warn!()`, and `tracing::error!()` calls within the handler automatically inherit the `request_id`, `method`, and `path` fields from this span.

## AI Agent Tips

- **Always set `NOTI_LOG_FORMAT=json`** in production for machine-parseable logs
- **Use `RUST_LOG`** for temporary debugging with per-module filtering — revert to `NOTI_LOG_LEVEL` for normal operation
- **Correlate by `request_id`** — pass `X-Request-Id` in your requests and search logs by the same value
- **Monitor startup logs** to verify configuration is applied correctly (backend type, worker count, auth status)
- **Combine with `/api/v1/metrics`** for operational monitoring — see the [Metrics & Monitoring](/guide/metrics-monitoring) guide
- **Use `--fields`** to limit response sizes when querying the API programmatically
