# noti-clearup Memory

## 2026-04-03 22:02 — Cleanup round: closed lint + assessment follow-ups after dual-backend dedup

- Continued an already-started round on `auto-improve`; finalized the remaining phase-4 style pass by standardizing `common::dual_backend_test!` doc examples in `crates/noti-server/tests/common/mod.rs` and shipped `77d8f30` (`chore(cleanup): lint: standardize dual-backend macro doc examples`).
- Phase 5 dependency review stayed low-risk: workspace deps remain centralized in `Cargo.toml`; `humantime` is only pulled directly by `noti-server`; `cargo tree -d` showed only transitive duplicate versions, so no safe direct dependency removals or version-lock changes were warranted this round.
- Phase 6 recorded deferred structural work instead of rewriting behavior: `CLEANUP_TODO.md` now adds 5 follow-ups (monolithic `parse_notification_url()`, oversized `e2e_test.rs`, repetitive provider send contract tests, crowded `handlers/queue.rs`, long manual `register_all_providers()` list) and tightens wording on two remaining dual-backend E2E pairs; shipped as `24f5000` (`chore(cleanup): assess: record deferred structural follow-ups`).
- Final gate on latest HEAD passed: `cargo test --workspace` = **1193 passed / 0 failed / 2 ignored**, `cargo clippy --workspace --all-targets --all-features -- -D warnings` = clean, and the earlier targeted `scheduled_send` regression run stayed green after the macro-doc cleanup.
- Coverage could not be re-measured locally because `cargo llvm-cov` is not installed in this environment; do not auto-install without approval, so this round records coverage as **not sampled** rather than guessing.
- Next focus: continue only safe InMemory/SQLite test dedup (`flaky_*`, `backoff_*`, `exponential_backoff_*`) and keep structural items as deferred TODOs unless a future iteration commit creates a low-risk extraction window.

---


## 2026-04-03 13:33 — Cleanup round: synced docs drift after commits 70607bc..ff233ef

- Baseline remained strong on `auto-improve`: tests stayed at **1185 passed / 0 failed / 2 ignored**, `cargo clippy --all-targets --all-features -- -D warnings` stayed clean, and `auto-improve` is not behind `origin/main`.
- Reviewed recent iteration work around status purge, tracing, config/docs expansion, CORS logging, queue test helper extraction, queue backend parsing, and e2e helper cleanup.
- Applied low-risk governance fixes only: added `POST /api/v1/status/purge` back into API summary docs, normalized stale Slack server examples from `config.webhook` to `config.webhook_url`, corrected one lingering Kubernetes image reference to `ghcr.io/loonghao/noti-server:latest`, and refreshed `CLEANUP_TODO.md` (`Client::new()` count now 171; removed brittle line-number references from active dedup items).
- Shipped as commit `6d3473f` (`chore(cleanup): docs: sync status purge and stale config examples`) and pushed to `origin/auto-improve`; remote still reports the same 1 moderate default-branch vulnerability (unchanged).
- Next focus remains structural/test dedup only: `reqwest::Client::new()` boilerplate and InMemory-vs-SQLite near-duplicate e2e pairs.

---



## 2026-04-03 00:27 — Cleanup round: reviewed iteration agent commits 0f84118 + f5982b6 (OpenAPI guide, Metrics & Monitoring guide)

### Baseline
- Branch: `auto-improve` (HEAD = f5982b6 before cleanup)
- Tests: 1185 passed, 0 failed, 2 ignored (doc-tests)
- Clippy: 0 warnings, 0 errors
- Known: 1 moderate Dependabot vulnerability (unchanged)

### Iteration Agent Activity
2 new commits since last round:
- `0f84118` — docs(guide): add standalone OpenAPI & Swagger UI guide page
  - New: `docs/guide/openapi.md` (259 lines — comprehensive OpenAPI/Swagger docs)
  - Modified: `docs/.vitepress/config.mts` (sidebar entry added under Usage)
- `f5982b6` — docs(guide): add standalone Metrics & Monitoring guide page
  - New: `docs/guide/metrics-monitoring.md` (422 lines — comprehensive monitoring docs)
  - Modified: `docs/.vitepress/config.mts` (sidebar entry added under Operations)

