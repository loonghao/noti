# CLI Commands

## Global Options

```
noti [OPTIONS] <COMMAND>
```

| Option | Description |
|:-------|:------------|
| `--json` | Output in JSON format for machine parsing |
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
| `--to <URL>` | Target URL (e.g., `wecom://<key>`) |
| `--profile <NAME>` | Use a saved profile |
| `--provider <NAME>` | Provider name (with `--param`) |
| `--param <KEY=VALUE>` | Provider parameter (repeatable) |
| `--message <TEXT>` | **Required.** Message body |
| `--title <TEXT>` | Message title/subject |
| `--format <FORMAT>` | Message format: `text`, `markdown`, `html` |
| `--priority <PRIORITY>` | Message priority: `low`, `normal`, `high`, `urgent` (default: `normal`) |
| `--file <PATH>`, `-f` | File attachment (repeatable). Auto-detects MIME type. |
| `--timeout <SECONDS>` | Request timeout in seconds (default: 30) |

**Examples:**

```bash
noti send --to "wecom://<key>" --message "Hello"
noti send --profile my-team --message "Build passed"
noti send --provider slack --param webhook_url=... --message "Hello"
noti --json send --to "tg://<bot>/<chat>" --message "Alert"
noti send --to "slack://<tokens>" --message "Report" --file report.pdf
noti send --to "discord://<webhook>" --message "Images" -f a.png -f b.png
```

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
noti config remove <NAME>
```

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
