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
