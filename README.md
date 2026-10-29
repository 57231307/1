# Bingxi Management Platform

[![Build Status](https://github.com/57231307/bingxi-erp/actions/workflows/ci-cd.yml/badge.svg)](https://github.com/57231307/bingxi-erp/actions/workflows/ci-cd.yml)
[![License](https://img.shields.io/badge/license-Proprietary-blue)](<>)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

> **Bingxi Management Platform** 是一套自托管的企业资源计划（ERP）平台：Rust 单体后端提供 HTTP 与 WebSocket 接口，Vue 3 单页前端经 Nginx 托管，数据落在 PostgreSQL。

---

## 目录

- [项目介绍](#项目介绍)
- [技术栈](#技术栈)
- [系统架构](#系统架构)
- [快速开始](#快速开始)
- [部署](#部署)
- [文档索引](#文档索引)
- [贡献](#贡献)
- [许可证](#许可证)
- [致谢](#致谢)
- [联系我们](#联系我们)

---

## 项目介绍

Bingxi Management Platform 是一套自托管的企业资源计划（ERP）平台，采用**单体 Rust 后端 + 单体 Vue 3 前端**的前后端分离形态：后端以 Axum 提供 HTTP 与 WebSocket 接口、SeaORM 访问 PostgreSQL、迁移以 Rust 代码内联 SQL 实现并按业务域聚合；前端为 Vue 3 + TypeScript + Element Plus 单页应用，经 Vite 构建为静态资源由 Nginx 托管。同一套后端还通过 Tauri 壳与 Android 工程产出桌面与移动客户端。

数据持久化在 PostgreSQL。热点读与 TTL 缓存走进程内 moka 缓存，Redis 承载限流计数与令牌撤销（JTI）等需要跨实例共享的状态，实时推送走 WebSocket；鉴权采用 JWT 配合服务端权限校验与 PostgreSQL 行级安全，配置由 `.env` 与 `config.yaml` 提供，服务以 systemd 直部署运行并支持 `bingxi update` 增量升级（SHA256 校验）。本文只描述**如何把它跑起来和改起来**；业务域划分与实现细节见「文档索引」中的架构与接口文档。

---

## 技术栈

### 后端（单体 Rust 服务）

| 类别           | 技术                             | 版本  |
| -------------- | -------------------------------- | ----- |
| 语言           | Rust                             | 1.94+ |
| Web 框架       | Axum                             | 0.8   |
| ORM            | SeaORM                           | 2.0   |
| 数据库         | PostgreSQL                       | 15+   |
| 缓存           | Redis                            | 7+    |
| 异步运行时     | Tokio                            | 1.x   |
| 事件总线       | rskafka                          | 0.5   |
| 序列化         | serde                            | 1.0   |
| 密码           | argon2                           | 0.5   |
| JWT            | jsonwebtoken                     | 11.0  |
| 验证           | validator                        | 0.21  |
| 日志           | tracing                          | 0.1   |
| 缓存（进程内） | moka                             | 0.12  |
| 全局单例       | arc-swap                         | 1.7   |
| Excel 导出     | rust_xlsxwriter                  | 0.98  |
| Excel 导入     | calamine                         | 0.36  |
| Word 生成      | docx-rs                          | 0.4   |
| PDF 生成       | printpdf                         | 0.7   |
| API 文档       | utoipa                           | 5.2   |
| TOTP 2FA       | totp-rs                          | 6.0   |
| HTTP 客户端    | reqwest                          | 0.13  |
| 配置           | config                           | 0.15  |
| 错误处理       | thiserror                        | 2.0   |
| 并发数据结构   | dashmap                          | 6.2   |
| 测试           | cargo test + mockall + criterion | —     |

### 前端（单体 Vue 3 SPA）

| 类别     | 技术         | 版本  |
| -------- | ------------ | ----- |
| 语言     | TypeScript   | 5.9   |
| 框架     | Vue          | 3.5   |
| 构建     | Vite         | 8.2   |
| UI 库    | Element Plus | 2.14+ |
| 状态     | Pinia        | 4.0   |
| 路由     | Vue Router   | 5.2   |
| HTTP     | Axios        | 1.19  |
| 国际化   | vue-i18n     | 11.4  |
| 图表     | ECharts      | 6.1   |
| 虚拟列表 | el-table-v2  | 2.14+ |
| 单元测试 | Vitest       | 4.1   |
| E2E 测试 | Playwright   | 1.40  |
| 规范     | ESLint       | 10.8  |
| 格式化   | Prettier     | 3.9   |

### 基础设施

| 类别     | 技术                         | 用途            |
| -------- | ---------------------------- | --------------- |
| 部署     | systemd + CLI 工具（bingxi） | 服务管理        |
| 反向代理 | Nginx                        | HTTP / WS / CSP |
| 监控     | Prometheus                   | 指标采集        |
| 可视化   | Grafana                      | 仪表盘          |
| 告警     | Alertmanager                 | 告警路由        |
| 日志     | Loki                         | 日志聚合        |
| CI/CD    | GitHub Actions               | 自动化          |

> **注意**：项目**不使用** Docker / Kubernetes / Helm，采用 systemd 直部署方式。

---

## 系统架构

### 整体架构（分层）

```
┌─────────────────────────────────────────────────────────────────┐
│                         客户端层 (Clients)                        │
│  ┌────────────┐  ┌────────────┐  ┌──────────┐                   │
│  │  Web (Vue) │  │  Desktop   │  │  3rd API │                   │
│  └─────┬──────┘  └─────┬──────┘  └─────┬────┘                   │
└────────┼───────────────┼───────────────┼────────────────────────┘
         │               │               │
         └───────────────┴───────────────┘
                                 │
┌─────────────────────────────────┴───────────────────────────────┐
│                       网关层 (Gateway)                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │    Nginx   │  │ Rate Limit │  │  Auth/JWT  │  │   CSP    │  │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └─────┬────┘  │
└────────┼───────────────┼───────────────┼───────────────┼────────┘
         │               │               │               │
         └───────────────┴───────────────┴───────────────┘
                                 │
┌─────────────────────────────────┴───────────────────────────────┐
│                      应用层 (Application)                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                Axum 0.8 (Rust 1.94+)                     │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐               │  │
│  │  │ handlers │  │ services │  │  models  │               │  │
│  │  │ (路由入站)│  │ (业务逻辑)│  │ (数据实体)│               │  │
│  │  └──────────┘  └──────────┘  └──────────┘               │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐               │  │
│  │  │   migration  │  │  event bus   │  │  middleware  │   │  │
│  │  │  (迁移承载)  │  │  (领域事件)   │  │ (鉴权·RLS)  │   │  │
│  │  └──────────┘  └──────────┘  └──────────┘               │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                 │
┌─────────────────────────────────┴───────────────────────────────┐
│                       数据层 (Data)                              │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │ PostgreSQL │  │   Redis    │  │   Kafka    │  │   Loki   │  │
│  │  (主+备)   │  │  (缓存)    │  │ (事件总线) │  │  (日志)  │  │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                 │
┌─────────────────────────────────┴───────────────────────────────┐
│                     基础设施层 (Infrastructure)                  │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │  systemd   │  │ Prometheus │  │  Grafana   │  │  告警    │  │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 前端架构

```
┌────────────────────────────────────────────────────────────┐
│                       Vue 3.5 App                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
│  │  Router    │  │   Pinia    │  │  i18n      │           │
│  │  (路由)    │  │  (状态)    │  │  (国际化)  │           │
│  └────────────┘  └────────────┘  └────────────┘           │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
│  │ Element+   │  │  V2Table   │  │  WebSocket │           │
│  │  (UI 库)   │  │ (虚拟列表) │  │  (实时)    │           │
│  └────────────┘  └────────────┘  └────────────┘           │
│  源码按 views / api / composables / components 分层组织     │
└────────────────────────────────────────────────────────────┘
```

---

## 快速开始

### 1. 环境要求

- **Rust**：1.94+ （含 clippy + rustfmt）
- **Node.js**：22+ （含 npm）
- **PostgreSQL**：15+
- **Redis**：7+
- **Kafka**：3+（事件总线，可选，缺省时降级为内存通道）

### 2. 克隆项目

```bash
git clone https://github.com/57231307/bingxi-erp.git
cd 1
```

### 3. 数据库准备

```bash
# 创建数据库（本地开发库名自定；deploy.sh 生产默认 bingxi、CI 测试库为 bingxi_test）
createdb bingxi_erp

# 创建用户
psql -U postgres -c "CREATE USER bingxi WITH PASSWORD '<your-strong-db-password>';"
psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE bingxi_erp TO bingxi;"
```

### 4. 后端启动（开发模式）

```bash
cd backend

# 复制环境配置模板（后端使用 .env 文件 + config.yaml）
cp .env.example .env
cp config.yaml.example config.yaml

# 编辑 .env 和 config.yaml，替换占位符为真实密钥
# 密钥生成方法：openssl rand -base64 32

# 安装依赖
cargo fetch

# 数据库迁移（需要先设置 DATABASE_URL 环境变量）
export DATABASE_URL="postgres://bingxi:<password>@localhost:5432/bingxi"
cargo run --bin bingxi -- migrate run

# 启动服务（监听 127.0.0.1:8082）
cargo run --bin server
```

服务将运行在 `http://localhost:8082`。

### 5. 前端启动（开发模式）

```bash
cd frontend

# 安装依赖
npm install

# 启动开发服务器（端口 3000，反向代理 /api 到 localhost:8082）
npm run dev
```

前端将运行在 `http://localhost:3000`。

### 6. 访问系统

打开浏览器访问 [http://localhost:3000](http://localhost:3000)。

使用初始化时在 Setup 页面设置的管理员账号登录（首次启动时通过 `/setup` 页面创建）。

> 禁止在文档中暴露默认密码。密码由部署者在 Setup 流程中自行设定，不存在预置默认密码。

### 7. systemd 直部署

项目采用 systemd 直部署方式，由 CLI 工具 `bingxi` 管理更新：

```bash
# 首次安装（通过 快速部署/install.sh 一键脚本）
sudo bash 快速部署/install.sh

# 后续更新（CLI 工具拉取 GitHub Release 并校验 SHA256）
sudo bingxi update

# 查看服务状态（前端为 Nginx 静态资源，无独立 systemd 服务）
sudo systemctl status bingxi-backend

# 查看日志
sudo journalctl -u bingxi-backend -f
```

> 项目不使用 Docker / Kubernetes / Helm，采用 systemd 直部署方式。

---

## 部署

### 两类部署环境（只允许生产与测试）

| 环境 | 用途       | 部署方式                                                                                      | 配置                             |
| ---- | ---------- | --------------------------------------------------------------------------------------------- | -------------------------------- |
| 生产 | 正式环境   | `deploy/deploy.sh`（服务器执行）或 `deploy/deploy-latest.sh`（远程执行，拉取 GitHub Release） | `/etc/bingxi/.env` + config.yaml |
| 测试 | 自动化验证 | GitHub Actions 上的独立测试环境（每次 PR 与主干推送各自成环境）                                               | CI 环境变量注入                  |

### 部署架构

- **反向代理**：Nginx（HTTP / WebSocket / CSP 安全头）
- **应用层**：systemd 服务（CLI 工具 + SHA256 校验 + 健康检查门禁 + 优雅停机 + 回滚机制）
- **数据层**：PostgreSQL（连接池 + 迁移自动执行）+ Redis（缓存/限流/会话）
- **事件总线**：Kafka（rskafka 纯 Rust 实现；缺省降级为内存通道）
- **初始化**：首次部署通过 Setup 向导（/setup）完成数据库连接 + 管理员创建

详细部署见 `deploy/` 目录和 `快速部署/install.sh`。

---

## 文档索引

- [贡献指南](CONTRIBUTING.md) — 提交 / 代码 / 测试 / 文档规范
- [PR 模板](.github/PULL_REQUEST_TEMPLATE.md) — PR 描述强制模板
- [Release 模板](.github/RELEASE_TEMPLATE.md) — 发布变更说明模板
- [代码规范](.monkeycode/docs/CODE_STYLE_GUIDE.md) — 命名 / 注释 / 风格
- [架构文档](.monkeycode/docs/ARCHITECTURE.md) — 分层与模块划分
- [前端架构](.monkeycode/docs/frontend-architecture.md) — Vue 3 组件拆分与状态管理
- [开发者指南](.monkeycode/docs/DEVELOPER_GUIDE.md) — 开发流程
- [接口文档](.monkeycode/docs/INTERFACES.md) — API 接口
- [安全策略](.monkeycode/docs/SECURITY.md) — 漏洞响应
- [文档索引](.monkeycode/docs/INDEX.md) — 全部文档导航
- [重构计划](.monkeycode/docs/refactoring/) — 重构任务清单

数据库结构变更一律以 Rust 代码内联 SQL 的迁移实现承载，按业务域聚合注册于 `backend/migration/src/lib.rs` 的 `Migrator::migrations()`，通过 `bingxi migrate run` 执行。

---

## 贡献

我们欢迎所有形式的贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解：

- 提交流程（fork → branch → commit → push → PR）
- 提交规范（conventional commits）
- 代码规范（rustfmt + clippy + eslint + tsc）
- 测试要求（新增功能必须带测试）
- 文档要求（新增 API 必须更新 docs）
- PR 流程（PR 描述强制依据模板填写 + CI 全绿 + review）

### 贡献者

感谢所有为 Bingxi Management Platform 做出贡献的开发者！

<a href="https://github.com/57231307/bingxi-erp/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=57231307/bingxi-erp" />
</a>

---

## 许可证

Copyright © 2026 成都秉羲布艺商行. 保留所有权利。

本项目为**专有软件**，未经授权禁止复制、修改、分发。

详细条款见 [LICENSE](LICENSE) 文件。

---

## 致谢

本项目使用了以下优秀的开源项目：

### 后端

- [Rust](https://www.rust-lang.org/) — 系统级语言
- [Axum](https://github.com/tokio-rs/axum) — Web 框架
- [SeaORM](https://www.sea-ql.org/SeaORM/) — 异步 ORM
- [Tokio](https://tokio.rs/) — 异步运行时
- [PostgreSQL](https://www.postgresql.org/) — 关系型数据库
- [Redis](https://redis.io/) — 内存数据库
- [rskafka](https://github.com/influxdata/rskafka) — Kafka 客户端（纯 Rust）

### 前端

- [Vue.js](https://vuejs.org/) — 渐进式框架
- [Vite](https://vitejs.dev/) — 构建工具
- [Element Plus](https://element-plus.org/) — UI 组件库
- [Pinia](https://pinia.vuejs.org/) — 状态管理
- [TypeScript](https://www.typescriptlang.org/) — 类型系统
- [ECharts](https://echarts.apache.org/) — 数据可视化
- [Playwright](https://playwright.dev/) — E2E 测试

### 基础设施

- [Prometheus](https://prometheus.io/) — 监控系统
- [Grafana](https://grafana.com/) — 可视化平台
- [Loki](https://grafana.com/oss/loki/) — 日志聚合

---

## 联系我们

- **GitHub Issues**：[https://github.com/57231307/bingxi-erp/issues](https://github.com/57231307/bingxi-erp/issues)
- **GitHub Discussions**：[https://github.com/57231307/bingxi-erp/discussions](https://github.com/57231307/bingxi-erp/discussions)
