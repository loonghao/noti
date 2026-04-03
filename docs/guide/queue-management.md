# Queue Management

The noti async queue system decouples notification submission from delivery, enabling background processing with priority ordering, retry backoff, scheduled sends, and webhook callbacks.

## Quick Start

```bash
# Start the server with the in-memory queue (default)
noti-server

# Start with SQLite persistent queue
NOTI_QUEUE_BACKEND=sqlite NOTI_QUEUE_DB_PATH=./noti.db noti-server

# Enqueue a notification
curl -X POST http://localhost:3000/api/v1/send/async \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "config": {"webhook": "https://hooks.slack.com/..."}, "text": "Hello!"}'

# Check queue stats
curl http://localhost:3000/api/v1/queue/stats
```

## Queue Backends

noti ships with two queue backends. Choose one based on your deployment requirements.

| Backend | Env Value | Persistence | Best For |
|:--------|:----------|:------------|:---------|
| In-Memory | `memory` (default) | No — tasks lost on restart | Development, low-latency, single-process |
| SQLite | `sqlite` | Yes — tasks survive restarts | Production, reliability, crash recovery |

### Configuration

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_QUEUE_BACKEND` | `memory` | Backend type. Accepts: `memory`, `mem`, `in-memory`, `sqlite`, `sql`, `db` |
| `NOTI_QUEUE_DB_PATH` | `noti-queue.db` | SQLite database file path (only used when backend is `sqlite`) |
| `NOTI_WORKER_COUNT` | `4` | Number of concurrent background worker tasks |

### Backend Comparison

| Feature | In-Memory | SQLite |
|:--------|:----------|:-------|
| Speed | Fastest (heap-based) | Fast (WAL mode + index) |
| Persistence | None | Full (file-backed) |
| Crash recovery | Tasks lost | Automatic stale task recovery |
| Capacity limit | Optional | Optional |
| Priority ordering | BinaryHeap (O(log n)) | SQL `ORDER BY priority DESC, created_at ASC` |
| Multi-process | No (single process) | Shared file (single writer) |

### SQLite Configuration

The SQLite backend uses optimal settings for queue workloads:

- **WAL mode** — concurrent readers during writes
- **`PRAGMA synchronous=NORMAL`** — balanced durability/performance
- **`PRAGMA busy_timeout=5000`** — 5-second wait on lock contention
- **Composite index**: `(status, priority DESC, created_at ASC)` for efficient dequeue

## API Endpoints

All queue endpoints are under `/api/v1/`.

| Method | Endpoint | Description | Status |
|:-------|:---------|:------------|:-------|
| `POST` | `/send/async` | Enqueue a single notification | 202 |
| `POST` | `/send/async/batch` | Enqueue multiple notifications | 202 |
| `GET` | `/queue/tasks` | List tasks (filter by status) | 200 |
| `GET` | `/queue/tasks/{task_id}` | Get a single task's status | 200/404 |
| `POST` | `/queue/tasks/{task_id}/cancel` | Cancel a queued task | 200 |
| `GET` | `/queue/stats` | Queue statistics | 200 |
| `POST` | `/queue/purge` | Remove all terminal tasks | 200 |

### Enqueue a Notification

```bash
curl -X POST http://localhost:3000/api/v1/send/async \
  -H 'Content-Type: application/json' \
  -d '{
    "provider": "slack",
    "config": {"webhook": "https://hooks.slack.com/..."},
    "text": "Deployment complete",
    "title": "Deploy Alert",
    "priority": "high",
    "retry": {"max_retries": 3, "delay_ms": 1000},
    "metadata": {"deploy_id": "d-123", "env": "production"},
    "callback_url": "https://your-server.com/webhook/noti-callback",
    "delay_seconds": 60
  }'
