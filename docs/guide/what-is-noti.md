# What is noti?

**noti** is a unified multi-channel notification CLI built for AI agents and automation. It replaces the need to integrate different SDKs for Slack, email, SMS, and push notifications with a single, blazing-fast Rust binary that speaks a universal URL scheme.

## Why noti?

> **One binary. One command. 125+ notification channels.**

Most projects cobble together different SDKs for Slack, email, SMS, and push notifications. **noti** replaces all of that with a single CLI that works perfectly in shell scripts, CI pipelines, and AI agents.

```bash
# That's it. One line. Any channel.
noti send --to "wecom://<key>" --message "Deploy complete ✅"
noti send --to "slack://<token>" --message "Build passed 🎉"
noti send --to "tg://<bot>/<chat>" --message "Alert: CPU > 90%"
```

## Key Design Principles

| Feature | Benefit |
|:--------|:--------|
| **CLI-first** | One-line commands, no config files needed |
| **URL scheme addressing** | Intuitive `provider://credentials` format |
| **Profile management** | Save, reuse, and test notification configs |
| **Machine-friendly** | `--json` output + deterministic exit codes |
| **Blazing fast** | Native Rust binary, < 10ms startup |
| **File attachments** | Send images, documents & media — auto-detected MIME types |
| **125+ providers** | Chat, SMS, email, push, webhooks, IoT — all built-in |

## Architecture

noti is organized as a Rust workspace with five crates:

```
noti/
├── crates/
│   ├── noti-cli/        # CLI binary — argument parsing, output formatting
│   ├── noti-core/       # Core abstractions — NotifyProvider trait, ProviderRegistry, URL parsing
│   ├── noti-providers/  # 126 provider implementations (one file each)
│   ├── noti-queue/      # Async message queue — background task processing
│   └── noti-server/     # REST API server — HTTP endpoints, middleware
├── scripts/             # Install scripts (bash, PowerShell) & utilities
├── skills/              # OpenClaw skill definitions
└── justfile             # Task runner recipes (via vx)
```

## Supported Platforms

| Platform | Architecture |
|:---------|:------------|
| **Windows** | x86_64 |
| **macOS** | x86_64 (Intel), ARM64 (Apple Silicon) |
| **Linux** | x86_64 |
