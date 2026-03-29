# Push Notification Providers

20 providers for push notifications to mobile devices, desktops, and browsers.

| Provider | Scheme | Description |
|:---------|:-------|:------------|
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

## Examples

### Pushover

```bash
noti send --to "pushover://<user_key>/<api_token>" --message "Hello"
```

### ntfy

```bash
noti send --to "ntfy://<topic>" --message "Hello"
noti send --to "ntfy://<topic>?server=https://ntfy.example.com" --message "Hello"
```

### Gotify

```bash
noti send --to "gotify://<host>/<app_token>" --message "Hello"
```

### Bark (iOS)

```bash
noti send --to "bark://<device_key>" --message "Hello"
```

### ServerChan (Server酱)

```bash
noti send --to "serverchan://<send_key>" --message "Hello"
```
