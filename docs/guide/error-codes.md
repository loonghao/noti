# Error Codes

Every noti-server API error response includes a standard `{error, message}` shape. Most errors also carry a granular `code` field that provides precise, machine-readable classification for programmatic handling.

## Quick Start

```bash
# Trigger a PROVIDER_NOT_FOUND error
curl -s http://localhost:3000/api/v1/providers/nonexistent | jq .
# {
#   "error": "not_found",
#   "message": "provider 'nonexistent' not found",
#   "code": "PROVIDER_NOT_FOUND"
# }

# Use the code field for precise error routing
curl -s http://localhost:3000/api/v1/send \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "text": "hello", "config": {}}' | jq .code
# "CONFIG_VALIDATION_FAILED"
```

## Error Response Structure

All API errors follow this JSON shape:

```json
{
  "error": "not_found",
  "message": "provider 'nonexistent' not found",
  "code": "PROVIDER_NOT_FOUND"
}
```

| Field | Type | Description |
|:------|:-----|:------------|
| `error` | `string` | HTTP-level error category (`bad_request`, `not_found`, `internal_error`, `service_unavailable`, etc.) |
| `message` | `string` | Human-readable description of the error |
| `code` | `string?` | Granular error code in `UPPER_SNAKE_CASE` — **omitted when not applicable** |

### Backward Compatibility

The `code` field is **optional**. It is omitted from the response body when no granular code applies (e.g., generic JSON parse errors, validation errors handled by `ValidatedJson`). Clients should always check for its presence before using it:

```javascript
// JavaScript / TypeScript
const resp = await fetch('/api/v1/send', { ... });
if (!resp.ok) {
  const body = await resp.json();
  if (body.code === 'CONFIG_VALIDATION_FAILED') {
    // Handle specific error
  } else {
    // Fall back to HTTP status or body.error
  }
}
```

```python
# Python (requests)
resp = requests.post(f"{base}/api/v1/send", json=payload)
if not resp.ok:
    body = resp.json()
    code = body.get("code")  # May be None
    if code == "PROVIDER_NOT_FOUND":
        print(f"Unknown provider: {body['message']}")
    elif code == "CONFIG_VALIDATION_FAILED":
        print(f"Bad config: {body['message']}")
    else:
        print(f"Error ({body['error']}): {body['message']}")
```

## Error Code Reference

### 400 — Bad Request

Errors caused by invalid client input.

| Code | Description | Trigger |
|:-----|:------------|:--------|
| `CONFIG_VALIDATION_FAILED` | Provider-specific configuration failed validation | Missing required config keys (e.g., `webhook` for Slack) |
| `INVALID_PARAMETER` | A query or path parameter value is invalid | Bad `?status=` filter, `delay_seconds` + `scheduled_at` both provided, invalid RFC 3339 timestamp |
| `TEMPLATE_VARIABLE_MISSING` | Required template variables are missing during render | Rendering a template without providing all required (no-default) variables |

#### Examples

**CONFIG_VALIDATION_FAILED** — missing `webhook` for Slack:

```bash
curl -s -X POST http://localhost:3000/api/v1/send \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "text": "hello", "config": {}}'
```

```json
{
  "error": "bad_request",
  "message": "slack requires parameter 'webhook'",
  "code": "CONFIG_VALIDATION_FAILED"
}
```

**INVALID_PARAMETER** — bad status filter:

```bash
curl -s "http://localhost:3000/api/v1/queue/tasks?status=bogus"
```

```json
{
  "error": "bad_request",
  "message": "invalid status filter 'bogus'; expected one of: queued, processing, completed, failed, cancelled",
  "code": "INVALID_PARAMETER"
}
```

**INVALID_PARAMETER** — mutually exclusive scheduling:

```bash
curl -s -X POST http://localhost:3000/api/v1/send/async \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "config": {"webhook": "..."}, "text": "test", "delay_seconds": 60, "scheduled_at": "2025-08-15T09:00:00Z"}'
```

```json
{
  "error": "bad_request",
  "message": "delay_seconds and scheduled_at are mutually exclusive; provide only one",
  "code": "INVALID_PARAMETER"
}
```

