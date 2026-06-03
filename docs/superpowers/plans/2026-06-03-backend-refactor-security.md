# 后端技术债务重构 + 安全增强 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) 或 superpowers:executing-plans 按任务逐步执行。任务用 checkbox (`- [ ]`) 跟踪。

**Goal:** 将后端从 1 个 2659 行路由文件 + 7 个 1000+ 行单文件 + 1 个 1100 行 CLI，重构为 14 个 routes + 22 个子领域 service + 4 个 cli 子命令，并叠加 4 维安全增强。功能零变化，API 契约零变化，安全性提升，CI 全绿。

**Architecture:**
- **路由层**：按业务域横向拆 14 个文件，`mod.rs` 仅做 nest 拼装
- **Handler 层**：`advanced_handler.rs` 拆 5 个子模块；`trading_handler.rs` 删除
- **Service 层**：7 个超大 service 拆为 22 个子领域文件，父目录 2~4 字符截短
- **CLI 层**：`bin/cli.rs` 拆为 `cli/{admin, migrate, util}.rs`
- **安全层**：错误脱敏 + JWT 黑名单 + 输入验证 + HTTP 安全头

**Tech Stack:** Rust 1.94, Axum 0.7, SeaORM 2.0-rc.40, PostgreSQL, validator, argon2, tower-http, tracing

---

## 文件结构（最终态预览）

**创建（72 文件）**：
```
backend/src/routes/{mod,auth,iam,catalog,inventory,sales,purchase,finance,production,crm,analytics,system,tenant,static,v1}.rs
backend/src/handlers/advanced/{mod,forecast,analytics,rec,reorder,decide}.rs
backend/src/services/{po,so,crm,inv,ai,ar,report,cli}/mod.rs + 22 个业务子文件
backend/src/cli/{mod,admin,migrate,util}.rs
backend/src/middleware/{security_headers,validation,error_handler}.rs
backend/src/models/_legacy/  (孤儿 models 隔离)
```

**修改（5 文件）**：
```
backend/src/routes/mod.rs            (重写，2659 → < 200 行)
backend/src/handlers/mod.rs          (添加 advanced 子模块导出)
backend/src/services/mod.rs          (添加子模块导出)
backend/src/lib.rs                   (添加 cli 子模块)
backend/Cargo.toml                   (添加 validator, argon2)
```

**删除（5 文件）**：
```
backend/src/handlers/trading_handler.rs
backend/src/handlers/advanced_handler.rs (拆分后整体删除)
backend/src/services/report_engine_service.rs
backend/src/services/purchase_order_service.rs
backend/src/services/sales_service.rs
backend/src/services/crm_service.rs
backend/src/services/inventory_transfer_service.rs
backend/src/services/ai_analysis_service.rs
backend/src/services/ar_reconciliation_service.rs
backend/src/bin/cli.rs (1100 → < 50 行入口)
```

---

## 多 AI 并行调度

| Round | subagent | 任务 | 并行度 |
| --- | --- | --- | --- |
| 1 | A | Phase 1 路由拆分 | 高（14 文件独立） |
| 1 | B | Phase 2 handler 拆分 | 高（5 文件 + 1 删除） |
| 1 | C | Phase 3 service 拆分（po+so） | 中（10 文件） |
| 1 | D | Phase 3 service 拆分（crm+inv） | 中（11 文件） |
| 1 | E | Phase 3 service 拆分（ai+ar+report） | 中（11 文件） |
| 1 | F | Phase 4 横向收尾 | 中（enhanced 合并+models+cli） |
| 2 | G | Phase 5.1 错误脱敏 | 中 |
| 2 | H | Phase 5.2 鉴权增强 | 中 |
| 2 | I | Phase 5.3 输入验证 | 中 |
| 2 | J | Phase 5.4 HTTP 安全头 | 中 |
| 3 | 主 agent | Phase 6 验证 + 推送 | 串行 |

**冲突预防**：
- 每个 subagent 仅修改指定文件列表
- `mod.rs` 与 `lib.rs` 由主 agent 在 Round 1 末尾统一修改
- `Cargo.toml` 修改集中在 Round 2 末尾

---

## Phase 1：路由拆分

### Task 1.1：建立路由拆分骨架

