# Configuration & Profiles

## Config File

noti stores configuration in a TOML file:

- **Default path:** `~/.config/noti/config.toml`
- **Override:** Set the `NOTI_CONFIG` environment variable

```bash
# Check config file location
noti config path
```

## Profile Management

Profiles let you save, reuse, and share notification configurations.

### Save a profile

```bash
noti config set --name <name> --provider <provider> --param key=value
```

**Examples:**

```bash
# WeCom
noti config set --name team-wecom --provider wecom --param key=<webhook_key>

# Slack
noti config set --name team-slack --provider slack --param webhook_url=https://hooks.slack.com/...

# Email
noti config set --name alerts-email --provider email \
  --param host=smtp.gmail.com \
  --param username=user@gmail.com \
  --param password=app-password \
  --param to=team@company.com
```

### List profiles

```bash
noti config list

# JSON output
noti --json config list
```

### View a profile

```bash
noti config get <name>
```

### Test a profile

```bash
noti config test <name>
```

### Remove a profile

```bash
noti config remove <name>
```

### Send using a profile

```bash
noti send --profile <name> --message "Hello from noti!"
```

## Environment Variables

| Variable | Description |
|:---------|:------------|
| `NOTI_CONFIG` | Override config file path (default: `~/.config/noti/config.toml`) |
| `NOTI_INSTALL_DIR` | Custom install directory for install scripts |
| `NOTI_INSTALL_VERSION` | Pin version for install scripts |
| `NOTI_INSTALL_REPOSITORY` | Override GitHub repo (default: `loonghao/noti`) |