**TEMPLATE_VARIABLE_MISSING** — incomplete render variables:

```bash
# Create a template that requires variables "a" and "b"
curl -s -X POST http://localhost:3000/api/v1/templates \
  -H 'Content-Type: application/json' \
  -d '{"name": "greeting", "body": "Hello {{a}} and {{b}}"}'

# Render with only "a" — missing "b"
curl -s -X POST http://localhost:3000/api/v1/templates/greeting/render \
  -H 'Content-Type: application/json' \
  -d '{"variables": {"a": "Alice"}}'
```

```json
{
  "error": "bad_request",
  "message": "missing required template variables: b",
  "code": "TEMPLATE_VARIABLE_MISSING"
}
```

### 404 — Not Found

Errors caused by referencing a non-existent resource.

| Code | Description | Trigger |
|:-----|:------------|:--------|
| `PROVIDER_NOT_FOUND` | The requested notification provider does not exist | `GET /api/v1/providers/:name`, send with unknown provider |
| `TEMPLATE_NOT_FOUND` | The requested message template does not exist | `GET /api/v1/templates/:name`, render/update/delete unknown template |
| `NOTIFICATION_NOT_FOUND` | The requested notification (delivery tracking) does not exist | `GET /api/v1/status/:id` with unknown notification ID |
| `TASK_NOT_FOUND` | The requested queue task does not exist | `GET /api/v1/queue/tasks/:id` with unknown task ID |

#### Examples

**PROVIDER_NOT_FOUND:**

```bash
curl -s http://localhost:3000/api/v1/providers/nonexistent
```

```json
{
  "error": "not_found",
  "message": "provider 'nonexistent' not found",
  "code": "PROVIDER_NOT_FOUND"
}
```

**TEMPLATE_NOT_FOUND:**

```bash
curl -s http://localhost:3000/api/v1/templates/nonexistent
```

```json
{
  "error": "not_found",
  "message": "template 'nonexistent' not found",
  "code": "TEMPLATE_NOT_FOUND"
}
```

**NOTIFICATION_NOT_FOUND:**

```bash
curl -s http://localhost:3000/api/v1/status/nonexistent-id
```

```json
{
  "error": "not_found",
  "message": "notification 'nonexistent-id' not found",
  "code": "NOTIFICATION_NOT_FOUND"
}
```

**TASK_NOT_FOUND:**

```bash
curl -s http://localhost:3000/api/v1/queue/tasks/nonexistent-id
```

```json
{
  "error": "not_found",
  "message": "task 'nonexistent-id' not found",
  "code": "TASK_NOT_FOUND"
}
```

### 500 — Internal Server Error

Errors caused by server-side failures. These typically indicate transient issues and may be retried.

| Code | Description | Trigger |
|:-----|:------------|:--------|
| `QUEUE_BACKEND_ERROR` | An internal queue backend error occurred | SQLite I/O failure, database corruption |
| `QUEUE_SERIALIZATION_ERROR` | Serialization/deserialization error in the queue layer | Corrupt task data in SQLite, JSON encoding failure |
| `QUEUE_SHUT_DOWN` | The queue has been shut down | Server is shutting down gracefully |
| `NOTIFICATION_SEND_ERROR` | A notification send error from the core layer | Provider send failure propagated through queue worker |

#### Client Handling

500-level errors with queue-related codes are often transient. Recommended client behavior:

```python
import time

def send_with_retry(url, payload, max_retries=3):
    for attempt in range(max_retries):
        resp = requests.post(url, json=payload)
        if resp.ok:
            return resp.json()

        body = resp.json()
        code = body.get("code", "")

        # Transient queue errors — retry with backoff
        if code in ("QUEUE_BACKEND_ERROR", "QUEUE_SHUT_DOWN"):
            delay = 2 ** attempt  # 1s, 2s, 4s
            print(f"Transient error ({code}), retrying in {delay}s...")
            time.sleep(delay)
            continue

        # Non-retryable errors — fail immediately
        raise Exception(f"API error: {body['message']} (code={code})")

    raise Exception("Max retries exceeded")
```

### 503 — Service Unavailable

