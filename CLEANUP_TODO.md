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

- [ ] `config.rs`: `from_str_lossy` and `TryFrom<&str>` for `QueueBackendType` have asymmetric match branches — `from_str_lossy` accepts any unknown as Memory, `TryFrom` additionally recognizes `"memory"/"mem"/"in-memory"`; consider aligning or documenting the difference
- [x] ~~`e2e_test.rs`: 10 `spawn_server*` variants share ~10 lines of boilerplate (registry+state+listener+spawn) — extract a core `start_server(Router) -> String` helper~~ — extracted to `tests/common/mod.rs` with `bind_and_serve()` core helper (e9846ee)
- [ ] `e2e_test.rs`: `reqwest::Client::new()` repeated 169 times (was 167 before 57d99f3) — low-impact boilerplate; each test independently creates a client
- [x] ~~`e2e_test.rs`: spawn helpers scattered across file (lines 22-134, 906, 1021, 1160-1211, 1659-1707) — consolidate all spawn helpers at file top~~ — all spawn helpers extracted to `tests/common/mod.rs` (e9846ee)
- [x] ~~`e2e_test.rs`: `use` statements split between file top (lines 7-19) and mid-file (lines 1578-1582) — move all imports to file top~~ — all `use` statements now at file top (lines 10-24); no mid-file imports (e9846ee)

## Tests — E2E Test Quality (noti-server)

- [x] ~~`e2e_priority_ordering_urgent_before_low` — name claims to verify ordering but only asserts all tasks completed~~ — iteration agent rewrote to verify all tasks completed; `e2e_priority_ordering_verified_by_completion_order` now verifies callback arrival order (e9846ee)
- [ ] `e2e_priority_high_tasks_processed_before_normal` (line 2064) — name claims ordering verification but only checks `stats.completed >= 4`; functionally identical to `e2e_worker_multiple_tasks_processed`
- [ ] `e2e_retry_zero_retries_fails_immediately` (line 2029) — near-duplicate of `e2e_worker_handles_failed_task` (line 1409); only unique assertion is `attempts == 1`, which should be added to the existing test instead
- [ ] **13 tests use inline server setup** (~15-21 lines each) instead of common helpers — up from 10 in prior round. The inline pattern (registry+AppState+build_router+TcpListener+bind+axum::serve+tokio::spawn) appears at lines: 1755, 1827, 2593, 2867, 2982, 3100, 3181, 3272, 3355, 3431, 4550, 4689, 4805. Should extract `spawn_server_without_workers() -> (String, AppState)` and `spawn_server_sqlite_without_workers() -> (String, AppState)` to `common/mod.rs`
- [ ] `e2e_batch_async_mixed_priorities_processed_in_order` (line 2855) ≈ `e2e_sqlite_batch_async_mixed_priorities_processed_in_order` (line 2970) — ~95% identical, only queue backend type and diagnostic string prefixes differ; consider a parameterized helper or macro
- [ ] `e2e_graceful_shutdown_waits_for_inflight_task` (line 3089) ≈ `e2e_sqlite_graceful_shutdown_waits_for_inflight_task` (line 3340) — ~95% identical, same pattern
- [ ] `e2e_batch_async_mixed_providers_and_priorities` (line 4541) ≈ `e2e_sqlite_batch_async_mixed_providers_and_priorities` (line 4678) — ~95% identical, same InMemory vs SQLite pattern
- [ ] `e2e_batch_async_mock_fail_provider_with_priorities` (line 4865) ≈ `e2e_sqlite_batch_async_mock_fail_provider_with_priorities` (line 4983) — ~95% identical, same pattern
- [ ] `e2e_batch_async_flaky_with_retry_succeeds` (line 5450) ≈ `e2e_sqlite_batch_async_flaky_with_retry_succeeds` (line 5916) — ~95% identical, same InMemory vs SQLite pattern (added 157cf8d–64d421c)
- [ ] `e2e_batch_async_flaky_retry_exhausted_fails` (line 5522) ≈ `e2e_sqlite_batch_async_flaky_retry_exhausted_fails` (line 5988) — ~95% identical, same pattern (added 157cf8d–64d421c)
- [ ] `e2e_batch_async_mixed_retry_policies` (line 5599) ≈ `e2e_sqlite_batch_async_mixed_retry_policies` (line 6064) — ~95% identical, same pattern (added 157cf8d–64d421c)
- [ ] `e2e_backoff_delay_timing_flaky_task` (line 6376) ≈ `e2e_sqlite_backoff_delay_timing_flaky_task` (line 6507) — ~95% identical, same InMemory vs SQLite pattern (added 251aca6)
- [ ] `e2e_backoff_delay_timing_exhausted_retries` (line 6425) ≈ `e2e_sqlite_backoff_delay_timing_exhausted_retries` (line 6553) — ~95% identical, same pattern (added 251aca6)
- [ ] `e2e_exponential_backoff_api_flaky_task` (line 6592) ≈ `e2e_sqlite_exponential_backoff_api_flaky_task` (line 6742) — ~95% identical, same pattern (added 481e7cf)
- [ ] `e2e_scheduled_send_delay_seconds_holds_task` (line 7217) ≈ `e2e_sqlite_scheduled_send_delay_seconds_holds_task` (line 7472) — ~95% identical, same InMemory vs SQLite pattern (added 9bdd527)
- [ ] `e2e_scheduled_send_delay_zero_is_immediate` (line 7271) ≈ `e2e_sqlite_scheduled_send_delay_zero_is_immediate` (line 7526) — ~95% identical, same pattern (added 9bdd527)
- [ ] `e2e_scheduled_send_rfc3339_timestamp` (line 7312) ≈ `e2e_sqlite_scheduled_send_rfc3339_timestamp` (line 7567) — ~95% identical, same pattern (added 9bdd527)
- [ ] `e2e_scheduled_send_mutually_exclusive_error` (line 7351) ≈ `e2e_sqlite_scheduled_send_mutually_exclusive_error` (line 7606) — ~95% identical, same pattern (added 9bdd527)
- [ ] `e2e_scheduled_send_invalid_timestamp_format` (line 7378) ≈ `e2e_sqlite_scheduled_send_invalid_timestamp_format` (line 7633) — ~95% identical, same pattern (added 9bdd527)
- [ ] `e2e_scheduled_send_no_scheduled_at_for_immediate` (line 7404) ≈ `e2e_sqlite_scheduled_send_no_scheduled_at_for_immediate` (line 7659) — ~95% identical, same pattern (added 9bdd527)
- [x] ~~`spawn_server_with_workers_serial` — near-duplicate of `spawn_server_with_workers`~~ — both extracted to `common/mod.rs` with distinct parameters: `spawn_server_with_workers()` (concurrency=2) and `spawn_server_with_workers_serial(extra_providers)` (concurrency=1) (e9846ee)

## Tests — Cross-Module Deduplication (noti-queue)

- [ ] `make_task()` helper defined identically in both `sqlite.rs:506` and `memory.rs:330` test modules — consider extracting to a shared `#[cfg(test)]` test_utils module
