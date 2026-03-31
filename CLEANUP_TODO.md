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
- [x] ~~Core features not documented anywhere: message templates, retry policies, batch/failover sending, delivery status tracking, priority system~~ — created `docs/guide/core-features.md` with full documentation

## Documentation — Fixed This Round

- [x] ~~`docs/guide/contributing.md` — Provider trait signature completely wrong (trait name, method names, params, return types)~~ — updated to match `NotifyProvider` (715bd8d)
- [x] ~~`docs/reference/environment-variables.md` — missing all 14 noti-server env vars~~ — added full server env var table (715bd8d)
- [x] ~~`docs/guide/architecture.md` — missing health check, CORS, rusqlite, tower-http, validator in tech stack~~ — added (715bd8d)
- [x] ~~`docs/guide/agent-integration.md` — references non-existent `skills/noti-cli/` path~~ — fixed to `skills/noti-openclaw/` (715bd8d)

## Code — Structural Refactoring (noti-server)

- [x] ~~`handlers/send.rs` and `handlers/queue.rs` share identical `build_message()` function~~ — extracted to `handlers/common.rs` (293b523)
- [x] ~~`handlers/send.rs` and `handlers/queue.rs` share identical `RetryConfig` struct~~ — extracted to `handlers/common.rs` (293b523)
- [x] ~~`build_retry_policy()` default behavior difference~~ — resolved: `common::build_retry_policy` now takes explicit `default_policy` param; `send.rs` passes `RetryPolicy::none()`, `queue.rs` passes `RetryPolicy::default()` (293b523)
- [x] ~~"provider not found" error pattern repeated 4× across handlers~~ — extracted `require_provider` helper to `common.rs` (e1c9e90)
- [x] ~~Consider defining a unified `ApiError` type implementing `IntoResponse`~~ — implemented in `handlers/error.rs`, all handlers migrated (fd630be)
- [x] ~~`health.rs` response type lacks `Debug` derive~~ — added (4efb813)
- [x] ~~`BatchAsyncItem` and `AsyncSendRequest` share near-identical fields — consider shared base type~~ — removed `BatchAsyncItem`, `BatchAsyncRequest.items` now uses `AsyncSendRequest` directly
- [x] ~~`StatsResponse` (queue.rs) and `QueueMetrics` (metrics.rs) have identical fields — unify~~ — unified in previous iteration (250dd0d)
- [x] ~~`send_batch` function exceeds 100 lines — extract result-mapping helper~~ — extracted `map_target_result()` helper function
- [x] ~~`main.rs` uses `.unwrap()` for TCP bind and serve~~ — replaced with `.expect()` (4efb813)
- [x] ~~`main.rs` graceful shutdown (`worker_handle.shutdown_and_join()`) is unreachable~~ — implemented signal handling with `axum::serve().with_graceful_shutdown()`, supports Ctrl+C and SIGTERM (4efb813)
- [x] ~~`fire_callback` creates a new `reqwest::Client` per call~~ — replaced with `LazyLock` shared static client (9d9a165)

## Code — Silent Error Discards (noti-queue)

- [x] ~~`worker.rs` `let _ = queue.ack/nack(...)` silently discards errors~~ — replaced with `if let Err(e) = ... { tracing::error!(...) }` (1de1311)

## Code — SQLite Backend Quality (noti-queue)

- [x] ~~`sqlite.rs` `epoch_ms_to_system_time` — unsafe `ms as u64` (negative i64 overflows)~~ — fixed with `ms.max(0) as u64` (112ce6f)
- [x] ~~`sqlite.rs` `list_tasks` — duplicated iteration logic across if/else branches~~ — simplified with collect (112ce6f)
- [x] ~~`sqlite.rs` — 14× repeated `.map_err(|e| QueueError::Backend(e.to_string()))` pattern — consider helper trait/function~~ — introduced `SqliteResultExt` and `SerdeResultExt` traits with `.backend_err()` / `.serde_err()` methods
- [x] ~~`sqlite.rs` `str_to_status` — silently falls back to `Queued` for unknown status values, should log warning~~ — added explicit `"queued"` match arm and `tracing::warn!` on unknown values
- [x] ~~`state.rs` `new()` vs `with_queue_backend()` — API inconsistency (one always succeeds, other can panic)~~ — `with_queue_backend()` now returns `Result<Self, QueueError>` (f6d21dd)

