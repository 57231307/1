# 运行时逻辑关系与守卫机制图谱

生成日期：2026-09-08
依据：源码静态梳理（backend main.rs / bootstrap/* / middleware/* / container / services/event_bus*；frontend src/router/index.ts / src/api/request.ts）

---

## 1. 启动链路（双模式判定）

`main.rs` 顺序：

1. `init_env_and_logging`（加载 .env、初始化 tracing）
2. `connect_database(settings)`：
   - `Ok(db)` → 完整模式：`bootstrap_full_mode(db, settings)` → `apply_full_mode_layers(app_state, cors)`
   - `Err(e)` → 初始化模式（Setup）：`routes_bootstrap::create_init_router()` → `apply_init_mode_layers(router, cors)`；仅暴露 `/init/*`，由 `init_token_middleware`（INIT_TOKEN）保护
3. HTTP server 启动；优雅停机时关闭事件总线 spawn task（L-27/28/29 修复，防 detached task 泄漏）

注意（影响 E2E 判定）：「空库可连接」时后端走完整模式；Setup 模式仅在 DB 完全连不上时出现。

### 1.1 完整模式内部顺序（`bootstrap_full_mode`）

```
run_defensive_migrations → run_seaorm_migrator
→ require_cookie_secret / require_webhook_secret
→ db Arc 包装
→ create_omni_audit_service → create_audit_log_service
→ resolve_audit_retention_days → create_audit_cleanup_service
→ connect_backup_database → create_failover_executor
→ build_app_state(...)
→ 后台任务 spawn（邮件队列 Worker、导出合规审查、追踪数据 90 天保留、
   库存告警调度、供应商评估调度、定时推送调度、PDA 心跳超时清理）
→ init_event_bus → init_assist_dimensions → init_es_indices
→ WebSocket Redis Pub/Sub 订阅器
```

---

## 2. 完整模式中间件栈（`apply_full_mode_layers`）

axum 语义：`.layer()` 调用顺序的逆序 = 请求执行顺序（后 layer 的在最外层、先执行）。

注册顺序（内 → 外）：

 1. `create_router`（路由树：iam + catalog + analytics + system + system_update_extra，约 1506 个 `.route()`）
 2. `apply_body_limit_and_context`：DefaultBodyLimit → audit_context → trace_context（trace_id 注入，最内层）
 3. `apply_metrics_layer`
 4. `apply_http_trace_layer`
 5. `rls_context_middleware`（A.21.2，先于 cors/auth_chain 注册，见下）
 6. `cors`（max_age 86400s）
 7. `apply_auth_chain`（见 §3）
 8. `circuit_breaker_middleware`（V15 P1 20.6-B，5s 窗口失败率 > 50% open，30s half-open）
 9. `dynamic_router_middleware`（V15 P2 20.6-A，按 api_endpoints 表状态放行/拒绝）
 10. `apply_rate_limiting`
 11. `apply_security_headers`
 12. `timeout_middleware`（最外层，`TIMEOUT_SECONDS` 超时包裹）

 请求实际执行顺序为上述的逆序：timeout → security_headers → rate_limiting → dynamic_router → circuit_breaker → auth_chain → cors → rls → http_trace → metrics → body_limit/audit_context/trace_context → handler。

 **RLS 注册位置说明（已修复的缺陷，2026-09-08）**：`rls_context_middleware` 从 `request.extensions()` 读取 auth 中间件注入的 `AuthContext`，按 axum「后注册 layer 位于最外层、先执行」的语义，它必须比 auth_chain 先注册（成为其内层），auth 才能在其之前注入上下文。原实现把 RLS 注册在 `apply_auth_chain` 之后（auth 链外侧），导致 RLS 先于 auth 执行、永远读不到 `AuthContext`、`SET LOCAL app.user_id` 静默跳过，PostgreSQL RLS 策略从未激活。现注册在 cors 之前（cors/auth_chain 更外层）：CORS 预检、非法 Origin、401/403 均在 cors/auth 层短路，RLS 的 SET/RESET 只对真正穿过认证链的请求执行。

---

## 3. auth_chain 内部（`apply_auth_chain`）

注册顺序（内 → 外）：`request_logging` → `permission` → `csrf` → `omni_audit` → `auth`。
执行顺序（外 → 内）：**auth → omni_audit → csrf → permission → request_logging → handler**（与源码注释一致）。

### 3.1 auth_middleware 判定链（middleware/auth.rs）

1. `is_public_path(path)` 判定；命中则直接放行（并在 extensions 写入 `PublicPathCache` 供下游复用）
2. Token 提取优先级：
   - `PrivateCookieJar`（cookie_secret 签名）中的 `access_token`（HttpOnly，主路径）
   - 旧版 `jwt` Cookie（向后兼容，B03-P2-1 后登录/刷新已停止写入）
   - `Authorization: Bearer ...` 头（兜底）
3. 验签（jwt_secret，支持 `previous_jwt_secret` 轮换双密钥）→ 注入 `AuthContext` 到 extensions
4. 失败返回 401；日志对 Token 脱敏（前缀 + 长度）

### 3.2 permission_middleware（middleware/permission.rs）

1. 复用 `PublicPathCache`/`is_public_path` 放行公开路径
2. `admin_checker::is_admin_role(db, role_id)`：管理员通配放行
3. 非 admin：`PERMISSION_CACHE`（DashMap）按 role_id 查缓存；未命中则查 DB `role_permission` 并回填
4. fail-closed：DB 查询失败时拒绝（warn 日志），避免放行敏感操作
5. `check_permission`（行 534）做具体权限码比对

### 3.3 csrf_middleware

- 非安全方法（POST/PUT/PATCH/DELETE）校验 `X-CSRF-Token`
- CSRF token 为一次性消费；并发竞败时后端经 `X-New-CSRF-Token` 头下发恢复 token（前端拦截器据此重放）

### 3.4 公开路径白名单（middleware/public_routes.rs，节选）

```
/health /ready /live
/api/v1/erp/health /ready /live
/health/liveness /health/readiness
/api/v1/erp/auth/login /auth/refresh
/api/v1/erp/lock-status
/api/v1/erp/init/status
/api/v1/erp/webhooks/integrations/callback
/api/v1/erp/init/initialize /initialize-with-db /initialize-with-db-async
```

### 3.5 RLS（middleware/rls_context.rs）

- 设计：已认证请求进入 handler 前绑定行级上下文，激活 PostgreSQL RLS；admin/data_scope=all 与未认证请求无上下文，策略 NULL 放行
- 机制（2026-09-08 重设计，连接池钩子方案）：
  1. `rls_context_middleware` 把非 admin 用户的 user_id 写入 **tokio task-local**（`RLS_USER_ID`），包裹整个请求处理链（handler 及其全部查询同 task）
  2. `connect_database` 安装 sqlx 池钩子（`map_sqlx_postgres_before_acquire` + `map_sqlx_postgres_pool_opts.after_connect`）：**空闲连接借出前 / 新建连接后**，回调在发起查询的请求 task 内执行，读取 task-local，直接在**即将使用的这条连接**上执行 `SELECT set_config('app.user_id', '<id>', false)`（会话级）；无上下文借出则 `RESET app.user_id` 清理残留
  3. GUC 与业务查询天然同连接、同会话 → finance 域迁移的 RLS 策略（`owner_id = current_setting('app.user_id', true)::int`）真正激活
- 安全语义：策略 fail-open（GUC 为 NULL 放行），应用层 `apply_data_scope` 始终兜底；超时被 drop 的连接残留由下次借出的 RESET 分支清理，无跨请求/跨用户泄漏；spawn 旁路任务无 task-local → 无上下文借出 → 与旧行为一致（fail-open）
- 失败降级：钩子内 set_config/RESET 失败仅 warn/debug，连接照常借出（fail-open），不阻断业务
- 旧实现缺陷（已修复）：中间件内 `execute_unprepared("SET LOCAL ...")` 在事务块外无效（PG 丢弃），且独立连接与全局池业务查询不保证同连接——RLS 从未激活，仅每请求两次无效池往返；同时原注册顺序（auth_chain 外侧）使 AuthContext 永远读不到（见 §2「RLS 注册位置说明」）
- PG 机制锚点测试：`tests/rls_context_test.rs::test_rls_guc_visible_in_same_pool_pg`（#[ignore]，TEST_DATABASE_URL + `cargo test -- --ignored`；CI nextest 不跑 ignored，需手动/专项验证）

---

## 4. 依赖注入容器（AppState / DIContainer）

`container/mod.rs` `AppState` 核心字段：

```
db: Arc<DatabaseConnection>
omni_audit / audit_log / audit_cleanup
jwt_secret / previous_jwt_secret（轮换双密钥）
cookie_secret / cookie_key（PrivateCookieJar 签名）
webhook_secret
cache: Arc<AppCache>   metrics: Arc<MetricsService>
di_container: Arc<DIContainer>（utils/di_container.rs，Any 单例注册器）
email_service / event_notification_service（Option）
data_permission_service: Arc<DataPermissionService>
```

`bootstrap_full_mode` 按 §1.1 顺序构建后传入 `create_router` 与各中间件（`from_fn_with_state`）。

---

## 5. 事件总线与业务副作用（services/event_bus*）

- `BusinessEvent` 枚举（event_bus.rs:54 起）覆盖 20+ 业务事件
- 发布点约 50 处（`EVENT_BUS.publish`），例：`purchase_receipt_handler.rs:136` 收货完成后发布 `PurchaseReceiptCompleted`
- 双通道：Kafka 优先，不可达时自动降级 Broadcast（`tracing::error!` 中文日志）；batch-120 P2-10 已删除未接线的 EventBackend/BroadcastBackend 死代码，KafkaBackend 直接持有于 `EventBusState.kafka`
- 订阅分发（event_bus_ops/listener.rs）副作用映射（节选）：

| 事件 | 副作用方向 |
|---|---|
| PurchaseReceiptCompleted | 库存/财务联动 |
| SalesOrderShipped | 发货后联动 |
| BpmProcessFinished | BPM 收尾 |
| LowStockAlert / MaterialShortageAlert | 告警通知 |
| FinancialIndicatorUpdate | 财务指标刷新 |
| CustomerUpdated / SupplierUpdated | 主数据联动 |
| InventoryTransactionCreated / InventoryCountCompleted | 库存台账 |
| QualityInspectionCompleted / FabricInspectionGraded | 质检分级（A/B/C 流向：入库/降级/返工） |
| DyeBatchStatusChanged / DyeBatchCompleted | 染批状态机联动 |
| ProcessStepReported / ProductionQuantityReported | 工序/产量 |
| EnergyConsumptionRecorded | 能耗 |
| ColorCardIssued | 色卡发放 |
| PaymentCompleted / PurchaseOrderApproved | 收付/采购审批 |
| CollectionCompleted | 收账完成（含幂等判定分支） |
| BusinessModeChanged | 业务模式切换广播 |

- 可靠性：`EVENT_BUS_STATE` 统一加锁封装（锁中毒优雅降级恢复）

状态机范式（抽样）：`bom_ops/state.rs`、`custom_order_state_machine_adapter.rs`、`dye_batch_state_machine_ops/{state_machine_adapter,state_rule}.rs`、`dye_batch_state_machine_service.rs`。

---

## 6. 前端守卫链

### 6.1 路由守卫（router/index.ts:1242 `router.beforeEach`）

执行顺序：

1. 设置页面标题（meta.title）
2. `/setup` 直接放行
3. `!to.meta.public && path !== '/login'`：`await checkInitStatus()`；未初始化 → 重定向 `/setup`
4. `to.meta.requiresAuth`：`userStore.userInfo` 为空 → `/login?redirect=<fullPath>`（access_token 在 HttpOnly Cookie，JS 不可读，仅以 userInfo 判定登录态）
5. `to.meta.permission`：`hasRoutePermission(meta.permission, user.permissions)` 失败 → `/403`（批次 3 权限码校验；批次 22 v5 P0-4 补齐各路由 meta.permission）
6. 放行 `next()`

路由表约 150+ 条，meta 携带 `{ title, icon, permission?, requiresAuth }`。

### 6.2 请求拦截器（api/request.ts）

请求侧：

- `withCredentials: true`（HttpOnly Cookie 自动携带；Wave B-3 已移除 localStorage token 与 Authorization 头）
- 非安全方法且非公开路径：从 `document.cookie` 读 `csrf_token` 注入 `X-CSRF-Token`；缺失时 console.warn 提示刷新
- 公开路径（login/refresh/health 等）跳过 CSRF 头

响应侧：

- Blob 响应直接放行交调用方
- 业务码 401 → 跳登录页（Cookie 由后端登出时清除）
- HTTP 403 + `CSRF_TOKEN_MISSING/INVALID` → 恢复重放：优先 `X-New-CSRF-Token` 恢复头，否则读最新 Cookie 重放一次（`_csrfRetry` 标记防循环）；仍失败才清 token 跳登录（P2 修复并发误踢）
- 401 自动刷新：`isRefreshing` + `refreshSubscribers` 队列去重并发刷新（refresh_token Cookie 自动携带，无需前端取 token）
- 拦截器返回 `ApiResponse`（P2 1-11：保留类型信息）

### 6.3 前端状态层

Pinia stores（src/store/）：user / system / dashboard / inventory / fabric / colorCardIssue / sales / index。

---

## 7. 关键调用链示例（真实链路）

「采购收货完成」端到端：

```
前端页面 → axios(interceptors) → POST /api/v1/erp/...
→ timeout → security_headers → rate_limit → dynamic_router → circuit_breaker
→ auth(Cookie/验签/AuthContext) → omni_audit → csrf → permission(权限码)
→ request_logging → cors → [rls(SET LOCAL)] → body_limit/audit_context/trace_context
→ purchase_receipt_handler（事务提交 + EVENT_BUS.publish(PurchaseReceiptCompleted)）
→ event_bus（Kafka/Broadcast 降级）→ listener 副作用（库存/财务联动）
→ 响应沿中间件栈逆序返回（omni_audit 落审计、request_logging 落日志）
```

---

## 8. 风险与待复核清单

1. **RLS layer 顺序（已修复，2026-09-08）**（§2/§3.5）：原注册在 auth_chain 外侧，RLS 先于 auth 执行、`AuthContext` 未注入，`SET LOCAL` 静默跳过、PG RLS 从未激活；已移到 cors 之前（auth_chain 内层）。待 CI（PostgreSQL service）验证。
2. **RLS 事务/连接池有效性（已修复，2026-09-08）**（§3.5）：旧实现 `SET LOCAL` 事务外无效 + 独立连接与业务查询不保证同连接，RLS 从未激活。已重设计为「task-local + 连接池借出钩子」方案：钩子在业务查询的同一连接上执行会话级 `set_config`，无上下文借出时 RESET 清理残留；无泄漏、fail-open、零 service 层侵入。PG 机制锚点测试 `test_rls_guc_visible_in_same_pool_pg`（#[ignore]）待手动验证。
3. **RLS 激活的功能回归风险（已修复——dept 语义落地，2026-09-08）**（§3.5）：RLS 策略 self 级与应用层 DataScope 三级粒度错位，经需求定义（.monkeycode/specs/dept-data-scope-semantics/requirements.md，三项决策：D1 数据部门=归属人部门动态 / D2 可见集合=主+兼职+子部门 / D3 冗余列+触发器维护）与技术设计（design.md）后落地实施：
   - **R3.1（高）dept 级锁出 → 已修复**：迁移 m_rls_dept_domain 给 5 表加冗余 `department_id` 列（触发器 sync_data_department 随 owner_id/created_by 自动维护），策略统一为四分支（NULL fail-open / self 匹配 / 公海-历史分支 / `department_id = ANY(app_dept_ids())`）；rls_context_middleware 构造 RlsGuc{user_id, dept_ids} 写 task-local，钩子单条 SQL 双 set_config；auth.rs 对 dept 用户预加载可见集合（get_user_dept_scope_ids_cached，5min TTL，部门变更主动失效）。
   - **R3.2（高）crm_lead 公海断裂 → 已修复**：新策略补 `OR lead_status='pool'`（crm_lead）/ `OR opportunity_status='pool'`（crm_opportunity）公海分支。
   - **R3.3（中）自动分配无门禁 → 顺带缓解**：应用层 dept 分支修复后（DataScopeContext.dept_ids + DepartmentId 列 IN 过滤），分配者可见范围正确，fetch 不再被错误过滤；强制 admin 门禁仍待加（低优先）。
   - **R3.4（低）BI JOIN 标签 / R3.5（低）导出收窄 → 行为已按 dept 语义修正**：dept 用户 JOIN customers 行不再被锁出；self 用户导出收窄语义不变（顺带修复越权导出）。
   - **应用层既有 bug 顺带修复**：原 6 处 apply_data_scope 调用点把 CreatedBy/OwnerId 误传为 dept_column（dept 用户执行 `created_by = department_id` 恒空集）；已改传 DepartmentId 列（owner_column 语义同步修正：customers/lead/opp 用 OwnerId，supplier/sales_order 用 CreatedBy）。check_resource_owner 的 Dept 分支同步改为 `resource_dept_id ∈ ctx.dept_ids`；build_data_scope_sql 改为 `<alias>department_id = ANY($N::int[])`。
   - 部署 sequencing：先跑迁移（旧后端不设 app.dept_ids → app_dept_ids() 空数组 → dept 退化为 self+公海，安全降级）→ 再发布新后端（恢复部门视角）。
   - 验证：PG 锚点测试 test_rls_guc_visible_in_same_pool_pg（#[ignore]，双 GUC 断言）+ utils_data_scope_test 适配；CI nextest 不跑 ignored，PG 断言需手动/专项验证。
4. **初始化模式面窄依赖**：Setup 模式仅 `/init/*` + INIT_TOKEN；deploy.sh 已自动生成 INIT_TOKEN，人工部署遗漏会导致 init 401。
5. **CSRF 一次性消费**：多标签页并发依赖 `X-New-CSRF-Token` 恢复链路，若后端某路径未下发恢复头，前端只能重放一次。
6. **permission fail-closed**：DB 抖动时全部非公开请求 403，属设计取舍，需在运维监控中区分。
