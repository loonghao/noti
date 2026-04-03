# Webhook Callbacks

Webhook callbacks provide real-time notifications when async tasks reach a terminal state. Instead of polling the `/api/v1/queue/tasks/:task_id` endpoint, your system receives an HTTP POST at the moment a task completes, fails, or is cancelled.

## How It Works

```
Client                  noti-server                  Your Server
  │                         │                             │
  │── POST /send/async ────►│                             │
  │   { callback_url: ... } │                             │
  │◄── 202 { task_id } ─────│                             │
  │                         │── process notification ──►  │
  │                         │                             │
  │                         │── POST callback_url ───────►│
  │                         │   { task_id, status, ... }  │
  │                         │◄── 200 OK ──────────────────│
```

1. Submit an async notification with a `callback_url` field.
2. The server enqueues the task and returns `202 Accepted` with the `task_id`.
3. A background worker picks up and processes the task.
4. When the task reaches a **terminal state** (`completed`, `failed`, or `cancelled`), the server sends an HTTP POST to your `callback_url`.

## Request Format

### Sending with a Callback URL

Include `callback_url` in your async send request:

```bash
curl -X POST http://localhost:3000/api/v1/send/async \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "slack",
    "config": {"webhook_url": "https://hooks.slack.com/services/..."},
    "text": "Deployment complete",
    "title": "Deploy Alert",
    "priority": "high",
    "metadata": {"deploy_id": "d-123", "env": "production"},
    "callback_url": "https://your-server.com/webhook/noti-callback"
  }'

```

The same field works for batch async requests:

```bash
curl -X POST http://localhost:3000/api/v1/send/async/batch \
  -H "Content-Type: application/json" \
  -d '{
    "items": [
      {
        "provider": "slack",
        "config": {"webhook_url": "https://hooks.slack.com/..."},
        "text": "Alert: disk full",
        "callback_url": "https://your-server.com/webhook/noti-callback"
      },

      {
        "provider": "email",
        "config": {"smtp_host": "smtp.example.com", "to": "ops@example.com"},
        "text": "Disk usage critical",
        "callback_url": "https://your-server.com/webhook/noti-callback"
      }
    ]
  }'
```

## Callback Payload

When a task reaches a terminal state, the server sends a POST request with a JSON body:

```json
{
  "task_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "provider": "slack",
  "status": "completed",
  "attempts": 1,
  "metadata": {
    "deploy_id": "d-123",
    "env": "production"
  }
}
```

### Payload Fields

| Field | Type | Description |
|:------|:-----|:------------|
| `task_id` | `string` | Unique identifier of the notification task |
| `provider` | `string` | The provider used for sending (e.g. `"slack"`, `"email"`) |
| `status` | `string` | Terminal status: `"completed"`, `"failed"`, or `"cancelled"` |
| `attempts` | `number` | Total number of delivery attempts made |
| `last_error` | `string?` | Error message from the last failed attempt (only present on failure) |
| `metadata` | `object?` | The same metadata key-value pairs you supplied in the original request |

### Status Values

| Status | Meaning |
|:-------|:--------|
| `completed` | Notification was delivered successfully |
| `failed` | All retry attempts exhausted; delivery failed |
| `cancelled` | Task was cancelled via the cancel endpoint before delivery |

### Example: Failed Delivery

```json
{
  "task_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "provider": "email",
  "status": "failed",
  "attempts": 4,
  "last_error": "SMTP connection timeout after 10s",
  "metadata": {
    "deploy_id": "d-123"
  }
}
```

## Delivery Semantics

### Best-Effort Delivery

Webhook callbacks use **best-effort delivery**:

- The callback is fired **once** when the task reaches a terminal state.
- If the callback fails (network error, timeout, non-2xx response), it is **not retried**.
- Callback failures are logged at `WARN` level but do **not** affect the task's final status.

This means your callback endpoint should be:
- **Available** — minimize downtime on the receiving end.
- **Idempotent** — in rare edge cases, you might receive duplicate callbacks.
- **Fast** — respond within 10 seconds (the callback HTTP client timeout).

### Timeout