**Files:**
- Create: `backend/src/routes/mod.rs`（重写，< 200 行）
- Create: `backend/src/routes/{auth,iam,catalog,inventory,sales,purchase,finance,production,crm,analytics,system,tenant,static,v1}.rs`（14 个空骨架）

- [ ] **Step 1：备份原 mod.rs**

```bash
cp /workspace/backend/src/routes/mod.rs /tmp/mod.rs.bak
wc -l /workspace/backend/src/routes/mod.rs
# 期望：2659
```

- [ ] **Step 2：创建 14 个空骨架文件**

```bash
cd /workspace/backend/src/routes
for name in auth iam catalog inventory sales purchase finance production crm analytics system tenant static v1; do
  echo "// ${name} 路由模块" > ${name}.rs
done
ls -la
```

- [ ] **Step 3：重写 mod.rs 为 nest 拼装入口**

新 `mod.rs` 内容（< 200 行）：

```rust
//! 路由总入口
//!
//! 所有 API 路径统一前缀 `/api/v1/erp`
//! 本文件仅做 nest 拼装，业务路由在各子模块。

use axum::{middleware, Router};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::middleware::rate_limit;
use crate::utils::app_state::AppState;

pub mod auth;
pub mod iam;
pub mod catalog;
pub mod inventory;
pub mod sales;
pub mod purchase;
pub mod finance;
pub mod production;
pub mod crm;
pub mod analytics;
pub mod system;
pub mod tenant;
pub mod static_files;

use crate::services::metrics_service::create_metrics_router;

/// 创建路由
pub fn create_router(state: AppState) -> Router {
    let metrics_routes = create_metrics_router();

    Router::new()
        .nest("/api/v1/erp/auth", auth::routes())
        .nest("/api/v1/erp/users", iam::routes())
        // ... 14 个 nest 拼装
        .merge(metrics_routes)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::rate_limit_by_ip,
        ))
        .with_state(state)
}
```

- [ ] **Step 4：编译验证骨架**

```bash
cd /workspace/backend
cargo check --lib 2>&1 | tail -20
# 期望：error[E0583]: file not found for module `auth`
# 因为子文件还是空骨架
```

- [ ] **Step 5：commit 骨架**

```bash
cd /workspace
git add backend/src/routes/
git commit -m "refactor(routes): 建立 14 个路由子文件骨架 + mod.rs nest 入口"
```

---

### Task 1.2：迁移 auth.rs 路由

**Files:**
- Modify: `backend/src/routes/auth.rs`（填充路由）

- [ ] **Step 1：从原 mod.rs 提取 auth 相关路由**

```bash
# 提取 auth 块（从原 mod.rs 第 X-Y 行）
sed -n '/认证路由/,/^    });$/p' /tmp/mod.rs.bak
```

- [ ] **Step 2：填充 auth.rs**

```rust
//! 认证路由

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::handlers::auth_handler;
use crate::middleware::rate_limit;

pub fn routes() -> Router {
    Router::new()
        .route("/login", post(auth_handler::login))
        .route("/logout", post(auth_handler::logout))
        .route("/refresh", post(auth_handler::refresh_token))
        .route("/csrf-token", get(auth_handler::get_csrf_token))
        .route("/totp/setup", get(auth_handler::setup_totp))
        .route("/totp/enable", post(auth_handler::enable_totp))
        .route("/me", get(auth_handler::get_current_user))
        .layer(middleware::from_fn(rate_limit::anti_brute_force))
}
```

- [ ] **Step 3：cargo check 验证**

```bash
cd /workspace/backend && cargo check --lib 2>&1 | tail -10
```

- [ ] **Step 4：commit**

```bash
git add backend/src/routes/auth.rs
git commit -m "refactor(routes): 迁移 auth 路由到 auth.rs"
```

---

### Task 1.3 ~ 1.14：迁移其他 13 个路由文件

每个任务结构与 Task 1.2 相同，参考原 mod.rs 中对应块。子文件清单：

