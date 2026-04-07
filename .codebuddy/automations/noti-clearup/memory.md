## 2026-04-07 20:09 — Cleanup round: reviewed iteration agent commits aa1162a + 53b73a9

### Baseline
- Branch: `auto-improve` (HEAD = 53b73a9 before cleanup)
- Tests: **1250 passed / 0 failed / 2 ignored** (up from 1218 — 32 new DLQ tests)
- Clippy: 0 warnings, 0 errors
- Known: 1 moderate Dependabot vulnerability (unchanged)

### Iteration Agent Activity (since last cleanup round 2d7a36c)
- `aa1162a` — fix(noti-queue): DLQ implementation: `nack()` now calls `mark_failed()` before serializing; `move_to_dlq()` added to both InMemory/SQLite; `get_task()` falls back to DLQ; `QueueBackend` trait extended with `move_to_dlq/list_dlq/dlq_stats/requeue_from_dlq/delete_from_dlq`; `DlqEntry` + `DlqStats` types in `task.rs`/`queue.rs` — **substantial and correct**
- `53b73a9` — fix(queue): `get_task` fallback to DLQ; update 3 e2e tests to reflect new DLQ behavior (tasks with no retries now appear in DLQ with Failed status rather than being unfindable) — **clean fix, correct**

### Cleanup Actions (2 substantive commits + done tag)
1. `23387dd` — dead-code: fix `stats.mark_idle()` missing in 3 worker branches:
   - `Ok(resp) if resp.success` (task completed) — no mark_idle
   - `Ok(resp)` (provider returned failure) — no mark_idle
   - provider-not-found early `continue` — no mark_idle
   - circuit-open early `continue` — no mark_idle
   - Only `Err(e)` branch had `mark_idle()`; effect: worker active/idle counters permanently out of balance on success paths
   - Fix: moved single `stats.mark_idle()` call after `match send_result`, added `stats.mark_idle()` before each `continue`
2. `4cfc3b4` — docs: added DLQ to `architecture.md` noti-queue feature list; recorded DLQ HTTP API gap in `CLEANUP_TODO.md`

### Full Scan Results
- **Dead code**: 0 `#[allow(dead_code)]`, 0 new TODO/FIXME/HACK markers; all existing ones are in test code or comments (false positives)
- **Tests**: 0 `#[ignore]` markers; 1250 passed (32 more than baseline)
- **Code quality**: 0 `println!/dbg!` in production code; 0 `.unwrap()` in production code
- **Dependencies**: No new deps in DLQ commits — uses existing `serde_json`, `rusqlite`, etc.
- **DLQ API gap**: `list_dlq`, `dlq_stats`, `requeue_from_dlq`, `delete_from_dlq` fully implemented in queue layer but NO HTTP handler in noti-server; recorded in CLEANUP_TODO

### CLEANUP_TODO Status: 4 open structural items (up from 3 — new DLQ API gap)
- `url.rs` — `parse_notification_url()` monolithic (deferred)
- `lib.rs` — `register_all_providers()` long manual list (deferred)
- `queue_throughput.rs` — Runtime recreated per bench iteration (deferred)
- **NEW**: DLQ HTTP API not yet exposed — iteration agent needs to add DLQ endpoints

### Quality Gate
- Tests: 1250 passed, 0 failed (> baseline 1218) ✅
- Clippy: 0 warnings ✅
- No new lint warnings ✅
- All changes pushed to origin ✅

### Next Round Focus
- Monitor new iteration agent commits for DLQ HTTP API implementation
- 4 structural items remain deferred — 3 low severity + 1 new DLQ API gap for iteration agent

---


## 2026-04-07 16:00 — Cleanup round: reviewed iteration agent commits 62b019e + 706b5e9 + 1fdd434 + 9008c30 + ab8383f + 4759323

### Baseline
- Branch: `auto-improve` (HEAD = 4759323 before cleanup)
- Tests: **1218 passed / 0 failed / 2 ignored** (up from 1193 — new prometheus, circuit breaker, APNs tests)
- Clippy: 0 warnings, 0 errors
- Known: 1 moderate Dependabot vulnerability (unchanged)

