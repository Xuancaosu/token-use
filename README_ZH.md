<div align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="Token Use 图标">

# Token Use

面向 macOS 的 Token 使用统计工具，保留 CC Switch 的用量采集能力，并加入可选 GitHub 排行。

[![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)](https://github.com/Xuancaosu/token-use/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-active%20development-2ea44f.svg)](#项目状态)

[快速开始](#快速开始) | [后端部署](#排行后端) | [开发](#开发) | [Fork 声明](#fork-声明)
</div>

## 项目简介

Token Use 是 [CC Switch](https://github.com/farion1231/cc-switch) 的一个聚焦型 fork。它砍除了 CC Switch 中的供应商切换、多工具配置、MCP、Prompt、Skill、代理和同步等大多数能力，只保留并强化本产品需要的核心能力：Token 使用统计和本地用量观测。

Token Use 当前是一个轻量 macOS 菜单栏 + 桌面应用：

- 本地优先的 Token 消耗统计
- 菜单栏展示今日消耗，用紧凑单位显示
- 统计与排行分开展示
- 排行需要 GitHub 登录
- 排行完全由用户选择是否加入
- 排行维度支持当天、7 天、30 天
- 后端可 Docker 自部署

当前产品定位是 macOS 优先。代码库仍继承了一部分 CC Switch 的跨平台构建脚手架，但这个 fork 的产品体验和 UI 都按 macOS 应用设计。

## 为什么做 Token Use

AI 编程工具很容易在用户无感的情况下消耗大量 Token。Token Use 只回答日常使用中最关键的几个问题：

- 我今天用了多少 Token？
- 最近 7 天或 30 天的趋势如何？
- 输入、输出、缓存读取、缓存创建分别占多少？
- 我是否愿意和其他选择加入的用户做用量排行？

这个项目不是账号切换器，也不是 AI 网关。它只是一个轻量、安静、可长期放在菜单栏里的用量监控工具。

## 功能

### 本地统计

- 复用 CC Switch 继承下来的本地用量采集来源。
- 聚合当天、7 天、30 天窗口。
- 展示总 Token、请求数、成本估算、缓存命中率和 Token 构成。
- 默认只在本地保存统计与请求历史。
- macOS 状态栏展示今日用量，格式如 `999`、`1.2K`、`12.4M`、`1.2B`。

### 排行

- 使用 GitHub Device Flow 登录。
- 未登录时无法查看排行。
- 用户必须主动选择加入后，自己的用量才会进入排行。
- 排行页面与统计页面完全分开。
- 排行按总 Token 消耗计算：
  - 输入 Token
  - 输出 Token
  - 缓存读取 Token
  - 缓存创建 Token
- 支持当天、7 天、30 天排行窗口。

### 后端

- 轻量 Node.js HTTP 服务，无运行时 npm 依赖。
- Docker 优先部署。
- 默认用 JSON 文件持久化，适合早期轻量版本。
- 当前公开服务域名：`https://token-use.lucsun.cn`。
- 容器对外端口：`6655`。

## 截图

第一个公开签名版本发布后会补充 macOS 截图。

当前 Tauri 构建已经可以产出完整的 macOS `.app` 和 `.dmg`。

## 项目状态

Token Use 仍处在早期活跃开发阶段。

已经适合：

- macOS 本地用量查看
- 开发环境使用
- 自部署后端测试
- GitHub Device Flow 登录测试

尚未最终完善：

- 签名后的公开发布流程
- 自动更新签名
- 完整发布说明
- 生产级多节点后端存储

## 快速开始

### 环境要求

- macOS 12 或更高版本
- Node.js 20 或兼容版本
- pnpm
- Rust 1.85 或更高版本
- Tauri 2 macOS 开发依赖

### 安装依赖

```bash
pnpm install
```

### 启动开发环境

```bash
pnpm dev
```

### 构建 macOS App

```bash
pnpm build
```

构建产物：

```text
src-tauri/target/release/bundle/macos/Token Use.app
src-tauri/target/release/bundle/dmg/Token Use_0.1.0_aarch64.dmg
```

## 排行后端

App 默认连接：

```text
https://token-use.lucsun.cn
```

开发时可以覆盖后端地址：

```bash
TOKEN_USE_BACKEND_URL=http://127.0.0.1:6655 pnpm dev
```

### 本地运行

```bash
cd backend
docker compose up -d --build
curl http://127.0.0.1:6655/healthz
```

### 部署到当前服务器

仓库包含当前后端服务器的部署脚本：

```bash
REMOTE_USER=root ./scripts/deploy-token-use-backend.sh
```

默认值：

- 主机：`10.31.0.10`
- 远端目录：`/opt/token-use-backend`
- 公开域名：`https://token-use.lucsun.cn`
- 容器端口：`6655`

### API

```text
GET  /healthz
POST /api/github/device/start
POST /api/github/device/poll
GET  /api/me
POST /api/me/opt-in
POST /api/leaderboard/snapshot
GET  /api/leaderboard?range=today|7d|30d
```

除健康检查和 GitHub Device Flow 登录开始/轮询接口外，其余接口都需要：

```text
Authorization: Bearer <token>
```

## 隐私模型

Token Use 是本地优先的工具。

- 本地统计默认只保存在用户设备上。
- 排行是可选功能。
- 用户退出排行后，仍可保持本地登录状态，但不会进入排行结果。
- 上传到后端的是聚合后的 Token 快照，不是原始请求日志。
- GitHub 登录只用于排行身份识别。

## 配置

常用环境变量：

| 变量 | 用途 |
| --- | --- |
| `TOKEN_USE_BACKEND_URL` | 覆盖桌面 App 使用的排行后端地址。 |
| `TOKEN_USE_GITHUB_CLIENT_ID` | 覆盖桌面 App 或后端使用的 GitHub OAuth Client ID。 |
| `TOKEN_MONITOR_GITHUB_CLIENT_ID` | 旧本地构建的临时兼容变量。 |
| `PORT` | 后端 HTTP 端口，默认 `6655`。 |
| `DATA_DIR` | 后端数据目录，默认 `/data`。 |
| `DATA_FILE` | 后端 JSON 存储路径，默认 `/data/token-use-backend.json`。 |
| `CORS_ORIGIN` | 后端 CORS 来源，默认 `*`。 |

## 开发

### 前端

```bash
pnpm typecheck
pnpm test:unit
pnpm build:renderer
```

### Tauri 与 Rust

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

### 后端

```bash
node --check backend/src/server.js
docker build -t token-use-backend:local backend
```

## 架构

```text
Token Use.app
  ├─ React UI
  │  ├─ Usage 页面
  │  └─ Leaderboard 页面
  ├─ Tauri commands
  │  ├─ 本地用量聚合
  │  ├─ GitHub 登录桥接
  │  └─ 排行快照同步
  ├─ SQLite 本地数据库
  └─ macOS 状态栏集成

Token Use Backend
  ├─ GitHub Device Flow 代理
  ├─ 用户资料与加入排行状态
  ├─ 聚合用量快照
  └─ 排行查询 API
```

## 路线图

- 签名后的 macOS 发布包。
- 自动更新元数据与签名流程。
- 更完整的首次启动空状态。
- 本地用量报表导出。
- 后端存储从 JSON 文件升级到 SQLite 或 Postgres 的迁移路径。
- 公开截图与发布说明。

## 贡献

项目仍在稳定期，欢迎围绕 bug 修复、文档、部署和 macOS 体验进行贡献。

请先阅读：

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
- [SECURITY.md](SECURITY.md)

提交 PR 前，请先运行 [开发](#开发) 章节中的相关检查。

## Fork 声明

Token Use 是 [CC Switch](https://github.com/farion1231/cc-switch) 的 fork 和产品裁剪版本。CC Switch 是一个更完整的跨平台管理工具，覆盖 Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw 等开发者工具。

本 fork 保留用量采集和 Token 分析基础，并移除大多数供应商管理能力，形成一个专注 macOS Token 用量统计与可选排行的独立应用。

原始项目的贡献与许可历史属于 CC Switch 维护者和贡献者。Token Use 的新增工作在本仓库中维护。

## License

MIT。详见 [LICENSE](LICENSE)。
