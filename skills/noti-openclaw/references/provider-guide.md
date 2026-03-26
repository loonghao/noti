# Provider Guide

## URL Scheme Quick Reference

| Provider | URL Format | Key Parameters |
|----------|-----------|----------------|
| WeCom | `wecom://<key>` | `key` (required) |
| Feishu | `feishu://<hook_id>` | `hook_id` (required), `secret` (optional) |
| Slack | `slack://<a>/<b>/<c>` | `webhook_url` (required) |
| Telegram | `tg://<bot_token>/<chat_id>` | `bot_token`, `chat_id` (required) |
| Discord | `discord://<id>/<token>` | `webhook_id`, `webhook_token` (required) |
| Email | `smtp://user:pass@host:port?to=addr` | `host`, `username`, `password`, `to` (required) |
| Webhook | `webhook://host/path` | `url` (required) |
| DingTalk | `dingtalk://<access_token>` | `access_token` (required), `secret` (optional) |
| Pushover | `pushover://<user_key>/<api_token>` | `user_key`, `api_token` (required) |
| ntfy | `ntfy://<topic>` | `topic` (required), `server` (optional) |
| Gotify | `gotify://<host>/<app_token>` | `host`, `app_token` (required) |
| Bark | `bark://<device_key>` | `device_key` (required), `server` (optional) |
| PushDeer | `pushdeer://<push_key>` | `push_key` (required) |
| ServerChan | `serverchan://<send_key>` | `send_key` (required) |
| Teams | `teams://<webhook_url_encoded>` | `webhook_url` (required) |
| Google Chat | `gchat://<space>/<key>/<token>` | `webhook_url` (required) |
| Mattermost | `mattermost://<host>/<hook_id>` | `host`, `hook_id` (required) |
| Rocket.Chat | `rocketchat://<host>/<token_a>/<token_b>` | `host`, `token_a`, `token_b` (required) |
| Matrix | `matrix://<access_token>/<room_id>` | `access_token`, `room_id` (required), `server` (optional) |
| Twilio | `twilio://<sid>:<token>@<from>/<to>` | `account_sid`, `auth_token`, `from`, `to` (required) |
| Zulip | `zulip://<email>:<key>@<domain>/<stream>/<topic>` | `bot_email`, `api_key`, `domain` (required), `stream`, `topic` (optional) |
| Webex | `webex://<access_token>/<room_id>` | `access_token`, `room_id` (required) |
| LINE | `line://<access_token>` | `access_token` (required) |
| Revolt | `revolt://<bot_token>/<channel_id>` | `bot_token`, `channel_id` (required) |
| PushBullet | `pushbullet://<access_token>` | `access_token` (required) |
| SimplePush | `simplepush://<key>` | `key` (required) |
| IFTTT | `ifttt://<webhook_key>/<event_name>` | `webhook_key`, `event` (required) |
| PagerDuty | `pagerduty://<integration_key>` | `integration_key` (required), `severity` (optional) |
| Vonage | `vonage://<key>:<secret>@<from>/<to>` | `api_key`, `api_secret`, `from`, `to` (required) |
| Mailgun | `mailgun://<api_key>@<domain>/<to>` | `api_key`, `domain` (required), `to`, `from` (optional) |
| Opsgenie | `opsgenie://<api_key>` | `api_key` (required), `region`, `priority`, `alias`, `tags` (optional) |
| SendGrid | `sendgrid://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required), `from_name`, `to_name`, `cc`, `bcc` (optional) |
| Notica | `notica://<token>` | `token` (required) |
| Mastodon | `mastodon://<access_token>@<instance>` | `access_token`, `instance` (required), `visibility`, `spoiler_text` (optional) |
| JSON Webhook | `json://<host>/<path>` | `url` (required), `method`, `header`, `type` (optional) |
| Form Webhook | `form://<host>/<path>` | `url` (required), `method`, `header`, `type` (optional) |
| XML Webhook | `xml://<host>/<path>` | `url` (required), `method`, `header`, `type`, `root` (optional) |
| Prowl | `prowl://<api_key>` | `api_key` (required), `priority`, `url`, `application`, `provider_key` (optional) |
| Join | `join://<api_key>/<device_id>` | `api_key` (required), `device_id` (optional, default: group.all) |
| Pushsafer | `pushsafer://<private_key>` | `private_key` (required), `device`, `sound`, `vibration`, `icon`, `priority` (optional) |
| Home Assistant | `hassio://<access_token>@<host>` | `access_token`, `host` (required), `scheme`, `target` (optional) |
| Signal | `signal://<from>/<to>` | `from`, `to` (required), `server` (optional) |
| SparkPost | `sparkpost://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required), `from_name`, `region`, `cc`, `bcc` (optional) |
| Ryver | `ryver://<organization>/<token>` | `organization`, `token` (required), `webhook_type` (optional) |
| Twist | `twist://<webhook_params>` | `webhook_url` (required) |
| Flock | `flock://<token>` | `token` (required) |
| Guilded | `guilded://<webhook_id>/<webhook_token>` | `webhook_id`, `webhook_token` (required), `username`, `avatar_url` (optional) |
| Misskey | `misskey://<access_token>@<instance>` | `access_token`, `instance` (required), `visibility`, `cw` (optional) |
| OneSignal | `onesignal://<app_id>:<api_key>` | `app_id`, `api_key` (required), `include_segments`, `player_ids`, `url`, `image` (optional) |
| Techulus Push | `push://<api_key>` | `api_key` (required), `link` (optional) |
| Pushy | `pushy://<api_key>/<device_token>` | `api_key`, `device_token` (required), `sound`, `badge` (optional) |
| D7 Networks | `d7sms://<api_token>@<from>/<to>` | `api_token` (required), `to`, `from`, `channel` (optional) |
| Sinch | `sinch://<plan_id>:<token>@<from>/<to>` | `service_plan_id`, `api_token`, `from`, `to` (required), `region` (optional) |
| Clickatell | `clickatell://<api_key>/<to>` | `api_key` (required), `to`, `from` (optional) |
| BulkSMS | `bulksms://<token_id>:<secret>@<from>/<to>` | `token_id`, `token_secret`, `to` (required), `from` (optional) |
| Kavenegar | `kavenegar://<api_key>/<from>/<to>` | `api_key` (required), `to`, `from` (optional) |
| AWS SES | `ses://<access_key>:<secret>@<region>/<from>/<to>` | `access_key`, `secret_key`, `region`, `from`, `to` (required), `cc`, `bcc`, `from_name` (optional) |
| AWS SNS | `sns://<access_key>:<secret>@<region>/<topic_arn>` | `access_key`, `secret_key`, `region`, `topic_arn` (required), `subject` (optional) |
| LaMetric | `lametric://<api_key>@<host>` | `api_key`, `host` (required), `icon`, `sound`, `priority`, `cycles` (optional) |
| LunaSea | `lunasea://<user_token>` | `user_token` (required), `target`, `image` (optional) |
| Reddit | `reddit://<client_id>:<secret>@<user>:<pass>/<to>` | `client_id`, `client_secret`, `user`, `password`, `to` (required) |
| Chanify | `chanify://<token>` | `token` (required), `server` (optional) |
| Pushplus | `pushplus://<token>` | `token` (required), `topic`, `template`, `channel` (optional) |
| WxPusher | `wxpusher://<app_token>/<uid>` | `app_token`, `uid` (required), `topic_id`, `content_type` (optional) |
| Resend | `resend://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required), `reply_to` (optional) |
| Bluesky | `bluesky://<handle>:<app_password>` | `handle`, `app_password` (required), `server` (optional) |
| MessageBird | `msgbird://<access_key>@<from>/<to>` | `access_key`, `from`, `to` (required) |
| Plivo | `plivo://<auth_id>:<auth_token>@<from>/<to>` | `auth_id`, `auth_token`, `from`, `to` (required) |
| BurstSMS | `burstsms://<api_key>:<api_secret>@<from>/<to>` | `api_key`, `api_secret`, `from`, `to` (required) |
| Gitter | `gitter://<token>/<room_id>` | `token`, `room_id` (required) |
| Nextcloud | `ncloud://<user>:<password>@<host>/<target_user>` | `user`, `password`, `host` (required), `target_user`, `scheme` (optional) |
| PagerTree | `pagertree://<integration_id>` | `integration_id` (required), `urgency` (optional) |
| SIGNL4 | `signl4://<team_secret>` | `team_secret` (required), `s4_severity`, `s4_service` (optional) |
| Splunk On-Call | `victorops://<api_key>/<routing_key>` | `api_key`, `routing_key` (required), `message_type` (optional) |
| Spike.sh | `spike://<webhook_url_path>` | `webhook_url` (required) |
| PopcornNotify | `popcorn://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required) |
| FCM | `fcm://<server_key>/<device_token>` | `server_key`, `device_token` (required), `topic`, `priority`, `ttl`, `icon`, `sound`, `image` (optional) |
| Threema | `threema://<gateway_id>:<api_secret>@<recipient_id>` | `gateway_id`, `api_secret`, `to` (required), `to_phone`, `to_email` (optional) |
| ClickSend | `clicksend://<username>:<api_key>@<from>/<to>` | `username`, `api_key`, `to` (required), `from`, `schedule` (optional) |
| Brevo | `brevo://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required), `from_name`, `to_name`, `cc`, `bcc`, `reply_to` (optional) |
| SMTP2Go | `smtp2go://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required), `cc`, `bcc` (optional) |
| Apprise API | `apprise://<host>/<config_key>` | `host` (required), `config_key` or `urls` (required), `notification_type`, `tag` (optional) |
| Free Mobile | `freemobile://<user_id>/<api_key>` | `user`, `password` (required) |
| httpSMS | `httpsms://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required), `encrypt` (optional) |
| MSG91 | `msg91://<authkey>/<sender>/<to>` | `authkey`, `sender`, `to` (required), `route`, `country`, `DLT_TE_ID` (optional) |
| Pushjet | `pushjet://<secret_key>` | `secret` (required), `server`, `level`, `link` (optional) |
| SMSEagle | `smseagle://<access_token>@<host>/<to>` | `host`, `access_token`, `to` (required), `scheme`, `port`, `priority` (optional) |
| Seven | `seven://<api_key>/<to>` | `api_key`, `to` (required), `from`, `flash`, `foreign_id` (optional) |
| Web Push | `webpush://<endpoint>` | `endpoint`, `p256dh`, `auth` (required), `vapid_private`, `vapid_email`, `ttl`, `urgency` (optional) |
| WhatsApp | `whatsapp://<access_token>@<phone_number_id>/<to>` | `access_token`, `phone_number_id`, `to` (required), `api_version`, `preview_url` (optional) |
| Kodi | `kodi://<user>:<password>@<host>:<port>` | `host` (required), `port`, `user`, `password`, `scheme`, `display_time`, `image` (optional) |
| Notifico | `notifico://<project_id>/<msghook>` | `project_id`, `msghook` (required), `host` (optional) |
| 46elks | `46elks://<api_username>:<api_password>@<from>/<to>` | `api_username`, `api_password`, `from`, `to` (required), `flash` (optional) |
| BulkVS | `bulkvs://<username>:<password>@<from>/<to>` | `username`, `password`, `from`, `to` (required) |
| Jira | `jira://<user>:<api_token>@<host>/<issue_key>` | `host`, `user`, `api_token`, `issue_key` (required), `scheme` (optional) |
| PushMe | `pushme://<push_key>` | `push_key` (required), `type` (optional) |
| SendPulse | `sendpulse://<client_id>:<client_secret>@<from>/<to>` | `client_id`, `client_secret`, `from`, `to` (required), `from_name`, `to_name` (optional) |
| Streamlabs | `streamlabs://<access_token>` | `access_token` (required), `type`, `image_href`, `sound_href`, `duration` (optional) |
| Synology Chat | `synology://<token>@<host>` | `host`, `token` (required), `port`, `scheme` (optional) |
| Africa's Talking | `africastalking://<username>:<api_key>@<to>` | `username`, `api_key`, `to` (required), `from`, `sandbox` (optional) |
| Office 365 | `o365://<client_id>:<client_secret>@<tenant_id>/<from>/<to>` | `tenant_id`, `client_id`, `client_secret`, `from`, `to` (required), `cc`, `bcc`, `save_to_sent` (optional) |
| Nextcloud Talk | `nctalk://<user>:<password>@<host>/<room_token>` | `user`, `password`, `host`, `room_token` (required), `scheme` (optional) |
| Emby | `emby://<api_key>@<host>/<user_id>` | `api_key`, `host` (required), `user_id`, `scheme` (optional) |
| Jellyfin | `jellyfin://<api_key>@<host>/<user_id>` | `api_key`, `host` (required), `user_id`, `scheme` (optional) |
| Pushcut | `pushcut://<api_key>/<notification_name>` | `api_key`, `notification_name` (required), `url`, `image`, `sound` (optional) |
| MQTT | `mqtt://<user>:<password>@<host>/<topic>` | `host`, `topic` (required), `user`, `password`, `scheme`, `qos`, `retain` (optional) |
| VoIP.ms | `voipms://<email>:<password>@<did>/<to>` | `email`, `password`, `did`, `to` (required) |
| SFR | `sfr://<phone>:<password>` | `phone`, `password` (required) |
| Pushed | `pushed://<app_key>:<app_secret>` | `app_key`, `app_secret` (required), `target_type`, `target_alias` (optional) |
| Growl | `growl://<password>@<host>:<port>` | `host` (required), `port`, `password`, `scheme`, `priority`, `sticky` (optional) |
| Kumulos | `kumulos://<api_key>:<server_key>` | `api_key`, `server_key` (required), `channel` (optional) |
| Parse | `parse://<app_id>:<rest_api_key>@<host>` | `app_id`, `rest_api_key` (required), `host`, `channel` (optional) |
| Remote Syslog | `rsyslog://<host>/<token>` | `host` (required), `token`, `port`, `scheme`, `facility`, `severity` (optional) |
| SMS Manager | `smsmanager://<api_key>@<from>/<to>` | `api_key`, `to` (required), `from` (optional) |
| X (Twitter) | `twitter://<bearer_token>` | `bearer_token` (required), `mode`, `dm_user_id` (optional) |
| Boxcar | `boxcar://<access_token>` | `access_token` (required), `sound`, `source_name`, `icon_url` (optional) |
| DAPNET | `dapnet://<callsign>:<password>@<to_callsign>` | `callsign`, `password`, `to` (required), `txgroup`, `emergency` (optional) |
| Enigma2 | `enigma2://<host>` | `host` (required), `port`, `user`, `password`, `scheme`, `timeout`, `msg_type` (optional) |
| Notifiarr | `notifiarr://<api_key>` | `api_key` (required), `discord_channel`, `color`, `ping_user`, `ping_role`, `image` (optional) |
| Statuspage | `statuspage://<api_key>@<page_id>` | `api_key`, `page_id` (required), `status`, `impact`, `component_ids`, `component_status` (optional) |
| Dot. | `dot://<token>@<device_id>` | `token`, `device_id` (required), `signature`, `mode` (optional) |
| Fluxer | `fluxer://<webhook_id>/<webhook_token>` | `webhook_id`, `webhook_token` (required), `botname`, `avatar_url`, `tts`, `host` (optional) |
| Workflows | `workflows://<host>/<workflow>/<signature>` | `host`, `workflow`, `signature` (required), `port`, `api_version` (optional) |
| NotificationAPI | `napi://<client_id>/<client_secret>/<user_id>` | `client_id`, `client_secret`, `user_id` (required), `notification_type`, `region` (optional) |
| SpugPush | `spugpush://<token>` | `token` (required) |

