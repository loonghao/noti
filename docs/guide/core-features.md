# Core Features

Beyond simple notification sending, noti provides a rich set of built-in features for production-grade notification workflows.

## Message Templates

Create reusable message templates with `{{variable}}` placeholder syntax for dynamic content.

### Template Syntax

```rust
use noti_core::{MessageTemplate, TemplateRegistry};
use std::collections::HashMap;

// Create a template with placeholders
let tpl = MessageTemplate::new("deploy-alert", "{{service}} deployed to {{env}} by {{user}}")
    .with_title("Deploy: {{service}}")
    .with_default("env", "staging");

// Render with variables
let mut vars = HashMap::new();
vars.insert("service".to_string(), "api-gateway".to_string());
vars.insert("user".to_string(), "ci-bot".to_string());

let msg = tpl.render(&vars);
// msg.text  = "api-gateway deployed to staging by ci-bot"
// msg.title = Some("Deploy: api-gateway")
```

### Template Registry

The `TemplateRegistry` stores named templates for lookup:

```rust
let mut registry = TemplateRegistry::new();
registry.register(MessageTemplate::new("alert", "Alert: {{message}}"));
registry.register(MessageTemplate::new("info", "Info: {{message}}"));

// Look up and render
if let Some(tpl) = registry.get("alert") {
    let msg = tpl.render(&vars);
}
```

### Variable Validation

Templates can validate that all required variables (those without defaults) are provided:

```rust
let tpl = MessageTemplate::new("test", "{{a}} and {{b}}")
    .with_default("b", "default_b");

let mut vars = HashMap::new();
vars.insert("a".to_string(), "value_a".to_string());
tpl.validate_vars(&vars).unwrap(); // OK — "b" has a default

let empty = HashMap::new();
tpl.validate_vars(&empty).unwrap_err(); // Error — "a" is missing
```

### REST API

The noti-server exposes full CRUD for templates:

| Method | Endpoint | Description |
|:-------|:---------|:------------|
| `POST` | `/api/v1/templates` | Create a template |
| `GET` | `/api/v1/templates` | List all templates |
| `GET` | `/api/v1/templates/:name` | Get a template by name |
| `PUT` | `/api/v1/templates/:name` | Update a template |
| `DELETE` | `/api/v1/templates/:name` | Delete a template |
| `POST` | `/api/v1/templates/:name/render` | Render a template with variables |

## Retry Policies

Configure automatic retry with fixed or exponential backoff for transient failures.

### Policy Types

| Factory | Behavior | Example |
|:--------|:---------|:--------|
| `RetryPolicy::none()` | No retries | Fire-and-forget sends |
| `RetryPolicy::fixed(n, delay)` | Fixed delay between retries | `fixed(3, Duration::from_secs(2))` → 2s, 2s, 2s |
| `RetryPolicy::exponential(n, initial, max)` | Exponential backoff capped at max | `exponential(5, 1s, 30s)` → 1s, 2s, 4s, 8s, 16s |
| `RetryPolicy::default()` | 3 retries, 1s initial, 30s max, 2× multiplier | General-purpose default |

### Retryable vs Non-Retryable Errors

| Error Type | Retryable? | Examples |
|:-----------|:-----------|:---------|
| `Network` | Yes | Timeout, DNS failure |
| `Provider` | Yes | 500 Internal Server Error |
| `Io` | Yes | Transient file read failure |
| `Validation` | No | Missing required parameter |
| `Config` | No | Malformed configuration |
| `UrlParse` | No | Invalid URL scheme |

### Usage

```rust
use noti_core::{RetryPolicy, send_with_retry};

let policy = RetryPolicy::exponential(3, Duration::from_secs(1), Duration::from_secs(10));

let outcome = send_with_retry(&provider, &message, &config, &policy).await;
println!("Attempts: {}, Success: {}", outcome.attempts, outcome.result.is_ok());
```

For non-provider operations, use the generic `execute_with_retry`:

```rust
use noti_core::retry::execute_with_retry;

let outcome = execute_with_retry(&policy, || async {
    // Any fallible async operation
    do_something().await
}).await;
```

### REST API Retry Configuration

When sending notifications via the REST API, you can configure retry behavior
in the `retry` object of the request body:

