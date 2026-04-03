# Health Check

The `/health` endpoint provides a comprehensive, real-time view of the noti-server's operational status — including dependency health, version, and uptime. It is designed for load balancers, orchestrators, and monitoring systems.

## Quick Start

```bash
curl -s http://localhost:3000/health | jq .
```

```json
{
  "status": "ok",
  "version": "0.1.5",
  "uptime_seconds": 3600,
  "dependencies": {
    "queue": { "status": "up", "detail": "queued=5 processing=2 completed=100" },
    "providers": { "status": "up", "detail": "125 registered" }
  }
}
```

## Endpoint

| Method | Path | Auth Required | Rate Limited |
|:-------|:-----|:-------------|:-------------|
| `GET` | `/health` | No (excluded by default) | Yes |

The health endpoint is **version-independent** — it is mounted at the root path (`/health`), not under `/api/v1`. This ensures it remains stable across API version changes.

## Response Schema

### `HealthResponse`

| Field | Type | Description |
|:------|:-----|:------------|
| `status` | `string` | Overall service status: `"ok"` or `"degraded"` |
| `version` | `string` | Server version (from `Cargo.toml`) |
| `uptime_seconds` | `integer` | Seconds since server started |
| `dependencies` | `DependencyHealth` | Per-component health details |

### `DependencyHealth`

| Field | Type | Description |
|:------|:-----|:------------|
| `queue` | `ComponentHealth` | Queue backend health |
| `providers` | `ComponentHealth` | Provider registry health |

### `ComponentHealth`

| Field | Type | Description |
|:------|:-----|:------------|
| `status` | `string` | `"up"` or `"down"` |
| `detail` | `string?` | Optional human-readable details (omitted when `null`) |

## Status Values

| Status | HTTP Code | Meaning |
|:-------|:----------|:--------|
| `ok` | `200` | All dependencies are healthy |
| `degraded` | `200` | One or more dependencies are unhealthy |

::: tip Why always 200?
Even when the service is degraded, the endpoint returns `200 OK`. This lets load balancers distinguish between "process alive but partially degraded" and "process completely down" (no response / connection refused). If you need stricter health gating, check the `status` field in the response body.
:::

## Dependency Checks

### Queue Backend

The health check calls `queue.stats()` internally. If the call succeeds, the queue is `"up"` with a detail string showing current counters:

```
queued=5 processing=2 completed=100
```

If the stats call fails (e.g., SQLite connection lost), the queue status becomes `"down"` with the error message in `detail`.

### Provider Registry

The health check counts registered providers via `registry.all_providers().len()`. If at least one provider is registered, the status is `"up"`:

```
125 registered
```

If zero providers are registered (which should not happen in normal operation), the status is `"down"`.

## Authentication Bypass

The `/health` endpoint is **excluded from API key authentication** by default. This is critical for:

- Load balancer health probes (which typically cannot send auth headers)
- Kubernetes liveness/readiness probes
- Docker `HEALTHCHECK` instructions
- External monitoring systems

### Default Excluded Paths

```bash
# Default value (no configuration needed)
NOTI_AUTH_EXCLUDED_PATHS=/health
```

You can add more excluded paths if needed:

```bash
NOTI_AUTH_EXCLUDED_PATHS=/health,/api/v1/metrics
```

::: warning
Do not remove `/health` from the excluded paths unless you have a specific reason and have updated all health probes to include authentication headers.
:::

## Docker Health Check

### Dockerfile

The official Docker image includes a built-in health check using `curl`:

```dockerfile
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["curl", "-sf", "http://localhost:3000/health"]
```

| Parameter | Value | Description |
|:----------|:------|:------------|
| `--interval` | `30s` | Time between health checks |
| `--timeout` | `5s` | Maximum time for a single check |
| `--start-period` | `10s` | Grace period after container start |
| `--retries` | `3` | Consecutive failures before marking unhealthy |

The runtime image includes `curl` specifically for health checking.

### Docker Compose

The production service in `docker-compose.yml` mirrors the Dockerfile configuration:

```yaml
services:
  noti-server:
    build: .
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      start_period: 10s
      retries: 3
```