```

**Response (202 Accepted):**

```json
{
  "task_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "queued",
  "message": "Notification enqueued for async processing"
}
```

#### Request Fields

| Field | Type | Required | Description |
|:------|:-----|:---------|:------------|
| `provider` | `string` | Yes | Provider name (e.g. `"slack"`, `"email"`) |
| `config` | `object` | No | Provider-specific configuration |
| `text` | `string` | Yes | Message body |
| `title` | `string` | No | Message title/subject |
| `format` | `string` | No | `"text"`, `"markdown"`, or `"html"` |
| `priority` | `string` | No | `"low"`, `"normal"` (default), `"high"`, `"urgent"` |
| `retry` | `object` | No | Retry policy (see below) |
| `metadata` | `object` | No | Key-value pairs for tracking/correlation |
| `callback_url` | `string` | No | Webhook URL for task completion notification |
| `delay_seconds` | `integer` | No | Delay before sending (mutually exclusive with `scheduled_at`) |
| `scheduled_at` | `string` | No | RFC 3339 timestamp (mutually exclusive with `delay_seconds`) |

#### Retry Configuration

| Field | Type | Default | Description |
|:------|:-----|:--------|:------------|
| `max_retries` | `u32` | `3` | Maximum retry attempts |
| `delay_ms` | `u64` | `1000` | Base delay in milliseconds |
| `backoff_multiplier` | `f64` | `1.0` | Exponential backoff multiplier (> 1.0 enables exponential) |
| `max_delay_ms` | `u64` | `30000` | Maximum delay cap |

### Batch Enqueue

Enqueue multiple notifications in a single request:

```bash
curl -X POST http://localhost:3000/api/v1/send/async/batch \
  -H 'Content-Type: application/json' \
  -d '{
    "items": [
      {"provider": "slack", "config": {"webhook": "..."}, "text": "Alert 1", "priority": "urgent"},
      {"provider": "email", "config": {"to": "user@example.com"}, "text": "Alert 2"},
      {"provider": "webhook", "config": {"url": "..."}, "text": "Alert 3", "delay_seconds": 300}
    ]
  }'
```

**Response (202 Accepted):**

```json
{
  "results": [
    {"index": 0, "provider": "slack", "success": true, "task_id": "abc-123"},
    {"index": 1, "provider": "email", "success": true, "task_id": "def-456"},
    {"index": 2, "provider": "webhook", "success": true, "task_id": "ghi-789"}
  ],
  "enqueued": 3,
  "failed": 0,
  "total": 3
}
```

Each item in the batch is processed independently — a failure in one item does not block others.

### Get Task Status

```bash
curl http://localhost:3000/api/v1/queue/tasks/abc-123
```

**Response:**

```json
{
  "id": "abc-123",
  "provider": "slack",
  "status": "completed",
  "attempts": 1,
  "priority": "High",
  "metadata": {"deploy_id": "d-123"}
}
```

### List Tasks

```bash
# List all tasks (default limit: 50)
curl http://localhost:3000/api/v1/queue/tasks

# Filter by status
curl "http://localhost:3000/api/v1/queue/tasks?status=failed&limit=10"
```

| Parameter | Type | Default | Description |
|:----------|:-----|:--------|:------------|
| `status` | `string` | — | Filter: `queued`, `processing`, `completed`, `failed`, `cancelled` |
| `limit` | `integer` | `50` | Max results (capped at 1000) |

### Cancel a Task

Only tasks in `queued` status can be cancelled. Processing or terminal tasks cannot be cancelled.

```bash
curl -X POST http://localhost:3000/api/v1/queue/tasks/abc-123/cancel
```

**Response:**

```json
{
  "task_id": "abc-123",
  "cancelled": true,
  "message": "Task cancelled successfully"
}
```

### Queue Statistics

```bash
curl http://localhost:3000/api/v1/queue/stats
```

**Response:**

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

### Purge Terminal Tasks

Remove all completed, failed, and cancelled tasks from the queue. In-memory backend removes entries from the HashMap; SQLite backend DELETEs rows.

```bash
curl -X POST http://localhost:3000/api/v1/queue/purge
```

**Response:**

```json
{
  "purged": 104,
  "message": "Purged 104 terminal tasks"
}
```

After purging, the `completed`, `failed`, and `cancelled` counters in stats reset to zero.

## Task Lifecycle

Every notification task follows this state machine:

```
Queued → Processing → Completed
                    ↘ Failed (retries exhausted)
       Queued → Cancelled (via cancel endpoint)
