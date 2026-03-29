# Sending Notifications

noti supports three ways to specify the target for your notification.

## Addressing Modes

| Mode | Flag | When to use |
|:-----|:-----|:------------|
| **URL scheme** | `--to <url>` | Quick one-off sends, no saved config needed |
| **Profile** | `--profile <name>` | Repeated use, credentials pre-stored |
| **Direct** | `--provider <name> --param k=v` | Explicit parameter control |

## Via URL Scheme

The simplest way — encode everything in a single URL:

```bash
noti send --to "wecom://<key>" --message "Hello!"
noti send --to "slack://<token_a>/<token_b>/<token_c>" --message "Hello!"
noti send --to "tg://<bot_token>/<chat_id>" --message "Hello!"
noti send --to "discord://<webhook_id>/<webhook_token>" --message "Hello!"
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=addr@example.com" \
  --message "Hello!" --title "Subject"
```

URL schemes follow the pattern: `provider://credentials[?options]`

## Via Saved Profile

For repeated use, save credentials once and send by name:

```bash
# Save
noti config set --name my-team --provider wecom --param key=<webhook_key>

# Send
noti send --profile my-team --message "Hello from noti!"
```

## Via Direct Provider Flags

For full control without URL encoding:

```bash
noti send --provider wecom --param key=<webhook_key> --message "Hello!"
noti send --provider slack --param webhook_url=https://hooks.slack.com/... --message "Hello!"
```

## Message Options

### Title

```bash
noti send --to "..." --message "Body text" --title "Subject Line"
```

### Message Format

All providers support the `--format` flag:

```bash
noti send --to "..." --message "**bold text**" --format markdown
noti send --to "..." --message "<b>bold</b>" --format html
noti send --to "..." --message "plain text" --format text
```

::: tip
Not all providers support all formats — unsupported formats fall back to plain text.
:::

## JSON Output

Add the `--json` flag for structured, machine-readable output:

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

## Discovering Providers

Before sending, discover what providers are available:

```bash
# List all providers
noti providers list

# Inspect a specific provider
noti providers info wecom

# JSON output
noti --json providers list
noti --json providers info wecom
```
