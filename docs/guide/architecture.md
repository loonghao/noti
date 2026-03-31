# Architecture

noti is organized as a Rust workspace with five crates, each with a focused responsibility.

## Project Structure

```
noti/
├── crates/
│   ├── noti-cli/        # CLI binary
│   ├── noti-core/       # Core abstractions
│   ├── noti-providers/  # 125 provider implementations
│   ├── noti-queue/      # Async message queue for background processing
│   └── noti-server/     # REST API server
├── docs/                # VitePress documentation (this site)
├── scripts/             # Install scripts & utilities
├── skills/              # OpenClaw skill definitions
└── justfile             # Task runner recipes (via vx)
```

## Crates

### `noti-cli`

The CLI binary crate. Handles:

- Argument parsing with [clap](https://docs.rs/clap)
- Output formatting (plain text and JSON)
- Subcommand routing (`send`, `config`, `providers`)

### `noti-core`

Core abstractions shared across the workspace:

- **`NotifyProvider` trait** — async interface all providers implement
- **`ProviderRegistry`** — provider discovery and instantiation
- **`URL parsing`** — universal `provider://credentials` scheme parser
- **`Config`** — profile management and TOML persistence
- **`Message templates`** — variable substitution with `{{placeholder}}` syntax
- **`Retry policies`** — configurable retry with fixed/exponential backoff
- **`Batch & failover sending`** — multi-target parallel or sequential delivery
- **`Delivery status tracking`** — per-notification delivery state machine
- **`Priority system`** — low, normal, high, urgent message priorities
- **`Error types`** — structured error handling

### `noti-providers`

All 125 provider implementations, one file per provider. Each provider:

1. Implements the `NotifyProvider` trait
2. Registers itself with the `ProviderRegistry`
3. Parses its URL scheme format
4. Sends the notification via the provider's API

### `noti-queue`

Async message queue for background notification processing:

- **Priority-based queue** — tasks ordered by priority level
- **In-memory backend** — `InMemoryQueue` with configurable capacity
- **SQLite backend** — `SqliteQueue` for persistent task storage across restarts
- **Stale task recovery** — automatically recovers tasks left in `Processing` state after an unclean shutdown via `QueueBackend::recover_stale_tasks()`
- **Worker pool** — concurrent workers dequeue and deliver notifications
- **Webhook callbacks** — HTTP POST on task completion or failure
- **Task lifecycle** — queued → processing → completed/failed/cancelled

### `noti-server`

REST API server for the noti notification service:

- **Axum-based HTTP server** — sync and async send endpoints
- **Queue management API** — enqueue, list, cancel, purge tasks
- **Configurable queue backend** — `NOTI_QUEUE_BACKEND=memory|sqlite`
- **Template CRUD** — create, read, update, delete message templates
- **Rate limiting** — token-bucket per-IP or global rate limiting
- **API key authentication** — Bearer token and X-API-Key header support
- **Request ID middleware** — UUID tracking for every request
- **Structured logging** — text or JSON log format with automatic request ID correlation
- **Metrics endpoint** — operational metrics for monitoring
- **Health check** — dependency-aware `/health` endpoint (queue + providers status, uptime)
- **Configurable CORS** — `NOTI_CORS_ALLOWED_ORIGINS` for cross-origin control
- **OpenAPI / Swagger UI** — auto-generated API docs at `/swagger-ui`

## Technology Stack

| Component | Technology |
|:----------|:-----------|
| Language | Rust 2024 edition (MSRV 1.85) |
| CLI framework | clap 4.5 |
| HTTP server | axum 0.8 |
| HTTP client | reqwest 0.12 |
| Async runtime | tokio 1.44 |
| Email | lettre 0.11 |
| API docs | utoipa + utoipa-swagger-ui |
| Observability | tracing + tracing-subscriber |
| Serialization | serde / serde_json |
| Persistent storage | rusqlite 0.34 (bundled SQLite) |
| HTTP middleware | tower-http 0.6 (CORS, tracing) |
| Validation | validator 0.19 |
| Testing | rstest, assert_cmd, wiremock |
| Task runner | just (via vx) |