### Review of Iteration Agent Commits
- **openapi.md**: comprehensive and highly accurate — 22 endpoints, 41 schemas, 8 tags all match `openapi.rs`; SendRequest example verified field-by-field; SDK generation examples, Docker tips, AI agent tips all correct
  - Minor: `/swagger-ui/` vs `/swagger-ui` (trailing slash) — functionally equivalent, utoipa-swagger-ui handles both
- **metrics-monitoring.md**: comprehensive and accurate — HealthResponse, MetricsResponse, StatsResponse, ProviderMetrics all verified against code; 4 env vars verified; 125 providers confirmed; X-Request-Id middleware confirmed with full E2E test coverage; health always returns 200 confirmed
  - Monitoring integration patterns (Prometheus JSON Exporter, Pushgateway, Datadog/StatsD) are practical and well-documented
  - Alerting recommendations with AlertManager rules are well-structured
- **VitePress config**: sidebar entries correctly placed (openapi under Usage, metrics under Operations)
- **Issue found**: `openapi.rs` line 36 described "130+ providers" but actual count is 125 — **fixed**

### Cleanup Actions (1 commit)
1. `a770dfc` — docs: fix OpenAPI description provider count 130+ to 125+ to match actual registration count

### Full Scan Results
- **Dead code**: 0 TODO/FIXME/HACK markers, 0 `#[allow(dead_code)]`, 0 compiler unused warnings
- **Documentation**: 1 inaccuracy fixed (openapi.rs "130+" → "125+"); all other doc references verified accurate
- **Tests**: 0 `#[ignore]` or `#[should_panic]` markers. No test target drift.
- **Code quality**: 0 `println!/dbg!` in production code (only in CLI output modules — expected), 0 `.unwrap()` in production code
- **Dependencies**: All deps workspace-locked; no changes this round
- **Temporary files**: None found

### CLEANUP_TODO Status: 22 open items (unchanged)
- Minor code: 2 (config.rs asymmetric match, 169× Client::new())
- E2E test quality: 19 (1 misleading name, 13× inline setup, 16× near-dup pairs)
- Cross-module dedup: 1 (make_task())

### Quality Gate
- Tests: 1185 passed, 0 failed (= baseline) ✅
- Clippy: 0 warnings ✅
- No new lint warnings ✅
- All changes pushed to origin ✅

### Assessment
- Codebase remains excellent — 0 regressions, all quality metrics at or above baseline
- Iteration agent's 2 commits are pure documentation — both highly comprehensive and well-organized
- 1 code inaccuracy fixed: openapi.rs description said "130+ providers" but actual count is 125
- Documentation coverage continues to improve — now has dedicated guides for OpenAPI, Metrics, Authentication, Rate Limiting, Webhook Callbacks, API Versioning, CI/CD, Deployment, Scheduled Send, and Core Features
- All open items remain structural/dedup — low severity, no functional impact

### Next Round Focus
- Monitor new iteration agent commits for review
- Structural items remain: inline setup (13), near-dup pairs (16), make_task() (1)
- config.rs asymmetric match remains a minor code smell

---

## 2026-04-02 22:19 — Cleanup round: reviewed iteration agent commits 5e23d91 + fbb80a5 (webhook callbacks guide, auth/rate-limiting docs)

### Baseline
- Branch: `auto-improve` (HEAD = fbb80a5 before cleanup)
- Tests: 1185 passed, 0 failed, 2 ignored (doc-tests)
- Clippy: 0 warnings, 0 errors
- Known: 1 moderate Dependabot vulnerability (unchanged)

### Iteration Agent Activity
2 new commits since last round:
- `5e23d91` — feat(docs): add webhook callbacks guide + multi-arch Docker + Compose profiles
  - New: `docs/guide/webhook-callbacks.md` (full webhook callbacks documentation)
  - Modified: `.github/workflows/docker.yml` (added QEMU + multi-arch build)
  - Modified: `Dockerfile` (multi-arch support: cross-compilation toolchain for arm64, TARGETPLATFORM/BUILDPLATFORM args)
  - Modified: `docker-compose.yml` (added dev profile service with relaxed settings)
  - Modified: `docs/.vitepress/config.mts` (sidebar entry added)
  - Modified: `docs/guide/deployment.md` (updated for profiles and multi-arch)