## Provider Details

### WeCom (企业微信)

```bash
noti send --to "wecom://<webhook_key>" --message "Hello"
```

Parameters: `key` (required), `mentioned_list` (optional), `mentioned_mobile_list` (optional)

### Feishu / Lark (飞书)

```bash
noti send --to "feishu://<hook_id>" --message "Hello"
noti send --to "feishu://<hook_id>?secret=<secret>" --message "Hello"
```

Parameters: `hook_id` (required), `secret` (optional — webhook signature)

### Slack

```bash
noti send --to "slack://<token_a>/<token_b>/<token_c>" --message "Hello"
```

Parameters: `webhook_url` (required), `channel`, `username`, `icon_emoji` (optional)

### Telegram

```bash
noti send --to "tg://<bot_token>/<chat_id>" --message "Hello"
```

Parameters: `bot_token`, `chat_id` (required), `disable_notification`, `disable_web_page_preview` (optional)

### Discord

```bash
noti send --to "discord://<webhook_id>/<webhook_token>" --message "Hello"
```

Parameters: `webhook_id`, `webhook_token` (required), `username`, `avatar_url` (optional)

### Email (SMTP)

```bash
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=addr@example.com" --message "Hello" --title "Subject"
```

Parameters: `host`, `username`, `password`, `to` (required), `port`, `from` (optional)