| Field | Type | Default | Description |
|:------|:-----|:--------|:------------|
| `max_retries` | `u32` | `3` | Maximum retry attempts |
| `delay_ms` | `u64` | `1000` | Base delay in milliseconds |
| `backoff_multiplier` | `f64` | `1.0` | Multiplier for exponential backoff (> 1.0 enables exponential) |
| `max_delay_ms` | `u64` | `30000` | Maximum delay cap for exponential backoff |

**Fixed delay** (default when `backoff_multiplier` is absent or ≤ 1.0):

```json
{
  "retry": { "max_retries": 3, "delay_ms": 500 }
}
```

**Exponential backoff** (set `backoff_multiplier` > 1.0):

```json
{
  "retry": {
    "max_retries": 5,
    "delay_ms": 100,
    "backoff_multiplier": 2.0,
    "max_delay_ms": 10000
  }
}
```

With the above config, retry delays are: 100 ms → 200 ms → 400 ms → 800 ms → 1600 ms.

## Batch & Failover Sending

Send notifications to multiple providers simultaneously or with automatic failover.

### Batch Mode (Parallel)

Send to **all** targets concurrently. Each target independently applies the retry policy.

```rust
use noti_core::{SendTarget, send_batch, RetryPolicy, Message};

let targets = vec![
    SendTarget::new(&slack_provider, &slack_config),
    SendTarget::new(&email_provider, &email_config),
    SendTarget::new(&teams_provider, &teams_config),
];

let result = send_batch(&targets, &message, &RetryPolicy::default()).await;
println!("Succeeded: {}/{}", result.success_count(), targets.len());
```

### Failover Mode (Sequential)

Try each target in order — **stop at the first success**. Useful for redundant delivery.

```rust
use noti_core::send_failover;

let targets = vec![
    SendTarget::new(&primary_provider, &primary_config),
    SendTarget::new(&backup_provider, &backup_config),
];

let result = send_failover(&targets, &message, &RetryPolicy::default()).await;
if result.any_succeeded() {
    println!("Delivered via: {}", result.results.last().unwrap().provider_name);
}
```

### BatchResult API

| Method | Returns | Description |
|:-------|:--------|:------------|
| `success_count()` | `usize` | Number of targets that succeeded |
| `failure_count()` | `usize` | Number of targets that failed |
| `all_succeeded()` | `bool` | Whether every target succeeded |
| `any_succeeded()` | `bool` | Whether at least one succeeded |
| `results` | `Vec<TargetResult>` | Per-target details (provider name, attempts, duration) |

## Delivery Status Tracking

Track the full lifecycle of every notification delivery with an in-memory status store.

### Status Lifecycle

```
Pending → Sending → Delivered
                  ↘ Failed
                  ↘ Cancelled
         Delivered → Read
```

| Status | Terminal? | Description |
|:-------|:----------|:------------|
| `Pending` | No | Queued, not yet dispatched |
| `Sending` | No | Currently in-flight |
| `Delivered` | Yes | Successfully delivered |
| `Failed` | Yes | All retries exhausted |
| `Cancelled` | Yes | Cancelled before completion |
| `Read` | Yes | End user confirmed receipt |

### StatusTracker

The `StatusTracker` provides thread-safe, async-aware tracking:

```rust
use noti_core::{StatusTracker, DeliveryStatus};

let tracker = StatusTracker::new();

// Register deliveries
tracker.track("notif-001", "slack").await;
tracker.track("notif-001", "email").await;

// Update status
tracker.update_status("notif-001", "slack", DeliveryStatus::Delivered, None).await;
tracker.update_status("notif-001", "email", DeliveryStatus::Failed, Some("timeout".into())).await;

// Query
let records = tracker.get_records("notif-001").await;  // 2 records
let summary = tracker.summary().await;
println!("Delivered: {}, Failed: {}", summary.delivered, summary.failed);
```

### DeliveryRecord

Each record maintains a full event history:

```rust
let record = tracker.get_record("notif-001", "slack").await.unwrap();
assert_eq!(record.events.len(), 2);  // Pending → Delivered
assert!(record.is_terminal());
```

## Priority System

Assign priority levels to control processing order and provider-specific urgency flags.

### Priority Levels

| Level | Numeric | Description | String Aliases |
|:------|:--------|:------------|:---------------|
| `Low` | 0 | Informational, non-urgent | `"low"`, `"0"`, `"min"` |
| `Normal` | 1 | Default priority | `"normal"`, `"1"`, `"default"` |
| `High` | 2 | Prompt attention needed | `"high"`, `"2"` |
| `Urgent` | 3 | Critical alert | `"urgent"`, `"critical"`, `"3"`, `"max"`, `"emergency"` |