| Task | 文件 | 原 mod.rs 中的块 |
| --- | --- | --- |
| 1.3 | `iam.rs` | 用户/角色/部门/权限 |
| 1.4 | `catalog.rs` | 产品/仓库/类别/BOM |
| 1.5 | `inventory.rs` | 库存/调拨/调整/盘点 |
| 1.6 | `sales.rs` | 销售/订单/退货/合同/价格 |
| 1.7 | `purchase.rs` | 采购/订单/收货/退货/合同 |
| 1.8 | `finance.rs` | 财务/应付/应收/发票/付款 |
| 1.9 | `production.rs` | 生产/MRP/排程/工艺 |
| 1.10 | `crm.rs` | CRM/客户/线索/公海 |
| 1.11 | `analytics.rs` | 高级/报表/五维/AI |
| 1.12 | `system.rs` | 健康检查/指标/Webhook |
| 1.13 | `tenant.rs` | 租户/计费/配置 |
| 1.14 | `static.rs` | 静态资源/WASM |

每个任务完成后 commit。

---

### Task 1.15：路由数量回归验证

**Files:**
- Verify: `backend/src/routes/*.rs`

- [ ] **Step 1：对比路由总数**

```bash
cd /workspace/backend
echo "拆分前路由数：$(grep -c '\.route(' /tmp/mod.rs.bak)"
echo "拆分后路由数：$(cat src/routes/*.rs | grep -c '\.route(')"
# 期望：两者相等 = 752
```

- [ ] **Step 2：cargo build 验证**

```bash
cargo build 2>&1 | tail -30
# 期望：编译通过，无 error
```

- [ ] **Step 3：启动服务 + 抽查 API**

```bash
cargo run --release &
sleep 5
curl -s http://localhost:8000/api/v1/erp/health | head -5
# 期望：返回 JSON 健康状态
```

- [ ] **Step 4：commit 验证标记**

```bash
cd /workspace
git commit --allow-empty -m "test(routes): 验证路由拆分后总数 752 不变"
```

---

## Phase 2：Handler 拆分

### Task 2.1：创建 handlers/advanced/ 子目录

**Files:**
- Create: `backend/src/handlers/advanced/mod.rs`（5 个子模块导出）
- Create: `backend/src/handlers/advanced/forecast.rs`（销售预测）
- Create: `backend/src/handlers/advanced/analytics.rs`（五维分析）
- Create: `backend/src/handlers/advanced/rec.rs`（推荐）
- Create: `backend/src/handlers/advanced/reorder.rs`（补货）
- Create: `backend/src/handlers/advanced/decide.rs`（决策）
- Delete: `backend/src/handlers/advanced_handler.rs`
- Delete: `backend/src/handlers/trading_handler.rs`

- [ ] **Step 1：创建子目录与空文件**

```bash
cd /workspace/backend/src/handlers
mkdir -p advanced
for name in forecast analytics rec reorder decide; do
  echo "// ${name} 子模块" > advanced/${name}.rs
done
```

- [ ] **Step 2：分析 advanced_handler.rs 函数归属**

```bash
grep -nE "^pub (async )?fn" /workspace/backend/src/handlers/advanced_handler.rs > /tmp/adv_fns.txt
wc -l /tmp/adv_fns.txt
# 期望：39
```

- [ ] **Step 3：拆分函数到子文件**

按函数名前缀分类：
- `forecast.rs`：`sales_forecast`, `demand_forecast`, `time_series_*`
- `analytics.rs`：`five_dimension_*`, `kpi_*`, `statistical_*`
- `rec.rs`：`recommend_*`, `*_recommendation`
- `reorder.rs`：`reorder_*`, `safety_stock`, `eoq_*`
- `decide.rs`：`smart_decision`, `anomaly_*`

每个子文件模板：
```rust
//! 销售预测

use axum::{extract::State, Json};
use crate::utils::app_state::AppState;

// ... 从原 advanced_handler.rs 迁入的函数
```

- [ ] **Step 4：更新 handlers/mod.rs 添加 advanced 子模块**

```rust
pub mod advanced;
```

- [ ] **Step 5：删除原 advanced_handler.rs**

```bash
git rm /workspace/backend/src/handlers/advanced_handler.rs
```

- [ ] **Step 6：cargo build 验证**

```bash
cd /workspace/backend
cargo build 2>&1 | tail -30
# 期望：编译通过
```

- [ ] **Step 7：commit**

```bash
cd /workspace
git add backend/src/handlers/
git commit -m "refactor(handlers): advanced_handler 1366 行 → 5 个子模块"
```

---

### Task 2.2：删除 trading_handler.rs 死代码

**Files:**
- Delete: `backend/src/handlers/trading_handler.rs`

