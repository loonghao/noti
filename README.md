<div align="center">

# 📢 noti

**A unified multi-channel notification CLI for AI agents**

*Send notifications to 125+ services from a single command line — built for scripts, agents, and automation.*

[![CI](https://github.com/loonghao/noti/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/noti/actions/workflows/ci.yml)
[![Release](https://github.com/loonghao/noti/actions/workflows/release.yml/badge.svg)](https://github.com/loonghao/noti/actions/workflows/release.yml)
[![Codecov](https://codecov.io/gh/loonghao/noti/graph/badge.svg)](https://codecov.io/gh/loonghao/noti)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.85+-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Crates](https://img.shields.io/badge/crates-3_workspace_crates-blue?logo=rust)](https://github.com/loonghao/noti)
[![GitHub release](https://img.shields.io/github/v/release/loonghao/noti?include_prereleases&logo=github)](https://github.com/loonghao/noti/releases)
[![GitHub Downloads](https://img.shields.io/github/downloads/loonghao/noti/total?logo=github&color=green)](https://github.com/loonghao/noti/releases)

[![Windows](https://img.shields.io/badge/Windows-x86__64-0078D6?logo=windows&logoColor=white)](#installation)
[![macOS](https://img.shields.io/badge/macOS-x86__64%20|%20ARM64-000000?logo=apple&logoColor=white)](#installation)
[![Linux](https://img.shields.io/badge/Linux-x86__64-FCC624?logo=linux&logoColor=black)](#installation)

[English](README.md) · [简体中文](README_zh.md)

</div>

---

## ✨ Highlights

<table>
<tr>
<td width="50%">

### 🚀 CLI-first Design
Built for agent/script integration — one-line commands, no config files needed.

### 📡 125 Built-in Providers
Chat, SMS, email, push, webhooks, incident management, IoT — all in one binary.

### 🔗 URL Scheme Addressing
Intuitive `provider://credentials` format — `wecom://key`, `slack://tokens`, `tg://bot/chat`.

</td>
<td width="50%">

### 📋 Profile Management
Save, reuse, and test notification configs — set up once, use by name.

### 🤖 JSON Output
`--json` flag for structured, machine-readable output — perfect for AI agents.

### ⚡ Blazing Fast
Native Rust binary with instant startup — no runtime, no interpreter overhead.

</td>
</tr>
</table>

## 📦 Installation

### Install script (recommended)

**macOS / Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.ps1 | iex
```

### From source

```bash
cargo install --git https://github.com/loonghao/noti noti-cli
```

### Download binary

Pre-built binaries for all platforms are available on the [GitHub Releases](https://github.com/loonghao/noti/releases) page.

| Platform | Architecture | Download |
|----------|-------------|----------|
| Windows | x86_64 | [`noti-x86_64-pc-windows-msvc.zip`](https://github.com/loonghao/noti/releases/latest) |
| macOS | x86_64 (Intel) | [`noti-x86_64-apple-darwin.tar.gz`](https://github.com/loonghao/noti/releases/latest) |
| macOS | ARM64 (Apple Silicon) | [`noti-aarch64-apple-darwin.tar.gz`](https://github.com/loonghao/noti/releases/latest) |
| Linux | x86_64 | [`noti-x86_64-unknown-linux-gnu.tar.gz`](https://github.com/loonghao/noti/releases/latest) |

## 🚀 Quick Start

### Send via URL scheme

```bash
# WeCom (企业微信)
noti send --to "wecom://<webhook_key>" --message "Hello from noti!"

# Feishu / Lark (飞书)
noti send --to "feishu://<hook_id>" --message "Hello from noti!"

# Slack
noti send --to "slack://<token_a>/<token_b>/<token_c>" --message "Hello!"

# Telegram
noti send --to "tg://<bot_token>/<chat_id>" --message "Hello!"

# Discord
noti send --to "discord://<webhook_id>/<webhook_token>" --message "Hello!"

# Email (SMTP)
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=recipient@example.com" \
  --message "Hello!" --title "Test Email"

# Generic Webhook
noti send --to "webhook://example.com/api/notify" --message "Hello!"
```

### Send via saved profile

```bash
# Save a profile
noti config set --name my-team --provider wecom --param key=<webhook_key>

# Send using profile
noti send --profile my-team --message "Hello from noti!"

# Test the profile
noti config test my-team
```

### Send with direct provider flags

```bash
noti send --provider wecom --param key=<webhook_key> --message "Hello!"
```

### JSON output (for agents)

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

## 📡 Supported Providers (125)

<details>
<summary><strong>💬 Chat & IM (22)</strong></summary>

| Provider | Scheme | Description |
|----------|--------|-------------|
| WeCom | `wecom://` | WeChat Work group bot webhook |
| Feishu | `feishu://` | Feishu / Lark group bot webhook |
| DingTalk | `dingtalk://` | DingTalk group bot webhook |
| Slack | `slack://` | Slack incoming webhooks |
| Telegram | `tg://` | Telegram Bot API |
| Discord | `discord://` | Discord webhooks |
| Teams | `teams://` | Microsoft Teams incoming webhook |
| Google Chat | `gchat://` | Google Chat space webhook |
| Mattermost | `mattermost://` | Mattermost incoming webhook |
| Rocket.Chat | `rocketchat://` | Rocket.Chat incoming webhook |
| Matrix | `matrix://` | Matrix via Client-Server API |
| Zulip | `zulip://` | Zulip team chat via Bot API |
| Webex | `webex://` | Cisco Webex Teams messaging |
| LINE | `line://` | LINE Notify push |
| Revolt | `revolt://` | Revolt open-source chat |
| Mastodon | `mastodon://` | Mastodon status post (toot) |
| Ryver | `ryver://` | Ryver team messaging |
| Twist | `twist://` | Twist async team messaging |
| Flock | `flock://` | Flock team messaging |
| Gitter | `gitter://` | Gitter developer chat |
| Guilded | `guilded://` | Guilded gaming chat webhooks |
| Misskey | `misskey://` | Misskey fediverse note posting |

</details>

<details>
<summary><strong>🔔 Push Notifications (20)</strong></summary>

| Provider | Scheme | Description |
|----------|--------|-------------|
| Pushover | `pushover://` | Pushover push notifications |
| ntfy | `ntfy://` | ntfy.sh push notifications |
| Gotify | `gotify://` | Gotify self-hosted push |
| Bark | `bark://` | Bark iOS push notifications |
| PushDeer | `pushdeer://` | PushDeer cross-platform push |
| ServerChan | `serverchan://` | ServerChan (Server酱) push |
| PushBullet | `pushbullet://` | PushBullet cross-platform push |
| SimplePush | `simplepush://` | SimplePush.io push notifications |
| Notica | `notica://` | Notica browser push notifications |
| Prowl | `prowl://` | Prowl iOS push notifications |
| Join | `join://` | Join by joaoapps push notifications |
| Pushsafer | `pushsafer://` | Pushsafer push notifications |
| OneSignal | `onesignal://` | OneSignal push notification platform |
| Techulus Push | `push://` | Techulus Push notifications |
| Pushy | `pushy://` | Pushy cross-platform push notifications |
| Chanify | `chanify://` | Chanify iOS/Android push notifications |
| Pushplus | `pushplus://` | Pushplus push (WeChat/SMS/Email) |
| WxPusher | `wxpusher://` | WxPusher WeChat push notifications |
| FCM | `fcm://` | Firebase Cloud Messaging push |
| Pushjet | `pushjet://` | Pushjet push notification service |

</details>

<details>
<summary><strong>📱 SMS & Messaging (17)</strong></summary>

| Provider | Scheme | Description |
|----------|--------|-------------|
| Twilio | `twilio://` | Twilio SMS via REST API |
| Vonage | `vonage://` | Vonage (Nexmo) SMS API |
| D7 Networks | `d7sms://` | D7 Networks SMS gateway |
| Sinch | `sinch://` | Sinch SMS via REST API |
| Clickatell | `clickatell://` | Clickatell SMS/messaging gateway |
| BulkSMS | `bulksms://` | BulkSMS gateway via REST API v2 |
| Kavenegar | `kavenegar://` | Kavenegar SMS gateway (Iran) |
| MessageBird | `msgbird://` | MessageBird SMS via REST API |
| Plivo | `plivo://` | Plivo SMS via REST API |
| BurstSMS | `burstsms://` | BurstSMS (Transmit SMS) gateway |
| PopcornNotify | `popcorn://` | PopcornNotify SMS messaging |
| ClickSend | `clicksend://` | ClickSend SMS messaging gateway |
| Seven | `seven://` | Seven (sms77) SMS gateway |
| SMSEagle | `smseagle://` | SMSEagle hardware SMS gateway |
| httpSMS | `httpsms://` | httpSMS — send SMS via Android phone |
| MSG91 | `msg91://` | MSG91 SMS gateway (India) |
| Free Mobile | `freemobile://` | Free Mobile SMS (France) |

</details>

<details>
<summary><strong>📧 Email (8)</strong></summary>

| Provider | Scheme | Description |
|----------|--------|-------------|
| Email | `smtp://` | Email via SMTP |
| Mailgun | `mailgun://` | Mailgun transactional email API |
| SendGrid | `sendgrid://` | SendGrid transactional email API v3 |
| SparkPost | `sparkpost://` | SparkPost transactional email API |
| AWS SES | `ses://` | AWS SES transactional email |
| Resend | `resend://` | Resend modern email API |
| Brevo | `brevo://` | Brevo (Sendinblue) transactional email |
| SMTP2Go | `smtp2go://` | SMTP2Go transactional email |

</details>

<details>
<summary><strong>🌐 Webhooks (4)</strong></summary>

| Provider | Scheme | Description |
|----------|--------|-------------|
| Webhook | `webhook://` | Generic HTTP webhook |
| JSON Webhook | `json://` | Generic JSON webhook |
| Form Webhook | `form://` | Generic form webhook |
| XML Webhook | `xml://` | Generic XML webhook |

</details>

<details>
<summary><strong>🚨 Incident & Automation (7)</strong></summary>

| Provider | Scheme | Description |
|----------|--------|-------------|
| IFTTT | `ifttt://` | IFTTT Maker Webhooks |
| PagerDuty | `pagerduty://` | PagerDuty Events API v2 |
| Opsgenie | `opsgenie://` | Atlassian Opsgenie alerts API v2 |
| PagerTree | `pagertree://` | PagerTree incident management |
| SIGNL4 | `signl4://` | SIGNL4 mobile alerting |
| Splunk On-Call | `victorops://` | Splunk On-Call (VictorOps) incidents |
| Spike.sh | `spike://` | Spike.sh incident management |

</details>

<details>
<summary><strong>🏠 IoT, Media & More (47)</strong></summary>

| Provider | Scheme | Description |
|----------|--------|-------------|
| Bluesky | `bluesky://` | Bluesky social network (AT Protocol) |
| Home Assistant | `hassio://` | Home Assistant notifications |
| LaMetric | `lametric://` | LaMetric Time smart clock |
| LunaSea | `lunasea://` | LunaSea self-hosted media push |
| Nextcloud | `ncloud://` | Nextcloud push notifications |
| Signal | `signal://` | Signal Messenger via signal-cli REST API |
| Reddit | `reddit://` | Reddit private messages |
| Threema | `threema://` | Threema Gateway secure messaging |
| Apprise API | `apprise://` | Apprise API notification relay |
| Web Push | `webpush://` | Web Push (VAPID) browser notifications |
| WhatsApp | `whatsapp://` | WhatsApp Business Cloud API messaging |
| Kodi | `kodi://` | Kodi (XBMC) GUI notifications via JSON-RPC |
| Notifico | `notifico://` | Notifico self-hosted notification service |
| 46elks | `46elks://` | 46elks SMS messaging via REST API |
| BulkVS | `bulkvs://` | BulkVS SMS messaging via REST API |
| Jira | `jira://` | Jira issue comment via REST API v3 |
| PushMe | `pushme://` | PushMe push notification service |
| SendPulse | `sendpulse://` | SendPulse transactional email via SMTP API |
| Streamlabs | `streamlabs://` | Streamlabs stream alerts |
| Synology Chat | `synology://` | Synology Chat incoming webhook |
| Africa's Talking | `africastalking://` | Africa's Talking SMS gateway |
| Office 365 | `o365://` | Office 365 / Outlook email via Microsoft Graph API |
| Nextcloud Talk | `nctalk://` | Nextcloud Talk chat messaging via OCS API |
| Emby | `emby://` | Emby media server notifications via REST API |
| Jellyfin | `jellyfin://` | Jellyfin media server notifications via REST API |
| Pushcut | `pushcut://` | Pushcut iOS automation notifications |
| MQTT | `mqtt://` | MQTT publish via broker HTTP API (EMQX, HiveMQ) |
| VoIP.ms | `voipms://` | VoIP.ms SMS messaging via REST API |
| SFR | `sfr://` | SFR free SMS notification (French carrier) |
| Pushed | `pushed://` | Pushed.co push notifications via REST API |
| Growl | `growl://` | Growl desktop notifications via GNTP/HTTP |
| Kumulos | `kumulos://` | Kumulos push notifications via Push API |
| Parse | `parse://` | Parse Platform push notifications via REST API |
| Remote Syslog | `rsyslog://` | Remote Syslog notifications via HTTP relay |
| SMS Manager | `smsmanager://` | SMS Manager bulk SMS messaging |
| X (Twitter) | `twitter://` | X (Twitter) post tweets or DMs via API v2 |
| Boxcar | `boxcar://` | Boxcar push notifications for iOS/Android |
| DAPNET | `dapnet://` | DAPNET ham radio paging network |
| Enigma2 | `enigma2://` | Enigma2 satellite receiver on-screen notifications via OpenWebif |
| Notifiarr | `notifiarr://` | Notifiarr media server notification aggregation |
| Statuspage | `statuspage://` | Atlassian Statuspage.io incident management |
| Dot. | `dot://` | Dot. IoT e-ink display notifications |
| Fluxer | `fluxer://` | Fluxer webhook notifications (Discord-style) |
| Workflows | `workflows://` | Microsoft Power Automate / Workflows (Adaptive Cards) |
| NotificationAPI | `napi://` | NotificationAPI multi-channel notifications |
| SpugPush | `spugpush://` | SpugPush webhook notifications (Spug monitoring) |
| AWS SNS | `sns://` | AWS SNS topic publishing |

</details>

### List providers

```bash
noti providers list          # List all available providers
noti providers info wecom    # Show provider details and parameters
```

## ⚙️ Configuration

Config file location: `~/.config/noti/config.toml`

Override with environment variable: `NOTI_CONFIG=/path/to/config.toml`

### Profile management

```bash
noti config set --name <name> --provider <provider> --param key=value
noti config get <name>
noti config list
noti config remove <name>
noti config test <name>
noti config path
```

## 📊 Exit Codes

| Code | Meaning |
|------|---------|
| `0` | ✅ Success |
| `1` | ❌ Send failure (network/API error) |
| `2` | ⚠️ Parameter/configuration error |

## 🤖 Agent Integration

noti is designed to be consumed by AI agents like [OpenClaw](https://github.com/nicepkg/openclaw). Key design principles:

| Feature | Benefit |
|---------|---------|
| **URL scheme** | One-line addressing — no config files needed |
| **`--json` flag** | Structured output for reliable parsing |
| **Exit codes** | Deterministic success/failure signals |
| **Profile system** | Pre-configure once, use by name |

### Example: OpenClaw integration

```bash
# Agent discovers available providers
noti --json providers list

# Agent inspects provider parameters
noti --json providers info wecom

# Agent sends notification
noti --json send --to "wecom://key123" --message "Task completed"
```

## 🏗️ Architecture

```
noti/
├── crates/
│   ├── noti-cli/        # CLI binary — argument parsing, output formatting
│   ├── noti-core/       # Core abstractions — Provider trait, Registry, URL parsing
│   └── noti-providers/  # 125 provider implementations
├── scripts/             # Install scripts (bash, PowerShell) & utilities
├── skills/              # OpenClaw skill definitions
└── justfile             # Task runner recipes
```

## 🛠️ Development

### Prerequisites

- [vx](https://github.com/loonghao/vx) — tool version manager
- Rust 1.85+ (stable, managed by vx)

### Commands

```bash
vx just fmt          # Format code
vx just check        # Type check
vx just lint         # Clippy lint
vx just test         # Run tests
vx just ci           # Full CI pipeline (fmt + check + lint + test)
vx just run -- send --help   # Run CLI in dev mode
```

## 🙏 Acknowledgements

noti is inspired by these excellent projects:

- [notifiers](https://github.com/liiight/notifiers) — The Python notification library that started it all
- [Apprise](https://github.com/caronc/apprise) — Push notifications with a unified API
- [OpenClaw](https://github.com/nicepkg/openclaw) — The AI agent framework noti was built for

## 📄 License

MIT © [Hal Long](https://github.com/loonghao)