```

| Status | Terminal? | Description |
|:-------|:----------|:------------|
| `queued` | No | Waiting in queue for a worker |
| `processing` | No | Currently being sent by a worker |
| `completed` | Yes | Successfully delivered |
| `failed` | Yes | All retry attempts exhausted |
| `cancelled` | Yes | Cancelled before processing began |

### Worker Processing Loop

Each worker follows this cycle:

1. **Dequeue** — Picks the highest-priority available task
2. **Lookup provider** — Resolves the provider from the registry
3. **Send** — Calls `provider.send(message, config)`
4. **Ack/Nack** — On success: `ack()` → Completed. On failure: `nack()` → retry or Failed
5. **Callback** — If `callback_url` is set and task reached a terminal state, fires a webhook POST
6. **Wait** — When queue is empty, waits for `task_notify` / `shutdown_notify` / `poll_interval`

## Priority Ordering

Tasks are dequeued in priority order. Within the same priority level, earlier tasks are processed first (FIFO).

| Priority | Numeric | Dequeue Order |
|:---------|:--------|:--------------|
| `urgent` | 3 | First |
| `high` | 2 | Second |
| `normal` | 1 | Third (default) |
| `low` | 0 | Last |

```bash
# Urgent tasks jump to the front of the queue
curl -X POST http://localhost:3000/api/v1/send/async \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "config": {"webhook": "..."}, "text": "Critical alert!", "priority": "urgent"}'
```

## Retry & Backoff

When a notification fails and the task is eligible for retry, the queue automatically re-enqueues it with a backoff delay.

### How Backoff Works

1. Worker calls `nack(task_id, error)`
2. Queue checks `task.should_retry()` based on the retry policy
3. If retryable, computes `task.retry_delay()` and sets `available_at = now + delay`
4. Task returns to `queued` status but won't be dequeued until `available_at` has passed
5. If not retryable, task moves to `failed` status

### Fixed vs Exponential

**Fixed delay** (default):

```json
{"retry": {"max_retries": 3, "delay_ms": 2000}}
```

Delays: 2s → 2s → 2s

**Exponential backoff**:

```json
{"retry": {"max_retries": 5, "delay_ms": 100, "backoff_multiplier": 2.0, "max_delay_ms": 10000}}
```

Delays: 100ms → 200ms → 400ms → 800ms → 1600ms

## Scheduled & Delayed Send

Defer notification delivery to a future time. See [Scheduled & Delayed Send](/guide/scheduled-send) for full details.

```bash
# Delay by 5 minutes
curl -X POST http://localhost:3000/api/v1/send/async \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "config": {"webhook": "..."}, "text": "Reminder!", "delay_seconds": 300}'

# Schedule for a specific time
curl -X POST http://localhost:3000/api/v1/send/async \
  -H 'Content-Type: application/json' \
  -d '{"provider": "slack", "config": {"webhook_url": "..."}, "text": "Morning report", "scheduled_at": "2025-08-15T09:00:00Z"}'

