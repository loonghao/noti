# Exit Codes

noti uses deterministic exit codes for reliable automation and agent integration.

| Code | Meaning | Use Case |
|:-----|:--------|:---------|
| `0` | Success | Message sent successfully |
| `1` | Send failure | Network error, API rejection |
| `2` | Config error | Missing parameters, invalid profile |

## Usage in Scripts

```bash
noti send --to "wecom://<key>" --message "Hello"
if [ $? -eq 0 ]; then
  echo "Sent successfully"
elif [ $? -eq 1 ]; then
  echo "Send failed — check network or API credentials"
elif [ $? -eq 2 ]; then
  echo "Configuration error — check parameters"
fi
```

## Usage with JSON

Combine exit codes with `--json` for full programmatic control:

```bash
result=$(noti --json send --to "wecom://<key>" --message "Hello")
exit_code=$?

if [ $exit_code -eq 0 ]; then
  echo "$result" | jq '.raw_response'
else
  echo "Failed with exit code: $exit_code"
  echo "$result" | jq '.message'
fi
```
