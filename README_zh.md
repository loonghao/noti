<div align="center">

<img src="https://img.shields.io/badge/📢-noti-blue?style=for-the-badge&labelColor=1a1a2e&color=16213e" alt="noti logo" height="60"/>

# noti

**统一的多渠道通知 CLI — 为 AI Agent 和自动化而生**

*通过一条命令向 125+ 服务发送通知。*

<br/>

[![CI](https://github.com/loonghao/noti/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/noti/actions/workflows/ci.yml)
[![Release](https://github.com/loonghao/noti/actions/workflows/release.yml/badge.svg)](https://github.com/loonghao/noti/actions/workflows/release.yml)
[![Codecov](https://codecov.io/gh/loonghao/noti/graph/badge.svg)](https://codecov.io/gh/loonghao/noti)
[![GitHub release](https://img.shields.io/github/v/release/loonghao/noti?include_prereleases&logo=github&label=release)](https://github.com/loonghao/noti/releases)
[![GitHub Downloads](https://img.shields.io/github/downloads/loonghao/noti/total?logo=github&color=green&label=downloads)](https://github.com/loonghao/noti/releases)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.85+-dea584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Crates](https://img.shields.io/badge/crates-3_workspace-e6893c?style=flat-square&logo=rust)](https://github.com/loonghao/noti)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square)](https://github.com/loonghao/noti/pulls)
[![GitHub Stars](https://img.shields.io/github/stars/loonghao/noti?style=flat-square&logo=github&color=yellow)](https://github.com/loonghao/noti/stargazers)

[![Windows](https://img.shields.io/badge/Windows-x86__64-0078D6?style=flat-square&logo=windows&logoColor=white)](#-安装)
[![macOS](https://img.shields.io/badge/macOS-x86__64%20|%20ARM64-000000?style=flat-square&logo=apple&logoColor=white)](#-安装)
[![Linux](https://img.shields.io/badge/Linux-x86__64-FCC624?style=flat-square&logo=linux&logoColor=black)](#-安装)

<br/>

[English](README.md) · [简体中文](README_zh.md)

</div>

---

<br/>

## 🎯 为什么选择 noti？

> **一个二进制文件。一条命令。125+ 通知渠道。**

大多数项目需要拼凑各种 SDK 来处理 Slack、邮件、短信和推送通知。**noti** 用一个极速的 Rust CLI 替代了所有这些，它使用通用的 URL Scheme 寻址 — 完美适配 Shell 脚本、CI 流水线和 AI Agent。

```bash
# 就这么简单。一行命令。任意渠道。
noti send --to "wecom://<key>" --message "部署完成 ✅"
noti send --to "slack://<token>" --message "构建通过 🎉"
noti send --to "tg://<bot>/<chat>" --message "告警：CPU > 90%"
```

<br/>

## ✨ 亮点

<table>
<tr>
<td width="50%" valign="top">

### 🚀 CLI 优先设计
为 Agent/脚本集成而生 — 一行命令，无需配置文件。在 CI/CD 流水线中完美运行。

### 📡 125+ 内置渠道
聊天、短信、邮件、推送、Webhook、事件管理、IoT — 全部集成在一个二进制文件中。无插件，无扩展。

### 🔗 URL Scheme 寻址
直观的 `provider://credentials` 格式 — `wecom://key`、`slack://tokens`、`tg://bot/chat`。零学习成本。

</td>
<td width="50%" valign="top">

### 📋 Profile 管理
保存、复用、测试通知配置 — 一次设定，按名使用。团队间轻松共享。

### 🤖 机器友好输出
`--json` 标志输出结构化机器可读数据 — 确定性退出码确保可靠自动化。

### ⚡ 极速响应
原生 Rust 二进制，**< 10ms 启动** — 无运行时，无解释器，无依赖地狱。

</td>
</tr>
</table>

<br/>

## 📦 安装

### 安装脚本（推荐）

**macOS / Linux：**

```bash
curl -fsSL https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.sh | bash
```

**Windows (PowerShell)：**

```powershell
irm https://raw.githubusercontent.com/loonghao/noti/main/scripts/install.ps1 | iex
```

### 从源码安装

```bash
cargo install --git https://github.com/loonghao/noti noti-cli
```

### 下载二进制

所有平台的预编译二进制文件可在 [Releases](https://github.com/loonghao/noti/releases) 页面下载：

| 平台 | 架构 | 文件 |
|:-----|:-----|:-----|
| **Windows** | x86_64 | [`noti-x86_64-pc-windows-msvc.zip`](https://github.com/loonghao/noti/releases/latest) |
| **macOS** | x86_64 (Intel) | [`noti-x86_64-apple-darwin.tar.gz`](https://github.com/loonghao/noti/releases/latest) |
| **macOS** | ARM64 (Apple Silicon) | [`noti-aarch64-apple-darwin.tar.gz`](https://github.com/loonghao/noti/releases/latest) |
| **Linux** | x86_64 | [`noti-x86_64-unknown-linux-gnu.tar.gz`](https://github.com/loonghao/noti/releases/latest) |

<br/>

## 🚀 快速开始

### 1. 通过 URL Scheme 发送

```bash
# 企业微信
noti send --to "wecom://<webhook_key>" --message "来自 noti 的问候！"

# 飞书 / Lark
noti send --to "feishu://<hook_id>" --message "来自 noti 的问候！"

# Slack
noti send --to "slack://<token_a>/<token_b>/<token_c>" --message "Hello!"

# Telegram
noti send --to "tg://<bot_token>/<chat_id>" --message "Hello!"

# Discord
noti send --to "discord://<webhook_id>/<webhook_token>" --message "Hello!"

# 邮件 (SMTP)
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=recipient@example.com" \
  --message "Hello!" --title "测试邮件"

# 通用 Webhook
noti send --to "webhook://example.com/api/notify" --message "Hello!"
```

### 2. 通过保存的 Profile 发送

```bash
# 保存 Profile
noti config set --name my-team --provider wecom --param key=<webhook_key>

# 使用 Profile 发送
noti send --profile my-team --message "来自 noti 的问候！"

# 测试 Profile
noti config test my-team
```

### 3. 直接指定 Provider 参数发送

```bash
noti send --provider wecom --param key=<webhook_key> --message "Hello!"
```

### 4. JSON 输出（适合 Agent）

```bash
noti --json send --to "wecom://<key>" --message "部署完成"
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

<br/>

## 📡 支持的渠道 (125)

<details>
<summary><strong>💬 聊天 & 即时通讯 — 22 个渠道</strong></summary>

<br/>

| 渠道 | Scheme | 说明 |
|:-----|:-------|:-----|
| 企业微信 | `wecom://` | 企业微信群机器人 Webhook |
| 飞书 | `feishu://` | 飞书/Lark 群机器人 Webhook |
| 钉钉 | `dingtalk://` | 钉钉群机器人 Webhook |
| Slack | `slack://` | Slack Incoming Webhooks |
| Telegram | `tg://` | Telegram Bot API |
| Discord | `discord://` | Discord Webhooks |
| Teams | `teams://` | Microsoft Teams Incoming Webhook |
| Google Chat | `gchat://` | Google Chat 空间 Webhook |
| Mattermost | `mattermost://` | Mattermost Incoming Webhook |
| Rocket.Chat | `rocketchat://` | Rocket.Chat Incoming Webhook |
| Matrix | `matrix://` | Matrix Client-Server API |
| Zulip | `zulip://` | Zulip 团队聊天 Bot API |
| Webex | `webex://` | Cisco Webex Teams 消息 |
| LINE | `line://` | LINE Notify 推送 |
| Revolt | `revolt://` | Revolt 开源聊天平台 |
| Mastodon | `mastodon://` | Mastodon 状态发布 (嘟文) |
| Ryver | `ryver://` | Ryver 团队消息 |
| Twist | `twist://` | Twist 异步团队消息 |
| Flock | `flock://` | Flock 团队消息 |
| Gitter | `gitter://` | Gitter 开发者聊天 |
| Guilded | `guilded://` | Guilded 游戏社区聊天 |
| Misskey | `misskey://` | Misskey 联邦宇宙笔记发布 |

</details>

<details>
<summary><strong>🔔 推送通知 — 20 个渠道</strong></summary>

<br/>

| 渠道 | Scheme | 说明 |
|:-----|:-------|:-----|
| Pushover | `pushover://` | Pushover 推送通知 |
| ntfy | `ntfy://` | ntfy.sh 推送通知 |
| Gotify | `gotify://` | Gotify 自托管推送 |
| Bark | `bark://` | Bark iOS 推送通知 |
| PushDeer | `pushdeer://` | PushDeer 跨平台推送 |
| Server酱 | `serverchan://` | Server酱微信推送 |
| PushBullet | `pushbullet://` | PushBullet 跨平台推送 |
| SimplePush | `simplepush://` | SimplePush.io 推送通知 |
| Notica | `notica://` | Notica 浏览器推送通知 |
| Prowl | `prowl://` | Prowl iOS 推送通知 |
| Join | `join://` | Join by joaoapps 推送通知 |
| Pushsafer | `pushsafer://` | Pushsafer 推送通知 |
| OneSignal | `onesignal://` | OneSignal 推送通知平台 |
| Techulus Push | `push://` | Techulus Push 推送通知 |
| Pushy | `pushy://` | Pushy 跨平台推送通知 |
| Chanify | `chanify://` | Chanify iOS/Android 推送通知 |
| Pushplus | `pushplus://` | Pushplus 推送通知（微信/短信/邮件） |
| WxPusher | `wxpusher://` | WxPusher 微信推送通知 |
| FCM | `fcm://` | Firebase Cloud Messaging 推送通知 |
| Pushjet | `pushjet://` | Pushjet 推送通知服务 |

</details>

<details>
<summary><strong>📱 短信 & 消息 — 17 个渠道</strong></summary>

<br/>

| 渠道 | Scheme | 说明 |
|:-----|:-------|:-----|
| Twilio | `twilio://` | Twilio 短信 REST API |
| Vonage | `vonage://` | Vonage (Nexmo) 短信 API |
| D7 Networks | `d7sms://` | D7 Networks 短信网关 |
| Sinch | `sinch://` | Sinch 短信 REST API |
| Clickatell | `clickatell://` | Clickatell 短信/消息网关 |
| BulkSMS | `bulksms://` | BulkSMS 短信网关 REST API v2 |
| Kavenegar | `kavenegar://` | Kavenegar 短信网关 (伊朗) |
| MessageBird | `msgbird://` | MessageBird 短信 REST API |
| Plivo | `plivo://` | Plivo 短信 REST API |
| BurstSMS | `burstsms://` | BurstSMS (Transmit SMS) 短信网关 |
| PopcornNotify | `popcorn://` | PopcornNotify 短信服务 |
| ClickSend | `clicksend://` | ClickSend 短信网关 |
| Seven | `seven://` | Seven (sms77) 短信网关 |
| SMSEagle | `smseagle://` | SMSEagle 硬件短信网关 |
| httpSMS | `httpsms://` | httpSMS — 通过安卓手机发送短信 |
| MSG91 | `msg91://` | MSG91 短信网关 (印度) |
| Free Mobile | `freemobile://` | Free Mobile 短信 (法国) |

</details>

<details>
<summary><strong>📧 邮件 — 8 个渠道</strong></summary>

<br/>

| 渠道 | Scheme | 说明 |
|:-----|:-------|:-----|
| 邮件 | `smtp://` | 通过 SMTP 发送邮件 |
| Mailgun | `mailgun://` | Mailgun 事务邮件 API |
| SendGrid | `sendgrid://` | SendGrid 事务邮件 API v3 |
| SparkPost | `sparkpost://` | SparkPost 事务邮件 API |
| AWS SES | `ses://` | AWS SES 事务邮件 |
| Resend | `resend://` | Resend 现代邮件 API |
| Brevo | `brevo://` | Brevo (Sendinblue) 事务邮件 |
| SMTP2Go | `smtp2go://` | SMTP2Go 事务邮件 |

</details>

<details>
<summary><strong>🌐 Webhook — 4 个渠道</strong></summary>

<br/>

| 渠道 | Scheme | 说明 |
|:-----|:-------|:-----|
| Webhook | `webhook://` | 通用 HTTP Webhook |
| JSON Webhook | `json://` | 通用 JSON Webhook |
| Form Webhook | `form://` | 通用表单 Webhook |
| XML Webhook | `xml://` | 通用 XML Webhook |

</details>

<details>
<summary><strong>🚨 事件管理 & 自动化 — 7 个渠道</strong></summary>

<br/>

| 渠道 | Scheme | 说明 |
|:-----|:-------|:-----|
| IFTTT | `ifttt://` | IFTTT Maker Webhooks |
| PagerDuty | `pagerduty://` | PagerDuty 事件 API v2 |
| Opsgenie | `opsgenie://` | Atlassian Opsgenie 告警 API v2 |
| PagerTree | `pagertree://` | PagerTree 事件管理 |
| SIGNL4 | `signl4://` | SIGNL4 移动告警 |
| Splunk On-Call | `victorops://` | Splunk On-Call (VictorOps) 事件管理 |
| Spike.sh | `spike://` | Spike.sh 事件管理和告警 |

</details>

<details>
<summary><strong>🏠 IoT、媒体及其他 — 47 个渠道</strong></summary>

<br/>

| 渠道 | Scheme | 说明 |
|:-----|:-------|:-----|
| Bluesky | `bluesky://` | Bluesky 社交网络 (AT Protocol) |
| Home Assistant | `hassio://` | Home Assistant 通知 |
| LaMetric | `lametric://` | LaMetric Time 智能时钟 |
| LunaSea | `lunasea://` | LunaSea 自托管媒体推送 |
| Nextcloud | `ncloud://` | Nextcloud 推送通知 |
| Signal | `signal://` | Signal Messenger (signal-cli REST API) |
| Reddit | `reddit://` | Reddit 私信 |
| Threema | `threema://` | Threema Gateway 安全消息 |
| Apprise API | `apprise://` | Apprise API 通知中继 |
| Web Push | `webpush://` | Web Push (VAPID) 浏览器推送通知 |
| WhatsApp | `whatsapp://` | WhatsApp Business Cloud API 消息 |
| Kodi | `kodi://` | Kodi (XBMC) 屏幕通知 JSON-RPC |
| Notifico | `notifico://` | Notifico 自托管通知服务 |
| 46elks | `46elks://` | 46elks 短信 REST API |
| BulkVS | `bulkvs://` | BulkVS 短信 REST API |
| Jira | `jira://` | Jira 问题评论 REST API v3 |
| PushMe | `pushme://` | PushMe 推送通知服务 |
| SendPulse | `sendpulse://` | SendPulse 事务邮件 SMTP API |
| Streamlabs | `streamlabs://` | Streamlabs 直播告警 |
| Synology Chat | `synology://` | Synology Chat 传入 Webhook |
| Africa's Talking | `africastalking://` | Africa's Talking 短信网关 |
| Office 365 | `o365://` | Office 365 / Outlook 邮件 Microsoft Graph API |
| Nextcloud Talk | `nctalk://` | Nextcloud Talk 聊天消息 OCS API |
| Emby | `emby://` | Emby 媒体服务器通知 REST API |
| Jellyfin | `jellyfin://` | Jellyfin 媒体服务器通知 REST API |
| Pushcut | `pushcut://` | Pushcut iOS 自动化通知 |
| MQTT | `mqtt://` | MQTT 通过代理 HTTP API 发布消息 |
| VoIP.ms | `voipms://` | VoIP.ms 短信 REST API |
| SFR | `sfr://` | SFR 免费短信通知 (法国运营商) |
| Pushed | `pushed://` | Pushed.co 推送通知 REST API |
| Growl | `growl://` | Growl 桌面通知 GNTP/HTTP |
| Kumulos | `kumulos://` | Kumulos 推送通知 Push API |
| Parse | `parse://` | Parse Platform 推送通知 REST API |
| Remote Syslog | `rsyslog://` | 远程 Syslog 通知 HTTP 中继 |
| SMS Manager | `smsmanager://` | SMS Manager 批量短信 |
| X (Twitter) | `twitter://` | X (Twitter) 发推/私信 API v2 |
| Boxcar | `boxcar://` | Boxcar iOS/Android 推送通知 |
| DAPNET | `dapnet://` | DAPNET 业余无线电寻呼网络 |
| Enigma2 | `enigma2://` | Enigma2 卫星接收器屏幕通知 OpenWebif |
| Notifiarr | `notifiarr://` | Notifiarr 媒体服务器通知聚合 |
| Statuspage | `statuspage://` | Atlassian Statuspage.io 事件管理 |
| Dot. | `dot://` | Dot. IoT 电子墨水屏通知 |
| Fluxer | `fluxer://` | Fluxer Webhook 通知 (Discord 风格) |
| Workflows | `workflows://` | Microsoft Power Automate / Workflows (自适应卡片) |
| NotificationAPI | `napi://` | NotificationAPI 多渠道通知 |
| SpugPush | `spugpush://` | SpugPush Webhook 通知 (Spug 监控) |
| AWS SNS | `sns://` | AWS SNS 主题推送 |

</details>

### 列出所有渠道

```bash
noti providers list          # 列出所有可用渠道
noti providers info wecom    # 查看渠道详情和参数
```

<br/>

## ⚙️ 配置

配置文件路径：`~/.config/noti/config.toml`

通过环境变量覆盖：`NOTI_CONFIG=/path/to/config.toml`

### Profile 管理

```bash
noti config set --name <name> --provider <provider> --param key=value
noti config get <name>
noti config list
noti config remove <name>
noti config test <name>
noti config path
```

<br/>

## 📊 退出码

| 退出码 | 含义 | 使用场景 |
|:------|:-----|:--------|
| `0` | ✅ 成功 | 消息发送成功 |
| `1` | ❌ 发送失败 | 网络错误、API 拒绝 |
| `2` | ⚠️ 配置错误 | 参数缺失、Profile 无效 |

<br/>

## 🤖 Agent 集成

noti 专为 [OpenClaw](https://github.com/nicepkg/openclaw) 等 AI Agent 设计。核心设计理念：

| 特性 | 优势 |
|:-----|:-----|
| **URL Scheme** | 一行寻址 — 无需配置文件 |
| **`--json` 标志** | 结构化输出，便于可靠解析 |
| **退出码** | 确定性的成功/失败信号 |
| **Profile 系统** | 预配置一次，按名称使用 |
| **`providers list`** | 自发现 — Agent 可枚举所有渠道 |

### 示例：OpenClaw / AI Agent 工作流

```bash
# 1. Agent 发现可用渠道
noti --json providers list

# 2. Agent 查看渠道参数
noti --json providers info wecom

# 3. Agent 发送通知
noti --json send --to "wecom://key123" --message "任务完成"

# 4. Agent 检查结果
echo $?  # 0 = 成功, 1 = 失败, 2 = 配置错误
```

### OpenClaw 技能

noti 内置了 [OpenClaw](https://github.com/nicepkg/openclaw) 技能定义（位于 `skills/noti-openclaw/`）— Agent 可自动发现 noti 的能力。

<br/>

## 🏗️ 项目架构

```
noti/
├── crates/
│   ├── noti-cli/        # CLI 二进制 — 参数解析、输出格式化
│   ├── noti-core/       # 核心抽象 — Provider trait、Registry、URL 解析
│   └── noti-providers/  # 125 个渠道实现（每个渠道一个文件）
├── scripts/             # 安装脚本 (bash, PowerShell) & 工具
├── skills/              # OpenClaw 技能定义
└── justfile             # 任务运行配方（通过 vx）
```

<br/>

## 🛠️ 开发

### 前置条件

- [vx](https://github.com/loonghao/vx) — 通用工具版本管理器
- Rust 1.85+（stable，由 vx 管理）

### 命令

```bash
vx just fmt          # 格式化代码
vx just check        # 类型检查
vx just lint         # Clippy 检查
vx just test         # 运行测试
vx just coverage     # 生成覆盖率报告
vx just ci           # 完整 CI 流程 (fmt + check + lint + test)
vx just run -- send --help   # 开发模式运行 CLI
```

<br/>

## 🙏 致谢

noti 的灵感来自以下优秀项目：

- [notifiers](https://github.com/liiight/notifiers) — 开启一切的 Python 通知库
- [Apprise](https://github.com/caronc/apprise) — 统一 API 的推送通知工具
- [OpenClaw](https://github.com/nicepkg/openclaw) — noti 为之而生的 AI Agent 框架

<br/>

## 📄 许可证

[MIT](LICENSE) © [Hal Long](https://github.com/loonghao)

---

<div align="center">

**[⬆ 回到顶部](#noti)**

以 ❤️ 用 Rust 打造

</div>
