# noti — Agent Guidelines

> Structured instructions for AI agents interacting with the `noti` CLI.
> Based on [Rewrite Your CLI for AI Agents](https://justin.poehnelt.com/posts/rewrite-your-cli-for-ai-agents/).

## Quick Start

```bash
# Always use --json for machine-readable output
export NOTI_OUTPUT=json

# Discover available providers and their schemas
noti schema
noti schema slack

# Validate before sending (dry-run)
noti send --provider slack --param webhook=https://hooks.slack.com/... --message "test" --dry-run

# Send with raw JSON payload (agent-preferred path)
noti send --provider slack --param webhook=https://hooks.slack.com/... \
  --json-payload '{"text": "Deploy complete", "format": "markdown", "priority": "high"}'
```

## Mandatory Rules

1. **Always use `--json` or set `NOTI_OUTPUT=json`** for all interactions. Human-formatted output is lossy and unpredictable.

2. **Always use `--dry-run` before mutations.** This validates inputs, config, and provider compatibility without calling external APIs. Review the dry-run output before proceeding.

3. **Use `noti schema <provider>` instead of guessing parameters.** Your training data may be stale. The schema command is the canonical source of truth for what each provider accepts.

4. **Use `--fields` to limit response size.** API responses can be large. Protect your context window:
   ```bash
   noti send --provider slack --param webhook=... --message "hi" --json --fields provider,success,message
   ```

5. **Prefer `--json-payload` over individual flags** for the `send` command. This maps directly to the Message schema and avoids flag-parsing ambiguity:
   ```bash
   noti send --provider wecom --param key=xxx \
     --json-payload '{"text": "Alert: disk full", "title": "Disk Alert", "format": "markdown", "priority": "urgent"}'
   ```

6. **Confirm with the user before executing write/send commands.** Never send notifications without explicit user approval.

## Input Constraints (Zero Trust)

The CLI enforces strict input validation:

- **Control characters**: ASCII < 0x20 (except `\n`, `\r`, `\t`) are rejected in all string inputs.
- **Path traversal**: File paths containing `..` are rejected to prevent directory escape.
- **Resource IDs**: The `?` and `#` characters are rejected in profile names and provider identifiers to prevent embedded query parameters.
- **Double encoding**: Do not pre-URL-encode strings. The CLI handles encoding internally.

## Schema Introspection

```bash
# List all providers with required params
noti schema --json

# Full schema for a specific provider
noti schema slack --json
```

The schema output includes:
- `provider`: Provider name
- `scheme`: URL scheme for notification URLs
- `params`: Array of parameter definitions (name, description, required, type, example)
- `send_command`: Pre-built command template with required params

## Output Modes

| Flag / Env | Effect |
|---|---|
| `--json` | Structured JSON to stdout |
| `NOTI_OUTPUT=json` | Same as `--json`, set once |
| `--fields a,b,c` | Filter JSON output to specific top-level fields |

### Error Format (JSON mode)

All errors in JSON mode follow this structure on stderr:
```json
{
  "status": "error",
  "code": 1,
  "message": "descriptive error message"
}
```

### Dry-Run Format (JSON mode)

```json
{
  "status": "dry_run",
  "valid": true,
  "provider": "slack",
  "message_preview": {
    "text": "...",
    "title": null,
    "format": "text",
    "priority": "normal",
    "attachment_count": 0
  },
  "config_keys": ["webhook"]
}
```

## Command Reference

### `noti send`
Send a notification. Supports both human flags and raw JSON payload.

### `noti config set`
Create or update a profile. Use `--dry-run` to validate without saving.

### `noti config test`
Test a saved profile by sending a test message.

### `noti config list`
List all saved profiles.

### `noti schema [provider]`
Introspect provider schemas. Without a provider name, lists all providers with their required params.

### `noti providers list`
List available notification providers.

### `noti providers info <name>`
Show detailed info about a provider.

## Authentication

Use notification URLs or saved profiles. Never hardcode credentials in commands.

```bash
# Save credentials once
noti config set --name my-slack --provider slack --param webhook=https://hooks.slack.com/...

# Use the profile
noti send --profile my-slack --message "Hello"
```

Environment variable `NOTI_CONFIG` can override the config file path for CI/CD environments.
