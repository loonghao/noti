# URL Schemes Reference

All providers use a consistent URL scheme format: `provider://credentials[?options]`

## Quick Reference

| Provider | URL Format | Key Parameters |
|:---------|:----------|:---------------|
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
| Matrix | `matrix://<access_token>/<room_id>` | `access_token`, `room_id` (required) |
| Twilio | `twilio://<sid>:<token>@<from>/<to>` | `account_sid`, `auth_token`, `from`, `to` (required) |
| Zulip | `zulip://<email>:<key>@<domain>` | `bot_email`, `api_key`, `domain` (required) |
| Webex | `webex://<access_token>/<room_id>` | `access_token`, `room_id` (required) |
| LINE | `line://<access_token>` | `access_token` (required) |
| Revolt | `revolt://<bot_token>/<channel_id>` | `bot_token`, `channel_id` (required) |
| PushBullet | `pushbullet://<access_token>` | `access_token` (required) |
| SimplePush | `simplepush://<key>` | `key` (required) |
| IFTTT | `ifttt://<webhook_key>/<event_name>` | `webhook_key`, `event` (required) |
| PagerDuty | `pagerduty://<integration_key>` | `integration_key` (required) |
| Vonage | `vonage://<key>:<secret>@<from>/<to>` | `api_key`, `api_secret`, `from`, `to` (required) |
| Mailgun | `mailgun://<api_key>@<domain>/<to>` | `api_key`, `domain` (required) |
| Opsgenie | `opsgenie://<api_key>` | `api_key` (required) |
| SendGrid | `sendgrid://<api_key>@<from>/<to>` | `api_key`, `from`, `to` (required) |
| Notica | `notica://<token>` | `token` (required) |
| Mastodon | `mastodon://<access_token>@<instance>` | `access_token`, `instance` (required) |
| JSON Webhook | `json://<host>/<path>` | `url` (required) |
| Form Webhook | `form://<host>/<path>` | `url` (required) |
| XML Webhook | `xml://<host>/<path>` | `url` (required) |
| Bluesky | `bluesky://<handle>:<app_password>` | `handle`, `app_password` (required) |
| Home Assistant | `hassio://<access_token>@<host>` | `access_token`, `host` (required) |
| Signal | `signal://<from>/<to>` | `from`, `to` (required) |
| WhatsApp | `whatsapp://<access_token>@<phone_id>/<to>` | `access_token`, `phone_number_id`, `to` (required) |
| MQTT | `mqtt://<user>:<pass>@<host>/<topic>` | `host`, `topic` (required) |
| AWS SES | `ses://<key>:<secret>@<region>/<from>/<to>` | `access_key`, `secret_key`, `region`, `from`, `to` (required) |
| AWS SNS | `sns://<key>:<secret>@<region>/<topic_arn>` | `access_key`, `secret_key`, `region`, `topic_arn` (required) |

::: tip
Use `noti --json providers info <provider>` to see the full parameter list for any provider.
:::
