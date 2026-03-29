# Chat & IM Providers

22 providers for chat and instant messaging platforms.

| Provider | Scheme | Description |
|:---------|:-------|:------------|
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

## Examples

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

### DingTalk (钉钉)

```bash
noti send --to "dingtalk://<access_token>" --message "Hello"
noti send --to "dingtalk://<access_token>?secret=<secret>" --message "Hello"
```

Parameters: `access_token` (required), `secret` (optional — for signed mode)

### Microsoft Teams

```bash
noti send --to "teams://<webhook_url_encoded>" --message "Hello"
```

Parameters: `webhook_url` (required), `theme_color` (optional)

### Google Chat

```bash
noti send --to "gchat://<space>/<key>/<token>" --message "Hello"
```

Parameters: `webhook_url` (required)