- `fbb80a5` — docs(guide): add standalone Authentication and Rate Limiting guide pages
  - New: `docs/guide/authentication.md` (full authentication documentation)
  - New: `docs/guide/rate-limiting.md` (full rate limiting documentation)
  - Modified: `docs/.vitepress/config.mts` (2 sidebar entries added)

### Review of Iteration Agent Commits
- **webhook-callbacks.md**: comprehensive, accurate — callback payload, delivery semantics, security considerations, integration patterns all match code (best-effort delivery, 10s timeout, fire-once)
- **Dockerfile multi-arch**: correct cross-compilation setup (gcc-aarch64-linux-gnu, CARGO_TARGET, binary copy from target triple path)
- **docker-compose.yml**: clean prod/dev profile split, all env vars correctly mapped
- **docker.yml workflow**: QEMU + buildx setup correct, `platforms: linux/amd64,linux/arm64` added
- **authentication.md**: detailed and accurate — env vars, headers, middleware order, error responses, K8s examples all match code
  - **Issue found**: Section "noti CLI" (lines 136-143) described `NOTI_SERVER_API_KEY` env var that does NOT exist in code — **removed**
- **rate-limiting.md**: accurate — token bucket algorithm, env vars, response headers, IP detection, all match code
- **VitePress config**: sidebar entries correctly placed under "Usage" section

### Cleanup Actions (1 commit)
1. `5657617` — docs: removed undocumented `NOTI_SERVER_API_KEY` env var from authentication guide (not implemented in code)

### Full Scan Results
- **Dead code**: 0 TODO/FIXME/HACK markers, 0 `#[allow(dead_code)]`, 0 compiler unused warnings
- **Documentation**: 1 doc inaccuracy fixed (nonexistent NOTI_SERVER_API_KEY); all other doc references verified accurate
- **Tests**: 0 `#[ignore]` or `#[should_panic]` markers. No test target drift.
- **Code quality**: 0 `println!/dbg!` in production code, 0 `.unwrap()` in production code
- **Dependencies**: All deps workspace-locked; no changes this round
- **Temporary files**: None found

### CLEANUP_TODO Status: 22 open items (unchanged)
- Minor code: 2 (config.rs asymmetric match, 169× Client::new())
- E2E test quality: 19 (1 misleading name, 13× inline setup, 16× near-dup pairs)
- Cross-module dedup: 1 (make_task())

### Quality Gate
- Tests: 1185 passed, 0 failed (= baseline) ✅
- Clippy: 0 warnings ✅
- No new lint warnings ✅
- All changes pushed to origin ✅

### Assessment
- Codebase remains excellent — 0 regressions, all quality metrics at or above baseline
- Iteration agent's 2 commits are primarily documentation + Docker infrastructure — clean and well-done
- 1 doc inaccuracy fixed: authentication guide referenced nonexistent `NOTI_SERVER_API_KEY` env var
- Multi-arch Docker build is a significant infrastructure improvement
- Compose dev profile is useful for local development
- No new e2e tests added (docs-only commits), so Client::new() and inline setup counts unchanged

### Next Round Focus
- Monitor new iteration agent commits for review
- Structural items remain: inline setup (13), near-dup pairs (16), make_task() (1)
- config.rs asymmetric match remains a minor code smell

---

## 2026-04-02 20:09 — Cleanup round: reviewed iteration agent commit 57d99f3 (API versioning)

### Baseline
- Branch: `auto-improve` (HEAD = 57d99f3 before cleanup)
- Tests: 1185 passed, 0 failed, 2 ignored (doc-tests)
- Clippy: 0 warnings, 0 errors
- Known: 1 moderate Dependabot vulnerability (unchanged)

### Iteration Agent Activity
1 new commit since last round:
- `57d99f3` — feat(api): add API versioning with Router::nest and /api/versions endpoint + docs + 3 tests
  - Modified: `crates/noti-server/src/routes.rs` (refactored to use `Router::nest` for `/api/v1/`, added `ApiVersion`, `ApiVersionsResponse`, `list_api_versions()`)
  - Modified: `crates/noti-server/src/openapi.rs` (added Meta tag, `routes::list_api_versions` path, `ApiVersion`/`ApiVersionsResponse` schemas)
  - Modified: `crates/noti-server/tests/e2e_test.rs` (+54 lines: 2 new e2e tests + `/api/versions` in OpenAPI paths check)
  - Modified: `crates/noti-server/tests/server_test.rs` (+16 lines: 1 new test)
  - New: `docs/guide/api-versioning.md` (full API versioning documentation)
  - Modified: `docs/.vitepress/config.mts` (sidebar entry added)

