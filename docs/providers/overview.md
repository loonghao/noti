# Providers Overview

noti supports **125+ notification providers** across 7 categories. Every provider uses a consistent URL scheme format and can be discovered via the CLI.

## Discover Providers

```bash
# List all available providers
noti providers list

# Show details for a specific provider
noti providers info wecom

# JSON output
noti --json providers list
noti --json providers info wecom
```

## Categories

| Category | Count | Description |
|:---------|:------|:------------|
| [Chat & IM](/providers/chat) | 22 | WeCom, Feishu, Slack, Telegram, Discord, Teams, and more |
| [Push Notifications](/providers/push) | 20 | Pushover, ntfy, Gotify, Bark, PushDeer, and more |
| [SMS & Messaging](/providers/sms) | 17 | Twilio, Vonage, Sinch, Clickatell, and more |
| [Email](/providers/email) | 8 | SMTP, Mailgun, SendGrid, Resend, and more |
| [Webhooks](/providers/webhooks) | 4 | Generic HTTP, JSON, Form, XML webhooks |
| [Incident & Automation](/providers/incident) | 7 | IFTTT, PagerDuty, Opsgenie, and more |
| [IoT, Media & More](/providers/iot-media) | 47 | Home Assistant, Bluesky, WhatsApp, and more |

## URL Scheme Pattern

All providers follow the same addressing pattern:

```
provider://credentials[?options]
```

**Examples:**

```bash
wecom://<key>
slack://<token_a>/<token_b>/<token_c>
tg://<bot_token>/<chat_id>
smtp://user:pass@host:port?to=addr
```

See [URL Schemes Reference](/reference/url-schemes) for the complete list.
