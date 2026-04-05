# 秉羲管理系统 (BingXi ERP) - 核心代码 Wiki

## 1. 项目整体架构

秉羲管理系统是一个专为面料行业定制的现代化企业资源规划（ERP）系统。项目采用**前后端分离**架构，并在全栈使用 **Rust** 语言进行开发，以保证极致的性能、内存安全和高并发处理能力。

- **后端应用 (`backend/`)**：基于 `Axum` 框架构建的 RESTful API 服务，使用 `SeaORM` 作为 ORM 访问 PostgreSQL 数据库，并集成了 JWT 认证、RBAC 权限控制、Tracing 日志和 Prometheus 监控。
- **前端应用 (`frontend/`)**：基于 `Yew` 框架构建的单页应用（SPA），通过 WebAssembly (WASM) 运行在浏览器端，通过 HTTP/JSON 与后端进行通信。
- **监控与部署 (`monitoring/`, `deploy/`)**：提供了基于 Nginx 反向代理、Systemd 服务管理以及 Prometheus + Grafana 的全套部署和监控配置。

---

## 2. 核心技术栈与依赖关系

### 2.1 后端核心依赖 (Cargo.toml)
| 依赖库 | 作用说明 |
|---|---|
| **axum** (0.7) | 核心 Web 框架，处理 HTTP 路由和请求。 |
| **sea-orm** (1.0) | 异步 ORM 框架，负责 PostgreSQL 数据库的映射与查询。 |
| **tokio** (1.0) | 异步运行时，提供非阻塞 I/O 支持。 |
| **jsonwebtoken** (9.0) | JWT 令牌的生成与验证，用于身份认证。 |
| **tracing** / **tracing-subscriber** | 结构化日志收集与输出，支持日志按天滚动切割。 |
| **tonic** / **prost** | 提供 gRPC 服务支持（内部服务间通信或高性能接口）。 |
| **rust_xlsxwriter** | 纯 Rust 实现的 Excel 导出库，用于报表导出。 |
| **utoipa** | OpenAPI (Swagger) 文档自动生成。 |

### 2.2 前端核心依赖 (Cargo.toml / package.json)
| 依赖库 | 作用说明 |
|---|---|
| **yew** (0.21) | 核心前端组件化框架，基于 Rust 和 WASM。 |
| **yew-router** | 客户端路由管理。 |
| **gloo-net** | 提供基于 `fetch` 的 HTTP 客户端调用封装。 |
| **serde** / **serde_json** | 用于 JSON 数据的前后端序列化/反序列化。 |
| **wasm-bindgen** | 提供 Rust 与 JavaScript 的交互桥梁。 |

---

## 3. 主要模块职责

项目遵循清晰的领域驱动设计（DDD）目录结构。

### 3.1 后端目录结构 (`backend/src/`)
- **`config/`**: 配置加载模块，读取 `.env` 或 `config.yaml`。
- **`models/`**: 数据库实体定义，由 SeaORM 自动生成或手动定义，包含业务字段及关联关系。
- **`handlers/`**: 控制器层（API 视图），负责接收 HTTP 请求，解析参数，调用 Service，并返回 JSON 响应。
- **`services/`**: 业务逻辑层，处理复杂的业务规则、事务控制及跨模块调用。
- **`middleware/`**: 包含认证（Auth）、权限拦截（Permission）、请求校验、日志和限流等中间件。
- **`utils/`**: 工具类集合，包含 `AppState` (全局应用状态)、错误处理统一定义、缓存及业务辅助函数（如单位换算）。
- **`grpc/`**: gRPC 服务实现端。

### 3.2 业务模块划分
1. **基础数据与系统管理**：用户(User)、角色(Role)、部门(Department)、系统初始化(Init)。
2. **面料行业特色模块**：
   - 批次与缸号管理 (`dye_batch`, `greige_fabric`, `batch`)。
   - 双计量单位换算 (`dual_unit_converter`)。
   - 五维度查询管理 (`five_dimension`：产品、批次、色号、等级、仓库)。
3. **供应链管理**：采购(Purchase)、销售(Sales)、库存(Inventory)、供应商与客户管理。
4. **财务管理**：应收/应付(AR/AP)、资金管理、总账/辅助核算(Assist Accounting)、财务分析。

---

## 4. 关键类与函数说明

### 4.1 后端关键类与函数

- **`crate::main()`** ( `backend/src/main.rs` )
  - **说明**：后端应用入口点。
  - **职责**：加载配置，初始化 Tracing 日志，建立 PostgreSQL 连接池。创建全局 `AppState`，挂载全局中间件（CORS, Auth, 日志），最终绑定端口并启动 Axum Server。在未初始化时，会自动启动配置路由 (`create_init_router`) 提供首次安装界面。

- **`AppState`** ( `backend/src/utils/app_state.rs` )
  - **说明**：全局状态容器，实现了 `Clone`。
  - **核心属性**：`db: Arc<DatabaseConnection>` (数据库连接池)，`jwt_secret: String`。

- **`AuthMiddleware`** ( `backend/src/middleware/auth.rs` )
  - **函数 `auth_middleware`**：拦截器，从请求头提取 `Bearer Token`，调用 `AuthService::verify_token` 校验，成功后将用户信息注入到请求的 Extension 中供下游 `Handler` 使用。

- **`AuthService::login`** ( `backend/src/services/auth_service.rs` )
  - **说明**：用户登录核心逻辑。
  - **流程**：根据用户名查询数据库，使用 `argon2` 验证密码 Hash，成功后调用 `jsonwebtoken` 签发 Token，并记录登录日志。