The callback HTTP client has a **10-second timeout**. If your server does not respond within this window, the callback is considered failed.

### Logging

All callback events are logged with structured fields:

```json
{"level": "INFO", "task_id": "abc-123", "callback_url": "https://...", "http_status": "200", "message": "webhook callback delivered"}
{"level": "WARN", "task_id": "abc-123", "callback_url": "https://...", "error": "connection refused", "message": "webhook callback failed"}
```

## Receiving Callbacks

### Example: Express.js Handler

```javascript
app.post('/webhook/noti-callback', (req, res) => {
  const { task_id, provider, status, attempts, last_error, metadata } = req.body;

  console.log(`Task ${task_id} via ${provider}: ${status} (${attempts} attempts)`);

  if (status === 'failed') {
    // Trigger alerting or manual review
    alertOps({ task_id, provider, error: last_error });
  }

  // Acknowledge receipt
  res.status(200).json({ received: true });
});
```

### Example: Python Flask Handler

```python
from flask import Flask, request, jsonify

app = Flask(__name__)

@app.post("/webhook/noti-callback")
def noti_callback():
    data = request.get_json()
    task_id = data["task_id"]
    status = data["status"]

    if status == "completed":
        mark_notification_delivered(task_id)
    elif status == "failed":
        trigger_alert(task_id, data.get("last_error"))

    return jsonify(received=True), 200
```

## Security Considerations

### Validate the Source

The callback does not include a signature or authentication token by default. To secure your callback endpoint:

1. **Use HTTPS** — always serve your callback URL over TLS.
2. **Include a secret in the URL** — use a hard-to-guess path segment:
   ```
   https://your-server.com/webhook/noti-callback/s3cr3t-t0k3n
   ```
3. **Restrict by IP** — if your noti-server has a known IP, restrict incoming requests at the firewall or reverse proxy level.
4. **Verify metadata** — check that the `task_id` and `metadata` match a task you actually submitted.

### Protect Against Replay Attacks

Since callbacks are best-effort and not signed, treat them as hints rather than authoritative state changes. For critical workflows:

1. Receive the callback.
2. Verify the task status by calling `GET /api/v1/queue/tasks/:task_id`.
3. Only then update your internal state.

## Integration Patterns

### CI/CD Pipeline Notifications

```bash
# Deploy step sends notification with callback
TASK_ID=$(curl -s -X POST http://noti:3000/api/v1/send/async \
  -H "Content-Type: application/json" \
  -d '{
    "provider": "slack",
    "config": {"webhook_url": "'$SLACK_WEBHOOK'"},
    "text": "Deploying '"$SERVICE"' to '"$ENV"'",

    "callback_url": "'$CI_CALLBACK_URL'",
    "metadata": {"pipeline_id": "'$CI_PIPELINE_ID'", "service": "'$SERVICE'"}
  }' | jq -r '.task_id')

echo "Notification task: $TASK_ID"
```

### Event-Driven Architecture

Use callbacks to bridge noti into an event-driven system:

```
noti callback → API Gateway → Event Bus → Multiple Consumers
```

Your callback handler can publish the event to a message queue (RabbitMQ, Kafka, SQS) for further processing by downstream services.

## Combining with Other Features

### Callbacks + Scheduled Send

Callbacks work with scheduled/delayed sends. The callback fires when the notification is actually processed, not when it's enqueued:

```json
{
  "provider": "slack",
  "text": "Reminder: standup in 5 minutes",
  "delay_seconds": 300,
  "callback_url": "https://your-server.com/webhook/noti-callback"
}
```

### Callbacks + Retry Policies

When a task has retries configured, the callback fires only after the final outcome — either successful delivery or exhaustion of all retry attempts:

```json
{
  "provider": "email",
  "text": "Important update",
  "retry": {"max_retries": 5, "delay_ms": 2000, "backoff_multiplier": 2.0},
  "callback_url": "https://your-server.com/webhook/noti-callback"
}
```

The `attempts` field in the callback payload tells you how many tries were needed.

### Callbacks + Batch Send

Each item in a batch can have its own `callback_url`, or they can all share the same one. Each task triggers its own independent callback.