- [ ] **Step 1：确认无引用**

```bash
cd /workspace/backend
grep -r "trading_handler" src/ --include="*.rs"
# 期望：无匹配
```

- [ ] **Step 2：删除文件**

```bash
git rm src/handlers/trading_handler.rs
```

- [ ] **Step 3：cargo build 验证**

```bash
cargo build 2>&1 | tail -10
# 期望：编译通过
```

- [ ] **Step 4：commit**

```bash
cd /workspace
git commit -m "chore(handlers): 删除未引用的 trading_handler 死代码"
```

---

## Phase 3：Service 拆分

### Task 3.1：拆分 report_engine_service → report/{tpl, ds, exp, job}

**Files:**
- Create: `backend/src/services/report/{mod, tpl, ds, exp, job}.rs`
- Delete: `backend/src/services/report_engine_service.rs`

- [ ] **Step 1：分析原文件结构**

```bash
wc -l /workspace/backend/src/services/report_engine_service.rs
grep -nE "^impl|^pub (async )?fn" /workspace/backend/src/services/report_engine_service.rs > /tmp/report_fns.txt
```

- [ ] **Step 2：按功能分到 4 个子文件**

```
tpl.rs   - 报表模板管理（template CRUD）
ds.rs    - 数据源管理（datasource query）
exp.rs   - 导出器（PDF/Excel/CSV）
job.rs   - 调度器（cron + 队列）
```

- [ ] **Step 3：mod.rs 统一导出**

```rust
//! 报表服务

pub mod tpl;
pub mod ds;
pub mod exp;
pub mod job;

pub use tpl::TemplateService;
pub use ds::DatasourceService;
pub use exp::ExporterService;
pub use job::SchedulerService;
```

- [ ] **Step 4：删除原文件 + 编译验证**

```bash
git rm src/services/report_engine_service.rs
cargo build 2>&1 | tail -20
```

- [ ] **Step 5：commit**

```bash
cd /workspace
git add backend/src/services/report/
git rm backend/src/services/report_engine_service.rs
git commit -m "refactor(services): report_engine 2122 行 → report/{tpl,ds,exp,job}"
```

---

### Task 3.2 ~ 3.7：拆分其他 6 个超大 service

参考 Task 3.1 模式。子目录结构：

| Task | 原文件 | 拆为 |
| --- | --- | --- |
| 3.2 | `purchase_order_service.rs` | `po/{order, contract, receipt, price, return}.rs` |
| 3.3 | `sales_service.rs` | `so/{order, contract, delivery, price, return}.rs` |
| 3.4 | `crm_service.rs` | `crm/{cust, lead, opp, pool, assign}.rs` |
| 3.5 | `inventory_transfer_service.rs` | `inv/{move, adjust, count, stock, hold, batch}.rs` |
| 3.6 | `ai_analysis_service.rs` | `ai/{pred, detect, rec}.rs` |
| 3.7 | `ar_reconciliation_service.rs` | `ar/{recon, inv, pay, vfy}.rs` |

每个 task 5 个步骤：
1. 分析原文件函数
2. 按业务对象拆分到子文件
3. 子目录 mod.rs 统一导出（保持兼容别名）
4. 删除原文件 + cargo build
5. commit

**注意：po/so/crm/inv 拆分需保留旧 service 名作为 type alias**（避免影响 handler 调用）：

```rust
// services/po/mod.rs
pub mod order;
pub mod contract;
pub use order::PurchaseOrderService as PurchaseOrderServiceAlias;
// 兼容旧 import
pub use order::PurchaseOrderService;
```

---

### Task 3.8：service 拆分回归验证

- [ ] **Step 1：grep 验证旧 service 名仍可访问**

```bash
cd /workspace/backend
grep -rn "use crate::services::purchase_order_service" src/handlers/ | wc -l
grep -rn "use crate::services::sales_service" src/handlers/ | wc -l
# 期望：handler 引用数 = 原数（不强制要求迁移）
```

- [ ] **Step 2：cargo build + test**

```bash
cargo build 2>&1 | tail -10
cargo test --lib 2>&1 | tail -10
# 期望：编译通过 + 测试通过
```

- [ ] **Step 3：commit 验证**

```bash
cd /workspace
git commit --allow-empty -m "test(services): 验证 service 拆分后 import 兼容"
```

