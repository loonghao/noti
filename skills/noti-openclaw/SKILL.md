# noti-openclaw

> OpenClaw skill for **noti** — unified multi-channel notification CLI.

## When to Use

Activate this skill whenever the user wants to:

- Send a notification (chat message, email, webhook, push notification)
- Configure or manage notification profiles
- Discover available notification providers
- Test a notification channel
- Install or update the `noti` CLI

## Repository Contract

1. **Prefer CLI commands** over ad-hoc API calls.
2. **Prefer JSON output** (`--json` flag) for machine parsing.
3. **Prefer capability discovery** (`noti providers list`) before composing new calls.
4. **Prefer profiles** for repeated use — configure once, send by name.

## Standard Workflow

### 1 · Choose Execution Mode

| Mode | How |
|---|---|
| **Released binary** | Read `references/install-and-usage.md`; prefer checksummed release archives. |
| **Local development** | `vx cargo run -p noti-cli -- …` or `vx just run -- …` |

### 2 · Discover Providers

Before sending, discover what providers are available and what parameters they need:

```bash
# List all providers
noti --json providers list

# Inspect a specific provider
noti --json providers info <provider_name>
```

### 3 · Choose the Right Addressing Mode

noti supports three ways to specify the target:

| Mode | Flag | When to use |
|---|---|---|
| **URL scheme** | `--to <url>` | Quick one-off sends, no saved config needed |
| **Profile** | `--profile <name>` | Repeated use, credentials pre-stored |
| **Direct** | `--provider <name> --param k=v` | Explicit parameter control |

### 4 · Send Notifications

```bash
# Via URL scheme (simplest)
noti --json send --to "wecom://<key>" --message "Task completed"

# Via saved profile
noti --json send --profile my-team --message "Build passed ✓"

# Via direct provider
noti --json send --provider slack --param webhook_url=https://... --message "Hello"
```

### 5 · Manage Profiles

```bash
# Save a profile
noti config set --name <name> --provider <provider> --param key=value

# List all profiles
noti --json config list

# Test a profile
noti config test <name>

# Remove a profile
noti config remove <name>
```

### 6 · Interpret Results

noti always returns structured output when `--json` is used:

```json
{
  "success": true,
  "provider": "wecom",
  "status_code": 200,
  "message": "message sent successfully",
  "raw_response": { "errcode": 0, "errmsg": "ok" }
}
```

**Exit codes:**

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Send failure (network/API error) |
| 2 | Parameter/configuration error |

### 7 · Debug in Contract-First Order

1. Check provider info: `noti --json providers info <name>`
2. Verify profile: `noti config test <name>`
3. Simplify to minimal send with `--json` to see raw response
4. Check exit code for error classification

## Supported Providers

