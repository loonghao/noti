# Contributing

## Prerequisites

- [vx](https://github.com/loonghao/vx) — universal tool version manager
- Rust 1.85+ (stable, managed by vx)

## Development Commands

All commands use `vx` as the universal tool version manager:

```bash
vx just fmt          # Format code
vx just check        # Type check
vx just lint         # Clippy lint
vx just test         # Run tests
vx just coverage     # Generate coverage report (LCOV)
vx just coverage-html # Generate HTML coverage report
vx just ci           # Full CI pipeline (fmt + check + lint + test)
vx just run -- send --help   # Run CLI in dev mode
```

## Adding a New Provider

1. Create a new file in `crates/noti-providers/src/` (e.g., `my_provider.rs`)
2. Implement the `Provider` trait from `noti-core`
3. Register the provider in `crates/noti-providers/src/lib.rs`
4. Add URL parsing support in `crates/noti-core/src/url.rs`
5. Add tests in `crates/noti-providers/tests/`

### Provider Trait

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn scheme(&self) -> &str;
    fn required_params(&self) -> Vec<&str>;
    fn optional_params(&self) -> Vec<&str>;
    async fn send(&self, message: &Message) -> Result<ProviderResponse, ProviderError>;
}
```

## Running Tests

```bash
# Run all tests
vx just test

# Run tests for a specific crate
vx cargo test -p noti-core
vx cargo test -p noti-providers
vx cargo test -p noti-cli
vx cargo test -p noti-queue
vx cargo test -p noti-server
```

## Code Style

- Format with `vx cargo fmt --all`
- Lint with `vx cargo clippy --workspace --all-targets -- -D warnings`
- All PRs must pass the full CI pipeline (`vx just ci`)