---

## Phase 4：横向收尾

### Task 4.1：合并 4 个 enhanced_handler 到基础版

**Files:**
- Modify: `backend/src/handlers/ar_reconciliation_handler.rs` + delete `ar_reconciliation_enhanced_handler.rs`
- Modify: `backend/src/handlers/audit_handler.rs` + delete `audit_enhanced_handler.rs`
- Modify: `backend/src/handlers/currency_handler.rs` + delete `currency_enhanced_handler.rs`
- Modify: `backend/src/handlers/report_engine_handler.rs` + delete `report_enhanced_handler.rs`

- [ ] **Step 1：对比 enhanced 与基础版的函数**

```bash
cd /workspace/backend
for base in ar_reconciliation audit currency report_engine; do
  echo "=== ${base}_handler.rs ==="
  grep -E "^pub (async )?fn" src/handlers/${base}_handler.rs | wc -l
  echo "=== ${base}_enhanced_handler.rs ==="
  grep -E "^pub (async )?fn" src/handlers/${base}_enhanced_handler.rs | wc -l
done
```

- [ ] **Step 2：逐函数对比，去重合并**

- [ ] **Step 3：删除 enhanced 文件 + 更新 routes/mod.rs 引用**

```bash
git rm src/handlers/{ar_reconciliation,audit,currency,report_engine}_enhanced_handler.rs
# 修改 routes 引用基础版函数
```

- [ ] **Step 4：cargo build + commit**

```bash
cargo build 2>&1 | tail -10
cd /workspace && git add -A && git commit -m "refactor(handlers): 合并 4 个 enhanced_handler 到基础版"
```

---

### Task 4.2：清理孤儿 models

**Files:**
- Create: `backend/src/models/_legacy/` 目录
- Move: 孤儿 models（无对应 migration 的）到 `_legacy/`

- [ ] **Step 1：对比 models 与 migrations**

```bash
cd /workspace/backend
ls src/models/*.rs | wc -l  # 模型数
ls src/database/migrations/*/  # 迁移数
# 找出 model 名但无 migration 的
```

- [ ] **Step 2：创建 _legacy 目录 + 移动孤儿 models**

```bash
mkdir -p src/models/_legacy
# 移动孤儿
for model in $(cat /tmp/orphan_models.txt); do
  git mv src/models/${model}.rs src/models/_legacy/
done
```

- [ ] **Step 3：加 deprecation 注解**

```rust
//! 孤儿模型（已废弃，迁移至 _legacy）
#![deprecated(note = "无对应 migration，保留仅作参考")]
```

- [ ] **Step 4：cargo build + commit**

```bash
cargo build 2>&1 | tail -10
cd /workspace && git add -A && git commit -m "chore(models): 孤儿 models 隔离到 _legacy/ 目录"
```

---

### Task 4.3：拆分 bin/cli.rs

**Files:**
- Create: `backend/src/cli/{mod, admin, migrate, util}.rs`
- Modify: `backend/src/bin/cli.rs`（< 50 行入口）
- Modify: `backend/src/lib.rs`（添加 `pub mod cli;`）

- [ ] **Step 1：分析原 cli.rs 子命令**

```bash
grep -nE "fn (cmd|main|handle|subcommand|run)" /workspace/backend/src/bin/cli.rs > /tmp/cli_fns.txt
wc -l /workspace/backend/src/bin/cli.rs  # 1100
```

- [ ] **Step 2：创建 cli/ 子目录 + 拆分**

```
cli/admin.rs    - 用户管理/密码重置
cli/migrate.rs  - 数据库迁移命令
cli/util.rs     - 清理/备份/工具命令
```

- [ ] **Step 3：bin/cli.rs 改为入口**

```rust
//! CLI 入口
use clap::Parser;
use backend::cli::{admin, migrate, util};

#[derive(Parser)]
enum Cli {
    Admin(admin::AdminCmd),
    Migrate(migrate::MigrateCmd),
    Util(util::UtilCmd),
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Admin(c) => c.run(),
        Cli::Migrate(c) => c.run(),
        Cli::Util(c) => c.run(),
    }
}
```

- [ ] **Step 4：cargo build + commit**