### Review of Iteration Agent Commit
- `Router::nest("/api/v1", build_v1_routes())` — clean architecture, makes future v2 a single line
- `ApiVersion` struct with `version`, `status`, `deprecated` — well-designed version metadata
- `list_api_versions()` handler — simple, correct, documented with utoipa annotation
- `build_v1_routes()` properly isolated — handlers only define relative paths
- All existing API endpoints correctly nested under `/api/v1/`
- Health and version-independent endpoints (`/health`, `/api/versions`, `/swagger-ui`) correctly outside nest
- Documentation (`api-versioning.md`) is comprehensive: version lifecycle, endpoint table, architecture, best practices, deprecation policy
- Tests cover: version discovery endpoint, OpenAPI spec inclusion, existing paths assertion updated
- No issues found — implementation is clean and well-documented

### Cleanup Actions (1 commit)
1. `5cb33b7` — docs: added API versioning to architecture.md noti-server features; updated CLEANUP_TODO Client::new() count 167→169

### Full Scan Results
- **Dead code**: 0 TODO/FIXME/HACK markers, 0 `#[allow(dead_code)]`, 0 compiler unused warnings
- **Documentation**: All doc files now mention API versioning consistently; architecture.md gap fixed
- **Tests**: 0 `#[ignore]` or `#[should_panic]` markers. No test target drift.
- **Code quality**: 0 `println!/dbg!` in production code, 0 `.unwrap()` in production code
- **Dependencies**: All deps workspace-locked; no changes this round
- **Temporary files**: None found

### CLEANUP_TODO Status: 22 open items (unchanged)
- Minor code: 2 (config.rs asymmetric match, 169× Client::new())
- E2E test quality: 19 (1 misleading name, 13× inline setup, 16× near-dup pairs)
- Cross-module dedup: 1 (make_task())

### Quality Gate
- Tests: 1185 passed, 0 failed (= baseline) ✅
- Clippy: 0 warnings ✅
- No new lint warnings ✅
- All changes pushed to origin ✅

### Assessment
- Codebase remains excellent — 0 regressions, all quality metrics at or above baseline
- Iteration agent's API versioning implementation is clean: proper Router::nest usage, comprehensive docs, good test coverage
- 1 doc gap fixed: architecture.md missing API versioning in noti-server feature list
- Client::new() count grew by 2 (169 total) — low impact boilerplate
- All open items remain structural/dedup — low severity, no functional impact

### Next Round Focus
- Monitor new iteration agent commits for review
- Structural items remain: inline setup (13), near-dup pairs (16), make_task() (1)
- config.rs asymmetric match remains a minor code smell

---

## 2026-04-02 18:00 — Cleanup round: reviewed iteration agent commits 7d3350e + 9bdd527 (scheduled/delayed send)

### Baseline
- Branch: `auto-improve` (HEAD = 9bdd527 before cleanup)
- Tests: 1182 passed, 0 failed, 2 ignored (doc-tests)
- Clippy: 0 warnings, 0 errors
- Known: 1 moderate Dependabot vulnerability (unchanged)

### Iteration Agent Activity
2 new commits since last round:
- `7d3350e` — feat(queue): add scheduled/delayed notification support (delay_seconds, scheduled_at) + 15 tests
- `9bdd527` — docs(scheduled-send): add scheduled/delayed send guide + 7 SQLite E2E tests
  - New: `docs/guide/scheduled-send.md` (full scheduled send documentation)
  - Modified: `crates/noti-queue/src/task.rs` (added `available_at` field + builder + 2 tests)
  - Modified: `crates/noti-server/Cargo.toml` (added `humantime` dep)
  - Modified: `crates/noti-server/src/handlers/queue.rs` (+134 lines: `parse_scheduled_time()`, `delay_seconds`/`scheduled_at` in `AsyncSendRequest`, `scheduled_at` in `TaskInfo`, 6 unit tests)
  - Modified: `crates/noti-server/tests/e2e_test.rs` (+525 lines: 13 new e2e tests)
  - Modified: `docs/.vitepress/config.mts` (sidebar entry added)