```

::: warning
`delay_seconds` and `scheduled_at` are **mutually exclusive**. Providing both returns a `400 Bad Request`.
:::

## Webhook Callbacks

When a task has a `callback_url`, the server fires an HTTP POST to that URL when the task reaches a terminal state.

```json
{
  "task_id": "abc-123",
  "provider": "slack",
  "status": "completed",
  "attempts": 1,
  "metadata": {"deploy_id": "d-123"}
}
```

Callbacks are **best-effort** — failures are logged but do not affect the task's final status. The callback client uses a shared connection pool with a 10-second timeout.

See [Webhook Callbacks](/guide/webhook-callbacks) for full documentation.

## Stale Task Recovery

When using the SQLite backend, tasks left in `processing` status after an unclean shutdown (crash, OOM kill, etc.) are automatically recovered on the next server startup.

**How it works:**

1. Server starts and calls `queue.recover_stale_tasks()`
2. All tasks with `status = 'processing'` are reset to `status = 'queued'`
3. Workers are notified and begin processing recovered tasks
4. Recovery count is logged: `recovered stale processing tasks`

The in-memory backend skips this step since all state is lost on restart anyway.

## Capacity Limits

Both backends support optional capacity limits to prevent unbounded queue growth.

When the queue is full, enqueue requests receive a `503 Service Unavailable` response:

```json
{
  "error": "service_unavailable",
  "message": "queue full: capacity 1000, current size 1000",
  "code": "QUEUE_FULL"
}
```

Capacity is configured programmatically via `InMemoryQueue::with_capacity(n)` or `SqliteQueue::open_with_capacity(path, n)`. A capacity of `0` means unlimited.

## Error Codes

Queue-related errors include a machine-readable `code` field for programmatic handling:

| Code | HTTP Status | Description |
|:-----|:------------|:------------|
| `TASK_NOT_FOUND` | 404 | The requested task does not exist |
| `QUEUE_FULL` | 503 | Queue at capacity, cannot accept more tasks |
| `QUEUE_BACKEND_ERROR` | 500 | Internal backend error (I/O, SQLite, etc.) |
| `QUEUE_SERIALIZATION_ERROR` | 500 | JSON serialization/deserialization failure |
| `QUEUE_SHUT_DOWN` | 500 | Queue has been shut down |
| `NOTIFICATION_SEND_ERROR` | 500 | Core notification send error |
| `INVALID_PARAMETER` | 400 | Invalid query/path parameter (e.g. bad status filter) |
| `CONFIG_VALIDATION_FAILED` | 400 | Provider config validation failed |

## Graceful Shutdown

On receiving `SIGTERM` or `Ctrl+C`:

1. Server stops accepting new connections
2. Workers receive shutdown signal via `AtomicBool` + `Notify`
3. Workers finish processing their current task (no interruption)
4. `worker_handle.shutdown_and_join().await` waits for all workers to complete
5. Server exits cleanly

No tasks are lost during graceful shutdown — in-progress tasks complete normally, and queued tasks remain in the queue (surviving restart with SQLite backend).

## Production Best Practices

### Worker Count Tuning

- **Default**: 4 workers
- **IO-bound workloads** (webhooks, HTTP APIs): increase to 8–16
- **Rate-limited providers**: keep lower to avoid hitting provider limits
- **Mixed workloads**: start with `NOTI_WORKER_COUNT=8` and monitor

### SQLite for Production

```bash
export NOTI_QUEUE_BACKEND=sqlite
export NOTI_QUEUE_DB_PATH=/var/lib/noti/queue.db
export NOTI_WORKER_COUNT=8
```

Ensure the database directory exists and is writable by the noti-server process.

### Docker Configuration

```yaml
services:
  noti-server:
    image: ghcr.io/loonghao/noti-server:latest
    environment:
      NOTI_QUEUE_BACKEND: sqlite
      NOTI_QUEUE_DB_PATH: /data/noti-queue.db
      NOTI_WORKER_COUNT: "8"
    volumes:
      - noti-data:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  noti-data:
```

### Monitoring Queue Health

Poll the stats endpoint to monitor queue depth:

```bash
# Alert if queued tasks exceed threshold
QUEUED=$(curl -s http://localhost:3000/api/v1/queue/stats | jq '.queued')
if [ "$QUEUED" -gt 1000 ]; then
  echo "WARNING: Queue depth is $QUEUED"
fi
```

See [Metrics & Monitoring](/guide/metrics-monitoring) for comprehensive monitoring setup.

### Periodic Purge

Terminal tasks accumulate over time. Schedule periodic purges:

```bash
# Cron job: purge terminal tasks every hour
0 * * * * curl -s -X POST http://localhost:3000/api/v1/queue/purge
```

## AI Agent Tips

- **Always use `--json` or `NOTI_OUTPUT=json`** for structured output
- **Check queue stats before sending** to avoid overwhelming a full queue
- **Use `callback_url`** instead of polling for task completion — it's more efficient
- **Use `metadata`** to store correlation IDs for tracing across systems
- **Use `priority: "urgent"`** only for truly critical notifications to avoid priority inversion
- **Prefer batch endpoint** for multiple notifications to reduce HTTP round-trips
- **Monitor `failed` count** in stats — a rising count indicates provider issues