### Webhook (Generic HTTP)

```bash
noti send --to "webhook://example.com/api/notify" --message "Hello"
```

Parameters: `url` (required), `method`, `content_type`, `headers`, `body_template` (optional)

### DingTalk (钉钉)

```bash
noti send --to "dingtalk://<access_token>" --message "Hello"
noti send --to "dingtalk://<access_token>?secret=<secret>" --message "Hello"
```

Parameters: `access_token` (required), `secret` (optional — for signed mode)

### Pushover

```bash
noti send --to "pushover://<user_key>/<api_token>" --message "Hello"
```

Parameters: `user_key`, `api_token` (required), `device`, `priority`, `sound` (optional)

### ntfy

```bash
noti send --to "ntfy://<topic>" --message "Hello"
noti send --to "ntfy://<topic>?server=https://ntfy.example.com" --message "Hello"
```

Parameters: `topic` (required), `server` (optional, default: `https://ntfy.sh`)

### Gotify

```bash
noti send --to "gotify://<host>/<app_token>" --message "Hello"
```

Parameters: `host`, `app_token` (required), `priority` (optional)

### Bark (iOS)

```bash
noti send --to "bark://<device_key>" --message "Hello"
noti send --to "bark://<device_key>?server=https://bark.example.com" --message "Hello"
```

Parameters: `device_key` (required), `server` (optional, default: `https://api.day.app`)

### PushDeer

```bash
noti send --to "pushdeer://<push_key>" --message "Hello"
```

Parameters: `push_key` (required), `server` (optional)

### ServerChan (Server酱)

```bash
noti send --to "serverchan://<send_key>" --message "Hello"
```

Parameters: `send_key` (required)

### Microsoft Teams

```bash
noti send --to "teams://<webhook_url_host>/<path...>" --message "Hello"
noti send --provider teams --param webhook_url=https://xxx.webhook.office.com/... --message "Hello"
```

Parameters: `webhook_url` (required), `theme_color` (optional)

### Google Chat

```bash
noti send --to "gchat://<space>/<key>/<token>" --message "Hello"
```

Parameters: `webhook_url` (required)

### Mattermost

```bash
noti send --to "mattermost://<host>/<hook_id>" --message "Hello"
```

Parameters: `host`, `hook_id` (required), `channel`, `username`, `icon_url`, `port`, `scheme` (optional)

### Rocket.Chat

```bash
noti send --to "rocketchat://<host>/<token_a>/<token_b>" --message "Hello"
```

Parameters: `host`, `token_a`, `token_b` (required), `channel`, `username`, `icon_url`, `port`, `scheme` (optional)

### Matrix

```bash
noti send --to "matrix://<access_token>/<room_id>" --message "Hello"
noti send --to "matrix://<access_token>/<room_id>?server=matrix.example.com" --message "Hello"
```

Parameters: `access_token`, `room_id` (required), `server` (optional, default: matrix.org), `port`, `scheme` (optional)

### Twilio SMS

```bash
noti send --to "twilio://<account_sid>:<auth_token>@<from_number>/<to_number>" --message "Hello"
```