### Usage

```rust
use noti_core::{Message, Priority};

// Set priority on a message
let msg = Message::text("Server disk at 95%!")
    .with_title("Disk Alert")
    .with_priority(Priority::Urgent);

// Parse from string (e.g., CLI argument)
let priority: Priority = "critical".parse().unwrap();  // → Urgent
```

### Queue Integration

In the async queue system (`noti-queue`), tasks are automatically ordered by priority — `Urgent` tasks are dequeued before `Low` tasks, ensuring critical notifications are processed first.

### REST API

Priority can be set in the JSON body of send requests:

```json
{
  "provider": "slack",
  "text": "Deployment complete",
  "priority": "high"
}
```

## Rate Limiting

Protect the server from abuse with configurable token-bucket rate limiting. Supports both global and per-IP modes.

### How It Works

The middleware uses a **token bucket** algorithm. Each bucket holds `max_requests` tokens and refills at a constant rate over the configured `window`. When a request arrives, it consumes one token; if no tokens remain, the server responds with `429 Too Many Requests`.

### Configuration

Rate limiting is configured via environment variables:

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_RATE_LIMIT_MAX` | `100` | Maximum requests allowed per window |
| `NOTI_RATE_LIMIT_WINDOW_SECS` | `60` | Window duration in seconds |
| `NOTI_RATE_LIMIT_PER_IP` | `true` | Per-IP rate limiting (`true`) or global (`false`) |

### Per-IP Tracking

When `per_ip` is enabled (default), each client IP gets its own independent token bucket. Client IP is extracted in this priority order:

1. `X-Forwarded-For` header (first IP in the comma-separated list)
2. `X-Real-IP` header
3. TCP connection address (`ConnectInfo`)

Up to 10,000 IPs are tracked simultaneously. When the limit is reached, idle (fully-refilled) buckets are evicted.

### Response Headers

Every successful response includes rate limit headers:

| Header | Description |
|:-------|:------------|
| `x-ratelimit-limit` | Maximum requests allowed per window |
| `x-ratelimit-remaining` | Remaining requests in the current window |

### 429 Response

When the rate limit is exceeded, the server responds with:

```json
{
  "error": "rate limit exceeded",
  "retry_after_seconds": 12,
  "limit": 100
}
```

The response also includes a `Retry-After` header with the number of seconds to wait.

## API Key Authentication

Protect API endpoints with API key authentication. Disabled by default; enabled by setting API keys via environment variables.

### Configuration

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_API_KEYS` | *(empty)* | Comma-separated API keys; empty = auth disabled |
| `NOTI_AUTH_EXCLUDED_PATHS` | `/health` | Comma-separated paths that bypass authentication |

### Key Delivery

Clients can provide their API key via either header:

- `Authorization: Bearer <key>`
- `X-API-Key: <key>`

### Behavior

- **Disabled** (no keys configured): all requests pass through.
- **Enabled** (one or more keys configured): requests without a valid key receive `401 Unauthorized`.
- **Excluded paths** (e.g., `/health`): bypass authentication regardless.
- **Middleware ordering**: authentication runs **before** rate limiting — invalid keys are rejected without consuming rate limit tokens.

### 401 Response

```json
{
  "error": "unauthorized",
  "message": "missing API key — provide via Authorization: Bearer <key> or X-API-Key header"
}
```

## CORS (Cross-Origin Resource Sharing)

Control which browser origins can access the noti-server API. CORS is the **outermost** middleware layer, ensuring preflight requests are handled before any other processing.