| Code | Description | Trigger |
|:-----|:------------|:--------|
| `QUEUE_FULL` | The queue is at capacity and cannot accept more tasks | Enqueue when queue has reached its capacity limit |

#### Example

```bash
# Queue is at capacity
curl -s -X POST http://localhost:3000/api/v1/send/async \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "config": {"webhook": "..."}, "text": "hello"}'
```

```json
{
  "error": "service_unavailable",
  "message": "queue full: capacity 1000, current size 1000",
  "code": "QUEUE_FULL"
}
```

#### Client Handling

When receiving `QUEUE_FULL`, clients should back off and retry:

```bash
# Shell: retry with exponential backoff on 503
for i in 1 2 3 4 5; do
  RESP=$(curl -s -w "\n%{http_code}" -X POST http://localhost:3000/api/v1/send/async \
    -H 'Content-Type: application/json' \
    -d '{"provider": "slack", "config": {"webhook": "..."}, "text": "hello"}')
  HTTP_CODE=$(echo "$RESP" | tail -1)
  if [ "$HTTP_CODE" != "503" ]; then
    echo "Success or non-retryable error"
    break
  fi
  DELAY=$((2 ** i))
  echo "Queue full, retrying in ${DELAY}s..."
  sleep "$DELAY"
done
```

## Errors Without a Code Field

Some error responses intentionally omit the `code` field for backward compatibility. These include:

| HTTP Status | Error Category | Scenario |
|:------------|:---------------|:---------|
| 400 | `invalid_json` | Malformed JSON request body (not parseable) |
| 401 | `unauthorized` | Missing or invalid API key |
| 413 | *(framework rejection)* | Request body exceeds `NOTI_MAX_BODY_SIZE` before the request reaches the handler |
| 422 | `validation_failed` | Field-level validation errors (from `ValidatedJson`) |
| 429 | `rate limit exceeded` | Rate limit exceeded |

### Validation Errors (422)

Field-level validation errors use a different structure with a `fields` object:

```json
{
  "error": "validation_failed",
  "message": "Request body validation failed",
  "fields": {
    "provider": [{"code": "length", "message": "provider must not be empty"}],
    "text": [{"code": "length", "message": "text must not be empty"}]
  }
}
```

Note: The `fields.*.code` values (e.g., `"length"`) are **validator-level codes**, distinct from the API-level error codes documented above.

### Rate Limit Errors (429)

Rate limit responses include additional fields for retry guidance:

```json
{
  "error": "rate limit exceeded",
  "retry_after_seconds": 12,
  "limit": 100
}
```

The response also includes a `Retry-After` HTTP header.

## Error Code Architecture

The error code system uses a two-level classification:

```
┌──────────────────────────────────────────────────────────┐
│                    API Response JSON                       │
│                                                            │
│  "error":   HTTP-level category (broad classification)     │
│             bad_request, not_found, internal_error, ...    │
│                                                            │
│  "code":    Business-level code (precise identification)   │
│             PROVIDER_NOT_FOUND, QUEUE_FULL, ...            │
│             (optional — omitted when not applicable)       │
│                                                            │
│  "message": Human-readable description                     │
│             (always present, may change between versions)  │
└──────────────────────────────────────────────────────────┘
```

### Error Flow

```
noti-core::NotiError ──┐
                       ├──→ noti-queue::QueueError ──→ queue_error() ──→ ApiError.with_code()
                       │                                                        │
Handlers ──────────────┘────────────────────────────────────────────────────────┘
                                                                                │
                                                                   JSON Response
                                                              {error, message, code?}
```

### Code Conventions

- All error codes use **`UPPER_SNAKE_CASE`** format
- Codes are defined as constants in `noti_server::handlers::error::codes`
- The `code` field is serialized only when present (via `#[serde(skip_serializing_if = "Option::is_none")]`)
- Clients should **never** match on `message` strings — use `error` and `code` for programmatic handling

## Complete Error Code Table