| Provider | Scheme | Description |
|----------|--------|-------------|
| WeCom | `wecom://` | WeChat Work group bot webhook |
| Feishu | `feishu://` | Feishu / Lark group bot webhook |
| Slack | `slack://` | Slack incoming webhooks |
| Telegram | `tg://` | Telegram Bot API |
| Discord | `discord://` | Discord webhooks |
| Email | `smtp://` | Email via SMTP |
| Webhook | `webhook://` | Generic HTTP webhook |
| DingTalk | `dingtalk://` | DingTalk group bot webhook |
| Pushover | `pushover://` | Pushover push notifications |
| ntfy | `ntfy://` | ntfy.sh push notifications |
| Gotify | `gotify://` | Gotify push notifications |
| Bark | `bark://` | Bark iOS push notifications |
| PushDeer | `pushdeer://` | PushDeer cross-platform push |
| ServerChan | `serverchan://` | ServerChan (Server酱) push |
| Teams | `teams://` | Microsoft Teams incoming webhook |
| Google Chat | `gchat://` | Google Chat space webhook |
| Mattermost | `mattermost://` | Mattermost incoming webhook |
| Rocket.Chat | `rocketchat://` | Rocket.Chat incoming webhook |
| Matrix | `matrix://` | Matrix via Client-Server API |
| Twilio | `twilio://` | Twilio SMS via REST API |
| Zulip | `zulip://` | Zulip team chat via Bot API |
| Webex | `webex://` | Cisco Webex Teams messaging |
| LINE | `line://` | LINE Notify push |
| Revolt | `revolt://` | Revolt open-source chat |
| PushBullet | `pushbullet://` | PushBullet cross-platform push |
| SimplePush | `simplepush://` | SimplePush.io push notifications |
| IFTTT | `ifttt://` | IFTTT Maker Webhooks |
| PagerDuty | `pagerduty://` | PagerDuty Events API v2 |
| Vonage | `vonage://` | Vonage (Nexmo) SMS API |
| Mailgun | `mailgun://` | Mailgun transactional email API |
| Opsgenie | `opsgenie://` | Atlassian Opsgenie alerts API v2 |
| SendGrid | `sendgrid://` | SendGrid transactional email API v3 |
| Notica | `notica://` | Notica browser push notifications |
| Mastodon | `mastodon://` | Mastodon status post (toot) |
| JSON Webhook | `json://` | Generic JSON webhook (POST JSON) |
| Form Webhook | `form://` | Generic form webhook (POST form-encoded) |
| XML Webhook | `xml://` | Generic XML webhook (POST XML) |
| Prowl | `prowl://` | Prowl iOS push notifications |
| Join | `join://` | Join by joaoapps push notifications |
| Pushsafer | `pushsafer://` | Pushsafer push notifications |
| Home Assistant | `hassio://` | Home Assistant notifications |
| Signal | `signal://` | Signal Messenger via signal-cli REST API |
| SparkPost | `sparkpost://` | SparkPost transactional email API |
| Ryver | `ryver://` | Ryver team messaging |
| Twist | `twist://` | Twist async team messaging |
| Flock | `flock://` | Flock team messaging |
| Gitter | `gitter://` | Gitter developer chat |
| Guilded | `guilded://` | Guilded gaming chat webhooks |
| Misskey | `misskey://` | Misskey fediverse note posting |
| Bluesky | `bluesky://` | Bluesky social network (AT Protocol) |
| OneSignal | `onesignal://` | OneSignal push notification platform |
| Techulus Push | `push://` | Techulus Push notifications |
| Pushy | `pushy://` | Pushy cross-platform push notifications |
| Chanify | `chanify://` | Chanify iOS/Android push notifications |
| Pushplus | `pushplus://` | Pushplus push (WeChat/SMS/Email) |
| WxPusher | `wxpusher://` | WxPusher WeChat push notifications |
| D7 Networks | `d7sms://` | D7 Networks SMS gateway |
| Sinch | `sinch://` | Sinch SMS via REST API |
| Clickatell | `clickatell://` | Clickatell SMS/messaging gateway |
| BulkSMS | `bulksms://` | BulkSMS gateway via REST API v2 |
| Kavenegar | `kavenegar://` | Kavenegar SMS gateway (Iran) |
| MessageBird | `msgbird://` | MessageBird SMS via REST API |
| Plivo | `plivo://` | Plivo SMS via REST API |
| BurstSMS | `burstsms://` | BurstSMS (Transmit SMS) gateway |
| PopcornNotify | `popcorn://` | PopcornNotify SMS messaging |
| AWS SES | `ses://` | AWS SES transactional email |
| Resend | `resend://` | Resend modern email API |
| AWS SNS | `sns://` | AWS SNS topic publishing |
| LaMetric | `lametric://` | LaMetric Time smart clock |
| LunaSea | `lunasea://` | LunaSea self-hosted media push |
| Nextcloud | `ncloud://` | Nextcloud push notifications |
| Reddit | `reddit://` | Reddit private messages |
| PagerTree | `pagertree://` | PagerTree incident management |
| SIGNL4 | `signl4://` | SIGNL4 mobile alerting |
| Splunk On-Call | `victorops://` | Splunk On-Call (VictorOps) incidents |
| Spike.sh | `spike://` | Spike.sh incident management |
| FCM | `fcm://` | Firebase Cloud Messaging push notifications |
| Threema | `threema://` | Threema Gateway secure messaging |
| ClickSend | `clicksend://` | ClickSend SMS messaging gateway |
| Brevo | `brevo://` | Brevo (Sendinblue) transactional email |
| SMTP2Go | `smtp2go://` | SMTP2Go transactional email |
| Apprise API | `apprise://` | Apprise API notification relay |
| Free Mobile | `freemobile://` | Free Mobile SMS (France) |
| httpSMS | `httpsms://` | httpSMS — send SMS via Android phone |
| MSG91 | `msg91://` | MSG91 SMS gateway (India) |
| Pushjet | `pushjet://` | Pushjet push notification service |
| SMSEagle | `smseagle://` | SMSEagle hardware SMS gateway |
| Seven | `seven://` | Seven (sms77) SMS gateway |
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

## References

- `references/install-and-usage.md` — Installation, update, and basic usage patterns
- `references/provider-guide.md` — Detailed provider parameters and URL scheme formats