### Configuration

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_CORS_ALLOWED_ORIGINS` | `*` | Comma-separated allowed origins; `*` = permissive |

### Modes

**Permissive mode** (default): When the variable is unset, empty, or contains `*`, all origins, methods, and headers are allowed.

**Restricted mode**: Set specific origins to lock down access:

```bash
export NOTI_CORS_ALLOWED_ORIGINS="https://dashboard.example.com,https://admin.example.com"
```

In restricted mode, only listed origins are allowed. Methods and headers remain permissive (`Any`).

### Middleware Order

CORS runs first in the middleware stack:

```
CORS → Trace → RequestId → Auth → Rate-limit → BodyLimit → Router
```

This ensures `OPTIONS` preflight requests receive correct headers without hitting auth or rate limiting.

## Request ID Tracking

Every request passing through the server is assigned a unique identifier for end-to-end log correlation.

### How It Works

1. If the client sends an `X-Request-Id` header, the server **preserves** it.
2. Otherwise, the server generates a new **UUID v4**.
3. The ID is injected into a `tracing::info_span`, so **all downstream log entries** automatically include the `request_id` field.
4. The same `X-Request-Id` is **echoed back** in the response headers.

### Usage

**Client-provided ID** (useful for distributed tracing):

```bash
curl -H "X-Request-Id: my-trace-123" http://localhost:3000/api/v1/send ...
# Response header: X-Request-Id: my-trace-123
```

**Server-generated ID** (automatic):

```bash
curl -v http://localhost:3000/health
# Response header: X-Request-Id: a1b2c3d4-e5f6-7890-abcd-ef1234567890
```

### JSON Log Correlation

When `NOTI_LOG_FORMAT=json`, every log line includes the request ID:

```json
{"timestamp":"...","level":"INFO","request_id":"a1b2c3d4-...","method":"POST","path":"/api/v1/send","message":"..."}
```

## Body Size Limit

Protect the server from oversized payloads with a configurable maximum request body size.

### Configuration

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_MAX_BODY_SIZE` | `2097152` (2 MiB) | Maximum request body size in bytes |

### Behavior

Requests exceeding the limit receive a `413 Payload Too Large` response before any parsing occurs. This prevents memory exhaustion from very large payloads.

```bash
# Set a 1 MiB limit
export NOTI_MAX_BODY_SIZE=1048576
```

## Request Validation

All API endpoints that accept JSON bodies use the `ValidatedJson` extractor, which automatically validates the deserialized payload using field-level rules.

### Validation Errors

When validation fails, the server returns `422 Unprocessable Entity` with structured field-level errors:

```json
{
  "error": "validation_failed",
  "message": "Request body validation failed",
  "fields": {
    "provider": ["provider must not be empty"],
    "text": ["text must not be empty"]
  }
}
```

This makes it straightforward for API clients to display per-field error messages.

## Queue Management

The async queue system provides endpoints for monitoring and managing background notification tasks.

### Queue Backends

| Backend | Env Value | Persistence | Description |
|:--------|:----------|:------------|:------------|
| In-Memory | `memory` | No (lost on restart) | Default, fastest, suitable for development |
| SQLite | `sqlite` | Yes | Survives restarts, auto-recovers stale tasks |

Configure via:

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_QUEUE_BACKEND` | `memory` | Queue backend type |
| `NOTI_QUEUE_DB_PATH` | `noti-queue.db` | SQLite database file path |
| `NOTI_WORKER_COUNT` | `4` | Number of background worker threads |

### API Endpoints

| Method | Endpoint | Description |
|:-------|:---------|:------------|
| `POST` | `/api/v1/send/async` | Enqueue a single notification (returns 202 + task_id) |
| `POST` | `/api/v1/send/async/batch` | Enqueue multiple notifications at once |
| `GET` | `/api/v1/queue/tasks` | List tasks with optional `?status=` filter and `?limit=` |
| `GET` | `/api/v1/queue/tasks/:task_id` | Get a single task's status and metadata |
| `POST` | `/api/v1/queue/tasks/:task_id/cancel` | Cancel a queued task |
| `GET` | `/api/v1/queue/stats` | Queue statistics (queued/processing/completed/failed/cancelled) |
| `POST` | `/api/v1/queue/purge` | Remove all terminal tasks (completed, failed, cancelled) |

### Async Send Request

```json
{
  "provider": "slack",
  "config": {"webhook": "https://hooks.slack.com/..."},
  "text": "Deployment complete",
  "title": "Deploy Alert",
  "priority": "high",
  "retry": {"max_retries": 3, "delay_ms": 1000},
  "metadata": {"deploy_id": "d-123", "env": "production"},
  "callback_url": "https://your-server.com/webhook/noti-callback",
  "delay_seconds": 60,
  "scheduled_at": "2025-08-15T09:00:00Z"
}
```

::: warning
`delay_seconds` and `scheduled_at` are **mutually exclusive** — provide at most one. See [Scheduled & Delayed Send](/guide/scheduled-send) for details.
:::

### Task Lifecycle

```
Queued → Processing → Completed
                    ↘ Failed (retries exhausted)
         Queued → Cancelled (via cancel endpoint)