| Code | HTTP | Category | Description |
|:-----|:-----|:---------|:------------|
| `CONFIG_VALIDATION_FAILED` | 400 | `bad_request` | Provider config validation failed |
| `INVALID_PARAMETER` | 400 | `bad_request` | Invalid query/path parameter |
| `TEMPLATE_VARIABLE_MISSING` | 400 | `bad_request` | Missing required template variables |
| `PROVIDER_NOT_FOUND` | 404 | `not_found` | Unknown notification provider |
| `TEMPLATE_NOT_FOUND` | 404 | `not_found` | Unknown message template |
| `NOTIFICATION_NOT_FOUND` | 404 | `not_found` | Unknown notification (delivery tracking) |
| `TASK_NOT_FOUND` | 404 | `not_found` | Unknown queue task |
| `QUEUE_BACKEND_ERROR` | 500 | `internal_error` | Queue backend I/O error |
| `QUEUE_SERIALIZATION_ERROR` | 500 | `internal_error` | Queue serialization failure |
| `QUEUE_SHUT_DOWN` | 500 | `internal_error` | Queue has been shut down |
| `NOTIFICATION_SEND_ERROR` | 500 | `internal_error` | Core notification send error |
| `QUEUE_FULL` | 503 | `service_unavailable` | Queue at capacity |

## Client SDK Patterns

### JavaScript / TypeScript

```typescript
class NotiError extends Error {
  constructor(
    public readonly httpStatus: number,
    public readonly error: string,
    public readonly code: string | null,
    message: string
  ) {
    super(message);
    this.name = 'NotiError';
  }

  get isRetryable(): boolean {
    return ['QUEUE_BACKEND_ERROR', 'QUEUE_SHUT_DOWN', 'QUEUE_FULL'].includes(this.code ?? '');
  }

  get isNotFound(): boolean {
    return this.error === 'not_found';
  }
}

async function notiSend(baseUrl: string, payload: object): Promise<object> {
  const resp = await fetch(`${baseUrl}/api/v1/send`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  });

  if (!resp.ok) {
    const body = await resp.json();
    throw new NotiError(resp.status, body.error, body.code ?? null, body.message);
  }

  return resp.json();
}

// Usage
try {
  await notiSend('http://localhost:3000', { provider: 'slack', text: 'hello', config: {} });
} catch (e) {
  if (e instanceof NotiError) {
    switch (e.code) {
      case 'CONFIG_VALIDATION_FAILED':
        console.error('Check your provider config:', e.message);
        break;
      case 'PROVIDER_NOT_FOUND':
        console.error('Unknown provider:', e.message);
        break;
      default:
        if (e.isRetryable) {
          console.warn('Transient error, will retry:', e.message);
        } else {
          console.error('Unrecoverable error:', e.message);
        }
    }
  }
}
```

### Go

```go
type NotiError struct {
    HTTPStatus int
    Error      string  `json:"error"`
    Message    string  `json:"message"`
    Code       *string `json:"code,omitempty"`
}

func (e *NotiError) IsRetryable() bool {
    if e.Code == nil {
        return false
    }
    switch *e.Code {
    case "QUEUE_BACKEND_ERROR", "QUEUE_SHUT_DOWN", "QUEUE_FULL":
        return true
    }
    return false
}

func (e *NotiError) IsNotFound() bool {
    return e.Error == "not_found"
}
```

### Rust

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NotiApiError {
    pub error: String,
    pub message: String,
    pub code: Option<String>,
}

impl NotiApiError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.code.as_deref(),
            Some("QUEUE_BACKEND_ERROR" | "QUEUE_SHUT_DOWN" | "QUEUE_FULL")
        )
    }
}
```

## AI Agent Tips

- **Always use `--json` or `NOTI_OUTPUT=json`** for structured error output
- **Match on `code` first**, fall back to `error` — the `code` field is the most precise identifier
- **Never match on `message`** — messages are human-readable and may change between versions
- **Check `code` existence** before using it — some errors omit the field entirely
- **Use `--dry-run` before mutations** to catch `CONFIG_VALIDATION_FAILED` without side effects
- **Implement exponential backoff** for `QUEUE_FULL` (503) and `QUEUE_BACKEND_ERROR` (500)
- **Treat `QUEUE_SHUT_DOWN`** as a signal to stop sending — the server is shutting down gracefully
- **Monitor error code frequencies** via logs to detect provider issues (`NOTIFICATION_SEND_ERROR`) or capacity problems (`QUEUE_FULL`)