Parameters: `account_sid`, `auth_token`, `from`, `to` (required)

### Zulip

```bash
noti send --to "zulip://bot@example.com:api_key@chat.zulip.org/general/greetings" --message "Hello"
```

Parameters: `bot_email`, `api_key`, `domain` (required), `stream`, `topic` (optional — defaults to direct message if omitted)

### Cisco Webex

```bash
noti send --to "webex://<access_token>/<room_id>" --message "Hello"
```

Parameters: `access_token`, `room_id` (required) — supports markdown formatting

### LINE Notify

```bash
noti send --to "line://<access_token>" --message "Hello"
```

Parameters: `access_token` (required)

### Revolt

```bash
noti send --to "revolt://<bot_token>/<channel_id>" --message "Hello"
```

Parameters: `bot_token`, `channel_id` (required) — supports markdown content

### PushBullet

```bash
noti send --to "pushbullet://<access_token>" --message "Hello"
```

Parameters: `access_token` (required) — sends as note type to all devices

### SimplePush

```bash
noti send --to "simplepush://<key>" --message "Hello"
```

Parameters: `key` (required)

### IFTTT (Maker Webhooks)

```bash
noti send --to "ifttt://<webhook_key>/<event_name>" --message "Hello"
```

Parameters: `webhook_key`, `event` (required) — message maps to `value1`, title to `value2`

### PagerDuty

```bash
noti send --to "pagerduty://<integration_key>" --message "Server down" --title "Critical Alert"
noti send --to "pagerduty://<integration_key>?severity=warning" --message "High CPU"
```

Parameters: `integration_key` (required), `severity` (optional: `critical`, `error`, `warning`, `info` — default: `error`)

### Vonage (Nexmo) SMS

```bash
noti send --to "vonage://<api_key>:<api_secret>@<from_number>/<to_number>" --message "Hello"
```

Parameters: `api_key`, `api_secret`, `from`, `to` (required)

### Mailgun

```bash
noti send --to "mailgun://<api_key>@<domain>/<to_email>" --message "Hello" --title "Subject"
noti send --to "mailgun://<api_key>@<domain>/<to_email>?region=eu" --message "Hello"
```

Parameters: `api_key`, `domain` (required), `to`, `from` (optional — defaults to `noti <noti@domain>`), `region` (optional: `us` or `eu`, default: `us`)

### Opsgenie (Atlassian)

```bash
noti send --to "opsgenie://<api_key>" --message "Server CPU at 95%" --title "High CPU Alert"
noti send --to "opsgenie://<api_key>?region=eu&priority=P2" --message "Disk space low"
```

Parameters: `api_key` (required), `region` (optional: `us` or `eu`, default: `us`), `priority` (optional: `P1`-`P5`, default: `P3`), `alias` (optional — deduplication key), `tags` (optional — comma-separated), `entity` (optional), `responders` (optional — comma-separated team names or emails)

### SendGrid

```bash
noti send --to "sendgrid://<api_key>@sender@example.com/recipient@example.com" --message "Hello" --title "Subject"
```

Parameters: `api_key`, `from`, `to` (required), `from_name`, `to_name` (optional — display names), `cc`, `bcc` (optional — comma-separated emails)

### Notica

```bash
noti send --to "notica://<token>" --message "Hello from noti"
```

Parameters: `token` (required) — browser push notification via notica.us

### Mastodon

```bash
noti send --to "mastodon://<access_token>@mastodon.social" --message "Hello Fediverse!"
noti send --to "mastodon://<access_token>@mastodon.social?visibility=unlisted" --message "Quiet post"
```

Parameters: `access_token`, `instance` (required), `visibility` (optional: `public`, `unlisted`, `private`, `direct` — default: `public`), `spoiler_text` (optional — content warning)

### JSON Webhook

```bash
noti send --to "json://example.com/api/notify" --message "Hello"
noti send --provider json --param url=https://example.com/api --param header="X-Api-Key=abc" --message "Hello"
```

Parameters: `url` (required), `method` (optional: `POST`, `PUT`, `PATCH` — default: `POST`), `header` (optional — semicolon-separated `key=value` pairs), `type` (optional — notification type field, default: `info`)

Sends JSON body: `{"message": "...", "title": "...", "type": "info"}`

### Form Webhook

```bash
noti send --to "form://example.com/api/notify" --message "Hello"
noti send --provider form --param url=https://example.com/api --param header="X-Api-Key=abc" --message "Hello"
```

Parameters: `url` (required), `method` (optional: `POST`, `PUT`, `PATCH` — default: `POST`), `header` (optional — semicolon-separated `key=value` pairs), `type` (optional — notification type field, default: `info`)

Sends form-encoded body: `message=...&title=...&type=info`

### XML Webhook

```bash
noti send --to "xml://example.com/api/notify" --message "Hello"
noti send --provider xml --param url=https://example.com/api --param header="X-Api-Key=abc" --message "Hello"
```

Parameters: `url` (required), `method` (optional: `POST`, `PUT`, `PATCH` — default: `POST`), `header` (optional — semicolon-separated `key=value` pairs), `type` (optional — notification type field, default: `info`), `root` (optional — XML root element name, default: `notification`)

Sends XML body: `<notification><title>...</title><message>...</message><type>info</type></notification>`

### Prowl (iOS)

```bash
noti send --to "prowl://<api_key>" --message "Hello" --title "Alert"
noti send --to "prowl://<api_key>?priority=2" --message "Emergency!"
```

Parameters: `api_key` (required), `priority` (optional: -2 to 2, default: 0), `url` (optional — URL to attach), `application` (optional, default: noti), `provider_key` (optional — for higher API rate limits)

### Join (by joaoapps)

```bash
noti send --to "join://<api_key>/group.all" --message "Hello"
noti send --to "join://<api_key>" --message "Hello"  # defaults to group.all
```

Parameters: `api_key` (required), `device_id` (optional, default: group.all — sends to all devices), `icon` (optional — notification icon URL), `smallicon` (optional), `url` (optional — URL to open)

### Pushsafer

```bash
noti send --to "pushsafer://<private_key>" --message "Hello" --title "Alert"
noti send --to "pushsafer://<private_key>?device=12345&sound=10" --message "Hello"
```

Parameters: `private_key` (required), `device` (optional, default: a = all devices), `sound` (optional: 0-62), `vibration` (optional: 0-3), `icon` (optional: 1-176), `icon_color` (optional — hex color), `url` (optional), `url_title` (optional), `priority` (optional: -2 to 2)

### Home Assistant

```bash
noti send --to "hassio://<access_token>@homeassistant.local:8123" --message "Hello"
noti send --to "hassio://<access_token>@ha.local:8123?target=notify.mobile_app_phone" --message "Hello"
```

