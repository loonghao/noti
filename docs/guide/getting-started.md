# Getting Started

## Installation

### Install script (recommended)

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.ps1 | iex
```

### From source

```bash
cargo install --git https://github.com/loonghao/noti noti-cli
```

### Download binary

Pre-built binaries for all platforms are available on the [Releases](https://github.com/loonghao/noti/releases) page:

| Platform | Architecture | File |
|:---------|:------------|:-----|
| **Windows** | x86_64 | [`noti-x86_64-pc-windows-msvc.zip`](https://github.com/loonghao/noti/releases/latest) |
| **macOS** | x86_64 (Intel) | [`noti-x86_64-apple-darwin.tar.gz`](https://github.com/loonghao/noti/releases/latest) |
| **macOS** | ARM64 (Apple Silicon) | [`noti-aarch64-apple-darwin.tar.gz`](https://github.com/loonghao/noti/releases/latest) |
| **Linux** | x86_64 | [`noti-x86_64-unknown-linux-gnu.tar.gz`](https://github.com/loonghao/noti/releases/latest) |

### Default install paths

| Platform | Path |
|:---------|:-----|
| macOS / Linux | `~/.local/bin/noti` |
| Windows | `%USERPROFILE%\.noti\bin\noti.exe` |
| Custom | Set `NOTI_INSTALL_DIR` |

## Quick Start

### 1. Send via URL scheme

```bash
# WeCom (企业微信)
noti send --to "wecom://<webhook_key>" --message "Hello from noti!"

# Slack
noti send --to "slack://<token_a>/<token_b>/<token_c>" --message "Hello!"

# Telegram
noti send --to "tg://<bot_token>/<chat_id>" --message "Hello!"

# Discord
noti send --to "discord://<webhook_id>/<webhook_token>" --message "Hello!"

# Email (SMTP)
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=recipient@example.com" \
  --message "Hello!" --title "Test Email"
```

### 2. Send via saved profile

```bash
# Save a profile
noti config set --name my-team --provider wecom --param key=<webhook_key>

# Send using profile
noti send --profile my-team --message "Hello from noti!"

# Test the profile
noti config test my-team
```

### 3. Send with direct provider flags

```bash
noti send --provider wecom --param key=<webhook_key> --message "Hello!"
```

### 4. JSON output (for agents)

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

## Next Steps

- [Sending Notifications](/guide/sending-notifications) — learn all the ways to send
- [Configuration & Profiles](/guide/configuration) — manage saved configs
- [Providers Overview](/providers/overview) — browse all 125+ channels
- [AI Agent Integration](/guide/agent-integration) — integrate noti with your agent
