# noti

A unified multi-channel notification CLI for AI agents.

[![CI](https://github.com/loonghao/wecom-bot-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/wecom-bot-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

**noti** is a Rust CLI tool inspired by [notifiers](https://github.com/liiight/notifiers) and [Apprise](https://github.com/caronc/apprise). It provides a unified interface to send notifications through multiple channels, designed to be easily consumed by AI agents like [OpenClaw](https://github.com/nicepkg/openclaw).

### Key Features

- 🚀 **CLI-first** — designed for agent/script integration
- 📡 **7 built-in providers** — WeCom, Feishu, Slack, Telegram, Discord, Email, Webhook
- 🔗 **URL scheme addressing** — `wecom://<key>`, `slack://<tokens>`, etc.
- 📋 **Profile management** — save and reuse notification configs
- 🤖 **JSON output** — `--json` flag for structured machine-readable output
- ⚡ **Fast** — native Rust binary, instant startup

## Installation

### From source

```bash
cargo install --git https://github.com/loonghao/wecom-bot-cli noti-cli
```

## Quick Start

### Send via URL scheme

```bash
# WeCom
noti send --to "wecom://<webhook_key>" --message "Hello from noti!"

# Feishu / Lark
noti send --to "feishu://<hook_id>" --message "Hello from noti!"

# Slack
noti send --to "slack://<token_a>/<token_b>/<token_c>" --message "Hello!"

# Telegram
noti send --to "tg://<bot_token>/<chat_id>" --message "Hello!"

# Discord
noti send --to "discord://<webhook_id>/<webhook_token>" --message "Hello!"

# Email
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=recipient@example.com" \
  --message "Hello!" --title "Test Email"

# Generic Webhook
noti send --to "webhook://example.com/api/notify" --message "Hello!"
```

### Send via saved profile

```bash
# Save a profile
noti config set --name my-team --provider wecom --param key=<webhook_key>

# Send using profile
noti send --profile my-team --message "Hello from noti!"

# Test the profile
noti config test my-team
```

### Send with direct provider flags

```bash
noti send --provider wecom --param key=<webhook_key> --message "Hello!"
```

### JSON output (for agents)

```bash
noti --json send --to "wecom://<key>" --message "deploy complete"
```

```json
{
  "success": true,
  "provider": "wecom",
  "status_code": 200,
  "message": "message sent successfully",
  "raw_response": { "errcode": 0, "errmsg": "ok" }
}
```

## Supported Providers

| Provider | Scheme | Description |
|----------|--------|-------------|
| WeCom | `wecom://` | WeChat Work group bot webhook |
| Feishu | `feishu://` | Feishu / Lark group bot webhook |
| Slack | `slack://` | Slack incoming webhooks |
| Telegram | `tg://` | Telegram Bot API |
| Discord | `discord://` | Discord webhooks |
| Email | `smtp://` | Email via SMTP |
| Webhook | `webhook://` | Generic HTTP webhook |

### List providers

```bash
noti providers list
noti providers info wecom
```

## Configuration

Config file location: `~/.config/noti/config.toml`

Override with: `NOTI_CONFIG=/path/to/config.toml`

### Profile management

```bash
noti config set --name <name> --provider <provider> --param key=value
noti config get <name>
noti config list
noti config remove <name>
noti config test <name>
noti config path
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Send failure (network/API error) |
| 2 | Parameter/configuration error |

## Agent Integration

noti is designed for AI agents. Key features:

1. **URL scheme** — one-line addressing, no config files needed
2. **`--json` flag** — structured output for parsing
3. **Exit codes** — deterministic success/failure signals
4. **Profile system** — pre-configure once, use by name

### Example: OpenClaw integration

```bash
# Agent can discover available providers
noti --json providers list

# Agent can inspect provider parameters
noti --json providers info wecom

# Agent sends notification
noti --json send --to "wecom://key123" --message "Task completed"
```

## Development

### Prerequisites

- [vx](https://github.com/loonghao/vx) — tool version manager
- Rust (stable, managed by vx)

### Commands

```bash
vx just fmt          # Format code
vx just check        # Type check
vx just lint         # Clippy lint
vx just test         # Run tests
vx just ci           # Full CI pipeline
vx just run -- send --help   # Run CLI
```

## License

MIT
