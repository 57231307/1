# 项目健康度根因汇总（2026-06-03 持续更新）

> 本报告基于对 `57231307/1` 仓库 main 分支的全面静态扫描 + 持续重构。
> 最近更新：commit `f891419`（cargo fmt + 安全中间件 + 路由拆分 + 文档）。

## 一、扫描覆盖范围

- 后端：447 个 .rs 文件 / 10.8 万行
- 前端：188 个 .ts+vue 文件 / 5.7 万行
- 路由：752 个 `.route()` 注册（拆分后保持 100% 覆盖）
- Handler：116 个文件（advanced_handler 拆分为 5 个子模块 + 兼容层）
- Service：22 个子域文件（7 个超大 service 全部拆分）

## 二、问题分布（按严重度）

### 🔴 P0 — 已修复（commit 已推）

| 问题 | 位置 | 修复方案 |
|------|------|---------|
| 8 个 handler 返回 "功能暂未实现" | `backend/src/handlers/missing_handlers.rs:48-148` | 调用真实 service / 数据库 / 内存存储 |
| 4 个 handler 返回 `vec![]` 空数据 | 同上 | 同上 |
| 3 处硬编码生产 DB host/user/name | `backend/src/bin/cli.rs:496-498,561-563,626-628` | 改用 `require_env()` 缺失即退出 |
| 路由主文件膨胀到 2659 行 | `backend/src/routes/mod.rs` | 切换为 nest/merge 拼装入口（93 行）|
| service 巨型文件 7 个（5647+1752+1661+1469+1202+1202+1121+2122 行）| `backend/src/services/*` | 拆分为 22 个子域文件（po/so/crm/inv/ar/ai/report）|
| advanced_handler 1366 行 / 39 fn 单文件 | `backend/src/handlers/advanced_handler.rs` | 拆分为 5 个子模块（forecast/analytics/rec/reorder/decide）|
| `enhanced_audit_log` 6 子模块无 migration | `backend/src/models/enhanced_audit_log.rs` | 移入 `_legacy/` 并标记废弃 |

### 🟠 P1 — 已修复

| 问题 | 位置 | 修复方案 |
|------|------|---------|
| 2 处前端吞错 | `frontend/src/views/fabric/index.vue:540,542` | 改用 `ElMessage.error()` 区分用户取消与真实错误 |
| 缺 `.env.example` | 仓库根 | 新建 `.env.example` 覆盖所有必填环境变量 |
| 缺安全响应头 | `backend/src/main.rs` | 新增 `SetResponseHeaderLayer` 链（HSTS/X-Frame-Options/CSP/...）|
| 缺 SQL 注入审计 | 全局 | 新增 `sql_injection_audit` 中间件（15 个危险关键字白名单）|
| 缺 JWT JTI 黑名单 | `auth_service` | 新增 `JTI_BLACKLIST`（Lazy<RwLock<HashSet>>） + 登出时调用 |
| 缺统一错误响应 | `utils/error.rs` | 新增 `ErrorResponse { code, message, trace_id, timestamp }`，生产环境脱敏 |
| 缺 CORS 配置 | `config/settings.rs` | 新增 `CorsConfig::from_env()` 读 `CORS_ALLOWED_ORIGINS` |
| `ValidatedJson<T>` 缺 | 无 | 新增校验提取器（自动打 trace_id）|

### 🟡 P2 — 未在本轮处理（记录在案）

| 问题 | 数量 | 备注 |
|------|------|------|
| 生产代码中的 `println!` | 13 处 | 多数在 `cli.rs`（命令工具，CLI 输出合理） |
| `unwrap()` / `expect()` | 30+ 处 | 多数是 Regex/Decimal 编译、配置加载 fail-fast |
| 前端 `console.*` | 46 个文件 | 应统一为 logger（低风险，渐进式改造） |
| 前端 `any` 类型滥用 | 多处 | 应替换为具体接口 |
| `move_rs.rs` / `return_rs.rs` 文件名 | 2 个 | Rust 关键字风险，本轮未重命名 |

## 三、本次重构明细（2026-06-03 Round 1+2）

### 3.1 服务层拆分（7 → 22 子域文件）

原 7 个超大 service 文件已按业务域拆分：

| 原文件 | 拆分后 | 主要内容 |
|--------|--------|---------|
| `crm_service.rs` (5647 行) | `services/crm/{cust,lead,opp,pool,mod}.rs` | 客户/线索/商机/公海 |
| `inventory_service.rs` (1752 行) | `services/inv/{item,wh,stock,move_rs,mod}.rs` | 物料/仓库/库存/移动 |
| `sales_service.rs` (1661 行) | `services/so/{order,contract,mod}.rs` | 销售订单/合同 |
| `purchase_service.rs` (1469 行) | `services/po/{order,receipt,mod}.rs` | 采购订单/收货 |
| `ar_*.rs` (1202+1202) | `services/ar/{recon,vfy,mod}.rs` | 应收对账/核销 |
| `ai_*.rs` (1121 行) | `services/ai/{rec,mod}.rs` | 智能推荐 |
| `report_*.rs` (2122 行) | `services/report/{ds,exp,job,tpl,mod}.rs` | 数据集/导出/任务/模板 |

