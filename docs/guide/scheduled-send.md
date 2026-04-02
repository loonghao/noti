# Scheduled & Delayed Send

noti supports scheduling notifications for future delivery. You can delay a notification by a relative number of seconds or specify an absolute timestamp.

## Overview

| Parameter | Type | Description |
|:----------|:-----|:------------|
| `delay_seconds` | `integer` | Relative delay from now (in seconds) |
| `scheduled_at` | `string` | Absolute timestamp in RFC 3339 / ISO 8601 format |

::: warning
`delay_seconds` and `scheduled_at` are **mutually exclusive**. Providing both in the same request returns a `400 Bad Request` error.
:::

## How It Works

1. The client submits a notification with either `delay_seconds` or `scheduled_at`.
2. The queue stores the task with an `available_at` timestamp.
3. Workers skip tasks whose `available_at` is in the future.
4. Once the time is reached, the task is dequeued and delivered normally.

```
Client ──▶ POST /api/v1/send/async ──▶ Queue (available_at = now + delay)
                                           │
                                           ▼
                                   Worker polls queue
                                   ├── available_at > now → skip
                                   └── available_at ≤ now → deliver
```

## API Usage

### Delay by Seconds

Send a notification that will be delivered after a relative delay:

```bash
curl -X POST http://localhost:3000/api/v1/send/async \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "slack",
    "text": "Reminder: standup in 5 minutes!",
    "config": {"webhook_url": "https://hooks.slack.com/services/..."},
    "delay_seconds": 300
  }'
```

Response:

```json
{
  "task_id": "a1b2c3d4-...",
  "status": "queued",
  "message": "Notification scheduled for delayed processing"
}
```

::: tip
Setting `delay_seconds` to `0` is treated as **immediate** delivery — no delay is applied and the response message says "enqueued" instead of "scheduled".
:::

### Schedule at a Specific Time

Send a notification at an exact future time using an RFC 3339 timestamp:

```bash
curl -X POST http://localhost:3000/api/v1/send/async \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "email",
    "text": "Weekly report is ready",
    "title": "Weekly Report",
    "config": {"smtp_url": "smtp://..."},
    "scheduled_at": "2025-08-15T09:00:00Z"
  }'
```

### Batch Scheduled Send

Each item in a batch request can have its own schedule independently:

```bash
curl -X POST http://localhost:3000/api/v1/send/async/batch \
  -H "Content-Type: application/json" \
  -d '{
    "items": [
      {
        "provider": "slack",
        "text": "Deploy starts in 10 minutes",
        "config": {"webhook_url": "https://hooks.slack.com/..."},
        "delay_seconds": 600
      },
      {
        "provider": "slack",
        "text": "Deploy window closed",
        "config": {"webhook_url": "https://hooks.slack.com/..."},
        "scheduled_at": "2025-08-15T18:00:00Z"
      }
    ]
  }'
```

## Checking Scheduled Tasks

When you query a scheduled task, the response includes a `scheduled_at` field showing the RFC 3339 timestamp of when the task becomes eligible for delivery:

```bash
curl http://localhost:3000/api/v1/queue/tasks/a1b2c3d4-...
```

```json
{
  "id": "a1b2c3d4-...",
  "provider": "slack",
  "status": "queued",
  "attempts": 0,
  "priority": "Normal",
  "scheduled_at": "2025-08-15T09:00:00.000000000Z"
}
```

::: info
The `scheduled_at` field is **only present** for delayed/scheduled tasks. Immediate tasks omit this field entirely.
:::

## Error Handling

### Mutually Exclusive Parameters

```bash
# ❌ Both delay_seconds and scheduled_at — returns 400
curl -X POST http://localhost:3000/api/v1/send/async \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "slack",
    "text": "test",
    "config": {"webhook_url": "..."},
    "delay_seconds": 60,
    "scheduled_at": "2025-08-15T10:30:00Z"
  }'
```

```json
{
  "error": "bad_request",
  "message": "delay_seconds and scheduled_at are mutually exclusive; provide only one"
}
```

### Invalid Timestamp Format

```json
{
  "error": "bad_request",
  "message": "invalid scheduled_at timestamp (expected RFC 3339 / ISO 8601): ..."
}
```

## Rust SDK

If you're using the noti crates directly:

```rust
use std::time::{Duration, SystemTime};
use noti_core::{Message, ProviderConfig};
use noti_queue::NotificationTask;

// Delay by 5 minutes
let task = NotificationTask::new("slack", ProviderConfig::new(), Message::text("hello"))
    .with_available_at(SystemTime::now() + Duration::from_secs(300));

// Schedule at a specific time
let scheduled_time = humantime::parse_rfc3339("2025-08-15T09:00:00Z").unwrap();
let task = NotificationTask::new("email", ProviderConfig::new(), Message::text("report"))
    .with_available_at(scheduled_time);
```

## Queue Backend Support

Both queue backends fully support scheduled/delayed delivery:

| Backend | Deferred Delivery | Precision |
|:--------|:-----------------|:----------|
| **InMemory** | ✅ `available_at` field checked on dequeue | ~poll interval |
| **SQLite** | ✅ `available_at` column with SQL filter | ~poll interval |

The actual delivery precision depends on the worker poll interval (default: 100ms).