- **`InventoryStockService`** ( `backend/src/services/inventory_stock_service.rs` )
  - **关键函数 `adjust_stock`**：库存调整。
  - **行业特色**：处理库存时不仅处理产品 ID 和仓库，还强关联 `batch_no` (批次号) 和 `color_code` (色号)，并同时处理米(meters)和公斤(kg)的双计量单位同步扣减/增加，确保库存数据的五维度准确性。

- **`SalesOrderService`** ( `backend/src/services/sales_service.rs` / `sales_order_service.rs` )
  - **说明**：处理销售订单。
  - **关联函数**：创建销售订单时，自动调用财务服务生成应收账款（AR）记录，并锁定对应仓库的产品库存。

### 4.2 前端关键类与函数

- **`App`** ( `frontend/src/app/mod.rs` )
  - **说明**：Yew 根组件。
  - **职责**：配置 `yew_router::BrowserRouter`，处理页面级别的组件路由分发（如跳转到 `/login` 或 `/dashboard`）。

- **`Navigation`** ( `frontend/src/components/navigation.rs` )
  - **说明**：侧边栏导航组件。读取当前用户的权限列表，动态渲染可见的菜单项。

- **前端 Services (例如 `frontend/src/services/auth.rs`)**
  - **说明**：基于 `gloo_net::http::Request` 封装的异步请求函数，处理与后端的交互。所有响应数据使用 `serde_json` 自动反序列化为 `models/` 下定义的结构体。

---

## 5. 数据流与依赖流转示例

**以“创建销售订单”为例：**
1. **Frontend**: 用户在 `sales_order.rs` 页面填写表单，触发 `SalesService::create_order`。
2. **Frontend -> Backend**: HTTP POST `/api/v1/erp/sales/orders`。
3. **Backend Middleware**: `auth_middleware` 校验 Token，`permission_middleware` 校验是否有 `sales:write` 权限。
4. **Backend Handler**: `sales_order_handler::create_sales_order` 接收 JSON，验证字段合法性。
5. **Backend Service**: `SalesOrderService::create` 开启数据库事务 (`txn`)：
   - 插入 `sales_orders` 主表和 `sales_order_items` 明细表。
   - 调用 `InventoryStockService::reserve_stock` 预扣减库存。
   - 调用 `ArInvoiceService::create_from_sales` 生成应收预估账款。
   - 提交事务 (`txn.commit()`)。
6. **Backend -> Frontend**: 返回 200 OK 及新订单 ID。
7. **Frontend**: 页面提示成功，调用路由跳转回订单列表页。

---

## 6. 项目运行方式及使用说明

### 6.1 环境准备
- 安装 **Rust 工具链** (推荐使用 `rustup`，版本 >= 1.70)。
- 安装 **PostgreSQL** (>= 14) 数据库服务。
- 前端需安装 **Trunk**：`cargo install trunk`
- 添加 WASM 编译目标：`rustup target add wasm32-unknown-unknown`

### 6.2 数据库初始化
后端程序内置了数据库迁移（Migrations）和首次启动引导：
1. 启动空的 PostgreSQL 实例。
2. 首次运行后端时，若未配置数据库或表为空，系统将提供 `/init` 系列接口进行系统初始化建表及创建超级管理员账号。

### 6.3 运行后端
```bash
cd backend
# 复制并配置环境变量
cp .env.example .env
# 配置数据库连接，例如：DATABASE_URL=postgres://user:pass@localhost/bingxi_erp

# 开发模式运行
cargo run

# 或编译为 Release 版本（生产环境）
cargo build --release
```
默认情况下，后端服务监听 `0.0.0.0:8080`。

### 6.4 运行前端
```bash
cd frontend

# 使用 Trunk 启动开发服务器（支持热更新）
trunk serve --port 8081

# 或构建生产环境静态文件
trunk build --release
# 构建产物将输出到 frontend/dist 目录，可通过 Nginx 部署
```

### 6.5 生产部署与监控
项目在 `deploy/` 和 `monitoring/` 目录提供了相关脚本：
1. **Nginx 配置**：将 `deploy/nginx.conf` 放入 `/etc/nginx/conf.d/`。
2. **Systemd**：使用 `deploy/bingxi-backend.service` 管理后端进程守护。
3. **监控接入**：启动 Prometheus 并加载 `monitoring/prometheus/prometheus.yml`，在 Grafana 中导入 `bingxi-erp-overview.json` 面板，即可监控系统的 QPS、数据库连接池及内存使用情况。

---

## 7. 代码规范与约定

1. **统一错误处理**：后端通过 `thiserror` 统一定义 `AppError`，并在 handler 层通过实现 `IntoResponse` 统一返回标准的 `{"error": "...", "message": "..."}` 格式。不要在业务代码中直接 `unwrap()` 导致 panic。
2. **异步数据库操作**：业务逻辑在操作数据库时必须传递或使用 `&DatabaseConnection` 或 `&DatabaseTransaction`。跨表修改必须包裹在事务中。
3. **前端状态管理**：跨组件共享状态尽量使用 Yew Context API 或通过 URL 路由参数传递，保持组件状态的单一数据源。
4. **日志打印**：使用 `tracing::info!`, `warn!`, `error!` 记录关键业务节点，尤其是在核心单据流转和权限拦截处。

---
*文档维护者：秉羲团队*  
*生成时间：2026-04-05*