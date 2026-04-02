# CI/CD Pipeline

noti uses GitHub Actions for continuous integration, testing, release management, and Docker image publishing.

## Pipeline Overview

| Workflow | Trigger | Purpose |
|:---------|:--------|:--------|
| **CI** | Push to `main`, PRs | Lint, test (3 platforms), coverage, build |
| **Docker** | Push to `main`, releases | Build & push Docker image to GHCR |
| **Deploy Docs** | Changes to `docs/` | Build VitePress docs → GitHub Pages |
| **Release Please** | Push to `main` | Auto-generate changelogs and release PRs |
| **Release** | Tag created by Release Please | Build platform binaries, upload to GitHub Releases |
| **Generate Skills** | Changes to `skills/` | Package and publish skills to ClawHub |

## CI Workflow

The main CI pipeline runs on every push to `main` and on all pull requests.

### Jobs

1. **Quality Checks** — `clippy` (with `-D warnings`) and `rustfmt` verification
2. **Tests** — Full test suite on Ubuntu, macOS, and Windows
3. **Coverage** — Generates LCOV coverage report and uploads to Codecov
4. **Build** — Release builds for 4 targets:
   - `x86_64-unknown-linux-gnu`
   - `x86_64-pc-windows-msvc`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`

Build jobs run after quality and test jobs pass.

### Running locally

```bash
# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check

# Full test suite
cargo test --workspace
```

## Docker Workflow

Automatically builds and pushes the `noti-server` Docker image to GitHub Container Registry (GHCR).

### Triggers

- **Push to `main`** — Tags the image as `latest` and `sha-<commit>`
- **Release published** — Tags with semver versions (`0.1.5`, `0.1`, etc.)
- **Manual dispatch** — Run on demand from the Actions tab

### Image tags

| Event | Tag examples |
|:------|:-------------|
| Push to main | `latest`, `main`, `sha-abc1234` |
| Release v0.1.5 | `0.1.5`, `0.1`, `sha-abc1234` |
| Release v1.2.0 | `1.2.0`, `1.2`, `1`, `sha-abc1234` |

### Pulling the image

```bash
# Latest from main branch
docker pull ghcr.io/loonghao/noti-server:latest

# Specific version
docker pull ghcr.io/loonghao/noti-server:0.1.5
```

### Build caching

The workflow uses GitHub Actions cache (`type=gha`) for Docker layer caching, which significantly speeds up subsequent builds.

## Release Workflow

Releases are managed by [Release Please](https://github.com/googleapis/release-please), which:

1. Watches conventional commits on `main`
2. Creates/updates a release PR with changelog entries
3. When the PR is merged, creates a GitHub Release with a tag
4. The Release workflow builds platform-specific binaries and uploads them

### Release artifacts

Each release includes:

- Pre-built binaries for Linux (x86_64), macOS (x86_64, aarch64), and Windows (x86_64)
- Both versioned (`noti-v0.1.5-x86_64-unknown-linux-gnu.tar.gz`) and unversioned archives
- SHA256 checksum file (`noti-checksums.txt`)

## Docs Workflow

The documentation site is built with [VitePress](https://vitepress.dev/) and deployed to GitHub Pages.

- **Trigger**: Push to `main` when files under `docs/` change
- **Build**: `npm ci && npm run docs:build`
- **Deploy**: Uploaded as a GitHub Pages artifact and deployed automatically

## Running CI checks before pushing

We recommend running the full quality + test suite locally before pushing:

```bash
# Quick pre-push check
cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

Or use the project's justfile:

```bash
# Run the full CI check (same as GitHub Actions)
just ci

# Run tests only
just test
```
