# Environment Variables

## CLI Runtime

| Variable | Description | Default |
|:---------|:------------|:--------|
| `NOTI_CONFIG` | Override config file path | `~/.config/noti/config.toml` |
| `NOTI_OUTPUT` | Output format: `json` for structured machine-readable output | *(human)* |

## Server (`noti-server`)

| Variable | Description | Default |
|:---------|:------------|:--------|
| `NOTI_HOST` | Bind address | `0.0.0.0` |
| `NOTI_PORT` | Listen port | `3000` |
| `NOTI_API_KEYS` | Comma-separated API keys; empty = auth disabled | *(empty)* |
| `NOTI_AUTH_EXCLUDED_PATHS` | Comma-separated paths that bypass auth | `/health` |
| `NOTI_RATE_LIMIT_MAX` | Max requests per rate-limit window | `100` |
| `NOTI_RATE_LIMIT_WINDOW_SECS` | Rate-limit window in seconds | `60` |
| `NOTI_RATE_LIMIT_PER_IP` | Per-IP rate limiting (`true`/`false`) | `true` |
| `NOTI_WORKER_COUNT` | Number of background queue workers | `4` |
| `NOTI_LOG_LEVEL` | Tracing log level filter | `info` |
| `NOTI_LOG_FORMAT` | Log output format: `text` or `json` | `text` |
| `NOTI_MAX_BODY_SIZE` | Max request body size in bytes | `2097152` (2 MiB) |
| `NOTI_QUEUE_BACKEND` | Queue backend: `memory` or `sqlite` | `memory` |
| `NOTI_QUEUE_DB_PATH` | SQLite database path (when backend=sqlite) | `noti-queue.db` |
| `NOTI_CORS_ALLOWED_ORIGINS` | Comma-separated allowed origins; `*` = permissive | `*` |

## Install Script

These variables are used by the install scripts (`install.sh` / `install.ps1`):

| Variable | Description | Default |
|:---------|:------------|:--------|
| `NOTI_INSTALL_DIR` | Custom install directory | `~/.local/bin` (Linux/macOS), `%USERPROFILE%\.noti\bin` (Windows) |
| `NOTI_INSTALL_VERSION` | Pin a specific version | Latest release |
| `NOTI_INSTALL_REPOSITORY` | Override GitHub repository | `loonghao/noti` |

## Examples

```bash
# Use custom config location
NOTI_CONFIG=/etc/noti/config.toml noti send --profile team --message "Hello"

# Install specific version
NOTI_INSTALL_VERSION=v0.1.5 curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash

# Install to custom directory
NOTI_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash
```
