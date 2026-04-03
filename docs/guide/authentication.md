# Authentication

Protect noti-server API endpoints with API key authentication. Authentication is **disabled by default** — enable it by configuring one or more API keys via environment variables.

## Quick Start

```bash
# Enable authentication with a single key
export NOTI_API_KEYS="my-secret-api-key-1234"
noti-server

# Test with curl
curl -H "Authorization: Bearer my-secret-api-key-1234" \
  http://localhost:3000/api/v1/providers
```

## Configuration

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_API_KEYS` | *(empty)* | Comma-separated list of valid API keys. Empty = auth disabled |
| `NOTI_AUTH_EXCLUDED_PATHS` | `/health` | Comma-separated paths that bypass authentication |

### Multiple API Keys

Support multiple keys for key rotation, multi-tenant access, or per-service credentials:

```bash
export NOTI_API_KEYS="service-a-key,service-b-key,admin-key"
```

All listed keys are equally valid — there is no key hierarchy or permission differentiation.

### Excluded Paths

By default, `/health` bypasses authentication so monitoring tools can check service status without credentials. Add more paths as needed:

```bash
export NOTI_AUTH_EXCLUDED_PATHS="/health,/api/v1/metrics"
```

## Key Delivery

Clients can provide their API key via either of two headers:

### Bearer Token (Recommended)

```
Authorization: Bearer <key>
```

This follows the standard OAuth 2.0 Bearer Token scheme and is widely supported by HTTP clients.

### X-API-Key Header

```
X-API-Key: <key>
```

A simpler alternative, useful when `Authorization` is already used for other purposes.

### Priority

If both headers are present, `Authorization: Bearer` takes priority.

## Behavior

| Scenario | Result |
|:---------|:-------|
| No keys configured (`NOTI_API_KEYS` empty) | All requests pass through — auth disabled |
| Keys configured, valid key provided | Request proceeds normally |
| Keys configured, invalid key provided | `401 Unauthorized` |
| Keys configured, no key provided | `401 Unauthorized` |
| Request to excluded path (e.g., `/health`) | Bypasses auth regardless |

## Error Responses

### 401 — Missing Key

```json
{
  "error": "unauthorized",
  "message": "missing API key — provide via Authorization: Bearer <key> or X-API-Key header"
}
```

### 401 — Invalid Key

```json
{
  "error": "unauthorized",
  "message": "invalid API key"
}
```

## Client Examples

### curl

```bash
# Using Bearer token
curl -H "Authorization: Bearer my-api-key" \
  http://localhost:3000/api/v1/providers

# Using X-API-Key
curl -H "X-API-Key: my-api-key" \
  http://localhost:3000/api/v1/providers
```

### Python (requests)

```python
import requests

API_KEY = "my-api-key"
BASE_URL = "http://localhost:3000"

# Using Bearer token
headers = {"Authorization": f"Bearer {API_KEY}"}
resp = requests.get(f"{BASE_URL}/api/v1/providers", headers=headers)
print(resp.json())
```

### JavaScript (fetch)

```javascript
const API_KEY = "my-api-key";
const BASE_URL = "http://localhost:3000";

const resp = await fetch(`${BASE_URL}/api/v1/providers`, {
  headers: { Authorization: `Bearer ${API_KEY}` },
});
const data = await resp.json();
```

## Middleware Order

Authentication runs **before** rate limiting in the middleware stack:

```
CORS → Trace → RequestId → Auth → Rate-limit → BodyLimit → Router
```

This means:
- **Invalid keys are rejected immediately**, before consuming rate limit tokens.
- Excluded paths bypass auth but still go through rate limiting.
- The `X-Request-Id` is available in auth rejection responses for debugging.

## Key Management Best Practices

### Key Generation

Generate cryptographically strong API keys:

```bash
# Using openssl
openssl rand -hex 32

# Using Python
python -c "import secrets; print(secrets.token_urlsafe(32))"

# Using Node.js
node -e "console.log(require('crypto').randomBytes(32).toString('hex'))"
```

### Key Rotation

To rotate keys without downtime:

1. **Add the new key** alongside the old one:
   ```bash
   export NOTI_API_KEYS="new-key-456,old-key-123"
   ```
2. **Update all clients** to use the new key.
3. **Remove the old key** once all clients have migrated:
   ```bash
   export NOTI_API_KEYS="new-key-456"
   ```

### Environment Security

- **Never** commit API keys to version control.
- Use secrets managers (Vault, AWS Secrets Manager, etc.) in production.
- Set restrictive file permissions on `.env` files: `chmod 600 .env`.

## Docker / Kubernetes

### Docker Compose

```yaml
services:
  noti-server:
    image: ghcr.io/loonghao/noti-server:latest
    environment:

      NOTI_API_KEYS: "${NOTI_API_KEYS}"
      NOTI_AUTH_EXCLUDED_PATHS: "/health"
    ports:
      - "3000:3000"
```

```bash
# .env file (git-ignored)
NOTI_API_KEYS=production-key-abc123,monitoring-key-xyz789
```

### Kubernetes Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: noti-api-keys
type: Opaque
stringData:
  api-keys: "production-key-abc123,monitoring-key-xyz789"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: noti-server
spec:
  template:
    spec:
      containers:
        - name: noti-server
          env:
            - name: NOTI_API_KEYS
              valueFrom:
                secretKeyRef:
                  name: noti-api-keys
                  key: api-keys
            - name: NOTI_AUTH_EXCLUDED_PATHS
              value: "/health"
```

## Combining with Other Middleware

### Auth + Rate Limiting

When both are enabled, authentication failures do **not** consume rate limit tokens. This prevents attackers from burning through rate limits with invalid credentials.

### Auth + CORS

CORS preflight (`OPTIONS`) requests are handled before authentication, so cross-origin checks work correctly even when auth is enabled.

### Auth + Metrics

To allow Prometheus or other monitoring tools to scrape metrics without authentication:

```bash
export NOTI_AUTH_EXCLUDED_PATHS="/health,/api/v1/metrics"
```

## Troubleshooting

| Symptom | Likely Cause | Fix |
|:--------|:-------------|:----|
| All requests return 401 | API key mismatch | Verify `NOTI_API_KEYS` matches the key in your request |
| Health check fails | `/health` not in excluded paths | Ensure `NOTI_AUTH_EXCLUDED_PATHS` includes `/health` |
| Auth disabled unexpectedly | `NOTI_API_KEYS` is empty or unset | Check environment variable is exported and non-empty |
| Key works in curl but not in app | Whitespace in key | Trim whitespace around the key in your application |
