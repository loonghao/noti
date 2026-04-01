# AI Agent Integration

noti is designed to be consumed by AI agents like [OpenClaw](https://github.com/nicepkg/openclaw). Its CLI-first design makes it a natural fit for agentic workflows.

## Design Principles

| Feature | Benefit |
|:--------|:--------|
| **URL scheme** | One-line addressing — no config files needed |
| **`--json` flag** | Structured output for reliable parsing |
| **Exit codes** | Deterministic success/failure signals |
| **Profile system** | Pre-configure once, use by name |
| **`providers list`** | Self-discovery — agent can enumerate channels |

## Typical Agent Workflow

```bash
# 1. Agent discovers available providers
noti --json providers list

# 2. Agent inspects provider parameters
noti --json providers info wecom

# 3. Agent sends notification
noti --json send --to "wecom://key123" --message "Task completed"

# 4. Agent checks result
echo $?  # 0 = success, 1 = failure, 2 = config error
```

## Exit Codes

| Code | Meaning | Use Case |
|:-----|:--------|:---------|
| `0` | Success | Message sent successfully |
| `1` | Send failure | Network error, API rejection |
| `2` | Config error | Missing parameters, invalid profile |

## JSON Output Format

All commands support `--json` for structured output:

### Send result

```json
{
  "success": true,
  "provider": "wecom",
  "status_code": 200,
  "message": "message sent successfully",
  "raw_response": { "errcode": 0, "errmsg": "ok" }
}
```

### Provider list

```bash
noti --json providers list
```

### Provider info

```bash
noti --json providers info wecom
```

## OpenClaw Skill

noti ships with a built-in [OpenClaw](https://github.com/nicepkg/openclaw) skill definition in `skills/noti-openclaw/`. Agents can auto-discover noti's capabilities through:

- **`SKILL.md`** — Skill metadata, activation conditions, standard workflow
- **`references/install-and-usage.md`** — Installation and usage patterns
- **`references/provider-guide.md`** — Complete provider parameters and URL schemes

## Best Practices for Agents

1. **Always use `--json`** for reliable output parsing
2. **Discover before sending** — use `providers list` and `providers info`
3. **Check exit codes** — `0` for success, non-zero for failure
4. **Use profiles** for repeated sends to avoid credential exposure in command lines
5. **Use URL schemes** for one-off dynamic sends
