# Cleanup TODO

Tracked issues found during cleanup rounds. Items here are deferred for future
rounds or require coordination with the iteration agent.

## Documentation — Out of Date

- [ ] README.md and README_zh.md architecture section says "3 crates" — actual is **5** (missing `noti-queue`, `noti-server`)
- [ ] Badge in both READMEs: `3_workspace_crates` → should be `5_workspace_crates`
- [ ] `docs/guide/architecture.md` — same "three crates" issue; project tree incomplete
- [ ] `docs/guide/what-is-noti.md` — same "three crates" issue
- [ ] `docs/guide/contributing.md` — test commands only list 3 crates; missing `noti-queue`, `noti-server`
- [ ] `docs/reference/cli.md` — `send` command table missing `--priority` parameter (already implemented in CLI)
- [ ] Core features not documented anywhere: message templates, retry policies, batch/failover sending, delivery status tracking, priority system

## Code — Structural Refactoring (noti-server)

> These affect the iteration agent's in-progress `noti-server` code. Do **not** refactor until their work is committed.

- [ ] `handlers/send.rs` and `handlers/queue.rs` share identical `build_message()` function — extract to `handlers/common.rs`
- [ ] `handlers/send.rs` and `handlers/queue.rs` share identical `RetryConfig` struct — extract to common module
- [ ] `build_retry_policy()` has intentional(?) behavioral difference: `send.rs` defaults to `RetryPolicy::none()`, `queue.rs` defaults to `RetryPolicy::default()` — verify intent and document
- [ ] "provider not found" error pattern repeated 4× across handlers — extract helper function
- [ ] Consider defining a unified `ApiError` type implementing `IntoResponse` to replace `(StatusCode, Json<Value>)` pattern
- [ ] `health.rs` response type lacks `Debug` derive (all other handlers have it)

## Code — Silent Error Discards (noti-queue)

- [ ] `worker.rs` lines 133/147/157/167: `let _ = queue.ack/nack(...)` silently discards errors — consider adding `tracing::warn!` logging

## Build

- [ ] `justfile` `build-release` only builds `noti-cli` — no recipe for building `noti-server`
