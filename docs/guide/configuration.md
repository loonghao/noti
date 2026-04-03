# Configuration & Profiles

noti uses a two-layer configuration model: **CLI profiles** for saved notification targets and **server environment variables** for runtime behavior. This guide covers both.

## Quick Start

```bash
# Check your config file location
noti config path

# Save a Slack profile
noti config set --name my-slack --provider slack --param webhook_url=https://hooks.slack.com/...

# Validate before saving (dry-run)
noti config set --name my-slack --provider slack --param webhook_url=https://hooks.slack.com/... --dry-run

# Send using the profile
noti send --profile my-slack --message "Hello from noti!"

# List all profiles
noti config list --json
```

## Config File

### Location

noti stores CLI configuration in a TOML file:

| Priority | Path |
|:---------|:-----|
| 1 (highest) | `NOTI_CONFIG` environment variable |
| 2 (default) | `~/.config/noti/config.toml` |

```bash
# Check the resolved path
noti config path

# JSON output
noti --json config path
# {"path": "/home/user/.config/noti/config.toml"}
```

### File Format

The config file uses [TOML](https://toml.io/) format with a `[profiles.<name>]` table per profile:

```toml
[profiles.team-slack]
provider = "slack"
webhook_url = "https://hooks.slack.com/services/T00/B00/xxxx"

[profiles.alerts-email]
provider = "email"
host = "smtp.gmail.com"
port = "587"
username = "alerts@company.com"
password = "app-password"
to = "oncall@company.com"

[profiles.team-wecom]
provider = "wecom"
key = "your-webhook-key"

[profiles.ops-dingtalk]
provider = "dingtalk"
access_token = "your-access-token"
```

Each profile stores:
- `provider` — the notification provider name (required)
- All other keys — provider-specific configuration values (flattened key-value pairs)

### Custom Config Path

Override the config file location for CI/CD, containers, or multi-environment setups:

```bash
# Linux/macOS
export NOTI_CONFIG=/etc/noti/config.toml
noti send --profile team --message "Deploy complete"

# Windows
$env:NOTI_CONFIG = "C:\noti\config.toml"
noti send --profile team --message "Deploy complete"
```

## Profile Management

Profiles let you save, reuse, and share notification configurations without repeating credentials.

### Create or Update a Profile

```bash
noti config set --name <name> --provider <provider> --param key=value [--param key2=value2 ...]
```

**Examples:**

```bash
# WeCom
noti config set --name team-wecom --provider wecom --param key=<webhook_key>

# Slack
noti config set --name team-slack --provider slack --param webhook_url=https://hooks.slack.com/...

# Email (SMTP)
noti config set --name alerts-email --provider email \
  --param host=smtp.gmail.com \
  --param port=587 \
  --param username=user@gmail.com \
  --param password=app-password \
  --param to=team@company.com

# DingTalk
noti config set --name ops-dingtalk --provider dingtalk \
  --param access_token=your-token

# Feishu/Lark
noti config set --name team-feishu --provider feishu \
  --param webhook_url=https://open.feishu.cn/open-apis/bot/v2/hook/xxxx
```

### Validate Before Saving (Dry-Run)

Use `--dry-run` to validate provider and parameters without writing to disk:

```bash
noti config set --name test --provider slack --param webhook_url=https://hooks.slack.com/... --dry-run
# ✓ dry-run: profile 'test' is valid (not saved)
```

JSON output:

```bash
noti --json config set --name test --provider slack --param webhook_url=https://hooks.slack.com/... --dry-run
```

```json
{
  "status": "dry_run",
  "valid": true,
  "profile": "test",
  "provider": "slack",
  "config_keys": ["webhook_url"]
}
```

### List Profiles

```bash
noti config list

# JSON output
noti --json config list
```

```json
[
  {"name": "team-slack", "provider": "slack"},
  {"name": "alerts-email", "provider": "email"},
  {"name": "team-wecom", "provider": "wecom"}
]
```

### View a Profile

```bash
noti config get <name>

# JSON output
noti --json config get my-slack
```

### Test a Profile

Send a test notification to verify the profile is correctly configured:

```bash
noti config test <name>
# ✓ profile 'my-slack' is working (slack)
```

This sends a built-in test message (`🔔 noti test message — if you see this, the profile is working!`) through the configured provider.

### Remove a Profile

```bash
noti config remove <name>

# Preview removal (dry-run)
noti config remove my-old-profile --dry-run
```

### Send Using a Profile

```bash
# Simple text
noti send --profile my-slack --message "Deployment complete"

# Markdown with title
noti send --profile my-slack --message "**v2.1.0** deployed to production" --title "Deploy Alert" --format markdown

# With priority
noti send --profile my-slack --message "Disk usage at 95%!" --priority urgent

# With file attachment
noti send --profile my-slack --message "Build report" --file ./report.pdf

# Agent-friendly JSON payload
noti send --profile my-slack --json-payload '{"text": "Deploy v2.1.0", "format": "markdown", "priority": "high"}'
```

### Alternative Target Methods

Besides profiles, you can specify targets directly:

```bash
# URL scheme (inline credentials)
noti send --to "slack://hooks.slack.com/services/T00/B00/xxxx" --message "Hello"

# Direct provider + params
noti send --provider slack --param webhook_url=https://hooks.slack.com/... --message "Hello"
```

Target resolution priority: `--to` (URL) > `--profile` > `--provider` + `--param`.

## Server Configuration

The noti-server is configured entirely through environment variables. All variables use the `NOTI_` prefix.

### Complete Environment Variable Reference

#### Network

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_HOST` | `0.0.0.0` | Bind address (IPv4 or IPv6) |
| `NOTI_PORT` | `3000` | HTTP listen port |

#### Authentication

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_API_KEYS` | *(empty)* | Comma-separated API keys; empty = auth disabled |
| `NOTI_AUTH_EXCLUDED_PATHS` | `/health` | Comma-separated paths that bypass auth |

See [Authentication](/guide/authentication) for detailed configuration.

#### Rate Limiting

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_RATE_LIMIT_MAX` | `100` | Maximum requests per window |
| `NOTI_RATE_LIMIT_WINDOW_SECS` | `60` | Window duration in seconds |
| `NOTI_RATE_LIMIT_PER_IP` | `true` | Per-IP rate limiting (`true`/`false`) |

See [Rate Limiting](/guide/rate-limiting) for detailed configuration.

#### Queue

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_QUEUE_BACKEND` | `memory` | Backend type: `memory` or `sqlite` |
| `NOTI_QUEUE_DB_PATH` | `noti-queue.db` | SQLite database path (when backend=sqlite) |
| `NOTI_WORKER_COUNT` | `4` | Number of background worker threads |

See [Queue Management](/guide/queue-management) for backend comparison and tuning.

#### Logging

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_LOG_LEVEL` | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `NOTI_LOG_FORMAT` | `text` | Output format: `text` (human) or `json` (structured) |

See [Logging & Observability](/guide/logging) for production configuration.

#### Security & Limits

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_MAX_BODY_SIZE` | `2097152` (2 MiB) | Maximum request body size in bytes |
| `NOTI_CORS_ALLOWED_ORIGINS` | `*` | Comma-separated allowed origins; `*` = permissive |

### Configuration Precedence

The server uses a single source of truth: **environment variables**. There is no config file for the server. This makes deployment straightforward in containers and CI/CD.

```
Environment variable → ServerConfig::from_env() → Server startup
```

If a variable is unset or unparseable, the server falls back to the default value documented above. No errors are raised for unset optional variables.

### Example: Production Configuration

```bash
# Network
export NOTI_HOST=0.0.0.0
export NOTI_PORT=8080

# Authentication (required in production)
export NOTI_API_KEYS="key-prod-abc123,key-prod-def456"
export NOTI_AUTH_EXCLUDED_PATHS="/health,/api-docs/openapi.json"

# Rate limiting
export NOTI_RATE_LIMIT_MAX=200
export NOTI_RATE_LIMIT_WINDOW_SECS=60
export NOTI_RATE_LIMIT_PER_IP=true

# Queue (persistent)
export NOTI_QUEUE_BACKEND=sqlite
export NOTI_QUEUE_DB_PATH=/data/noti-queue.db
export NOTI_WORKER_COUNT=8

# Logging (structured for log aggregation)
export NOTI_LOG_LEVEL=info
export NOTI_LOG_FORMAT=json

# Body size (4 MiB for attachments)
export NOTI_MAX_BODY_SIZE=4194304

# CORS (restricted)
export NOTI_CORS_ALLOWED_ORIGINS="https://dashboard.example.com,https://admin.example.com"
```

### Example: Development Configuration

```bash
export NOTI_HOST=127.0.0.1
export NOTI_PORT=3000
export NOTI_API_KEYS=""                    # Auth disabled
export NOTI_QUEUE_BACKEND=memory           # Fast, no persistence needed
export NOTI_WORKER_COUNT=2
export NOTI_LOG_LEVEL=debug
export NOTI_LOG_FORMAT=text
export NOTI_CORS_ALLOWED_ORIGINS="*"       # Allow all origins
```

## Docker Configuration

### Dockerfile Defaults

The official Docker image ships with production-ready defaults:

```dockerfile
ENV NOTI_HOST=0.0.0.0 \
    NOTI_PORT=3000 \
    NOTI_LOG_FORMAT=json \
    NOTI_QUEUE_BACKEND=sqlite \
    NOTI_QUEUE_DB_PATH=/data/noti-queue.db
```

### Docker Compose

Override any variable in `docker-compose.yml`:

```yaml
services:
  noti-server:
    image: ghcr.io/loonghao/noti-server:latest
    ports:
      - "${NOTI_PORT:-3000}:3000"
    volumes:
      - noti-data:/data
    environment:
      NOTI_HOST: "0.0.0.0"
      NOTI_PORT: "3000"
      NOTI_LOG_FORMAT: "json"
      NOTI_LOG_LEVEL: "${NOTI_LOG_LEVEL:-info}"
      NOTI_QUEUE_BACKEND: "sqlite"
      NOTI_QUEUE_DB_PATH: "/data/noti-queue.db"
      NOTI_WORKER_COUNT: "${NOTI_WORKER_COUNT:-4}"
      NOTI_API_KEYS: "${NOTI_API_KEYS:-}"
      NOTI_RATE_LIMIT_MAX: "${NOTI_RATE_LIMIT_MAX:-100}"
      NOTI_RATE_LIMIT_PER_IP: "${NOTI_RATE_LIMIT_PER_IP:-true}"
      NOTI_CORS_ALLOWED_ORIGINS: "${NOTI_CORS_ALLOWED_ORIGINS:-*}"
      NOTI_MAX_BODY_SIZE: "${NOTI_MAX_BODY_SIZE:-2097152}"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  noti-data:
```

Use a `.env` file alongside `docker-compose.yml` to set secrets:

```bash
# .env
NOTI_API_KEYS=key-prod-abc123,key-prod-def456
NOTI_LOG_LEVEL=info
```

### Kubernetes

Use `ConfigMap` for non-sensitive values and `Secret` for API keys:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: noti-config
data:
  NOTI_HOST: "0.0.0.0"
  NOTI_PORT: "3000"
  NOTI_LOG_FORMAT: "json"
  NOTI_LOG_LEVEL: "info"
  NOTI_QUEUE_BACKEND: "sqlite"
  NOTI_QUEUE_DB_PATH: "/data/noti-queue.db"
  NOTI_WORKER_COUNT: "8"
  NOTI_RATE_LIMIT_MAX: "200"
  NOTI_RATE_LIMIT_PER_IP: "true"
  NOTI_CORS_ALLOWED_ORIGINS: "https://dashboard.example.com"
  NOTI_MAX_BODY_SIZE: "4194304"
---
apiVersion: v1
kind: Secret
metadata:
  name: noti-secrets
type: Opaque
stringData:
  NOTI_API_KEYS: "key-prod-abc123,key-prod-def456"
```

Reference in your Deployment:

```yaml
spec:
  containers:
    - name: noti
      image: ghcr.io/loonghao/noti:latest
      envFrom:
        - configMapRef:
            name: noti-config
        - secretRef:
            name: noti-secrets
```

## Install Script Configuration

These variables are used only by the install scripts (`install.sh` / `install.ps1`):

| Variable | Default | Description |
|:---------|:--------|:------------|
| `NOTI_INSTALL_DIR` | `~/.local/bin` (Linux/macOS), `%USERPROFILE%\.noti\bin` (Windows) | Custom install directory |
| `NOTI_INSTALL_VERSION` | Latest release | Pin a specific version |
| `NOTI_INSTALL_REPOSITORY` | `loonghao/noti` | Override GitHub repository |

```bash
# Install specific version
NOTI_INSTALL_VERSION=v0.1.5 curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash

# Install to custom directory
NOTI_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Install noti
  run: curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash

- name: Setup noti profile
  env:
    NOTI_CONFIG: ${{ runner.temp }}/noti-config.toml
  run: |
    noti config set --name ci-slack --provider slack \
      --param webhook_url=${{ secrets.SLACK_WEBHOOK }}

- name: Notify on deploy
  env:
    NOTI_CONFIG: ${{ runner.temp }}/noti-config.toml
    NOTI_OUTPUT: json
  run: |
    noti send --profile ci-slack \
      --message "Deploy ${{ github.sha }} complete" \
      --title "CI/CD" \
      --format markdown
```

### GitLab CI

```yaml
notify:
  script:
    - export NOTI_CONFIG=/tmp/noti.toml
    - noti config set --name ci --provider wecom --param key=$WECOM_KEY
    - noti send --profile ci --message "Pipeline $CI_PIPELINE_ID completed"
```

### Jenkins

```groovy
withEnv(["NOTI_CONFIG=${WORKSPACE}/.noti/config.toml"]) {
    sh 'noti config set --name ci --provider slack --param webhook_url=${SLACK_WEBHOOK}'
    sh 'noti send --profile ci --message "Build #${BUILD_NUMBER} complete"'
}
```

## Security Best Practices

### API Key Management

1. **Generate strong keys**: Use 32+ character random strings
   ```bash
   openssl rand -hex 32
   ```

2. **Rotate keys regularly**: Add new keys to `NOTI_API_KEYS` before removing old ones (supports multiple comma-separated keys for zero-downtime rotation)

3. **Never commit keys**: Use environment variables, secrets managers, or CI/CD variables
   ```bash
   # Good: environment variable
   export NOTI_API_KEYS="$(vault kv get -field=api_keys secret/noti)"

   # Bad: hardcoded in config
   # NOTI_API_KEYS=my-secret-key  # DON'T DO THIS
   ```

### Profile Security

- The config file (`~/.config/noti/config.toml`) stores provider credentials in **plaintext**
- Set appropriate file permissions: `chmod 600 ~/.config/noti/config.toml`
- In CI/CD, use temporary config paths and clean up after use
- Never commit the config file to version control; add it to `.gitignore`:
  ```gitignore
  config.toml
  .noti/
  ```

### CORS Configuration

- In production, always restrict `NOTI_CORS_ALLOWED_ORIGINS` to known origins
- The default `*` (permissive) is suitable for development only
- Invalid origin strings are dropped during parsing with a `WARN`-level log entry for each invalid value
- If **all** configured origins are invalid, a warning is emitted indicating no origin will be allowed

## Output Mode

Control CLI output format globally:

| Method | Example |
|:-------|:--------|
| CLI flag | `noti --json config list` |
| Environment variable | `NOTI_OUTPUT=json noti config list` |

Priority: `--json` flag > `NOTI_OUTPUT` env var > human-readable (default).

For AI agents, set `NOTI_OUTPUT=json` once to avoid passing `--json` on every command:

```bash
export NOTI_OUTPUT=json
noti config list          # automatically outputs JSON
noti send --profile my-slack --message "test"  # JSON response
```

Use `--fields` to limit output and protect agent context windows:

```bash
noti send --profile my-slack --message "test" --json --fields provider,success,message
```

## Troubleshooting

| Issue | Cause | Fix |
|:------|:------|:----|
| `could not determine config directory` | Home directory not set | Set `HOME` (Linux/macOS) or `USERPROFILE` (Windows) |
| `profile not found: <name>` | Profile doesn't exist or wrong config file | Check `noti config list` and `noti config path` |
| `unknown provider: <name>` | Typo in provider name | Run `noti providers list` to see available providers |
| `failed to parse config.toml` | Invalid TOML syntax | Validate with an online TOML checker |
| Server ignores env var | Unparseable value (e.g. `NOTI_PORT=abc`) | Server falls back to default silently; check startup logs |
| Config file not created | Parent directory doesn't exist | `noti config set` creates directories automatically |

## AI Agent Tips

::: tip For AI Agents
- **Always set `NOTI_OUTPUT=json`** for structured output
- **Use `noti schema <provider>`** to discover required parameters — don't guess from documentation
- **Use `--dry-run`** before `config set` and `send` to validate without side effects
- **Use `--fields`** to limit response size and protect your context window
- **Prefer `--json-payload`** over individual flags for the `send` command
- **Use `noti config path --json`** to find the config file location
- **Use `noti providers list --json`** to enumerate available providers
:::
