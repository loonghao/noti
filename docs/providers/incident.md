# Incident & Automation Providers

7 providers for incident management and automation platforms.

| Provider | Scheme | Description |
|:---------|:-------|:------------|
| IFTTT | `ifttt://` | IFTTT Maker Webhooks |
| PagerDuty | `pagerduty://` | PagerDuty Events API v2 |
| Opsgenie | `opsgenie://` | Atlassian Opsgenie alerts API v2 |
| PagerTree | `pagertree://` | PagerTree incident management |
| SIGNL4 | `signl4://` | SIGNL4 mobile alerting |
| Splunk On-Call | `victorops://` | Splunk On-Call (VictorOps) incidents |
| Spike.sh | `spike://` | Spike.sh incident management |

## Examples

### IFTTT

```bash
noti send --to "ifttt://<webhook_key>/<event_name>" --message "Hello"
```

### PagerDuty

```bash
noti send --to "pagerduty://<integration_key>" --message "Server down" --title "Critical Alert"
noti send --to "pagerduty://<integration_key>?severity=warning" --message "High CPU"
```

Parameters: `integration_key` (required), `severity` (optional: `critical`, `error`, `warning`, `info`)

### Opsgenie

```bash
noti send --to "opsgenie://<api_key>" --message "Server CPU at 95%" --title "High CPU Alert"
noti send --to "opsgenie://<api_key>?region=eu&priority=P2" --message "Disk space low"
```

Parameters: `api_key` (required), `region`, `priority`, `alias`, `tags` (optional)

### Splunk On-Call (VictorOps)

```bash
noti send --to "victorops://<api_key>/<routing_key>" --message "Service degraded"
```

Parameters: `api_key`, `routing_key` (required), `message_type` (optional: CRITICAL, WARNING, INFO, RECOVERY)