## Code — QueueStats → StatsResponse Duplication (noti-server)

- [x] ~~`metrics.rs:57-64` and `queue.rs:435-442` — identical `QueueStats` → `StatsResponse` field-by-field conversion; implement `From<QueueStats> for StatsResponse`~~ — implemented `From<QueueStats> for StatsResponse`, used in both `get_stats()` and `get_metrics()`
- [x] ~~`metrics.rs:43` — `unwrap_or_default()` silently swallows queue stats error; should at least `tracing::warn` on failure~~ — replaced with explicit match + `tracing::warn!`
- [x] ~~`queue.rs:404-407` — invalid `?status=` query param silently ignored (returns all tasks); should return 400 for unrecognized values~~ — now returns 400 Bad Request with descriptive message

## Documentation — Missing Features

- [x] ~~`docs/guide/architecture.md` — noti-queue section missing "stale task recovery" feature~~ — added (f6d21dd)
- [x] ~~`docs/guide/contributing.md` — missing `vx just build-server`, `vx just docs-dev`, `vx just docs-build`, `vx just docs-preview` commands~~ — added (f6d21dd)

## Documentation — Fixed This Cleanup Round

- [x] ~~`docs/guide/architecture.md` — `Provider` trait / `Registry` should be `NotifyProvider` / `ProviderRegistry`~~ — fixed (a2d4b08)
- [x] ~~`docs/guide/contributing.md` — CI command comment says "fmt" but actual recipe uses "fmt-check"~~ — fixed (a2d4b08)
- [x] ~~`docs/.vitepress/config.mts` — nav version shows v0.1.2 but Cargo.toml is v0.1.3~~ — fixed (a2d4b08)

## Documentation — Remaining

- [x] ~~`docs/guide/what-is-noti.md:38` — same `Provider trait, Registry` naming issue~~ — fixed to `NotifyProvider`, `ProviderRegistry` (f53599a)

## Tests — Deferred Deduplication

- [x] ~~`url_parse_test.rs` is a strict subset of `url_parse_comprehensive_test.rs`~~ — removed (0730ac3)
- [x] ~~`provider_test.rs:test_message_builder` duplicates `core_types_test.rs:test_message_builder_chain`~~ — N/A: `provider_test.rs` does not exist (likely already merged)
- [x] ~~`provider_test.rs:test_provider_config_builder` duplicates `core_types_test.rs:test_provider_config_set_and_get`~~ — N/A: `provider_test.rs` does not exist

## Build

- [x] ~~`justfile` `build-release` only builds `noti-cli` — no recipe for building `noti-server`~~ — added `build-server` recipe (9c73d13)

## Code — Behavioral Issues (noti-queue)

- [ ] `InMemoryQueue::dequeue()` does not skip cancelled tasks in heap — a task that is cancelled after enqueue but before dequeue will be dequeued and marked as Processing; suggest adding a skip-cancelled loop in `dequeue()`
- [ ] `InMemoryQueue::stats()` vs `SqliteQueue::stats()` semantic mismatch after `purge_completed()` — InMemoryQueue returns cumulative counters (purge doesn't decrement completed/failed/cancelled), SQLiteQueue returns actual row counts (purge removes rows so counts drop)

## Code — Minor (noti-server)

- [ ] `config.rs`: `from_str_lossy` and `TryFrom<&str>` for `QueueBackendType` have asymmetric match branches — `from_str_lossy` accepts any unknown as Memory, `TryFrom` additionally recognizes `"memory"/"mem"/"in-memory"`; consider aligning or documenting the difference
- [ ] `e2e_test.rs`: `spawn_server()` + `reqwest::Client::new()` boilerplate repeated 16 times — consider extracting helper returning `(String, Client)` tuple

## Tests — Cross-Module Deduplication (noti-queue)

- [ ] `make_task()` helper defined identically in both `sqlite.rs:489` and `memory.rs:268` test modules — consider extracting to a shared `#[cfg(test)]` test_utils module