Parameters: `access_token` (required — long-lived access token), `host` (required — HA hostname:port), `scheme` (optional: `http` or `https`, default: `http`), `target` (optional — notification service, default: `notify.notify`)

### Signal (via signal-cli)

```bash
noti send --to "signal://+1234567890/+0987654321" --message "Hello"
noti send --to "signal://+1234567890/+0987654321?server=http://localhost:8080" --message "Hello"
```

Parameters: `from` (required — sender phone number registered in signal-cli), `to` (required — recipient phone number or group ID), `server` (optional — signal-cli REST API URL, default: `http://localhost:8080`)

### SparkPost

```bash
noti send --to "sparkpost://<api_key>@sender@example.com/recipient@example.com" --message "Hello" --title "Subject"
noti send --to "sparkpost://<api_key>@sender@example.com/recipient@example.com?region=eu" --message "Hello"
```

Parameters: `api_key`, `from`, `to` (required), `from_name` (optional — display name, default: noti), `region` (optional: `us` or `eu`, default: `us`), `cc`, `bcc` (optional — comma-separated emails)

### Ryver

```bash
noti send --to "ryver://<organization>/<token>" --message "Hello"
```

Parameters: `organization` (required), `token` (required — incoming webhook token), `webhook_type` (optional: `forum` or `team`, default: `forum`)

### Twist

```bash
noti send --provider twist --param webhook_url=https://twist.com/api/v3/... --message "Hello"
```

Parameters: `webhook_url` (required — Twist integration webhook URL)

### Flock

```bash
noti send --to "flock://<token>" --message "Hello"
```

Parameters: `token` (required — Flock incoming webhook token)

### Guilded

```bash
noti send --to "guilded://<webhook_id>/<webhook_token>" --message "Hello"
```

Parameters: `webhook_id`, `webhook_token` (required), `username` (optional — override display name), `avatar_url` (optional)

### Misskey

```bash
noti send --to "misskey://<access_token>@misskey.io" --message "Hello Fediverse!"
noti send --to "misskey://<access_token>@misskey.io?visibility=home" --message "Quiet post"
```

Parameters: `access_token`, `instance` (required), `visibility` (optional: `public`, `home`, `followers`, `specified` — default: `public`), `cw` (optional — content warning text)

### OneSignal

```bash
noti send --to "onesignal://<app_id>:<api_key>" --message "Hello" --title "Alert"
noti send --provider onesignal --param app_id=... --param api_key=... --param player_ids=id1,id2 --message "Hello"
```

Parameters: `app_id`, `api_key` (required), `include_segments` (optional — default: `Subscribed Users`), `player_ids` (optional — comma-separated), `url` (optional — click URL), `image` (optional — image URL)

### Techulus Push

```bash
noti send --to "push://<api_key>" --message "Hello"
```

Parameters: `api_key` (required), `link` (optional — URL to attach)

### Pushy

```bash
noti send --to "pushy://<api_key>/<device_token>" --message "Hello" --title "Alert"
```

Parameters: `api_key`, `device_token` (required), `sound` (optional — sound file name), `badge` (optional — badge count for iOS)

### D7 Networks SMS

```bash
noti send --to "d7sms://<api_token>@SENDER/+15559876543" --message "Hello"
noti send --provider d7sms --param api_token=... --param to=+15559876543 --message "Hello"
```

Parameters: `api_token` (required), `to` (required — recipient phone E.164), `from` (optional — sender ID), `channel` (optional: `sms`, `whatsapp`, `viber` — default: `sms`)

### Sinch SMS

```bash
noti send --to "sinch://<plan_id>:<token>@+15551234567/+15559876543" --message "Hello"
```

Parameters: `service_plan_id`, `api_token`, `from`, `to` (required), `region` (optional: `us` or `eu`, default: `us`)

### Clickatell SMS

```bash
noti send --to "clickatell://<api_key>/15559876543" --message "Hello"
```

Parameters: `api_key` (required), `to` (required — international format), `from` (optional — sender ID)

### BulkSMS

```bash
noti send --to "bulksms://<token_id>:<token_secret>@+15551234567/+15559876543" --message "Hello"
```

Parameters: `token_id`, `token_secret`, `to` (required), `from` (optional — sender number)

### Kavenegar SMS

```bash
noti send --to "kavenegar://<api_key>/10004346/09121234567" --message "Hello"
```

Parameters: `api_key` (required), `to` (required — recipient phone), `from` (optional — line number)

### AWS SES (Simple Email Service)

```bash
noti send --to "ses://<access_key>:<secret_key>@us-east-1/sender@example.com/recipient@example.com" --message "Hello" --title "Subject"
```

Parameters: `access_key`, `secret_key`, `region`, `from`, `to` (required), `cc`, `bcc` (optional — comma-separated), `from_name` (optional — display name, default: noti)

### AWS SNS (Simple Notification Service)

```bash
noti send --to "sns://<access_key>:<secret_key>@us-east-1/arn:aws:sns:us-east-1:123456789012:my-topic" --message "Hello"
```

Parameters: `access_key`, `secret_key`, `region`, `topic_arn` (required), `subject` (optional — for email subscriptions)

### LaMetric Time

```bash
noti send --to "lametric://<api_key>@192.168.1.100" --message "Hello" --title "Alert"
noti send --to "lametric://<api_key>@192.168.1.100?icon=i124&sound=notification" --message "Hello"
```

Parameters: `api_key` (required — device API key), `host` (required — device IP), `icon` (optional — icon ID), `sound` (optional — sound ID), `priority` (optional: `info`, `warning`, `critical` — default: `info`), `cycles` (optional — display cycles, default: 1)

### LunaSea

```bash
noti send --to "lunasea://<user_token>" --message "Hello" --title "Media Alert"
```

Parameters: `user_token` (required), `target` (optional: `user` or `device`, default: `user`), `image` (optional — image URL)

### Reddit

```bash
noti send --provider reddit --param client_id=... --param client_secret=... --param user=mybot --param password=... --param to=targetuser --message "Hello"
noti send --to "reddit://<client_id>:<client_secret>@<user>:<password>/<to_user>" --message "Hello" --title "Subject"
```

Parameters: `client_id`, `client_secret` (required — Reddit app credentials), `user`, `password` (required — Reddit account), `to` (required — recipient username)

### Chanify

```bash
noti send --to "chanify://<token>" --message "Hello"
noti send --to "chanify://<token>@chanify.example.com" --message "Hello"
```

Parameters: `token` (required — Chanify device token), `server` (optional — self-hosted Chanify server URL, default: `https://api.chanify.net`)

### Pushplus (推送加)

```bash
noti send --to "pushplus://<token>" --message "Hello" --title "Alert"
noti send --to "pushplus://<token>?template=markdown&channel=wechat" --message "**Bold**"
```