```

### Webhook Callbacks

When `callback_url` is provided, the server sends a POST request to the URL when the task reaches a terminal state (completed or failed):

```json
{
  "task_id": "abc-123",
  "status": "completed",
  "provider": "slack",
  "attempts": 1,
  "metadata": {"deploy_id": "d-123"}
}
```

### Stale Task Recovery

When using the SQLite backend, tasks left in `Processing` state after an unclean shutdown are automatically recovered to `Queued` state on the next server startup.

### Queue Statistics Response

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

## Logging

Configure structured logging for production observability.

### Configuration

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_LOG_LEVEL` | `info` | Tracing filter level (`trace`, `debug`, `info`, `warn`, `error`) |
| `NOTI_LOG_FORMAT` | `text` | Output format: `text` (human-readable) or `json` (structured) |

### JSON Format

Enable structured JSON logging for log aggregation pipelines:

```bash
export NOTI_LOG_FORMAT=json
export NOTI_LOG_LEVEL=info
```

Output includes span events (request lifecycle), flattened fields, and automatic `request_id` correlation from the Request ID middleware.

## Health Check

The `/health` endpoint provides service status including dependency health.

### Response

```json
{
  "status": "ok",
  "version": "0.1.5",
  "uptime_seconds": 3600,
  "dependencies": {
    "queue": {"status": "up", "detail": "queued=5 processing=2 completed=100"},
    "providers": {"status": "up", "detail": "125 registered"}
  }
}
```

| Status | Description |
|:-------|:------------|
| `ok` | All dependencies healthy |
| `degraded` | One or more dependencies unhealthy (still returns 200) |

The health endpoint is excluded from authentication by default (`NOTI_AUTH_EXCLUDED_PATHS=/health`).

## Structured Error Codes

All API error responses include a standard `{error, message}` shape. In addition, most errors now carry a granular `code` field that provides precise machine-readable classification.

### Error Response Structure

```json
{
  "error": "not_found",
  "message": "provider 'nonexistent' not found",
  "code": "PROVIDER_NOT_FOUND"
}
```

| Field | Type | Description |
|:------|:-----|:------------|
| `error` | `string` | HTTP-level error category (`bad_request`, `not_found`, `internal_error`, etc.) |
| `message` | `string` | Human-readable description of the error |
| `code` | `string?` | Granular error code in `UPPER_SNAKE_CASE` — omitted when not applicable |

The `code` field is optional for backward compatibility. Clients should check for its presence before using it.

### Error Code Reference

#### 400 — Bad Request

| Code | Description |
|:-----|:------------|
| `CONFIG_VALIDATION_FAILED` | Provider-specific configuration failed validation |
| `INVALID_PARAMETER` | A query or path parameter value is invalid |
| `TEMPLATE_VARIABLE_MISSING` | Required template variables are missing during render |

#### 404 — Not Found

| Code | Description |
|:-----|:------------|
| `PROVIDER_NOT_FOUND` | The requested notification provider does not exist |
| `TEMPLATE_NOT_FOUND` | The requested message template does not exist |
| `NOTIFICATION_NOT_FOUND` | The requested notification (delivery tracking) does not exist |
| `TASK_NOT_FOUND` | The requested queue task does not exist |

#### 500 — Internal Error

| Code | Description |
|:-----|:------------|
| `QUEUE_BACKEND_ERROR` | An internal queue backend error occurred |
| `QUEUE_SERIALIZATION_ERROR` | Serialization/deserialization error in the queue layer |
| `QUEUE_SHUT_DOWN` | The queue has been shut down |
| `NOTIFICATION_SEND_ERROR` | A notification send error from the core layer |

#### 503 — Service Unavailable

| Code | Description |
|:-----|:------------|
| `QUEUE_FULL` | The queue is at capacity and cannot accept more tasks |

### Usage Example

```bash
# The code field enables precise error handling
curl -s http://localhost:3000/api/v1/providers/nonexistent | jq .
# {
#   "error": "not_found",
#   "message": "provider 'nonexistent' not found",
#   "code": "PROVIDER_NOT_FOUND"
# }

# Validation errors from ValidatedJson do NOT include a code field
curl -s -X POST http://localhost:3000/api/v1/send \
  -H 'Content-Type: application/json' \
  -d '{"provider": "", "text": "hello"}' | jq .
# {
#   "error": "validation_failed",
#   "message": "Request body validation failed",
#   "fields": {"provider": ["provider must not be empty"]}
# }
```
