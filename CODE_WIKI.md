# 秉羲管理系统 (BingXi ERP) - 核心代码 Wiki (深度详解版)

## 1. 项目概述与业务定位

秉羲管理系统是一个基于 **Rust** 全栈开发的现代化企业资源规划（ERP）系统，专为**面料/纺织行业**深度定制。
与传统通用 ERP 相比，本项目在架构上追求极致的性能、内存安全和高并发处理能力；在业务模型上，深度集成了面料行业特有的**五维度管理**（产品、批次、色号、缸号、等级）、**双计量单位换算**（米/公斤自动同步）以及**全链路业务追溯**机制。

---

## 2. 系统整体架构设计

项目采用严格的**前后端分离**架构，并基于领域驱动设计（DDD）思想对代码目录进行了垂直拆分。

### 2.1 架构数据流图
```text
[浏览器 (Yew/WASM)] 
       │ 
       ▼ (HTTP/JSON + JWT)
[Nginx 反向代理 / 负载均衡] 
       │ 
       ▼
[Axum Web Server (Rust 后端)]
       ├─ [Middlewares]: CORS -> Tracing(日志) -> Validator -> Auth(JWT解析) -> Permission(RBAC鉴权)
       ├─ [Handlers]: 接收请求，解析 Payload，响应组装
       ├─ [Services]: 核心业务逻辑，事务管理，跨模块协同
       └─ [Models/Entities]: SeaORM 数据映射实体
       │
       ▼ (连接池: SQLx)
[PostgreSQL 数据库]
```

### 2.2 核心技术栈详述

#### 后端生态 (`backend/`)
- **Web 框架**: `Axum 0.7` - 基于 Tokio 异步运行时，提供路由、中间件挂载、提取器（Extractor）支持。
- **ORM 框架**: `SeaORM 1.0` - 纯 Rust 实现的异步动态 ORM，基于 `SQLx`，支持事务（`DatabaseTransaction`）和实体（`ActiveModel`）强类型约束。
- **权限与认证**: 
  - `jsonwebtoken`: 签发与验证 JWT 令牌。
  - `argon2`: 用户密码的强哈希加密。
- **日志监控**: `tracing` / `tracing-subscriber` - 结构化异步日志，支持按天滚动切割日志文件。
- **内部通信**: `tonic` (gRPC) - 预留并实现了内部微服务高性能通信协议。

#### 前端生态 (`frontend/`)
- **核心框架**: `Yew 0.21` - 基于宏和组件的 Rust 前端框架，编译为 WebAssembly (WASM) 运行在浏览器端。
- **路由**: `yew-router` - 提供基于 Hash 或 History 的客户端单页路由。
- **网络通信**: `gloo-net` - 对原生 Fetch API 的 Rust 封装。项目中封装了统一的 `ApiService`，实现了**拦截器与自动重试机制**。

---

## 3. 后端核心模块与业务逻辑深度剖析

后端的代码主要集中在 `backend/src/`，按职责严格分层。

### 3.1 核心业务领域 (Domains)

#### 3.1.1 面料行业特色：五维度管理 (`utils/fabric_five_dimension.rs`)
系统中的面料库存和单据流转不依赖单一的 Product ID，而是强依赖于**五维度模型**：
- **`product_id`**: 成品 ID
- **`batch_no`**: 批次号
- **`color_no`**: 色号
- **`dye_lot_no`**: 缸号（可选，染色批次）
- **`grade`**: 等级（如：一等品、二等品、优等品）

**核心机制**：
- `FabricFiveDimension::generate_unique_id()`: 会将上述属性合并为一个全局唯一 ID（例如 `P100|B20240101|C001|D20240101001|G一等品`），用于缓存键值和追溯链唯一标识。
- 在 `InventoryStockService` (库存服务) 中，每一次入库/出库，系统都会校验这五个维度，确保同款面料不同批次、缸号和等级的库存被严格隔离。