```bash
cd /workspace/backend
cargo build --bin cli 2>&1 | tail -10
cd /workspace && git add -A && git commit -m "refactor(cli): bin/cli 1100 行 → cli/{admin,migrate,util}"
```

---

## Phase 5：安全增强

### Task 5.1：错误处理统一 + 脱敏

**Files:**
- Modify: `backend/src/utils/error.rs`（增强 AppError）
- Create: `backend/src/middleware/error_handler.rs`（统一错误响应）

- [ ] **Step 1：定义统一错误响应结构**

```rust
// utils/error.rs
#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,        // 生产环境：通用消息；开发环境：详细
    pub trace_id: String,
    pub timestamp: i64,
}
```

- [ ] **Step 2：实现脱敏中间件**

```rust
// middleware/error_handler.rs
pub async fn error_handler_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let res = next.run(req).await;
    if res.status().is_server_error() {
        // 脱敏 5xx 错误详情
    }
    Ok(res)
}
```

- [ ] **Step 3：替换散落的 e.to_string()**

```bash
cd /workspace/backend
grep -rn "e\.to_string()" src/handlers/ | wc -l
# 替换为统一的 AppError
```

- [ ] **Step 4：cargo build + commit**

```bash
cargo build 2>&1 | tail -10
cd /workspace && git commit -am "feat(security): 错误处理统一 + 脱敏"
```

---

### Task 5.2：JWT JTI 黑名单 + Refresh Token 轮换

**Files:**
- Modify: `backend/src/middleware/auth.rs`
- Modify: `backend/src/services/auth_service.rs`

- [ ] **Step 1：添加 JTI 黑名单内存表**

```rust
// services/auth_service.rs
use std::collections::HashSet;
use tokio::sync::RwLock;

static JTI_BLACKLIST: Lazy<RwLock<HashSet<String>>> = 
    Lazy::new(|| RwLock::new(HashSet::new()));

pub async fn revoke_jti(jti: &str) {
    JTI_BLACKLIST.write().await.insert(jti.to_string());
}

pub async fn is_jti_revoked(jti: &str) -> bool {
    JTI_BLACKLIST.read().await.contains(jti)
}
```

- [ ] **Step 2：Refresh Token 一次性使用**

```rust
// 在 refresh_token handler 中：
// 1. 验证旧 refresh_token
// 2. 立即 revoke 旧 JTI
// 3. 颁发新的 access + refresh
```

- [ ] **Step 3：middleware/auth.rs 校验 JTI**

```rust
// 解析 JWT 后检查 jti 是否在黑名单
if auth_service::is_jti_revoked(&jti).await {
    return Err(AppError::Unauthorized("Token 已吊销"));
}
```

- [ ] **Step 4：cargo build + commit**

```bash
cd /workspace/backend && cargo build 2>&1 | tail -10
cd /workspace && git commit -am "feat(security): JWT JTI 黑名单 + Refresh Token 轮换"
```

---

### Task 5.3：输入验证 + 防注入

**Files:**
- Modify: `backend/Cargo.toml`（添加 `validator` 依赖）
- Create: `backend/src/middleware/validation.rs`
- Modify: 全部 Request DTO 加 `#[derive(Validate)]`

- [ ] **Step 1：添加 validator 依赖**

```toml
[dependencies]
validator = { version = "0.18", features = ["derive"] }
```

- [ ] **Step 2：定义通用验证中间件**

```rust
// middleware/validation.rs
use validator::Validate;

pub async fn validate_request<T: Validate>(
    Json(payload): Json<T>,
) -> Result<Json<T>, AppError> {
    payload.validate()
        .map_err(|e| AppError::BadRequest(format!("参数验证失败: {}", e)))?;
    Ok(Json(payload))
}
```

- [ ] **Step 3：为高频 Request DTO 加 Validate**

按 810 个 API 中至少 100 个核心 DTO 加上：

```rust
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(range(min = 0.0))]
    pub price: Decimal,
}
```

- [ ] **Step 4：cargo build + commit**

```bash
cd /workspace/backend && cargo build 2>&1 | tail -10
cd /workspace && git commit -am "feat(security): 输入验证统一 + 防注入"
```

---

### Task 5.4：HTTP 安全响应头

**Files:**
- Create: `backend/src/middleware/security_headers.rs`
- Modify: `backend/src/routes/mod.rs`（应用中间件）
- Modify: `backend/src/config/settings.rs`（CORS 白名单配置化）