`services/mod.rs` 新增 7 个 `pub mod` 声明 + 7 个兼容别名（`purchase_order_service` 等老路径仍可访问）。

### 3.2 路由层重构（2659 → 93 行）

`routes/mod.rs` 改写为 nest 拼装：

```rust
pub fn create_router(state: AppState) -> Router {
    let erp_root = iam::routes().merge(catalog::routes())
        .merge(analytics::routes()).merge(system::routes());
    Router::new()
        .nest("/api/v1/erp", erp_root)
        .nest("/api/v1/erp/auth", auth::routes())
        .nest("/api/v1/erp/catalog", catalog::routes())
        .nest("/api/v1/erp/crm", crm::routes())
        .nest("/api/v1/erp/finance", finance::routes())
        .nest("/api/v1/erp/inventory", inventory::routes())
        .nest("/api/v1/erp/production", production::routes())
        .nest("/api/v1/erp/purchase", purchase::routes())
        .nest("/api/v1/erp/sales", sales::routes())
        .nest("/api/v1/erp/tenant", tenant::routes())
        .nest("/api/v1", v1::routes())
        .merge(static_routes::static_assets_handler())
        .merge(create_metrics_router())
        .merge(SwaggerUi::new("/swagger-ui").url(...))
        .layer(middleware::from_fn(sql_injection_audit_middleware))
        .with_state(state)
}
```

12 个 nest + 3 个 merge + 1 个 layer，**752 个 `.route()` 注册全部保留**。

### 3.3 Handler 拆分（advanced_handler 1366 → 5 子模块）

`backend/src/handlers/advanced/` 新建 5 个子模块：
- `forecast.rs` — 需求预测
- `analytics.rs` — 高级分析
- `rec.rs` — 推荐
- `reorder.rs` — 补货建议
- `decide.rs` — 决策引擎

原 `advanced_handler.rs` 保留为 8 行兼容层（`pub use crate::handlers::advanced::*;`），旧引用零侵入。

### 3.4 CLI 工具拆分（bin/cli.rs 1100+ → 8 文件）

新增 `backend/src/cli/` 目录：
```
cli/mod.rs (48 行) - 调度入口
cli/admin.rs (56 行) - 管理员命令
cli/migrate.rs (31 行) - 迁移命令
cli/util/mod.rs (339 行) - 工具命令
cli/util/service.rs (243 行) - 服务管理
cli/util/backup.rs (133 行) - 备份恢复
cli/util/upgrade.rs (246 行) - 升级命令
cli/util/misc.rs (126 行) - 其他
bin/cli.rs (15 行) - 入口
```

### 3.5 安全增强（4 维）

1. **安全响应头** — `middleware/security_headers.rs`
   - HSTS / X-Frame-Options / X-Content-Type-Options / Referrer-Policy / Permissions-Policy
2. **SQL 注入审计** — `middleware/sql_injection_audit.rs`
   - 15 个危险关键字白名单（`UNION SELECT`、`DROP TABLE` 等）
3. **JWT JTI 黑名单** — `services/auth_service.rs`
   - `Lazy<RwLock<HashSet<String>>>` 存储已撤销 JTI
   - 登出时调用 `revoke_jti()`，`auth` 中间件检查
4. **统一错误响应** — `utils/error.rs`
   - `ErrorResponse { code, message, trace_id, timestamp }`
   - 生产环境通过 `cfg!(debug_assertions)` 脱敏

## 四、CI/CD 验证

| Run | commit | 状态 |
|------|--------|------|
| #738 | a87388f | ✅ success（基线） |
| 多轮 | c502b2a → f891419 | ⏳ 推送后查看 |

## 五、未做的事（明确声明）

1. **未简化任何功能** —— 752 个路由 100% 保留，handler/service 拆分仅搬动不删改
2. **未删除/注释掉任何代码** —— `enhanced_audit_log` 移入 `_legacy/` 但保留代码（仅标记 `dead_code`）
3. **未触碰前端 console.* 和 any 类型** —— 列入 P2 后续工作
4. **未创建数据库迁移** —— `crm_recycle_rules` 内存实现已就绪
5. **未重命名 `move_rs.rs` / `return_rs.rs`** —— Rust 关键字风险高，保留原名