Parameters: `token` (required — Pushplus user token), `topic` (optional — group topic code), `template` (optional: `html`, `txt`, `json`, `markdown`), `channel` (optional: `wechat`, `webhook`, `mail`, `sms`)

### WxPusher (微信推送)

```bash
noti send --to "wxpusher://AT_xxxx/UID_xxxx" --message "Hello"
noti send --to "wxpusher://AT_xxxx/UID_xxxx?content_type=3" --message "**Markdown**"
```

Parameters: `app_token` (required — application token), `uid` (required — target user UID), `topic_id` (optional — topic-based push), `content_type` (optional: `1`=text, `2`=html, `3`=markdown — default: `1`)

### Resend

```bash
noti send --to "resend://re_123abc@noti@yourdomain.com/user@example.com" --message "Hello" --title "Subject"
```

Parameters: `api_key` (required — Resend API key), `from` (required — sender email), `to` (required — recipient email), `reply_to` (optional — reply-to email)

### Bluesky

```bash
noti send --to "bluesky://user.bsky.social:xxxx-xxxx-xxxx-xxxx" --message "Hello Bluesky!"
```

Parameters: `handle` (required — Bluesky handle, e.g. `user.bsky.social`), `app_password` (required — app password), `server` (optional — PDS server URL, default: `https://bsky.social`). Note: Posts are limited to 300 characters.

### MessageBird SMS

```bash
noti send --to "msgbird://<access_key>@MyApp/+15559876543" --message "Hello"
```

Parameters: `access_key` (required — MessageBird access key), `from` (required — sender name or phone), `to` (required — recipient phone E.164)

### Plivo SMS

```bash
noti send --to "plivo://<auth_id>:<auth_token>@+15551234567/+15559876543" --message "Hello"
```

Parameters: `auth_id`, `auth_token` (required — Plivo credentials), `from`, `to` (required — E.164 format phone numbers)

### BurstSMS

```bash
noti send --to "burstsms://<api_key>:<api_secret>@MyApp/+61412345678" --message "Hello"
```

Parameters: `api_key`, `api_secret` (required — BurstSMS credentials), `from` (required — sender caller ID), `to` (required — recipient phone E.164)

### Gitter

```bash
noti send --to "gitter://<token>/<room_id>" --message "Hello developers!"
```

Parameters: `token` (required — Gitter personal access token), `room_id` (required — Gitter room ID)

### Nextcloud

```bash
noti send --to "ncloud://admin:app-token@cloud.example.com/john" --message "Hello"
noti send --to "ncloud://admin:app-token@cloud.example.com" --message "Hello"  # notifies self
```

Parameters: `user` (required — Nextcloud admin username), `password` (required — password or app token), `host` (required — server hostname), `target_user` (optional — user to notify, defaults to self), `scheme` (optional: `https` or `http`, default: `https`)

### PagerTree

```bash
noti send --to "pagertree://<integration_id>" --message "Service down" --title "Critical Alert"
noti send --to "pagertree://<integration_id>?urgency=critical" --message "Outage"
```

Parameters: `integration_id` (required — PagerTree integration ID), `urgency` (optional: `low`, `medium`, `high`, `critical` — default: `high`)

### SIGNL4

```bash
noti send --to "signl4://<team_secret>" --message "Server alert" --title "Alert"
noti send --to "signl4://<team_secret>?s4_severity=2&s4_service=noti-cli" --message "Critical!"
```

Parameters: `team_secret` (required — SIGNL4 team secret/webhook ID), `s4_severity` (optional: `0`=info, `1`=warning, `2`=critical), `s4_service` (optional — service name/category)

### Splunk On-Call (VictorOps)

```bash
noti send --to "victorops://<api_key>/<routing_key>" --message "Service degraded" --title "Warning"
noti send --to "victorops://<api_key>/<routing_key>?message_type=WARNING" --message "Alert"
```

Parameters: `api_key` (required — REST API key), `routing_key` (required — routing key), `message_type` (optional: `CRITICAL`, `WARNING`, `ACKNOWLEDGEMENT`, `INFO`, `RECOVERY` — default: `CRITICAL`)

### Spike.sh

```bash
noti send --provider spike --param webhook_url=https://hooks.spike.sh/custom/xxx --message "Alert"
```

Parameters: `webhook_url` (required — Spike.sh integration webhook URL)

### PopcornNotify SMS

```bash
noti send --to "popcorn://<api_key>@+15551234567/+15559876543" --message "Hello"
```

Parameters: `api_key` (required — PopcornNotify API key), `from` (required — sender phone), `to` (required — recipient phone)

### Firebase Cloud Messaging (FCM)

```bash
noti send --to "fcm://<server_key>/<device_token>" --message "Hello" --title "Alert"
noti send --provider fcm --param server_key=... --param topic=news --message "Breaking news!"
```

Parameters: `server_key` (required — FCM legacy API server key), `device_token` (required — target device registration token), `topic` (optional — FCM topic name, alternative to device_token), `condition` (optional — topic condition expression), `priority` (optional: `high` or `normal`, default: `high`), `collapse_key` (optional — message grouping key), `ttl` (optional — time-to-live in seconds), `icon` (optional), `sound` (optional, default: `default`), `click_action` (optional), `image` (optional — rich notification image URL)

### Threema Gateway

```bash
noti send --to "threema://*MY_GW:secret123@ABCD1234" --message "Hello securely"
noti send --provider threema --param gateway_id=*MY_GW --param api_secret=... --param to=ABCD1234 --message "Hello"
```

Parameters: `gateway_id` (required — Threema Gateway ID, starts with *), `api_secret` (required — API secret), `to` (required — recipient Threema ID, 8 characters), `to_phone` (optional — lookup by phone number), `to_email` (optional — lookup by email)

### ClickSend SMS

```bash
noti send --to "clicksend://user:apikey@+15551234567/+15559876543" --message "Hello"
```

Parameters: `username` (required — ClickSend account username), `api_key` (required — ClickSend API key), `to` (required — recipient phone E.164), `from` (optional — sender name or phone), `schedule` (optional — Unix timestamp for scheduled delivery)

### Brevo (Sendinblue) Email

```bash
noti send --to "brevo://<api_key>@sender@example.com/user@example.com" --message "Hello" --title "Subject"
```

Parameters: `api_key` (required — Brevo API key), `from` (required — sender email), `to` (required — recipient email), `from_name` (optional, default: noti), `to_name` (optional), `cc` (optional — comma-separated), `bcc` (optional — comma-separated), `reply_to` (optional)

### SMTP2Go Email

```bash
noti send --to "smtp2go://<api_key>@sender@example.com/user@example.com" --message "Hello" --title "Subject"
```