### Review of Iteration Agent Commits
- `parse_scheduled_time()` — clean implementation with proper mutual-exclusion check, delay_seconds=0 treated as immediate
- `AsyncSendRequest` correctly extends with `delay_seconds: Option<u64>` and `scheduled_at: Option<String>`
- `TaskInfo` adds `scheduled_at` with proper `skip_serializing_if`
- Queue handler correctly uses `humantime::parse_rfc3339` for RFC 3339 parsing
- `task.rs`: `with_available_at()` builder method properly integrates with existing task lifecycle
- Batch endpoint correctly handles per-item schedule/delay with proper error propagation
- Documentation is comprehensive with API examples, error handling, Rust SDK, and queue backend support
- All 6 unit tests for `parse_scheduled_time()` cover edge cases (both, none, zero, valid, invalid)
- No issues found in implementation — clean and well-documented

### Cleanup Actions (3 commits)
1. `9dd70a9` — dead-code: removed clippy.tmp + test_output.tmp, added `*.tmp` to .gitignore
2. `41922ba` — docs: added `delay_seconds`/`scheduled_at` to core-features.md async send example; added scheduled/delayed delivery to architecture.md noti-queue section
3. `b22814d` — tests: updated Client::new() count from 153→167; added 6 new near-dup scheduled send test pairs to CLEANUP_TODO.md

### Full Scan Results
- **Dead code**: 0 TODO/FIXME/HACK markers, 0 `#[allow(dead_code)]`, 0 compiler unused warnings
- **Documentation**: All doc files now mention scheduled/delayed send consistently (core-features, architecture, scheduled-send guide, VitePress sidebar)
- **Tests**: 0 `#[ignore]` or `#[should_panic]` markers. No test target drift.
- **Code quality**: 0 `println!/dbg!` in production code, 0 `.unwrap()` in production code
- **Dependencies**: humantime added correctly for timestamp parsing; all deps workspace-locked
- **Temporary files**: Removed 2 .tmp files; added *.tmp pattern to .gitignore

### CLEANUP_TODO Status: 22 open items (was 16)
- Minor code: 2 (config.rs asymmetric match, 167× Client::new())
- E2E test quality: 19 (1 misleading name, 13× inline setup, 16× near-dup pairs — 6 new from scheduled send)
- Cross-module dedup: 1 (make_task())

### Quality Gate
- Tests: 1182 passed, 0 failed (= baseline) ✅
- Clippy: 0 warnings ✅
- No new lint warnings ✅
- All changes pushed to origin ✅

### Assessment
- Codebase remains excellent — 0 regressions, all quality metrics at or above baseline
- Iteration agent's scheduled send implementation is clean and well-integrated
- 2 doc gaps fixed: core-features example missing schedule fields, architecture missing feature mention
- Near-dup test pairs grew by 6 (InMemory vs SQLite pattern continues)
- All open items are structural/dedup — low severity, no functional impact

### Next Round Focus
- Monitor new iteration agent commits for review
- Structural items remain: inline setup (13), near-dup pairs (16), make_task() (1)
- config.rs asymmetric match remains a minor code smell

---

## 2026-04-02 15:53 — Cleanup round: reviewed iteration agent commit 1685380 (Docker CI/CD)

### Baseline
- Branch: `auto-improve` (HEAD = 1685380)
- Tests: 1160 passed, 0 failed, 2 ignored (doc-tests) — unchanged
- Clippy: 0 warnings, 0 errors
- Known: 1 moderate Dependabot vulnerability (unchanged)

### Iteration Agent Activity
1 new commit since last round:
- `1685380` — feat(docker): HTTP health check + Docker CI/CD workflow + pipeline docs
  - New: `.github/workflows/docker.yml` (Docker build & push to GHCR)
  - New: `docs/guide/ci-cd.md` (full CI/CD pipeline documentation)
  - Modified: `Dockerfile` (added `curl` for HTTP health check)
  - Modified: `docker-compose.yml` (HEALTHCHECK uses HTTP `/health`)
  - Modified: `docs/guide/deployment.md` (curl mention in description)
  - Modified: `docs/.vitepress/config.mts` (CI/CD added to Operations sidebar)

