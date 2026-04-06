# Metrics & Monitoring

noti-server exposes dedicated endpoints for operational monitoring, giving you real-time insight into queue throughput, provider health, server uptime, and dependency status.

## Quick Start

```bash
# Health check — always available, no auth required
curl http://localhost:3000/health

# Operational metrics — includes queue stats, provider info, uptime
curl http://localhost:3000/api/v1/metrics
```

Both endpoints return structured JSON suitable for dashboards, alerting systems, and AI agent consumption.

## Endpoints Overview

| Endpoint | Method | Auth | Description |
|:---------|:-------|:-----|:------------|
| `/health` | `GET` | No (excluded by default) | Service health with dependency status |
| `/api/v1/metrics` | `GET` | Yes (if enabled) | Operational metrics for monitoring dashboards (JSON) |
| `/metrics` | `GET` | No | Prometheus text-format metrics for direct scraping |
| `/api/v1/queue/stats` | `GET` | Yes (if enabled) | Queue-specific statistics |

## Health Check (`/health`)

The health endpoint provides a holistic view of service status, including dependency health. It is **version-independent** (not under `/api/v1/`) and **excluded from authentication** by default.

### Response Format

```json
{
  "status": "ok",
  "version": "0.1.9",
  "uptime_seconds": 3600,
  "dependencies": {
    "queue": {
      "status": "up",
      "detail": "queued=5 processing=2 completed=100"
    },
    "providers": {
      "status": "up",
      "detail": "126 registered"
    }
  },
  "workers": {
    "total": 4,
    "active": 2,
    "idle": 2
  }
}
```

The `workers` field is omitted when the server is started without a worker pool (e.g. in proxy-only mode).

### Status Values

| Status | Meaning | HTTP Code |
|:-------|:--------|:----------|
| `ok` | All dependencies healthy | 200 |
| `degraded` | One or more dependencies unhealthy | 200 |

::: tip Always 200
The health endpoint **always returns 200**, even when degraded. This lets load balancers distinguish between "process alive but unhealthy" and "process down" (connection refused). Use the `status` field for fine-grained logic.
:::

### Dependency Health

Each dependency reports a `ComponentHealth` object:

| Field | Type | Description |
|:------|:-----|:------------|
| `status` | `string` | `"up"` or `"down"` |
| `detail` | `string?` | Human-readable detail (e.g., queue counts, provider count) |

**Queue health** is determined by attempting to fetch queue statistics. If the backend is unreachable or errored, the queue status is `"down"`.

**Provider health** is determined by the number of registered providers. If zero providers are registered (misconfiguration), the status is `"down"`.

### Use Cases

- **Kubernetes liveness/readiness probes**: Point both probes at `/health`; use `status == "ok"` for readiness
- **Load balancer health checks**: HTTP 200 = process alive; parse body for granularity
- **Uptime monitoring**: Track `uptime_seconds` for restart detection

### Docker Health Check

The official Docker image includes a built-in health check:

```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1
```

docker-compose equivalent:

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
  interval: 30s
  timeout: 5s
  start_period: 10s
  retries: 3
```

## Metrics Endpoint (`/api/v1/metrics`)

The metrics endpoint returns a comprehensive snapshot of server operational state, designed for monitoring dashboards and alerting pipelines.

### Response Format

```json
{
  "queue": {
    "queued": 5,
    "processing": 2,
    "completed": 1000,
    "failed": 12,
    "cancelled": 3,
    "total": 1022
  },
  "providers": {
    "total_registered": 125,
    "with_attachment_support": 15
  },
  "uptime_seconds": 86400,
  "version": "0.1.5"
}
```

### Response Schema

#### `MetricsResponse`

| Field | Type | Description |
|:------|:-----|:------------|
| `queue` | `StatsResponse` | Queue statistics (same shape as `GET /api/v1/queue/stats`) |
| `providers` | `ProviderMetrics` | Provider registry statistics |
| `uptime_seconds` | `u64` | Server uptime in seconds since startup |
| `version` | `string` | Server version (from `Cargo.toml`) |

#### `StatsResponse` (queue)

| Field | Type | Description |
|:------|:-----|:------------|
| `queued` | `usize` | Tasks waiting to be processed |
| `processing` | `usize` | Tasks currently being processed by workers |
| `completed` | `usize` | Successfully completed tasks |
| `failed` | `usize` | Tasks that failed after all retries |
| `cancelled` | `usize` | Tasks cancelled before completion |
| `total` | `usize` | Sum of all task counts |

#### `ProviderMetrics`

| Field | Type | Description |
|:------|:-----|:------------|
| `total_registered` | `usize` | Total notification providers available |
| `with_attachment_support` | `usize` | Providers supporting file attachments |

### Error Handling

If the queue backend is temporarily unavailable, the metrics endpoint still returns 200 with **zeroed queue stats** rather than failing. A warning is logged:

```
WARN failed to fetch queue stats for metrics: <error details>
```

This ensures monitoring dashboards remain functional even during transient queue issues.

## Queue Statistics (`/api/v1/queue/stats`)

For queue-focused monitoring, you can use the dedicated stats endpoint:

```bash
curl http://localhost:3000/api/v1/queue/stats
```

```json
{
  "queued": 5,
  "processing": 2,
  "completed": 100,
  "failed": 3,
  "cancelled": 1,
  "total": 111
}
```

This returns the same `StatsResponse` shape as the `queue` field in `/api/v1/metrics`, useful when you only need queue throughput data.

## Monitoring Integration Patterns

### Polling-Based Monitoring

Set up a cron job or monitoring agent to periodically poll the endpoints:

```bash
#!/bin/bash
# poll-metrics.sh — run every 30s via cron or systemd timer