#### 3.1.2 全链路业务追溯 (`services/business_trace_service.rs`)
系统实现了一个名为 `BusinessTraceService` 的全链路追溯系统。
- **追溯链 (`business_trace_chain`)**: 记录了面料从**采购收货** -> **入库** -> **调拨** -> **销售出库** 的全生命周期。
- **正向追溯 / 反向追溯**: 支持从“供应商+批次号”追踪到最终销往哪些客户 (`forward_trace`)，或从“客户+批次号”反向追溯其原始供应商 (`backward_trace`)。
- **数据快照**: `create_snapshot()` 会在关键节点将链路压缩存储为 JSON 格式的 `trace_path`，供前端可视化展示。

#### 3.1.3 权限拦截与 RBAC 模型 (`middleware/permission.rs`)
- **权限提取**: `extract_resource_info` 解析 URL（如 `/api/v1/erp/sales/orders` 提取出 `sales` 资源）。
- **权限映射**: 将 HTTP Method 映射为动作（GET -> read, POST -> create, PUT -> update, DELETE -> delete）。
- **鉴权逻辑**: 查询数据库 `role_permissions` 表，验证当前用户的 `role_id` 是否被授权对该 `resource_type` 执行对应 `action`。未授权则拦截并返回 `403 FORBIDDEN`。

#### 3.1.4 复杂事务控制示例：销售订单服务 (`services/sales_service.rs`)
`SalesService::create_order` 是一个典型的跨表强事务流程：
1. **防并发控制**: 生成唯一订单号 (`generate_order_no`)，开启数据库事务 `txn`。
2. **库存预检**: `check_inventory` 根据明细项遍历查询当前可用库存，若不足则立即 Rollback。
3. **库存锁定**: 调用 `lock_inventory`，在 `inventory_reservations` 表插入锁定记录，保护库存不被超卖。
4. **单据创建**: 插入 `sales_orders` 主表和 `sales_order_items` 明细表，计算折扣、税额和双计量单位（米/kg）价格。
5. **提交事务**: `txn.commit()`，成功后触发财务模块（生成应收账款）。

### 3.2 数据库实体层 (`models/`)
基于 SeaORM 宏生成的强类型实体，几个关键实体包括：
- `inventory_stock.rs`: 核心库存表，包含 `quantity_on_hand` (物理库存) 和 `quantity_available` (可用库存，减去预留)，并带有面料的五维字段。
- `budget_management.rs` / `assist_accounting.rs`: 财务与辅助核算核心表，用于业财一体化映射。

---

## 4. 前端核心模块深度剖析

前端应用位于 `frontend/src/`，是一个基于 Yew 的单页面应用（SPA）。

### 4.1 统一 API 客户端 (`services/api.rs`)
前端所有与后端的通信均通过封装的 `ApiService` 完成，具备以下特性：
- **请求拦截与 Auth 注入**: 从 `utils::storage::Storage` 读取 JWT Token，自动挂载到 `Authorization: Bearer <token>` 请求头。
- **指数退避重试机制 (`request_with_retry`)**: 在遇到网络异常时，最大重试 3 次 (`MAX_RETRIES`)，并且每次失败后的等待时间呈指数增长（1s -> 2s -> 4s）。
- **统一响应反序列化**: 强制解析外层结构 `ApiResponse<T>`，根据 `success` 字段决定返回 `Ok(data)` 或抛出 `Err(message)`，极大简化了业务页面的错误处理。

### 4.2 路由与状态管理
- **全局路由**: `app/mod.rs` 中定义了基于 `yew_router::Switch` 的枚举路由。
- **组件结构**: 
  - `components/main_layout.rs`: 页面外层框架，包含头部导航和侧边栏 (`navigation.rs`)。
  - `pages/`: 具体的业务页面，如 `dashboard.rs` (数据大盘), `sales_order.rs` (订单管理)。业务页面在 `use_effect_with` 钩子中触发 API 请求加载初始数据。

---

## 5. 系统数据库与初始化流程

### 5.1 系统启动与兜底初始化
由于这是一个企业级软件，必须考虑首次部署的体验。
- 在 `backend/src/main.rs` 中，系统启动时会尝试连接 PostgreSQL：
  - **连接成功**：启动正常业务路由体系，加载 CORS、Auth 和各种业务 Handler。
  - **连接失败**（如数据库未创建）：**不直接崩溃**，而是降级启动一个**初始化路由 (`create_init_router`)**。该路由暴露 `/init/test-database` 和 `/init/initialize-with-db`，允许管理员在前端页面配置数据库连接串、自动执行建表 SQL 脚本 (`database/migration/`) 并创建初始超级管理员账号。