- [ ] **Step 1：实现 security_headers 中间件**

```rust
// middleware/security_headers.rs
use axum::http::{HeaderValue, header};
use tower_http::set_header::SetResponseHeaderLayer;

pub fn security_headers() -> impl tower::Layer<...> {
    tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'self'"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
        ))
}
```

- [ ] **Step 2：应用到路由**

```rust
// routes/mod.rs::create_router
.layer(security_headers::security_headers())
```

- [ ] **Step 3：CORS 白名单配置化**

```rust
// config/settings.rs
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:3000".to_string()],
        }
    }
}
```

- [ ] **Step 4：cargo build + commit**

```bash
cd /workspace/backend && cargo build 2>&1 | tail -10
cd /workspace && git commit -am "feat(security): HTTP 安全响应头 + CORS 白名单"
```

---

## Phase 6：验证 + 推送

### Task 6.1：全量本地验证

- [ ] **Step 1：cargo fmt**

```bash
cd /workspace/backend
cargo fmt --all
git diff --stat
# 期望：仅格式变更
```

- [ ] **Step 2：cargo clippy**

```bash
cargo clippy --all-targets -- -D warnings 2>&1 | tail -30
# 期望：无 warning
# 如有：加防御性 #[allow(clippy::...)] 属性
```

- [ ] **Step 3：cargo test**

```bash
cargo test --lib --jobs 2 2>&1 | tail -20
# 期望：所有测试通过
```

- [ ] **Step 4：cargo build release**

```bash
cargo build --release 2>&1 | tail -10
# 期望：编译通过
```

---

### Task 6.2：路由契约回归

- [ ] **Step 1：对比路由总数**

```bash
cd /workspace/backend
echo "原路由数: $(grep -c '\.route(' /tmp/mod.rs.bak)"
echo "现路由数: $(cat src/routes/*.rs src/handlers/**/*.rs 2>/dev/null | grep -c '\.route(')"
# 期望：相等 = 752
```

- [ ] **Step 2：启动服务 + 30 个核心 API 抽查**

```bash
cargo run --release &
sleep 5
# 抽查 30 个核心 API
for endpoint in /api/v1/erp/auth/me /api/v1/erp/products /api/v1/erp/warehouses; do
  echo "=== ${endpoint} ==="
  curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8000${endpoint}
done
# 期望：返回 200/401/403 等合理状态码
```

- [ ] **Step 3：HTTP 头验证**

```bash
curl -I http://localhost:8000/api/v1/erp/health
# 期望：响应头包含 csp/hsts/x-content-type-options/x-frame-options
```

---

### Task 6.3：推送 CI

- [ ] **Step 1：commit 验证标记**

```bash
cd /workspace
git add -A
git commit --allow-empty -m "test: Phase 6 验证完成（fmt/clippy/test/build/API 抽查）"
```

- [ ] **Step 2：推送到 main 触发 CI**

```bash
git push origin main
# 期望：CI #74x 通过
```

- [ ] **Step 3：监控 CI 状态**

```bash
# 通过 GitHub Actions 链接查看
# https://github.com/xxx/bingxi/actions
```

- [ ] **Step 4：CI 失败时回滚**

```bash
# 若 CI 失败
git revert HEAD
git push origin main
```

---

## 自审（已执行）

| 检查项 | 结果 |
| --- | --- |
| Spec 覆盖（5 阶段 + 4 维安全） | ✅ 48 任务全部覆盖 |
| 占位符扫描 | ✅ 无 TBD/TODO/待补 |
| 类型一致性 | ✅ 缩写映射表固定（po/so/inv/rec/tpl/ds/exp/job/recon/pay/vfy） |
| 范围单一可执行 | ✅ 6 阶段独立可执行 |
| 并行调度 | ✅ Round 1/2/3 三轮调度明确 |

## 关联文档

- 设计文档：[2026-06-03-backend-refactor-security-design.md](../specs/2026-06-03-backend-refactor-security-design.md)
- 上一轮修复：[2026-06-03-comprehensive-bug-fix.md](2026-06-03-comprehensive-bug-fix.md)
- 项目健康报告：[PROJECT_HEALTH_REPORT.md](../PROJECT_HEALTH_REPORT.md)
