# CLI Commands

## Global Options

```
noti [OPTIONS] <COMMAND>
```

| Option | Description |
|:-------|:------------|
| `--json` | Output in JSON format for machine parsing. Can also be set via `NOTI_OUTPUT=json` |
| `--fields <FIELDS>` | Limit JSON output to specific fields (comma-separated). Protects agent context windows |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Commands

### `send`

Send a notification.

```bash
noti send [OPTIONS] --message <MESSAGE>
```

| Option | Description |
|:-------|:------------|
| `--to <URL>`, `-t` | Target URL (e.g., `wecom://<key>`) |
| `--profile <NAME>`, `-p` | Use a saved profile |
| `--provider <NAME>` | Provider name (with `--param`) |
| `--param <KEY=VALUE>` | Provider parameter (repeatable) |
| `--message <TEXT>` | **Required** (unless `--json-payload`). Message body |
| `--json-payload <JSON>` | Raw JSON message payload (agent-preferred). Fields: `text`, `title`, `format`, `priority`, `extra` |
| `--title <TEXT>` | Message title/subject |
| `--format <FORMAT>` | Message format: `text`, `markdown`, `html` |
| `--priority <PRIORITY>` | Message priority: `low`, `normal`, `high`, `urgent` (default: `normal`) |
| `--file <PATH>`, `-f` | File attachment (repeatable). Auto-detects MIME type |
| `--timeout <SECONDS>` | Request timeout in seconds (default: 30) |
| `--dry-run` | Validate inputs and config without sending. Use to verify parameters first |

**Examples:**

```bash
noti send --to "wecom://<key>" --message "Hello"
noti send --profile my-team --message "Build passed"
noti send --provider slack --param webhook_url=... --message "Hello"
noti --json send --to "tg://<bot>/<chat>" --message "Alert"
noti send --to "slack://<tokens>" --message "Report" --file report.pdf
noti send --to "discord://<webhook>" --message "Images" -f a.png -f b.png
noti send --provider wecom --param key=xxx --json-payload '{"text":"Deploy done","format":"markdown"}'
noti send --provider slack --param webhook_url=... --message "test" --dry-run
noti --json --fields provider,success send --to "wecom://<key>" --message "Hi"
```

### `schema`

Introspect provider schemas at runtime. AI agents should use this command
instead of guessing parameters from training data.

#### `schema` (all providers)

```bash
noti schema
noti --json schema
```

#### `schema <provider>`

```bash
noti schema slack
noti --json schema slack
```

The schema output includes:

- `provider` — Provider name
- `scheme` — URL scheme for notification URLs
- `description` — Provider description
- `params` — Parameter definitions (name, description, required, type, example)
- `supports_attachments` — Whether the provider supports file attachments
- `send_command` — Pre-built command template with required params

### `config`

Manage notification profiles.

#### `config set`

```bash
noti config set --name <NAME> --provider <PROVIDER> --param <KEY=VALUE>
```

#### `config get`

```bash
noti config get <NAME>
```

#### `config list`

```bash
noti config list
noti --json config list
```

#### `config remove`

```bash
noti config remove <NAME> [--dry-run]
```

| Option | Description |
|:-------|:------------|
| `--dry-run` | Show what would be removed without actually removing |

#### `config test`

```bash
noti config test <NAME>
```

#### `config path`

```bash
noti config path
```

### `providers`

Discover available notification providers.

#### `providers list`

```bash
noti providers list
noti --json providers list
```

#### `providers info`

```bash
noti providers info <PROVIDER>
noti --json providers info <PROVIDER>
```
