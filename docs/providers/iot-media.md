# IoT, Media & More Providers

47 providers covering IoT devices, media servers, social platforms, and more.

| Provider | Scheme | Description |
|:---------|:-------|:------------|
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
| MQTT | `mqtt://` | MQTT publish via broker HTTP API |
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
| Enigma2 | `enigma2://` | Enigma2 satellite receiver on-screen notifications |
| Notifiarr | `notifiarr://` | Notifiarr media server notification aggregation |
| Statuspage | `statuspage://` | Atlassian Statuspage.io incident management |
| Dot. | `dot://` | Dot. IoT e-ink display notifications |
| Fluxer | `fluxer://` | Fluxer webhook notifications (Discord-style) |
| Workflows | `workflows://` | Microsoft Power Automate / Workflows |
| NotificationAPI | `napi://` | NotificationAPI multi-channel notifications |
| SpugPush | `spugpush://` | SpugPush webhook notifications |
| AWS SNS | `sns://` | AWS SNS topic publishing |

## Highlighted Examples

### Bluesky

```bash
noti send --to "bluesky://user.bsky.social:xxxx-xxxx-xxxx-xxxx" --message "Hello Bluesky!"
```

### Home Assistant

```bash
noti send --to "hassio://<access_token>@homeassistant.local:8123" --message "Motion detected"
```

### WhatsApp Business

```bash
noti send --to "whatsapp://<access_token>@<phone_number_id>/<to>" --message "Hello"
```

### MQTT

```bash
noti send --to "mqtt://admin:password@broker.example.com:18083/noti/alerts" --message "Alert!"
```

### Jira

```bash
noti send --to "jira://<user>:<api_token>@<host>/<issue_key>" --message "Build complete"
```