Check container health status:

```bash
# View health status
docker inspect --format='{{.State.Health.Status}}' noti-server

# Watch health transitions
docker events --filter event=health_status
```

### Custom Port

If you change the server port, update the health check URL accordingly:

```yaml
services:
  noti-server:
    environment:
      NOTI_PORT: "8080"
    healthcheck:
      test: ["CMD", "curl", "-sf", "http://localhost:8080/health"]
```

## Kubernetes Integration

### Liveness Probe

Detects if the process is alive and restarts if not:

```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
    - name: noti-server
      image: ghcr.io/loonghao/noti-server:latest
      livenessProbe:
        httpGet:
          path: /health
          port: 3000
        initialDelaySeconds: 10
        periodSeconds: 30
        timeoutSeconds: 5
        failureThreshold: 3
```

### Readiness Probe

Controls whether the pod receives traffic:

```yaml
      readinessProbe:
        httpGet:
          path: /health
          port: 3000
        initialDelaySeconds: 5
        periodSeconds: 10
        timeoutSeconds: 3
        failureThreshold: 2
```

### Startup Probe

Prevents liveness probe kills during slow startups:

```yaml
      startupProbe:
        httpGet:
          path: /health
          port: 3000
        initialDelaySeconds: 5
        periodSeconds: 5
        failureThreshold: 12   # 5 + 12*5 = 65s max startup
```

### Advanced: Check Degraded Status

For readiness probes that should remove degraded instances from the load balancer, use a script-based probe:

```yaml
      readinessProbe:
        exec:
          command:
            - sh
            - -c
            - |
              STATUS=$(curl -sf http://localhost:3000/health | jq -r '.status')
              [ "$STATUS" = "ok" ]
        periodSeconds: 10
        failureThreshold: 2
```

## Monitoring Integration

### Polling Script

```bash
#!/bin/bash
# health-monitor.sh — Periodically check noti health

NOTI_URL="${NOTI_URL:-http://localhost:3000}"
INTERVAL="${INTERVAL:-30}"

while true; do
  RESPONSE=$(curl -sf "$NOTI_URL/health" 2>/dev/null)
  STATUS=$?

  if [ $STATUS -ne 0 ]; then
    echo "[$(date -Iseconds)] CRITICAL: Health endpoint unreachable"
  else
    HEALTH=$(echo "$RESPONSE" | jq -r '.status')
    UPTIME=$(echo "$RESPONSE" | jq -r '.uptime_seconds')
    QUEUE=$(echo "$RESPONSE" | jq -r '.dependencies.queue.status')
    PROVIDERS=$(echo "$RESPONSE" | jq -r '.dependencies.providers.status')

    if [ "$HEALTH" = "ok" ]; then
      echo "[$(date -Iseconds)] OK: uptime=${UPTIME}s queue=${QUEUE} providers=${PROVIDERS}"
    else
      echo "[$(date -Iseconds)] DEGRADED: queue=${QUEUE} providers=${PROVIDERS}"
    fi
  fi

  sleep "$INTERVAL"
done
```

### Python Health Monitor

```python
import requests
import time
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("noti-health")

def check_health(base_url: str = "http://localhost:3000") -> dict:
    """Check noti-server health and return parsed response."""
    try:
        resp = requests.get(f"{base_url}/health", timeout=5)
        resp.raise_for_status()
        data = resp.json()
        return {
            "reachable": True,
            "status": data["status"],
            "version": data["version"],
            "uptime": data["uptime_seconds"],
            "queue": data["dependencies"]["queue"]["status"],
            "providers": data["dependencies"]["providers"]["status"],
        }
    except requests.RequestException as e:
        return {"reachable": False, "error": str(e)}

# Example: periodic monitoring
while True:
    result = check_health()
    if not result["reachable"]:
        logger.critical("Health endpoint unreachable: %s", result["error"])
    elif result["status"] == "degraded":
        logger.warning("Service degraded: queue=%s providers=%s",
                       result["queue"], result["providers"])
    else:
        logger.info("Healthy: v%s uptime=%ds", result["version"], result["uptime"])
    time.sleep(30)
```

### Prometheus / Grafana