METRICS=$(curl -sf http://localhost:3000/api/v1/metrics)

# Extract key values
QUEUED=$(echo "$METRICS" | jq '.queue.queued')
FAILED=$(echo "$METRICS" | jq '.queue.failed')
UPTIME=$(echo "$METRICS" | jq '.uptime_seconds')

# Alert if queue backlog exceeds threshold
if [ "$QUEUED" -gt 100 ]; then
  echo "ALERT: Queue backlog at $QUEUED" | mail -s "noti queue alert" ops@example.com
fi

# Alert if failure rate is high
TOTAL=$(echo "$METRICS" | jq '.queue.total')
if [ "$TOTAL" -gt 0 ]; then
  FAIL_RATE=$(echo "scale=2; $FAILED * 100 / $TOTAL" | bc)
  echo "Failure rate: ${FAIL_RATE}%"
fi
```

### Python Monitoring Script

```python
import requests
import time

def check_noti_health(base_url: str = "http://localhost:3000"):
    """Check noti-server health and metrics."""
    # Health check
    health = requests.get(f"{base_url}/health").json()
    if health["status"] != "ok":
        print(f"WARNING: Service status is {health['status']}")
        for dep, info in health["dependencies"].items():
            if info["status"] != "up":
                print(f"  {dep}: {info['status']} — {info.get('detail', 'no detail')}")

    # Metrics
    metrics = requests.get(f"{base_url}/api/v1/metrics").json()
    queue = metrics["queue"]
    print(f"Queue: {queue['queued']} queued, {queue['processing']} processing, "
          f"{queue['completed']} completed, {queue['failed']} failed")
    print(f"Providers: {metrics['providers']['total_registered']} registered")
    print(f"Uptime: {metrics['uptime_seconds']}s")

    return health["status"] == "ok"
```

## Prometheus Metrics Endpoint (`/metrics`)

noti-server exposes a native Prometheus text-format endpoint at `GET /metrics` for direct scraping — no exporter required.

```bash
curl http://localhost:3000/metrics
```

Sample output:

```text
# HELP noti_queue_total Total tasks in queue by status
# TYPE noti_queue_total gauge
noti_queue_total{status="queued"} 5
noti_queue_total{status="processing"} 2
noti_queue_total{status="completed"} 150
noti_queue_total{status="failed"} 3
noti_queue_total{status="cancelled"} 0
# HELP noti_providers_registered Number of registered providers
# TYPE noti_providers_registered gauge
noti_providers_registered 126
# HELP noti_providers_with_attachments Number of providers supporting attachments
# TYPE noti_providers_with_attachments gauge
noti_providers_with_attachments 15
# HELP noti_server_uptime_seconds Server uptime in seconds
# TYPE noti_server_uptime_seconds gauge
noti_server_uptime_seconds 86400
# HELP noti_server_version Server version
# TYPE noti_server_version gauge
noti_server_version{version="0.1.9"} 1
```

The endpoint returns `Content-Type: text/plain; version=0.0.4; charset=utf-8`, which Prometheus recognizes as the standard exposition format.

### Prometheus Scrape Config

```yaml
scrape_configs:
  - job_name: noti
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: /metrics
```

### Grafana / Prometheus Integration

For dashboards, point Prometheus at the `/metrics` endpoint above. For custom integrations using the JSON endpoint or Pushgateway:

**Custom scraper with Pushgateway**

```python
from prometheus_client import CollectorRegistry, Gauge, push_to_gateway
import requests

registry = CollectorRegistry()
queue_queued = Gauge("noti_queue_queued", "Queued tasks", registry=registry)
queue_failed = Gauge("noti_queue_failed", "Failed tasks", registry=registry)
uptime = Gauge("noti_uptime_seconds", "Server uptime", registry=registry)

metrics = requests.get("http://localhost:3000/api/v1/metrics").json()
queue_queued.set(metrics["queue"]["queued"])
queue_failed.set(metrics["queue"]["failed"])
uptime.set(metrics["uptime_seconds"])

push_to_gateway("localhost:9091", job="noti", registry=registry)
```

### Datadog / StatsD

```python
from datadog import statsd
import requests

metrics = requests.get("http://localhost:3000/api/v1/metrics").json()
statsd.gauge("noti.queue.queued", metrics["queue"]["queued"])
statsd.gauge("noti.queue.processing", metrics["queue"]["processing"])
statsd.gauge("noti.queue.completed", metrics["queue"]["completed"])
statsd.gauge("noti.queue.failed", metrics["queue"]["failed"])
statsd.gauge("noti.providers.total", metrics["providers"]["total_registered"])
statsd.gauge("noti.uptime_seconds", metrics["uptime_seconds"])
```

## Alerting Recommendations

### Key Metrics to Monitor

| Metric | Condition | Severity | Action |
|:-------|:----------|:---------|:-------|
| `health.status` | `!= "ok"` | Critical | Investigate dependency health |
| `queue.queued` | `> threshold` | Warning | Scale workers or investigate slow processing |
| `queue.processing` | stuck at N for > 5 min | Warning | Check for deadlocks or slow providers |
| `queue.failed` | increasing rate | Warning | Review provider errors, check retry config |
| `uptime_seconds` | sudden drop | Info | Server restarted — verify expected |
| `providers.total_registered` | `== 0` | Critical | Misconfiguration — no providers loaded |

### Example Alerting Rules (Prometheus AlertManager)

```yaml
groups:
  - name: noti-alerts
    rules:
      - alert: NotiServiceDegraded
        expr: noti_health_status != 1
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "noti service is degraded"

      - alert: NotiQueueBacklog
        expr: noti_queue_queued > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "noti queue backlog exceeds 100 tasks"

      - alert: NotiHighFailureRate
        expr: rate(noti_queue_failed[5m]) > 0.1
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "noti notification failure rate is elevated"
```

## Production Configuration

### Environment Variables

Monitoring-related environment variables:

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_LOG_LEVEL` | `info` | Log verbosity (`trace`, `debug`, `info`, `warn`, `error`) |
| `NOTI_LOG_FORMAT` | `text` | Log format: `text` or `json` (for log aggregation) |
| `NOTI_AUTH_EXCLUDED_PATHS` | `/health` | Paths that bypass authentication (health check always accessible) |
| `NOTI_WORKER_COUNT` | `4` | Number of background workers (affects processing throughput) |
| `NOTI_OTEL_ENDPOINT` | *(empty)* | OTLP collector gRPC endpoint (e.g. `http://localhost:4317`). When set, distributed traces are exported to Jaeger, Tempo, Honeycomb, etc. |
| `NOTI_OTEL_SERVICE_NAME` | `noti-server` | Service name used in OTEL resource attributes and span names. |

### Recommended Production Setup

```bash
# Structured JSON logging for log aggregation (ELK, Loki, etc.)
export NOTI_LOG_FORMAT=json
export NOTI_LOG_LEVEL=info

# Ensure health check is always accessible
export NOTI_AUTH_EXCLUDED_PATHS=/health

# Tune workers based on expected throughput
export NOTI_WORKER_COUNT=8
```

### Log Correlation

When `NOTI_LOG_FORMAT=json`, every log line includes the `request_id` field from the Request ID middleware, enabling end-to-end request tracing:

```json
{
  "timestamp": "2025-08-15T09:30:00.123Z",
  "level": "INFO",
  "request_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "method": "GET",
  "path": "/api/v1/metrics",
  "status": 200,
  "duration_ms": 2
}
```

Use the `X-Request-Id` header to correlate client requests with server logs:

```bash
curl -H "X-Request-Id: my-trace-001" http://localhost:3000/api/v1/metrics
```

## OpenAPI Schema

Both monitoring endpoints are documented in the auto-generated OpenAPI specification:

- **Tag**: `Health` — `/health` endpoint
- **Tag**: `Monitoring` — `/api/v1/metrics` endpoint
- **Schemas**: `HealthResponse`, `DependencyHealth`, `ComponentHealth`, `MetricsResponse`, `ProviderMetrics`, `StatsResponse`

Browse the interactive documentation at `/swagger-ui` when the server is running. See the [OpenAPI & Swagger UI](/guide/openapi) guide for details.

## AI Agent Tips

- **Always use `--json`** or set `NOTI_OUTPUT=json` for machine-readable output
- **Poll `/api/v1/metrics`** for a single-request operational snapshot
- Use **`--fields queue,uptime_seconds`** to limit response size and protect your context window
- **Parse `health.status`** before attempting send operations — if `"degraded"`, diagnostic before retrying
- The health endpoint is **never rate-limited or auth-gated** by default, making it safe for frequent polling
