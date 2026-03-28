# Installation & Usage Reference

## Installation

### Default paths

| Platform | Path |
|----------|------|
| macOS / Linux | `~/.local/bin/noti` |
| Windows | `%USERPROFILE%\.noti\bin\noti.exe` |
| Custom | Set `NOTI_INSTALL_DIR` |

### Option A: Install script (recommended)

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/wecom-bot-cli/main/scripts/install.sh | bash
```

Pin a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/wecom-bot-cli/main/scripts/install.sh | bash -s -- v0.2.0
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/loonghao/wecom-bot-cli/main/scripts/install.ps1 | iex
```

Pin a specific version:

```powershell
.\install.ps1 -Version v0.2.0
```

### Option B: From source

```bash
cargo install --git https://github.com/loonghao/wecom-bot-cli noti-cli
```

### Option C: Download binary

Download the appropriate archive from
[GitHub Releases](https://github.com/loonghao/wecom-bot-cli/releases):

| Platform | Archive |
|----------|---------|
| Linux x64 | `noti-x86_64-unknown-linux-gnu.tar.gz` |
| macOS x64 | `noti-x86_64-apple-darwin.tar.gz` |
| macOS ARM | `noti-aarch64-apple-darwin.tar.gz` |
| Windows x64 | `noti-x86_64-pc-windows-msvc.zip` |

Verify with checksums:

```bash
sha256sum -c noti-checksums.txt
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `NOTI_CONFIG` | Override config file path (default: `~/.config/noti/config.toml`) |
| `NOTI_INSTALL_DIR` | Custom install directory for install scripts |
| `NOTI_INSTALL_VERSION` | Pin version for install scripts |
| `NOTI_INSTALL_REPOSITORY` | Override GitHub repo (default: `loonghao/wecom-bot-cli`) |

## Quick Usage Patterns

### Discover providers

```bash
noti --json providers list
noti --json providers info wecom
```

### Send via URL

```bash
noti --json send --to "wecom://<key>" --message "Hello"
```

### Send via profile

```bash
noti config set --name team --provider wecom --param key=<key>
noti --json send --profile team --message "Hello"
```

### Test a profile

```bash
noti config test team
```

## Development (local source)

```bash
vx just run -- --json providers list
vx just run -- --json send --to "wecom://<key>" --message "test"
```
