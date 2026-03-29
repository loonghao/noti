# Architecture

noti is organized as a Rust workspace with three crates, each with a focused responsibility.

## Project Structure

```
noti/
├── crates/
│   ├── noti-cli/        # CLI binary
│   ├── noti-core/       # Core abstractions
│   └── noti-providers/  # 125 provider implementations
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

- **`Provider` trait** — async interface all providers implement
- **`Registry`** — provider discovery and instantiation
- **`URL parsing`** — universal `provider://credentials` scheme parser
- **`Config`** — profile management and TOML persistence
- **`Error types`** — structured error handling

### `noti-providers`

All 125 provider implementations, one file per provider. Each provider:

1. Implements the `Provider` trait
2. Registers itself with the `Registry`
3. Parses its URL scheme format
4. Sends the notification via the provider's API

## Technology Stack

| Component | Technology |
|:----------|:-----------|
| Language | Rust 2024 edition (MSRV 1.85) |
| CLI framework | clap 4.5 |
| HTTP client | reqwest 0.12 |
| Async runtime | tokio 1.44 |
| Email | lettre 0.11 |
| Serialization | serde / serde_json |
| Testing | rstest, assert_cmd, wiremock |
| Task runner | just (via vx) |