Parameters: `api_key` (required — SMTP2Go API key), `from` (required — sender email), `to` (required — recipient email), `cc` (optional — comma-separated), `bcc` (optional — comma-separated)

### Apprise API

```bash
# Stateful mode (uses persistent config on server)
noti send --to "apprise://localhost:8000/my-config" --message "Hello"
# Stateless mode (provide notification URLs directly)
noti send --provider apprise --param host=http://localhost:8000 --param urls="slack://token_a/token_b/token_c" --message "Hello"
```

Parameters: `host` (required — Apprise API server URL), `config_key` (optional — persistent configuration key), `urls` (optional — comma-separated Apprise notification URLs), `notification_type` (optional: `info`, `success`, `warning`, `failure`, default: `info`), `tag` (optional — filter tag)

### Free Mobile SMS (France)

```bash
noti send --to "freemobile://12345678/myapikey" --message "Bonjour!"
```

Parameters: `user` (required — Free Mobile user login / phone number), `password` (required — Free Mobile API key)

### httpSMS

```bash
noti send --to "httpsms://<api_key>@+15551234567/+15559876543" --message "Hello"
```

Parameters: `api_key` (required — httpSMS API key), `from` (required — sender phone number, your Android phone), `to` (required — recipient phone E.164), `encrypt` (optional: `true` or `false` — enable end-to-end encryption)

### MSG91 SMS (India)

```bash
noti send --to "msg91://<authkey>/NOTIAP/919876543210" --message "Hello"
```

Parameters: `authkey` (required — MSG91 authentication key), `sender` (required — sender ID, 6 characters), `to` (required — recipient phone with country code), `route` (optional: `1`=Promotional, `4`=Transactional, default: `4`), `country` (optional, default: `91`), `DLT_TE_ID` (optional — DLT template entity ID, required for India)

### Pushjet

```bash
noti send --to "pushjet://<secret_key>" --message "Hello" --title "Alert"
noti send --to "pushjet://<secret_key>?server=https://pushjet.example.com" --message "Hello"
```

Parameters: `secret` (required — Pushjet service secret key), `server` (optional — Pushjet server URL, default: `https://api.pushjet.io`), `level` (optional — importance level 1-5, default: 3), `link` (optional — URL to attach)

### SMSEagle

```bash
noti send --to "smseagle://<access_token>@192.168.1.100/+15559876543" --message "Hello"
```

Parameters: `host` (required — SMSEagle device host/IP), `access_token` (required — API access token), `to` (required — recipient phone), `scheme` (optional: `http` or `https`, default: `https`), `port` (optional), `priority` (optional: 0-9, default: 0)

### Seven (sms77) SMS

```bash
noti send --to "seven://<api_key>/+15559876543" --message "Hello"
```

Parameters: `api_key` (required — Seven.io API key), `to` (required — recipient phone E.164), `from` (optional — sender name or phone), `flash` (optional: `1` or `0` — flash SMS), `foreign_id` (optional — custom tracking ID)

### Web Push (VAPID)

```bash
noti send --provider webpush --param endpoint=https://push.example.com/sub/... --param p256dh=... --param auth=... --message "Hello"
```

Parameters: `endpoint` (required — push subscription endpoint URL), `p256dh` (required — push subscription p256dh key, base64url), `auth` (required — push subscription auth secret, base64url), `vapid_private` (optional — VAPID private key), `vapid_email` (optional — VAPID contact email), `ttl` (optional — time-to-live in seconds, default: 86400), `urgency` (optional: `very-low`, `low`, `normal`, `high`, default: `normal`)

### Emby

```bash
noti send --to "emby://<api_key>@localhost:8096" --message "Media ready" --title "Emby"
noti send --to "emby://<api_key>@localhost:8096/<user_id>" --message "Your show is ready"
```

Parameters: `api_key` (required — Emby API key), `host` (required — Emby server hostname:port), `user_id` (optional — target user ID), `scheme` (optional: `http` or `https`, default: `http`)

### Jellyfin

```bash
noti send --to "jellyfin://<api_key>@localhost:8096" --message "Media ready" --title "Jellyfin"
noti send --to "jellyfin://<api_key>@localhost:8096/<user_id>" --message "Your show is ready"
```

Parameters: `api_key` (required — Jellyfin API key), `host` (required — Jellyfin server hostname:port), `user_id` (optional — target user ID), `scheme` (optional: `http` or `https`, default: `http`)

### Pushcut (iOS)

```bash
noti send --to "pushcut://<api_key>/My%20Notification" --message "Task done" --title "Automation"
```

Parameters: `api_key` (required — Pushcut API key), `notification_name` (required — notification name), `url` (optional — URL to open on tap), `image` (optional — image URL), `sound` (optional — custom sound name)

### MQTT

```bash
noti send --to "mqtt://admin:password@broker.example.com:18083/noti/alerts" --message "Alert!"
noti send --to "mqtt://broker.example.com:18083/status" --message "System OK"
```

Parameters: `host` (required — MQTT broker HTTP API host:port), `topic` (required — MQTT topic), `user`, `password` (optional — broker credentials), `scheme` (optional: `http` or `https`, default: `http`), `qos` (optional: 0, 1, or 2, default: 0), `retain` (optional: `true`/`false`, default: `false`)

### VoIP.ms SMS

```bash
noti send --to "voipms://user@example.com:apipass@15551234567/15559876543" --message "Hello"
```

Parameters: `email` (required — VoIP.ms account email), `password` (required — VoIP.ms API password), `did` (required — source DID phone number), `to` (required — destination phone number)

### SFR SMS (France)

```bash
noti send --to "sfr://0612345678:password" --message "Bonjour!"
```

Parameters: `phone` (required — SFR phone number), `password` (required — SFR account password/API key)

### Pushed.co

```bash
noti send --to "pushed://<app_key>:<app_secret>" --message "Hello"
```

Parameters: `app_key`, `app_secret` (required — Pushed application credentials), `target_type` (optional: `app`, `channel`, or `pushed_id`, default: `app`), `target_alias` (optional — channel alias or pushed_id)

### Growl

```bash
noti send --to "growl://192.168.1.100" --message "Hello" --title "Alert"
noti send --to "growl://password@192.168.1.100:23053" --message "Hello"
```

Parameters: `host` (required — Growl host IP), `port` (optional, default: 23053), `password` (optional), `scheme` (optional: `http`/`https`, default: `http`), `priority` (optional: -2 to 2, default: 0), `sticky` (optional: `true`/`false`, default: `false`)

### Kumulos

```bash
noti send --to "kumulos://<api_key>:<server_key>" --message "Hello" --title "Push"
```

Parameters: `api_key`, `server_key` (required — Kumulos credentials), `channel` (optional — broadcast channel ID)