### Review of Iteration Agent Commit
- Docker workflow: well-structured, uses buildx, GHA cache, correct GHCR registry, concurrency group, proper permissions
- All 6 workflows in `.github/workflows/` are documented in `ci-cd.md` — complete
- Version references (`0.1.5`, 4 build targets, 6 workflows) all consistent with code
- All `actions/checkout@v6` — consistent across all workflows
- Dockerfile health check correctly uses `curl -sf http://localhost:3000/health`
- No issues found — commit is clean and well-documented

### Full Scan Results
- **Dead code**: 0 TODO/FIXME/HACK markers, 0 `#[allow(dead_code)]`, 0 compiler unused warnings
- **Documentation**: All doc files verified — versions, crate names, provider count, CLI commands, env vars, CI/CD workflows, Docker config — all consistent
- **Tests**: 0 `#[ignore]` or `#[should_panic]` markers. No test target drift.
- **Code quality**: 0 `println!/dbg!` in production code, 0 `.unwrap()` in production code
- **Dependencies**: All dependencies referenced; versions locked via workspace
- **Temporary files**: None found (no .tmp, .bak, .orig, .old, .log, test_output)

### CLEANUP_TODO Status: 16 open items (unchanged count)
- Minor code: 2 (config.rs asymmetric match, 153× Client::new())
- E2E test quality: 13 (1 misleading name, 13× inline setup, 10× near-dup pairs)
- Cross-module dedup: 1 (make_task())

### Assessment
- Codebase remains excellent — 0 regressions, all quality metrics at or above baseline
- Iteration agent's Docker/CI commit is clean and well-integrated
- All open items are structural/dedup — low severity, no functional impact
- No actionable findings this round

### Next Round Focus
- Monitor new iteration agent commits for review
- Structural items remain: inline setup (13), near-dup pairs (10), make_task() (1)
- config.rs asymmetric match remains a minor code smell

---

## 2026-04-03 04:35 — Cleanup round: reviewed iteration agent commits 6762116 + ca46193 + d6dd86b + bcd9ace

### High-Level Summary
- Baseline unchanged: `auto-improve`; tests still **1185 passed / 0 failed / 2 ignored**; `cargo clippy --all-targets --all-features -- -D warnings` passed cleanly.
- Reviewed 4 recent iteration commits (Queue Management, Logging & Observability, Error Codes, Health Check guides). The new guides are broadly accurate and well-structured.
- Found and fixed 1 documentation drift pattern: 422 validation error examples in `error-codes.md` and `core-features.md` still showed the old object-array shape, while current `ValidatedJson` returns arrays of human-readable strings. Also softened the 413 description in `error-codes.md` to reflect framework-level body-limit rejection behavior.
- Created and pushed cleanup commit `41e2d19` — `chore(cleanup): docs: align validation error examples with actual API responses` (+ `chore(cleanup): done`). Remote still reports **1 moderate vulnerability** on the default branch (unchanged).

### Next Round Focus
- Continue reviewing fresh iteration-agent documentation for API/example drift.
- Revisit existing structural TODOs only when they can be changed without feature risk.

---

## 2026-04-03 17:30 — Cleanup round: finalized dual-backend E2E dedup after iteration commit cbb3a9b

- Baseline held on `auto-improve`: `cargo test --workspace` stayed at **1185 passed / 0 failed / 2 ignored** and `cargo clippy --all-targets --all-features -- -D warnings` stayed clean; branch is synced with `origin/main` and pushed to `origin/auto-improve`.
- Reviewed the latest iteration-agent test cleanup commit `cbb3a9b` (`test_client()` extraction) plus the in-progress local follow-up; the remaining safe cleanup opportunity was duplicated InMemory/SQLite E2E coverage.
- Completed one low-risk test-governance step: added `common::dual_backend_test!` usage for the mixed-priority ordering and graceful-shutdown pairs, fixed the macro-adjacent `unused doc comment` warning, and corrected the misleading `test_client()` helper comment so it matches the actual implementation.
- Updated `CLEANUP_TODO.md` to close the two deduplicated pair entries, then shipped commit `b848e01` — `chore(cleanup): tests: dedupe dual-backend e2e pairs` (+ `chore(cleanup): done`) and pushed it.
- Next focus remains the remaining near-duplicate dual-backend E2E pairs; keep changes small and only convert pairs that can be deduplicated without obscuring backend-specific assertions.