Since `/health` returns JSON (not Prometheus exposition format), use a JSON exporter or a custom scraper to convert health status into metrics:

```yaml
# prometheus.yml — Blackbox exporter for HTTP probe
scrape_configs:
  - job_name: "noti-health"
    metrics_path: /probe
    params:
      module: [http_2xx]
    static_configs:
      - targets: ["http://noti-server:3000/health"]
    relabel_configs:
      - source_labels: [__address__]
        target_label: __param_target
      - source_labels: [__param_target]
        target_label: instance
      - target_label: __address__
        replacement: blackbox-exporter:9115
```

### Uptime Services

External uptime monitoring services (UptimeRobot, Pingdom, Better Uptime, etc.) can be configured to:

1. **Probe URL**: `https://your-domain.com/health`
2. **Expected status code**: `200`
3. **Keyword check**: Verify response contains `"status":"ok"` for stricter monitoring
4. **Alert on**: Status code ≠ 200 or keyword missing

## Middleware Interaction

The health endpoint interacts with the middleware stack as follows:

```
CORS → Trace → RequestId → Auth (bypassed) → Rate-limit → BodyLimit → /health handler
```

Key behaviors:

| Middleware | Behavior with /health |
|:-----------|:---------------------|
| **CORS** | Applied — preflight requests handled normally |
| **Trace** | Applied — health check requests appear in logs |
| **Request ID** | Applied — each health check gets a unique `X-Request-Id` |
| **Auth** | **Bypassed** — `/health` is in `excluded_paths` by default |
| **Rate Limit** | Applied — health checks consume rate limit tokens |
| **Body Limit** | Applied (no effect — GET requests have no body) |

::: tip Rate limiting consideration
Health check probes from load balancers and monitoring systems count toward the rate limit. If you have aggressive health probing (e.g., every 5 seconds from multiple sources), increase `NOTI_RATE_LIMIT_MAX` or switch to `NOTI_RATE_LIMIT_PER_IP=true` (default) so probes from the load balancer IP don't starve real clients.
:::

## OpenAPI Schema

The health check endpoint is fully documented in the OpenAPI specification:

- **Tag**: `Health`
- **Path**: `/health`
- **Schemas**: `HealthResponse`, `DependencyHealth`, `ComponentHealth`

View the schema at:

```bash
# Via OpenAPI JSON
curl -s http://localhost:3000/api-docs/openapi.json | jq '.paths["/health"]'

# Via Swagger UI
open http://localhost:3000/swagger-ui/
```

## Troubleshooting

| Symptom | Cause | Solution |
|:--------|:------|:---------|
| `status: "degraded"`, queue `"down"` | SQLite connection lost or file locked | Check `NOTI_QUEUE_DB_PATH` permissions, ensure no file locks |
| `status: "degraded"`, providers `"down"` | Zero providers registered | This should not happen — check for startup errors in logs |
| Connection refused on `/health` | Server not running or wrong port | Verify `NOTI_HOST` and `NOTI_PORT`, check `docker ps` |
| `401 Unauthorized` on `/health` | `/health` removed from excluded paths | Reset `NOTI_AUTH_EXCLUDED_PATHS` to include `/health` |
| Docker health check failing | Port mismatch or `curl` not installed | Ensure `NOTI_PORT` matches the health check URL |
| High latency on `/health` | SQLite under heavy write load | Consider switching to in-memory queue for the health check path, or increase SQLite `busy_timeout` |

## AI Agent Tips

::: info For AI Agents
- Use `GET /health` as the first call to verify server availability before sending notifications.
- Parse `status` field: `"ok"` means all systems operational; `"degraded"` means proceed with caution.
- Check `dependencies.queue.status` before enqueuing async tasks — if `"down"`, fall back to synchronous `/api/v1/send`.
- The `uptime_seconds` field helps detect recent restarts that might indicate instability.
- No authentication is required for `/health` — it works out of the box.
- Example check:
  ```bash
  STATUS=$(curl -sf http://localhost:3000/health | jq -r '.status')
  if [ "$STATUS" != "ok" ]; then
    echo "Warning: service is $STATUS"
  fi
  ```
:::
