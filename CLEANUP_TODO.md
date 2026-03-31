# Cleanup TODO

Tracked issues found during cleanup rounds. Items here are deferred for future
rounds or require coordination with the iteration agent.

## Documentation — Out of Date

- [x] ~~README.md and README_zh.md architecture section says "3 crates"~~ — updated to 5 crates with noti-queue and noti-server (dcef4e9)
- [x] ~~Badge in both READMEs: `3_workspace_crates`~~ — updated to `5_workspace` (dcef4e9)
- [x] ~~`docs/guide/architecture.md` — same "three crates" issue; project tree incomplete~~ — rewritten with all 5 crates and full descriptions (dcef4e9)
- [x] ~~`docs/guide/what-is-noti.md` — same "three crates" issue~~ — updated to 5 crates (dcef4e9)
- [x] ~~`docs/guide/contributing.md` — test commands only list 3 crates~~ — added noti-queue and noti-server (dcef4e9)
- [x] ~~`docs/reference/cli.md` — `send` command table missing `--priority` parameter~~ — added (dcef4e9)
- [ ] Core features not documented anywhere: message templates, retry policies, batch/failover sending, delivery status tracking, priority system

## Code — Structural Refactoring (noti-server)

- [x] ~~`handlers/send.rs` and `handlers/queue.rs` share identical `build_message()` function~~ — extracted to `handlers/common.rs` (293b523)
- [x] ~~`handlers/send.rs` and `handlers/queue.rs` share identical `RetryConfig` struct~~ — extracted to `handlers/common.rs` (293b523)
- [x] ~~`build_retry_policy()` default behavior difference~~ — resolved: `common::build_retry_policy` now takes explicit `default_policy` param; `send.rs` passes `RetryPolicy::none()`, `queue.rs` passes `RetryPolicy::default()` (293b523)
- [x] ~~"provider not found" error pattern repeated 4× across handlers~~ — extracted `require_provider` helper to `common.rs` (e1c9e90)
- [x] ~~Consider defining a unified `ApiError` type implementing `IntoResponse`~~ — implemented in `handlers/error.rs`, all handlers migrated (fd630be)
- [x] ~~`health.rs` response type lacks `Debug` derive~~ — added (4efb813)
- [ ] `BatchAsyncItem` and `AsyncSendRequest` share near-identical fields — consider shared base type
- [ ] `StatsResponse` (queue.rs) and `QueueMetrics` (metrics.rs) have identical fields — unify
- [ ] `send_batch` function exceeds 100 lines — extract result-mapping helper
- [x] ~~`main.rs` uses `.unwrap()` for TCP bind and serve~~ — replaced with `.expect()` (4efb813)
- [x] ~~`main.rs` graceful shutdown (`worker_handle.shutdown_and_join()`) is unreachable~~ — implemented signal handling with `axum::serve().with_graceful_shutdown()`, supports Ctrl+C and SIGTERM (4efb813)
- [x] ~~`fire_callback` creates a new `reqwest::Client` per call~~ — replaced with `LazyLock` shared static client (9d9a165)

## Code — Silent Error Discards (noti-queue)

- [x] ~~`worker.rs` `let _ = queue.ack/nack(...)` silently discards errors~~ — replaced with `if let Err(e) = ... { tracing::error!(...) }` (1de1311)

## Build

- [x] ~~`justfile` `build-release` only builds `noti-cli` — no recipe for building `noti-server`~~ — added `build-server` recipe (9c73d13)