### Iteration Agent Activity (since last cleanup round b989ff5)
- `62b019e` — APNs: fix blocking `std::fs` → `tokio::task::spawn_blocking`; tune shared HTTP client (`pool_max_idle_per_host(8)`, `tcp_keepalive(30s)`, `tcp_nodelay(true)`); promote `p256` to workspace dep — **clean, no issues**
- `706b5e9` — Prometheus `/metrics`: migrated from manual string building to `prometheus-client` crate with proper `Registry`, `Family<Label, Gauge>`, and `encode()` — **clean, correct**
- `1fdd434` — CircuitBreaker: `Clock` trait + `MockClock` (Arc-based); all 3 timing-sensitive tests now deterministic — **clean**
- `ab8383f`/`9008c30` — Prometheus: added worker pool metrics and rate limiting metrics — **clean**
- `4759323` — Shared HTTP client: added `connect_timeout(10s)` + `timeout(30s)` to prevent hanging requests — **clean**

### Cleanup Actions (2 substantive commits + done tag)
1. `837be1b` — docs: fix prometheus metric type annotations in `metrics-monitoring.md` — 3 inaccuracies:
   - `noti_server_version{version="0.1.9"} 1` → `noti_server_version 1` (no version label in code)
   - `TYPE noti_ratelimit_requests_total counter` → `gauge` (code uses `Gauge`, not `Counter`)
   - `TYPE noti_ratelimit_rejected_total counter` → `gauge` (same)
   - Also: `total_registered: 125` → `126` in JSON example to match actual provider count
   - Also: `prometheus.rs` doc comment `(counter)` → `(gauge)` for both ratelimit metrics
2. `2895237` — assess: closed 2 CLEANUP_TODO items — `p256` workspace dep (done in 62b019e) and prometheus-client migration (done in 706b5e9)

### Full Scan Results
- **Dead code**: 0 new TODO/FIXME/HACK, 0 new `#[allow(dead_code)]` in production code; `MockClock::advance/set` still have `#[allow(dead_code)]` — justified (test-only methods in `#[cfg(not(test))]` invisible context)
- **Tests**: 0 `#[ignore]` markers (all 3 circuit breaker timing tests now run deterministically)
- **Code quality**: 0 `println!/dbg!` in production code, 0 `.unwrap()` in production code
- **Dependencies**: `p256` correctly promoted to workspace dep; `prometheus-client = "0.22"` and `p256 = "0.13"` both in `[workspace.dependencies]`

### CLEANUP_TODO Status: 3 open structural items (down from 5)
- `url.rs` — `parse_notification_url()` monolithic (deferred)
- `lib.rs` — `register_all_providers()` long manual list (deferred)
- `queue_throughput.rs` — Runtime recreated per bench iteration (deferred)

### Quality Gate
- Tests: 1218 passed, 0 failed (> baseline 1193) ✅
- Clippy: 0 warnings ✅
- No new lint warnings ✅
- All changes pushed to origin ✅

### Next Round Focus
- Monitor new iteration agent commits for review
- 3 structural items remain deferred — low severity, no functional impact
- `MockClock::set()` has no usages — safe to keep as API completeness, no cleanup needed

---


## 2026-04-03 22:02 — Cleanup round: closed lint + assessment follow-ups after dual-backend dedup

- Continued an already-started round on `auto-improve`; finalized the remaining phase-4 style pass by standardizing `common::dual_backend_test!` doc examples in `crates/noti-server/tests/common/mod.rs` and shipped `77d8f30` (`chore(cleanup): lint: standardize dual-backend macro doc examples`).
- Phase 5 dependency review stayed low-risk: workspace deps remain centralized in `Cargo.toml`; `humantime` is only pulled directly by `noti-server`; `cargo tree -d` showed only transitive duplicate versions, so no safe direct dependency removals or version-lock changes were warranted this round.
- Phase 6 recorded deferred structural work instead of rewriting behavior: `CLEANUP_TODO.md` now adds 5 follow-ups (monolithic `parse_notification_url()`, oversized `e2e_test.rs`, repetitive provider send contract tests, crowded `handlers/queue.rs`, long manual `register_all_providers()` list) and tightens wording on two remaining dual-backend E2E pairs; shipped as `24f5000` (`chore(cleanup): assess: record deferred structural follow-ups`).
- Final gate on latest HEAD passed: `cargo test --workspace` = **1193 passed / 0 failed / 2 ignored**, `cargo clippy --workspace --all-targets --all-features -- -D warnings` = clean, and the earlier targeted `scheduled_send` regression run stayed green after the macro-doc cleanup.
- Coverage could not be re-measured locally because `cargo llvm-cov` is not installed in this environment; do not auto-install without approval, so this round records coverage as **not sampled** rather than guessing.
- Next focus: continue only safe InMemory/SQLite test dedup (`flaky_*`, `backoff_*`, `exponential_backoff_*`) and keep structural items as deferred TODOs unless a future iteration commit creates a low-risk extraction window.

---
