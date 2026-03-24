# noti

统一的多渠道通知 CLI 工具，专为 AI Agent 设计。

[![CI](https://github.com/loonghao/noti/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/noti/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 概述

**noti** 是一个 Rust CLI 工具，灵感来自 [notifiers](https://github.com/liiight/notifiers) 和 [Apprise](https://github.com/caronc/apprise)。它提供了统一的接口，通过多个渠道发送通知，专门为 [OpenClaw](https://github.com/nicepkg/openclaw) 等 AI Agent 设计。

### 核心特性

- 🚀 **CLI 优先** — 为 Agent/脚本集成而设计
- 📡 **7 个内置渠道** — 企业微信、飞书、Slack、Telegram、Discord、邮件、Webhook
- 🔗 **URL Scheme 寻址** — `wecom://<key>`、`slack://<tokens>` 等
- 📋 **Profile 管理** — 保存和复用通知配置
- 🤖 **JSON 输出** — `--json` 标志输出结构化机器可读数据
- ⚡ **极速** — 原生 Rust 二进制，瞬间启动

## 安装

### 从源码安装

```bash
cargo install --git https://github.com/loonghao/noti noti-cli
```

## 快速开始

### 通过 URL Scheme 发送

```bash
# 企业微信
noti send --to "wecom://<webhook_key>" --message "来自 noti 的问候！"

# 飞书
noti send --to "feishu://<hook_id>" --message "来自 noti 的问候！"

# Slack
noti send --to "slack://<token_a>/<token_b>/<token_c>" --message "Hello!"

# Telegram
noti send --to "tg://<bot_token>/<chat_id>" --message "Hello!"

# Discord
noti send --to "discord://<webhook_id>/<webhook_token>" --message "Hello!"

# 邮件
noti send --to "smtp://user:pass@smtp.gmail.com:587?to=recipient@example.com" \
  --message "Hello!" --title "测试邮件"

# 通用 Webhook
noti send --to "webhook://example.com/api/notify" --message "Hello!"
```

### 通过保存的 Profile 发送

```bash
# 保存 Profile
noti config set --name my-team --provider wecom --param key=<webhook_key>

# 使用 Profile 发送
noti send --profile my-team --message "来自 noti 的问候！"

# 测试 Profile
noti config test my-team
```

### 直接指定 Provider 参数发送

```bash
noti send --provider wecom --param key=<webhook_key> --message "Hello!"
```

### JSON 输出（适合 Agent）

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

## 支持的渠道

| 渠道 | Scheme | 说明 |
|------|--------|------|
| 企业微信 | `wecom://` | 企业微信群机器人 Webhook |
| 飞书 | `feishu://` | 飞书/Lark 群机器人 Webhook |
| Slack | `slack://` | Slack Incoming Webhooks |
| Telegram | `tg://` | Telegram Bot API |
| Discord | `discord://` | Discord Webhooks |
| 邮件 | `smtp://` | 通过 SMTP 发送邮件 |
| Webhook | `webhook://` | 通用 HTTP Webhook |

### 列出所有渠道

```bash
noti providers list
noti providers info wecom
```

## 配置

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

## 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | 发送失败（网络/API 错误） |
| 2 | 参数/配置错误 |

## Agent 集成

noti 专为 AI Agent 设计。关键特性：

1. **URL Scheme** — 一行寻址，无需配置文件
2. **`--json` 标志** — 结构化输出便于解析
3. **退出码** — 确定性的成功/失败信号
4. **Profile 系统** — 预配置一次，按名称使用

### 示例：OpenClaw 集成

```bash
# Agent 可以发现可用的渠道
noti --json providers list

# Agent 可以查看渠道参数
noti --json providers info wecom

# Agent 发送通知
noti --json send --to "wecom://key123" --message "任务完成"
```

## 开发

### 前置条件

- [vx](https://github.com/loonghao/vx) — 工具版本管理器
- Rust（stable，由 vx 管理）

### 命令

```bash
vx just fmt          # 格式化代码
vx just check        # 类型检查
vx just lint         # Clippy 检查
vx just test         # 运行测试
vx just ci           # 完整 CI 流程
vx just run -- send --help   # 运行 CLI
```

## 许可证

MIT