### Parse Platform

```bash
noti send --to "parse://<app_id>:<rest_api_key>@api.parse.com" --message "Hello" --title "Push"
noti send --to "parse://<app_id>:<rest_api_key>" --message "Hello"  # uses default host
```

Parameters: `app_id`, `rest_api_key` (required), `host` (optional, default: `api.parse.com`), `channel` (optional — push channel)

### Remote Syslog

```bash
noti send --to "rsyslog://logs.example.com/mytoken" --message "System event"
noti send --to "rsyslog://logs.example.com?severity=warning" --message "Disk space low"
```

Parameters: `host` (required — syslog HTTP relay host), `token` (optional — auth token), `port` (optional), `scheme` (optional: `http`/`https`, default: `https`), `facility` (optional, default: `user`), `severity` (optional: `emerg`, `alert`, `crit`, `err`, `warning`, `notice`, `info`, `debug`, default: `info`)

### SMS Manager

```bash
noti send --to "smsmanager://<api_key>@+15551234567/+15559876543" --message "Hello"
```

Parameters: `api_key` (required), `to` (required — destination phone), `from` (optional — sender ID)

### X (Twitter)

```bash
noti send --to "twitter://<bearer_token>" --message "Hello from noti!"
noti send --to "twitter://<bearer_token>?mode=dm&dm_user_id=12345" --message "Hi there"
```

Parameters: `bearer_token` (required — X API v2 Bearer token), `mode` (optional: `tweet` or `dm`, default: `tweet`), `dm_user_id` (optional — recipient user ID for DM mode)

### Boxcar (iOS/Android Push)

```bash
noti send --to "boxcar://<access_token>" --message "Hello" --title "Alert"
noti send --to "boxcar://<access_token>?sound=bird-1" --message "Hello"
```

Parameters: `access_token` (required — Boxcar user access token), `sound` (optional — notification sound name), `source_name` (optional — source name, default: noti), `icon_url` (optional — notification icon URL)

### DAPNET (Ham Radio Paging)

```bash
noti send --to "dapnet://DL1ABC:password@DL2DEF" --message "Hello"
noti send --to "dapnet://DL1ABC:password@DL2DEF?txgroup=dl-all" --message "QRV?"
```

Parameters: `callsign` (required — your DAPNET login callsign), `password` (required — DAPNET API password), `to` (required — recipient callsign), `txgroup` (optional — transmitter group, default: dl-all), `emergency` (optional: true/false, default: false). Note: Messages are truncated to 80 characters (POCSAG limit).

### Enigma2 (Satellite Receiver)

```bash
noti send --to "enigma2://192.168.1.50" --message "Hello" --title "Alert"
noti send --to "enigma2://admin:pass@192.168.1.50:80" --message "Recording started"
```

Parameters: `host` (required — Enigma2 device hostname/IP), `port` (optional, default: 80), `user`, `password` (optional — HTTP auth), `scheme` (optional: http/https, default: http), `timeout` (optional — display duration in seconds, default: 13, -1 for permanent), `msg_type` (optional: 1=yes/no, 2=info, 3=message, 4=attention, default: 1)

### Notifiarr

```bash
noti send --to "notifiarr://<api_key>" --message "Media ready" --title "Sonarr"
noti send --to "notifiarr://<api_key>?color=%2300FF00" --message "Build passed"
```

Parameters: `api_key` (required — Notifiarr API key), `notification_type` (optional, default: passthrough), `discord_channel` (optional — Discord channel ID for routing), `color` (optional — hex color), `ping_user` (optional — Discord user ID to ping), `ping_role` (optional — Discord role ID to ping), `image` (optional — image URL)

### Statuspage.io (Incident Management)

```bash
noti send --to "statuspage://<api_key>@<page_id>" --message "We are investigating the issue" --title "API Degradation"
noti send --to "statuspage://<api_key>@<page_id>?status=identified&impact=major" --message "Root cause found"
```

Parameters: `api_key` (required — Statuspage OAuth API key), `page_id` (required — Statuspage page ID), `status` (optional: investigating, identified, monitoring, resolved, default: investigating), `impact` (optional: none, minor, major, critical, default: minor), `component_ids` (optional — comma-separated component IDs), `component_status` (optional: operational, degraded_performance, partial_outage, major_outage)

### Dot. (IoT e-ink Display)

```bash
noti send --to "dot://<token>@<device_id>" --message "Meeting in 5 minutes"
noti send --to "dot://<token>@<device_id>?mode=image" --message "Weather: Sunny" --title "Dashboard"
```

Parameters: `token` (required — Dot. API token, starts with `dot_app_`), `device_id` (required — 12-character hex device serial), `signature` (optional — footer text on device), `mode` (optional: text (default) or image)

### Fluxer

```bash
noti send --to "fluxer://<webhook_id>/<webhook_token>" --message "Build passed" --title "CI"
noti send --to "fluxer://<webhook_id>/<webhook_token>?tts=true" --message "Alert!"
```

Parameters: `webhook_id` (required — Fluxer webhook ID), `webhook_token` (required — Fluxer webhook token), `botname` (optional — bot display name), `avatar_url` (optional — bot avatar URL), `tts` (optional: true/false — text-to-speech), `host` (optional — self-hosted Fluxer server URL)

### Microsoft Power Automate / Workflows

```bash
noti send --to "workflows://<host>/<workflow>/<signature>" --message "Deployment complete" --title "Release"
noti send --to "workflows://<host>:443/<workflow>/<signature>?api_version=2016-06-01" --message "Alert"
```

Parameters: `host` (required — Azure Logic Apps host, e.g. `prod-XX.westus.logic.azure.com`), `workflow` (required — workflow ID), `signature` (required — URL signature from `sig=` parameter), `port` (optional, default: 443), `api_version` (optional, default: 2016-06-01)

### NotificationAPI

```bash
noti send --to "napi://<client_id>/<client_secret>/<user_id>" --message "Order shipped" --title "Shipping"
noti send --to "napi://<client_id>/<client_secret>/<user_id>?region=eu" --message "GDPR alert"
```

Parameters: `client_id` (required — NotificationAPI client ID), `client_secret` (required — client secret), `user_id` (required — target user ID), `notification_type` (optional, default: apprise), `region` (optional: us (default), ca, eu)

### SpugPush

```bash
noti send --to "spugpush://<token>" --message "Server load high" --title "Alert"
```

Parameters: `token` (required — SpugPush authentication token, 32-64 chars)

## Message Formats

All providers support the `--format` flag:

```bash
noti send --to "..." --message "**bold text**" --format markdown
noti send --to "..." --message "<b>bold</b>" --format html
noti send --to "..." --message "plain text" --format text
```

Not all providers support all formats — unsupported formats fall back to text.
