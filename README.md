# 冰溪 ERP 系统

[![Build Status](https://github.com/57231307/1/actions/workflows/ci-cd.yml/badge.svg)](https://github.com/57231307/1/actions/workflows/ci-cd.yml)
[![License](https://img.shields.io/badge/license-Proprietary-blue)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

> **冰溪 ERP** 是一款面向**面料纺织行业**的现代化企业资源计划系统，覆盖采购、销售、库存、生产、财务、CRM 等核心业务，并集成 **AI 智能分析**、**BI 数据仓库**、**WebSocket 实时通信**、**BPM 审批流** 等能力，赋能企业数字化转型。

---

## 📋 目录

- [项目介绍](#-项目介绍)
- [核心特性](#-核心特性)
- [技术栈](#-技术栈)
- [系统架构](#-系统架构)
- [功能矩阵](#-功能矩阵)
- [快速开始](#-快速开始)
- [部署](#-部署)
- [文档索引](#-文档索引)
- [测试](#-测试)
- [审计与质量](#-审计与质量)
- [贡献](#-贡献)
- [许可证](#-许可证)
- [致谢](#-致谢)

---

## 🌟 项目介绍

冰溪 ERP 系统是**面向纺织行业的全栈式企业资源计划系统**，单体 Rust 后端 + 单体 Vue 3 前端架构，针对面料纺织行业深度定制（色卡/面料/缸号/批号/定制订单/多色号定价等），并通过 V15 25 大类 195 维度审计与主线八维审计的严格质量保障。

**项目亮点**：

- 🏭 **行业深度**：覆盖染整全流程（化验室打样→大货处方→流转卡→验布打卷→产量工资→能耗管理→缸号状态机），针对纺织行业特殊业务深度定制
- 🤖 **AI 驱动**：工艺优化、质量预测、补货推荐、异常检测 4 类 AI 能力，含模型版本管理与可解释性
- 🔄 **实时通信**：WebSocket 推送订单状态、库存预警、审批进度、仪表板更新
- 🚀 **自动化部署**：systemd 直部署 + CLI 工具（bingxi update）+ 蓝绿部署 + SHA256 校验
- 📊 **BI 数据仓库**：多维分析 + 仪表板 + 报表引擎 + 订阅推送
- 🔒 **合规安全**：RBAC 权限矩阵 + 字段级权限 + 打印导出审计 + 二级审批 + 中国法律合规（劳动法/数据安全法/个人信息保护法）
- 📈 **可观测性**：trace 链路 + Prometheus 指标 + 慢查询审计 + API 网关熔断 + 流复制故障转移

**项目数据（截至 2026-07-30）**：

| 指标 | 数值 |
|------|------|
| 后端 Rust 代码 | ~241,000 行 |
| 前端 TS/Vue 代码 | ~136,000 行 |
| 后端 Handler | 148 个 |
| 后端 Service（含子目录） | 390 个 |
| 后端 Model | 275 个 |
| 后端 Route 模块 | 40 个 |
| 后端 Middleware | 18 个 |
| 数据库迁移 SQL | 57 个 |
| 后端集成测试 | 50 个 |
| 前端 Vue 文件 | 376 个 |
| 前端 TS 文件 | 224 个 |
| 前端 Views 子模块 | 86 个 |
| 前端 API 模块 | 96 个 |
| Clippy Baseline | 308 条（174 个为 P1 预留服务 dead_code，待接入路由消除） |
| 最新版本 | 2026.723.1842 |
| 最新 PR | #789（docs 同步）/ #788（委外收货主链路统一） |

---

## ✨ 核心特性

### 1. 📦 完整业务域覆盖

- **采购管理**：供应商 / 采购订单 / 采购合同 / 采购价格 / 采购入库 / 采购退货 / 供应商评估
- **销售管理**：客户 / 销售订单 / 销售合同 / 销售价格 / 销售出库 / 销售退货 / 客户信用
- **库存管理**：库存盘点 / 库存调拨 / 库存调整 / 批次管理 / 库存预警 / 安全库存 / 缸号分区
- **生产管理**：生产订单 / MRP 运算 / 工序管理 / 质量控制 / 产能规划 / 自动排程
- **财务管理**：总账 / 应收应付 / 固定资产 / 资金管理 / 成本核算 / 会计期间 / 辅助核算 / 预算管理 / 财务分析
- **CRM**：线索管理 / 商机管理 / 客户池 / 团队协作 / 转移审批 / 信用评估

### 2. 🏭 纺织行业特性（V15 类四~类五 17 维度）

- **四层级联关系**：面料 → 颜色 → 缸号 → 批号（匹号），库存四维标识
- **匹号唯一约束**：`UNIQUE(dye_lot_no, batch_no)`，所有含 batch_no 的表强制校验
- **化验室打样**：5 步闭环（打样通知单→ABCD 多版样→OK 样→复样→染色技术卡）
- **大货处方**：染色配料单 + 与打样 OK 样配方联动 + 加料处方
- **流转卡与车间工序**：条码 + 工序扫码上报 + 进度跟踪（前处理→染色→印花→后整理→验布）
- **验布打卷**：十项指标检验 + A/B/C 分级 + 打卷入库 + 匹号生成
- **产量工资核算**：按缸号计件 + A 级全额/B 级折扣/C 级不计 + 加班费（《劳动法》第 44 条）
- **能耗管理与成本归集**：水电汽分摊 + 按缸号归集 + 月末分摊到成本
- **缸号状态机**：投染→染色→出缸→质检→入库→发货→退货全生命周期
- **胚布拆匹**：胚布库存 + 委外流转 + 缸号匹号继承 + 8D 处理 + 不合格品降级返工报废
- **色卡发放**：4 service + 客户专属色卡库 + 复购同缸号 + 过期检查定时任务
- **大货批色**：剪大货样 + 客户批色确认 + 批色报表 + 历史追溯
- **多色号定价**：价格计算引擎 + 季节性价格 + 客户专属价格 + 批量定价

### 3. 🤖 AI 智能分析（V15 类十六 10 维度）

- **工艺优化**：染料-布类配伍性校验 + 参数推荐 + 化验室打样集成 + 模型版本管理
- **质量预测**：特征工程（dye_type/auxiliary_type/temperature_range 等）+ 实际结果回填 + 准确率对账
- **补货推荐**：与 MRP 引擎对账 + 差异标注人工复核
- **异常检测**：统计 + 机器学习双引擎
- **AI 治理**：模型可解释性 + 数据脱敏 + 推理超时降级 + 并发控制 + 缓存策略 + 决策审计日志

### 4. 📊 BI 数据仓库（V15 类十九 8 维度）

- **数据仓库**：4 张事实表 + 16 维
- **多维分析**：销售 / 库存 / 财务 / 经营
- **报表引擎**：模板版本管理 + 订阅推送重试 + BI 查询缓存（5min TTL）
- **仪表板**：dashboard_layouts + WebSocket 实时推送 + 角色数据范围过滤
- **通知协同**：通知中心多渠道去重 + 邮件 SMTP 队列重试 + OA 公告可见性 + 五维度分析

### 5. 🔒 RBAC 权限与安全（V15 类十二~十四 30 维度）

- **RBAC 数据模型**：角色 + 权限 + 用户多部门 + 字段级权限 + 权限委托
- **权限矩阵**：14 类业务角色差异化权限 + 职责分离 SoD 校验 + is_system 滥用治理
- **打印导出审计**：端点合理性 + 角色权限矩阵 + 二级审批 + 文件水印 + 并发控制 + 合规定期审查
- **安全防护**：JWT + refresh_token（2 天对齐）+ PUBLIC_PATHS 精确匹配 + Webhook payload 脱敏 + magic bytes 校验 + zip bomb 防护 + SSRF 防护 + 路径穿越防护
- **法律合规**：中国法律法规 + 数据脱敏（手机/邮箱/身份证/银行卡）+ 成品文档格式（xlsx/docx）+ 纺织行业法律财税环保劳动

### 6. 📈 可观测性与运维（V15 类二十 8 维度 + 类二十五 11 维度）

- **trace 链路**：HTTP/Kafka/WS 跨服务传递 + traceparent 注入
- **metrics 指标**：Prometheus + BusinessMetrics 上报 + 告警规则
- **WebSocket 实时**：ACK + Redis Pub/Sub + 连接重连心跳
- **故障转移**：流复制 + check_replication_sync + wait_for_backup_catchup
- **慢查询审计**：> 200ms 全记录 + 阈值告警
- **API 网关**：路由转发 + 限流 + 熔断（5s 窗口失败率 > 50% 触发 open）
- **系统升级**：灰度升级（10%/50%）+ SHA256 校验 + schema 兼容性检查 + 蓝绿部署 + 健康检查门禁 + 优雅停机 + 回滚机制 + 部署后自动回滚监控
- **日志增强**：结构化 JSON + 90 天保留期自动清理 + 7 个日志层（financial/permission/database/business/performance/health/security）

### 7. 🌍 国际化与前端体验（V15 类二十四 20 维度）

- **vue-i18n** 中英双语 + 8947 个唯一翻译键
- **PWA 支持**：manifest.json + Service Worker + 离线缓存
- **移动端适配**：响应式 + 侧边栏抽屉化 + 汉堡按钮 ≥44px（WCAG 2.5.5）
- **性能优化**：manualChunks 代码分割 + ECharts 按需引入 + optimizeDeps + V2Table 虚拟列表
- **错误处理**：ErrorBoundary + 前端监控 SDK + 表单脏数据检测
- **可访问性**：键盘导航焦点管理 + WCAG 2.1 AA
- **主题样式**：CSS 变量 + 暗黑模式 + localStorage 持久化
- **权限粒度**：v-permission 指令按钮/字段/行级 + keep-alive 状态保留

---

## 🔧 技术栈

### 后端（单体 Rust 服务）

| 类别 | 技术 | 版本 |
|------|------|------|
| 语言 | Rust | 1.94+ |
| Web 框架 | Axum | 0.7 |
| ORM | SeaORM | 1.1.20（2.0 升级暂缓） |
| 数据库 | PostgreSQL | 15+ |
| 缓存 | Redis | 7+ |
| 异步运行时 | Tokio | 1.x |
| 事件总线 | rskafka | 0.5 |
| 序列化 | serde | 1.0 |
| 密码 | argon2 | 0.5 |
| JWT | jsonwebtoken | 9.0 |
| 日志 | tracing | 0.1 |
| 限流 | governor | 0.6 |
| 缓存（进程内） | moka | 0.12 |
| 全局单例 | arc-swap | 1.7 |
| Excel 导出 | rust_xlsxwriter | 0.95 |
| Excel 导入 | calamine | 0.26 |
| Word 生成 | docx-rs | 0.4 |
| PDF 生成 | printpdf | 0.7 |
| API 文档 | utoipa | 5.2 |
| 测试 | cargo test + mockall + criterion | — |

### 前端（单体 Vue 3 SPA）

| 类别 | 技术 | 版本 |
|------|------|------|
| 语言 | TypeScript | 5.4 |
| 框架 | Vue | 3.4 |
| 构建 | Vite | 6.4 |
| UI 库 | Element Plus | 2.6+ |
| 状态 | Pinia | 2.1 |
| 路由 | Vue Router | 4.3 |
| HTTP | Axios | 1.6 |
| 国际化 | vue-i18n | 9.13 |
| 图表 | ECharts | 6.1 |
| 虚拟列表 | el-table-v2 | 2.6+ |
| 测试 | Vitest + Playwright | — |
| 规范 | ESLint + Prettier | — |

### 基础设施

| 类别 | 技术 | 用途 |
|------|------|------|
| 部署 | systemd + CLI 工具（bingxi） | 服务管理 |
| 反向代理 | Nginx | HTTP / WS / CSP |
| 监控 | Prometheus | 指标采集 |
| 可视化 | Grafana | 仪表盘 |
| 告警 | Alertmanager | 告警路由 |
| 日志 | Loki | 日志聚合 |
| CI/CD | GitHub Actions | 自动化 |

> **注意**：项目**不使用** Docker / Kubernetes / Helm（PR #777 已彻底移除），采用 systemd 直部署方式。

---

## 🏗️ 系统架构

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
│  │                Axum 0.7 (Rust 1.94+)                    │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ │  │
│  │  │  业务  │ │  行业  │ │  AI    │ │  BI    │ │  BPM   │ │  │
│  │  │  域   │ │  子模块 │ │  模块  │ │  模块  │ │ 审批流 │ │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘ │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐           │  │
│  │  │WebSocket│ │ 通知  │ │ 审计  │ │ 可观测 │           │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘           │  │
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
│                       Vue 3.4 App                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
│  │  Router    │  │   Pinia    │  │  i18n      │           │
│  │  (路由)    │  │  (状态)    │  │  (国际化)  │           │
│  └────────────┘  └────────────┘  └────────────┘           │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐           │
│  │ Element+   │  │  V2Table   │  │  WebSocket │           │
│  │  (UI 库)   │  │ (虚拟列表) │  │  (实时)    │           │
│  └────────────┘  └────────────┘  └────────────┘           │
│  ┌────────────────────────────────────────────┐            │
│  │     376 Vue 文件 + 224 TS 文件              │            │
│  │  (86 views 子模块 + 96 api + 7 composables) │            │
│  └────────────────────────────────────────────┘            │
└────────────────────────────────────────────────────────────┘
```

---

## 📋 功能矩阵

### 5 域核心业务

| 域 | 子功能 | 状态 |
|----|--------|------|
| 采购管理 | 供应商 / 订单 / 合同 / 价格 / 入库 / 退货 / 供应商评估 | ✅ 100% |
| 销售管理 | 客户 / 订单 / 合同 / 价格 / 出库 / 退货 / 客户信用 | ✅ 100% |
| 库存管理 | 盘点 / 调拨 / 调整 / 批次 / 预警 / 安全库存 / 缸号分区 | ✅ 100% |
| 生产管理 | 订单 / MRP / 工序 / 质量 / 产能 / 自动排程 | ✅ 100% |
| 财务管理 | 总账 / 应收应付 / 固定资产 / 资金 / 成本 / 期间 / 辅助核算 / 预算 / 财务分析 | ✅ 100% |
| CRM | 线索 / 商机 / 客户池 / 团队协作 / 转移审批 / 信用 | ✅ 100% |

### 纺织行业特性（V15 类四 17 维度）

| 子模块 | 状态 |
|--------|------|
| 化验室打样 / 大货处方 / 流转卡 / 验布打卷 / 产量工资 / 能耗管理 / 缸号状态机 | ✅ 100% |
| 胚布拆匹 / 质量处理 / 不合格品降级返工报废 | ✅ 100% |
| 色卡发放 / 大货批色 / 多色号定价 / 定制订单 / 销售报价 | ✅ 100% |

### 智能与协同能力

| 能力 | 状态 | 备注 |
|------|------|------|
| AI 智能分析 | ✅ 100% | 工艺优化 / 质量预测 / 补货推荐 / 异常检测 + 模型版本管理 |
| BI 数据仓库 | ✅ 100% | 4 表 + 16 维 + 仪表板 + 报表引擎 + 订阅推送 |
| BPM 审批流 | ✅ 100% | 流程定义 / 实例 / 任务 |
| WebSocket 实时 | ✅ 100% | 通知 / 订单 / 库存 / 审批 / 仪表板 |
| 国际化（i18n） | ✅ 100% | 中英双语 + 8947 翻译键 |
| systemd 部署 | ✅ 100% | CLI 工具 + 蓝绿部署 + 灰度升级 + SHA256 校验 |

---

## 🚀 快速开始

### 1. 环境要求

- **Rust**：1.94+ （含 clippy + rustfmt）
- **Node.js**：20+ （含 npm）
- **PostgreSQL**：15+
- **Redis**：7+
- **Kafka**：3+（事件总线，可选，缺省时降级为内存通道）

### 2. 克隆项目

```bash
git clone https://github.com/57231307/1.git
cd 1
```

### 3. 数据库准备

```bash
# 创建数据库
createdb bingxi_erp
createdb bingxi_erp_test

# 创建用户（密码请自行设定，禁止使用弱密码；此处仅为占位符）
psql -U postgres -c "CREATE USER bingxi WITH PASSWORD '<your-strong-db-password>';"
psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE bingxi_erp TO bingxi;"
psql -U postgres -c "GRANT ALL PRIVILEGES ON DATABASE bingxi_erp_test TO bingxi;"
```

### 4. 后端启动（开发模式）

```bash
cd backend

# 复制环境配置
cp .env.example .env.development

# 安装依赖
cargo fetch

# 数据库迁移
cargo run --bin migrate

# 启动服务（开发模式，热重载）
cargo run --bin bingxi-erp-server

# 或使用 cargo-watch
cargo watch -x 'run --bin bingxi-erp-server'
```

服务将运行在 `http://localhost:8080`。

### 5. 前端启动（开发模式）

```bash
cd frontend

# 安装依赖
npm install

# 复制环境配置
cp .env.development.example .env.development

# 启动开发服务器
npm run dev
```

前端将运行在 `http://localhost:5173`。

### 6. 访问系统

打开浏览器访问 [http://localhost:5173](http://localhost:5173)。

使用初始化时在 Setup 页面设置的管理员账号登录（首次启动时通过 `/setup` 页面创建）。

> ⚠️ **禁止在文档中暴露默认密码**。密码由部署者在 Setup 流程中自行设定，不存在预置默认密码。

### 7. systemd 直部署（推荐）

项目采用 systemd 直部署方式，由 CLI 工具 `bingxi` 管理更新：

```bash
# 首次安装（通过 install.sh 一键脚本）
sudo bash install.sh

# 后续更新（CLI 工具拉取 GitHub Release 并校验 SHA256）
sudo bingxi update

# 查看服务状态
sudo systemctl status bingxi-backend
sudo systemctl status bingxi-frontend

# 查看日志
sudo journalctl -u bingxi-backend -f
```

> **禁止** Docker 容器部署（不得创建 Dockerfile / docker-compose.yml）。
> **禁止** Kubernetes / Helm 部署（项目采用 systemd 直部署，不使用容器编排）。

---

## 🌍 部署

### 5 种环境

| 环境 | 用途 | 部署方式 | 配置 |
|------|------|---------|------|
| 开发 | 本地开发 | cargo run + npm run dev | `.env.development` |
| 测试 | 自动化测试 | cargo test + npm test | `.env.test` |
| 预发 | 上线前验证 | systemd（staging 服务器） | `.env.staging` |
| 生产 | 正式环境 | systemd（prod 服务器） | `.env.production` |
| 灾备 | 灾难恢复 | systemd（DR 服务器） | `.env.dr` |

### 部署架构

- **反向代理**：Nginx（HTTP / WebSocket / CSP 安全头）
- **应用层**：systemd 服务（蓝绿部署 + 灰度升级 + CLI 工具 + SHA256 校验 + 健康检查门禁 + 优雅停机 + 回滚机制）
- **数据层**：PostgreSQL 主备（流复制 + 故障转移）+ Redis 哨兵
- **事件总线**：Kafka（rskafka 纯 Rust 实现，无 C/C++ 依赖）
- **文件存储**：S3 / OSS 兼容
- **CDN**：静态资源 CDN 分发

详细部署见：
- [deploy/](deploy/) — systemd 部署脚本 + 服务文件 + Nginx 配置
- [.monkeycode/docs/DEVELOPER_GUIDE.md](.monkeycode/docs/DEVELOPER_GUIDE.md) — 开发者指南
- [.monkeycode/docs/ARCHITECTURE.md](.monkeycode/docs/ARCHITECTURE.md) — 架构文档

---

## 📚 文档索引

### 项目规范与架构

- [贡献指南](CONTRIBUTING.md) — 提交 / 代码 / 测试 / 文档规范
- [PR 模板](.github/PULL_REQUEST_TEMPLATE.md) — PR 描述强制模板
- [Release 模板](.github/RELEASE_TEMPLATE.md) — 发布变更说明模板
- [代码规范](.monkeycode/docs/CODE_STYLE_GUIDE.md) — 命名 / 注释 / 风格
- [架构文档](.monkeycode/docs/ARCHITECTURE.md) — 整体架构
- [前端架构](.monkeycode/docs/frontend-architecture.md) — Vue 3.4 + 组件拆分
- [开发者指南](.monkeycode/docs/DEVELOPER_GUIDE.md) — 开发流程
- [接口文档](.monkeycode/docs/INTERFACES.md) — API 接口
- [安全策略](.monkeycode/docs/SECURITY.md) — 漏洞响应
- [项目健康报告](.monkeycode/docs/PROJECT_HEALTH_REPORT.md) — 整体健康度
- [文档索引](.monkeycode/docs/INDEX.md) — 文档导航

### 权限与合规

- [RBAC 权限矩阵](docs/rbac-permission-matrix.md) — 角色权限详细矩阵
- [面料行业调研](.monkeycode/docs/research/fabric-industry-research.md) — 13 章节真实业务调研

### 数据库与重构

- [数据库文档](.monkeycode/docs/database/) — schema / 迁移 / 归档
- [重构计划](.monkeycode/docs/refactoring/) — 重构任务清单

---

## 🧪 测试

### 测试策略

| 层级 | 数量 | 工具 | 覆盖率 |
|------|------|------|-------|
| 后端单元测试 | 集成在 services | cargo test + mockall | 服务层 |
| 后端集成测试 | 50 | cargo test (integration) | — |
| 前端单元测试 | — | Vitest | 1.67%（阈值临时降级为 1%，待补齐测试后回调至 70%） |
| 前端 E2E 测试 | 3 | Playwright | — |
| 性能基准 | 4 | criterion | 库存核算 / 凭证生成 / 染整成本归集 / 产量工资计算 |

### 运行测试

```bash
# 后端单元测试
cd backend
cargo test --all

# 后端集成测试
cargo test --test '*'

# 前端单元测试
cd frontend
npm run test:run

# 前端 E2E 测试
npm run test:e2e

# 性能基准测试
cd backend
cargo bench --features bench
```

### 覆盖率报告

```bash
# 后端
cd backend
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --output-dir coverage/

# 前端
cd frontend
npm run test:coverage
```

---

## 🔍 审计与质量

### 审计体系（V15 25 大类 195 维度）

项目通过 V15 25 大类 195 维度最严格审计体系，详见 [.monkeycode/audit_assignment.md](.monkeycode/audit_assignment.md)。

| 审计阶段 | 状态 | 说明 |
|----------|------|------|
| V15 25 大类 195 维度审计 | ✅ 完成 | 详见 [.monkeycode/docs/audits/v15/](.monkeycode/docs/audits/v15/) |
| P0 修复（39 项 → 22 批次） | ✅ 完成 | 100% |
| P1 修复（257 项 → 25 批已合并 main） | ✅ 完成 | 100% |
| V15 主线八维审计（2026-07-30） | 🔄 进行中 | P0 11/11 + P2 3/3 已完成，P1 委外收货已合并（PR #788） |
| P2 修复（248 项） | ⏳ 待启动 | P1 完成后启动 |
| P3 修复（123 项） | ⏳ 按需 | 按需修复 |

### 质量规则

项目遵循严格的个人规则（IR）与项目规则（PR），详见 [.monkeycode/MEMORY.md](.monkeycode/MEMORY.md)：

- **规则 0**：真实实现强制，禁止 stub/placeholder
- **规则 3**：成品导出仅 .xlsx/.docx，禁 CSV/txt/html
- **规则 4**：`///` 注释精简为 1 行（最多 2 行）
- **规则 13**：修复流程自动化，步骤 0 确认审计内容存在 + 步骤 4 推送前自审
- **规则 14**：禁止 `#[allow(...)]`，所有警告视为错误
- **规则 20**：注释与功能一致性，CI 强制检查

### CI/CD

所有验证走 GitHub Actions（CI/CD Only，禁止本地构建）：

- Rust：fmt + clippy（baseline 308 条）+ 单元测试 + 后端构建 + 覆盖率
- 前端：fmt + ESLint + 类型检查 + 测试 + 构建 + 覆盖率
- 依赖审计 + 环境信息 + 依赖图记录 + 构建通知

---

## 🤝 贡献

我们欢迎所有形式的贡献！请阅读 [CONTRIBUTING.md](CONTRIBUTING.md) 了解：

- 提交流程（5 步：fork → branch → commit → push → PR）
- 提交规范（conventional commits）
- 代码规范（rustfmt + clippy + eslint + tsc）
- 测试要求（新增功能必须带测试）
- 文档要求（新增 API 必须更新 docs）
- PR 流程（PR 描述强制依据模板填写 + CI 全绿 + 2 人 review）

### 贡献者

感谢所有为冰溪 ERP 做出贡献的开发者！

<a href="https://github.com/57231307/1/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=57231307/1" />
</a>

---

## 📜 许可证

Copyright © 2026 冰溪 ERP. 保留所有权利。

本项目为**专有软件**，未经授权禁止复制、修改、分发。

详细条款见 [LICENSE](LICENSE) 文件。

---

## 🙏 致谢

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

### 基础设施

- [Prometheus](https://prometheus.io/) — 监控系统
- [Grafana](https://grafana.com/) — 可视化平台
- [Loki](https://grafana.com/oss/loki/) — 日志聚合

### 工具与规范

- [GitHub Actions](https://github.com/features/actions) — CI/CD
- [Playwright](https://playwright.dev/) — E2E 测试
- [clippy](https://github.com/rust-lang/rust-clippy) — Rust lint
- [criterion](https://github.com/bheisler/criterion.rs) — 性能基准

---

## 📞 联系我们

- **GitHub Issues**：[https://github.com/57231307/1/issues](https://github.com/57231307/1/issues)
- **GitHub Discussions**：[https://github.com/57231307/1/discussions](https://github.com/57231307/1/discussions)
- **安全漏洞上报**：security@57231307.com

---

<div align="center">

**⭐ 如果这个项目对您有帮助，请给我们一个 star！**

Made with ❤️ by 冰溪 ERP Team

</div>
