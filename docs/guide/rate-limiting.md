# Rate Limiting

Protect the noti-server from abuse and ensure fair resource allocation with configurable token-bucket rate limiting. Supports both **global** and **per-IP** modes.

## How It Works

The rate limiter uses a **token bucket** algorithm:

1. Each bucket starts with `max_requests` tokens.
2. Tokens refill at a constant rate over the configured `window` duration.
3. Every incoming request consumes **one token**.
4. When no tokens remain, the server responds with **429 Too Many Requests**.

This approach allows short bursts of traffic up to the bucket capacity while enforcing a sustainable average rate over time.

```
Bucket capacity: 100 tokens
Window: 60 seconds
Refill rate: 100 / 60 ≈ 1.67 tokens/sec

Request arrives → consume 1 token → 99 remaining → allowed
  ...
Token 0 → 429 Too Many Requests
  ...wait for refill...
Token replenished → next request allowed
```

## Configuration

Rate limiting is configured entirely via environment variables:

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_RATE_LIMIT_MAX` | `100` | Maximum requests allowed per window |
| `NOTI_RATE_LIMIT_WINDOW_SECS` | `60` | Window duration in seconds |
| `NOTI_RATE_LIMIT_PER_IP` | `true` | Per-IP rate limiting (`true`) or global (`false`) |

### Examples

**Default (100 req/min per IP)**:

```bash
# No configuration needed — defaults are production-ready
noti-server
```

**High-traffic API (1000 req/min global)**:

```bash
export NOTI_RATE_LIMIT_MAX=1000
export NOTI_RATE_LIMIT_WINDOW_SECS=60
export NOTI_RATE_LIMIT_PER_IP=false
noti-server
```

**Strict per-IP limiting (10 req/min)**:

```bash
export NOTI_RATE_LIMIT_MAX=10
export NOTI_RATE_LIMIT_WINDOW_SECS=60
export NOTI_RATE_LIMIT_PER_IP=true
noti-server
```

## Global vs Per-IP Mode

### Per-IP Mode (Default)

When `NOTI_RATE_LIMIT_PER_IP=true`, each client IP address gets its own independent token bucket. This means:

- One client hitting the limit does **not** affect other clients.
- Each IP is tracked independently with its own refill schedule.
- Up to **10,000** IPs are tracked simultaneously.

When the IP tracking table is full, idle (fully-refilled) buckets are automatically evicted to make room for new clients.

### Global Mode

When `NOTI_RATE_LIMIT_PER_IP=false`, all requests share a single token bucket regardless of source IP. This is useful for:

- Internal-only deployments behind a load balancer
- Protecting backend resources with an absolute throughput cap
- Simpler setups where per-client fairness is not required

## Client IP Detection

In per-IP mode, the client IP is extracted in priority order:

| Priority | Source | Use Case |
|:---------|:-------|:---------|
| 1 | `X-Forwarded-For` header (first IP) | Behind a reverse proxy (nginx, AWS ALB) |
| 2 | `X-Real-IP` header | Behind nginx with `proxy_set_header` |
| 3 | TCP connection address (`ConnectInfo`) | Direct connections |

::: warning
When deploying behind a reverse proxy, ensure the proxy sets `X-Forwarded-For` or `X-Real-IP` correctly. Without these headers, all requests may appear to come from the proxy's IP, effectively converting per-IP mode into global mode.
:::

### Reverse Proxy Configuration

**nginx**:

```nginx
location / {
    proxy_pass http://localhost:3000;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Real-IP $remote_addr;
}
```

**Caddy**:

```caddyfile
reverse_proxy localhost:3000 {
    header_up X-Forwarded-For {remote_host}
    header_up X-Real-IP {remote_host}
}
```

## Response Headers

Every successful response includes rate limit information:

| Header | Description | Example |
|:-------|:------------|:--------|
| `x-ratelimit-limit` | Maximum requests per window | `100` |
| `x-ratelimit-remaining` | Remaining requests in current window | `87` |

```bash
curl -v http://localhost:3000/api/v1/providers
# < x-ratelimit-limit: 100
# < x-ratelimit-remaining: 99
```

## 429 Response

When the rate limit is exceeded, the server responds with:

**HTTP Status**: `429 Too Many Requests`

**Headers**:

| Header | Description |
|:-------|:------------|
| `Retry-After` | Seconds to wait before retrying |
| `x-ratelimit-limit` | Maximum requests per window |
| `x-ratelimit-remaining` | `0` |

**Body**:

```json
{
  "error": "rate limit exceeded",
  "retry_after_seconds": 12,
  "limit": 100
}
```

### Handling 429 in Clients

**Shell (curl)**:

```bash
response=$(curl -s -w "\n%{http_code}" http://localhost:3000/api/v1/send -X POST \
  -H 'Content-Type: application/json' \
  -d '{"provider":"slack","text":"hello","config":{"webhook":"..."}}')

status=$(echo "$response" | tail -1)
if [ "$status" = "429" ]; then
  retry_after=$(echo "$response" | head -1 | jq -r '.retry_after_seconds')
  echo "Rate limited. Retrying in ${retry_after}s..."
  sleep "$retry_after"
fi
```

**Python**:

```python
import requests
import time

resp = requests.post("http://localhost:3000/api/v1/send", json={...})
if resp.status_code == 429:
    retry_after = resp.json()["retry_after_seconds"]
    time.sleep(retry_after)
    resp = requests.post(...)  # retry
```

**JavaScript (fetch)**:

```javascript
const resp = await fetch("http://localhost:3000/api/v1/send", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ provider: "slack", text: "hello", config: { webhook: "..." } }),
});

if (resp.status === 429) {
  const { retry_after_seconds } = await resp.json();
  await new Promise((r) => setTimeout(r, retry_after_seconds * 1000));
  // retry...
}
```

## Middleware Order

Rate limiting runs **after** authentication in the middleware stack:

```
CORS → Trace → RequestId → Auth → Rate-limit → BodyLimit → Router
```

This means:
- **Invalid API keys are rejected** without consuming rate limit tokens.
- Rate limit headers appear on all successful responses, including from the router layer.
- Health checks and other auth-excluded paths still consume rate limit tokens.

## Best Practices

### Production Recommendations

1. **Start conservative**: Begin with the default 100 req/min and adjust based on observed traffic.
2. **Use per-IP mode** for public-facing APIs to prevent a single client from starving others.
3. **Use global mode** for internal microservice communication where all traffic is trusted.
4. **Monitor `x-ratelimit-remaining`** headers to detect clients approaching the limit.
5. **Set up alerts** on 429 response rates to identify abuse or misconfigured clients.

### For AI Agents

When integrating noti-server with AI agents or automation:

1. Always check the `x-ratelimit-remaining` header after each request.
2. Implement exponential backoff when receiving 429 responses.
3. Batch notifications using `/api/v1/send/async/batch` to reduce request count.
4. Use the `Retry-After` header value for optimal retry timing.

### Docker / Kubernetes

```yaml
# docker-compose.yml
services:
  noti-server:
    environment:
      NOTI_RATE_LIMIT_MAX: "500"
      NOTI_RATE_LIMIT_WINDOW_SECS: "60"
      NOTI_RATE_LIMIT_PER_IP: "true"
```

```yaml
# Kubernetes ConfigMap
apiVersion: v1
kind: ConfigMap
metadata:
  name: noti-config
data:
  NOTI_RATE_LIMIT_MAX: "500"
  NOTI_RATE_LIMIT_WINDOW_SECS: "60"
  NOTI_RATE_LIMIT_PER_IP: "true"
```