### 5.2 数据库迁移脚本
在 `backend/database/migration/` 目录下，系统内置了按序号严格排列的 `.sql` 脚本：
- `001_init.sql`: 基础权限与用户表。
- `005_fabric_industry_adaptation.sql`: 面料行业特色表（缸号、双单位等）。
- `020_general_ledger.sql`: 财务总账表。
所有脚本会在执行 `/init` 接口时被依次解析和执行。

---

## 6. 部署与运维指南

### 6.1 本地开发环境运行

**1. 启动后端**
```bash
cd backend
cp .env.example .env
# 修改 .env 中的 DATABASE_URL
cargo run
```

**2. 启动前端**
```bash
cd frontend
# 需预先安装 trunk: cargo install trunk
# 需添加 wasm 目标: rustup target add wasm32-unknown-unknown
trunk serve --port 8081
```

### 6.2 生产环境构建与部署
系统提供了全套自动化部署脚本，位于 `deploy/` 目录。

**1. 构建产物**
```bash
# 后端构建 (使用 thin LTO 优化内存)
cd backend && cargo build --release

# 前端构建
cd frontend && trunk build --release
```

**2. Nginx 配置 (`deploy/nginx.conf`)**
Nginx 作为反向代理，负责：
- 静态资源托管：将 `/` 路由指向 `frontend/dist` 目录。
- API 代理：将 `/api` 转发至后端的 `127.0.0.1:8080`。
- WebSocket 支持（如果有）。

**3. Systemd 守护进程 (`deploy/bingxi-backend.service`)**
管理 Rust 后端进程的启动、重启和日志输出，保证服务的高可用性。

### 6.3 Prometheus 监控体系
项目在 `monitoring/` 下配置了完整的可观测性：
- **Prometheus**: 收集后端 Axum 暴露的 `/metrics` 接口数据（QPS、响应延迟、数据库连接池活跃度等）。
- **Grafana**: 预置了 `bingxi-erp-overview.json` 监控大盘，可直观查看内存占用和慢查询。
- **Alertmanager**: 配置了内存使用率过高、错误率激增等告警规则。

---

## 7. 二次开发指南

### 7.1 新增一个业务模块的标准步骤
1. **数据库层**: 在 `backend/database/migration/` 中新增建表 SQL，并在 `models/` 中使用 SeaORM CLI 生成对应的 Rust 实体结构。
2. **逻辑层**: 在 `backend/src/services/` 创建 `xxx_service.rs`，实现核心业务逻辑，确保跨表修改包裹在 `(*self.db).begin().await?` 事务中。
3. **接口层**: 在 `backend/src/handlers/` 创建 `xxx_handler.rs`，解析请求并调用 Service，使用 `thiserror` 定义模块级错误。
4. **路由注册**: 在 `backend/src/routes/mod.rs` 注册新接口，并为其配置相应的权限标识（如 `xxx:write`）。
5. **前端接入**: 在 `frontend/src/models/` 添加请求/响应实体，在 `frontend/src/pages/` 编写 UI 页面，并通过 `ApiService` 发起请求。

### 7.2 代码规范
- **错误处理**: 严禁在业务代码中使用 `.unwrap()` 或 `.expect()` 导致服务 Panic。必须使用 `?` 向上抛出错误，并在 Handler 层通过实现 `IntoResponse` 统一转为 JSON 错误返回。
- **数值精度**: 所有涉及金额和数量（如米、公斤）的字段，**必须**使用 `rust_decimal::Decimal`，绝对禁止使用 `f32/f64`，以防止浮点数精度丢失。
- **日志打印**: 重要的业务变更（如订单审核、库存扣减、权限失败）必须使用 `tracing::info!` 或 `tracing::warn!` 打印，方便问题排查。

---
*文档维护者：秉羲团队*  
*生成时间：2026-04-05*