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

- [x] ~~`InMemoryQueue::dequeue()` does not skip cancelled tasks in heap — a task that is cancelled after enqueue but before dequeue will be dequeued and marked as Processing~~ — fixed: `dequeue()` now loops and checks current task status from the HashMap, skipping non-Queued entries; 3 new tests added
- [x] ~~`InMemoryQueue::stats()` vs `SqliteQueue::stats()` semantic mismatch after `purge_completed()` — InMemoryQueue returns cumulative counters (purge doesn't decrement completed/failed/cancelled), SQLiteQueue returns actual row counts (purge removes rows so counts drop)~~ — fixed: `purge_completed()` now resets terminal counters to 0; QueueStats doc comments updated

## Code — Minor (noti-server)

- [x] ~~`config.rs`: `from_str_lossy` and `TryFrom<&str>` for `QueueBackendType` have asymmetric match branches~~ — aligned: `from_str_lossy` now explicitly matches `"memory"/"mem"/"in-memory"` and logs `tracing::warn!` for unknown values; added `test_queue_backend_type_from_str_lossy` unit test
- [x] ~~`e2e_test.rs`: 10 `spawn_server*` variants share ~10 lines of boilerplate (registry+state+listener+spawn) — extract a core `start_server(Router) -> String` helper~~ — extracted to `tests/common/mod.rs` with `bind_and_serve()` core helper (e9846ee)
- [x] ~~`e2e_test.rs`: `reqwest::Client::new()` repeated 171 times~~ — extracted `test_client()` helper to `common/mod.rs`, replaced all 171 call sites (cbb3a9b)
- [x] ~~`e2e_test.rs`: spawn helpers scattered across file (lines 22-134, 906, 1021, 1160-1211, 1659-1707) — consolidate all spawn helpers at file top~~ — all spawn helpers extracted to `tests/common/mod.rs` (e9846ee)
- [x] ~~`e2e_test.rs`: `use` statements split between file top (lines 7-19) and mid-file (lines 1578-1582) — move all imports to file top~~ — all `use` statements now at file top (lines 10-24); no mid-file imports (e9846ee)

## Tests — E2E Test Quality (noti-server)

- [x] ~~`e2e_priority_ordering_urgent_before_low` — name claims to verify ordering but only asserts all tasks completed~~ — iteration agent rewrote to verify all tasks completed; `e2e_priority_ordering_verified_by_completion_order` now verifies callback arrival order (e9846ee)
- [x] ~~`e2e_priority_high_tasks_processed_before_normal` (line 2064) — name claims ordering verification but only checks `stats.completed >= 4`~~ — rewritten: enqueues 3 normal + 1 high on server without workers, starts single worker, verifies via callback arrival order that high is processed first
- [x] ~~`e2e_retry_zero_retries_fails_immediately` — near-duplicate of `e2e_worker_handles_failed_task`; only unique assertion is `attempts == 1`, which should be added to the existing test instead~~ — obsolete: duplicate test was removed and `e2e_worker_handles_failed_task` now covers `attempts == 1`
- [x] ~~`e2e_test.rs`: 13 tests use inline server setup (~15-21 lines each) instead of common helpers~~ — extracted `spawn_server_without_workers()` and `spawn_server_sqlite_without_workers()` to `tests/common/mod.rs`; all 14 inline `TcpListener::bind` sites replaced
- [x] ~~`e2e_batch_async_mixed_priorities_processed_in_order` ≈ `e2e_sqlite_batch_async_mixed_priorities_processed_in_order`~~ — deduplicated via `common::dual_backend_test!` macro; backend-specific spawn function + label are now parameterized in one shared test body
- [x] ~~`e2e_graceful_shutdown_waits_for_inflight_task` ≈ `e2e_sqlite_graceful_shutdown_waits_for_inflight_task`~~ — deduplicated via `common::dual_backend_test!` macro; preserves the same assertions for both backends from one shared test body
- [x] ~~`e2e_concurrent_batch_async_with_rate_limit_partial_reject` ≈ `e2e_sqlite_concurrent_batch_async_with_rate_limit_partial_reject`~~ — deduplicated via `common::dual_backend_test!` macro
- [x] ~~`e2e_batch_async_within_rate_limit_succeeds` ≈ `e2e_sqlite_batch_async_within_rate_limit_succeeds`~~ — deduplicated via `common::dual_backend_test!` macro
- [x] ~~`e2e_sequential_batch_async_rate_limit_exhaustion` ≈ `e2e_sqlite_sequential_batch_async_rate_limit_exhaustion`~~ — deduplicated via `common::dual_backend_test!` macro

- [x] ~~`e2e_batch_async_mixed_providers_and_priorities` / `e2e_sqlite_batch_async_mixed_providers_and_priorities`~~ — same dual-backend scenario; still a good `common::dual_backend_test!` candidate, but current inputs and ordering assertions diverge slightly
- [x] ~~`e2e_batch_async_mock_fail_provider_with_priorities` / `e2e_sqlite_batch_async_mock_fail_provider_with_priorities`~~ — migrated via `dual_backend_test!`
- [x] ~~`e2e_batch_async_flaky_with_retry_succeeds` ≈ `e2e_sqlite_batch_async_flaky_with_retry_succeeds`~~ — ~95% identical, same InMemory vs SQLite pattern (migrated 2026-04-04)
- [x] ~~`e2e_batch_async_flaky_retry_exhausted_fails` ≈ `e2e_sqlite_batch_async_flaky_retry_exhausted_fails`~~ — ~95% identical, same pattern (migrated 2026-04-04)
- [x] ~~`e2e_batch_async_mixed_retry_policies` ≈ `e2e_sqlite_batch_async_mixed_retry_policies`~~ — ~95% identical, same pattern (migrated 2026-04-04)
- [x] ~~`e2e_backoff_delay_timing_flaky_task` ≈ `e2e_sqlite_backoff_delay_timing_flaky_task`~~ — ~95% identical, same InMemory vs SQLite pattern (migrated 2026-04-04)
- [x] ~~`e2e_backoff_delay_timing_exhausted_retries` ≈ `e2e_sqlite_backoff_delay_timing_exhausted_retries`~~ — ~95% identical, same pattern (migrated 2026-04-04)
- [x] ~~`e2e_exponential_backoff_api_flaky_task` ≈ `e2e_sqlite_exponential_backoff_api_flaky_task`~~ — ~95% identical, same pattern (migrated 2026-04-04)
- [x] ~~`e2e_concurrent_tasks_all_processed` ≈ `e2e_sqlite_concurrent_tasks_all_processed`~~ — ~95% identical, same pattern (migrated 2026-04-04)
- [x] ~~`e2e_scheduled_send_delay_seconds_holds_task` ≈ `e2e_sqlite_scheduled_send_delay_seconds_holds_task` — ~95% identical, same InMemory vs SQLite pattern (added 9bdd527)~~ — already deduplicated via `dual_backend_test!`
- [x] ~~`e2e_scheduled_send_delay_zero_is_immediate` ≈ `e2e_sqlite_scheduled_send_delay_zero_is_immediate` — ~95% identical, same pattern (added 9bdd527)~~ — already deduplicated via `dual_backend_test!`
- [x] ~~`e2e_scheduled_send_rfc3339_timestamp` ≈ `e2e_sqlite_scheduled_send_rfc3339_timestamp` — ~95% identical, same pattern (added 9bdd527)~~ — already deduplicated via `dual_backend_test!`
- [x] ~~`e2e_scheduled_send_mutually_exclusive_error` ≈ `e2e_sqlite_scheduled_send_mutually_exclusive_error` — ~95% identical, same pattern (added 9bdd527)~~ — already deduplicated via `dual_backend_test!`
- [x] ~~`e2e_scheduled_send_invalid_timestamp_format` ≈ `e2e_sqlite_scheduled_send_invalid_timestamp_format` — ~95% identical, same pattern (added 9bdd527)~~ — already deduplicated via `dual_backend_test!`
- [x] ~~`e2e_scheduled_send_no_scheduled_at_for_immediate` ≈ `e2e_sqlite_scheduled_send_no_scheduled_at_for_immediate` — ~95% identical, same pattern (added 9bdd527)~~ — already deduplicated via `dual_backend_test!`

- [x] ~~`spawn_server_with_workers_serial` — near-duplicate of `spawn_server_with_workers`~~ — both extracted to `common/mod.rs` with distinct parameters: `spawn_server_with_workers()` (concurrency=2) and `spawn_server_with_workers_serial(extra_providers)` (concurrency=1) (e9846ee)

## Structural Assessment — Deferred

- [ ] `crates/noti-core/src/url.rs` — `parse_notification_url()` still spans nearly the entire file and mixes scheme dispatch, validation, alias normalization, and config assembly; split by provider family or dispatch table in a future refactor
- [ ] `crates/noti-server/tests/e2e_test.rs` — the file is still monolithic and mixes health/auth/CORS/body-limit/queue/retry/scheduling scenarios; split by concern while keeping shared helpers in `tests/common/mod.rs`
- [ ] `crates/noti-providers/tests/provider_send_test.rs` + `provider_send_extended_test.rs` — many provider send-path tests repeat the same success/failure/metadata contract shape; consider a shared contract-test macro/DSL and family-based file splits
- [ ] `crates/noti-server/src/handlers/queue.rs` — request/response DTOs, schedule parsing, queue error mapping, HTTP handlers, and unit tests still live in one module; extract `dto`/`service`/`handlers` seams when safe
- [ ] `crates/noti-providers/src/lib.rs` — `register_all_providers()` is a long manual registry list; consider a list-driven or macro-driven registration table to reduce drift risk as providers keep growing

## Tests — Cross-Module Deduplication (noti-queue)

- [x] ~~`make_task()` helper defined identically in both `sqlite.rs:506` and `memory.rs:330` test modules — consider extracting to a shared `#[cfg(test)]` test_utils module~~ — extracted to `crates/noti-queue/src/test_utils.rs` with `#[cfg(test)] pub(crate) mod test_utils` in lib.rs; both `memory.rs` and `sqlite.rs` now import via `crate::test_utils::make_task`
