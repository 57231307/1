# 后端技术债务重构 + 安全增强 设计文档

| 字段 | 值 |
| --- | --- |
| 文档版本 | 1.0 |
| 创建日期 | 2026-06-03 |
| 关联实施计划 | docs/superpowers/plans/2026-06-03-backend-refactor-security.md |
| 状态 | 待用户审核 |
| 范围 | P0 + P1 + P2 全量 + 4 维安全增强 |

## 1. 背景与动机

### 1.1 当前现状（已实测）

| 维度 | 数据 |
| --- | --- |
| 后端 .rs 文件数 | 426 |
| 后端代码总行数 | 106 483 |
| 前端 .ts+vue 文件数 | 188 |
| 前端代码总行数 | 57 269 |
| 后端/前端行数比 | 1.86×（偏高） |
| handlers 文件数 | 107 |
| services 文件数 | 106 |
| models 文件数 | 155（含疑似孤儿） |
| 路由注册调用数 | 810 个 handler 调用 / 752 个 `.route()` |
| 死代码模块 | `trading_handler.rs`（1 文件） |
| 巨型单文件（>1000 行） | 7 个 |

### 1.2 已识别问题

| 严重度 | 问题 | 证据 |
| --- | --- | --- |
| P0 | `routes/mod.rs` 2659 行，752 路由全铺平 | `wc -l` 实测 |
| P0 | `advanced_handler.rs` 1366 行 / 39 个 fn | `wc -l` + grep 实测 |
| P0 | 1 个死代码模块未清理 | `trading_handler.rs` 未出现在 `routes/mod.rs` 引用列表 |
| P1 | 4 个 service 1500~2122 行 | `wc -l` 实测 |
| P1 | 4 个 `*_enhanced_handler.rs` 与基础版功能可能重复 | 文件名重复模式 |
| P1 | 155 个 models 数量大于 services | 比对实测 |
| P2 | `bin/cli.rs` 1100 行 + 静态资源耦合 | `wc -l` + 文件内容 |
| P2 | `utils/app_state` 被所有层引用 | grep `use AppState` |
| 安全 | 错误处理可能泄露敏感信息 | 散落的 `e.to_string()` |
| 安全 | 鉴权缺 JTI 黑名单、Token 轮换 | middleware/auth.rs 现状 |
| 安全 | 输入验证不统一 | 部分 handler 无 validator |
| 安全 | HTTP 安全响应头未统一 | 主路由层无 SetResponseHeaderLayer |

## 2. 目标

### 2.1 必须达成

1. **功能零变化**：所有 810 个 API 端点路径、参数、响应体保持一致
2. **行为零变化**：每个 handler 业务逻辑、状态机、并发模型不变
3. **安全性 ≥ 当前**：每一步都做安全检查，必要时提升
4. **CI 全绿**：`cargo fmt --all && cargo clippy -- -D warnings && cargo test --lib` 全通过
5. **可一键回滚**：单一 commit，必要时可 revert

### 2.2 加分项

1. **编译时间下降**：单文件巨型结构拆分后增量编译 -50%
2. **可读性提升**：所有文件 < 800 行
3. **Onboarding 加速**：新成员 1 天看懂 routes
4. **安全基线提升**：加 4 维安全防护

## 3. 总体方案

### 3.1 阶段划分（5 阶段，AI 并行可拆分）

```
Phase 1 ─ 路由拆分（routes/mod.rs 2659 → 15 个小文件）
   │       AI subagent 1: 创建新文件 + 迁移路由
   │       AI subagent 2: 同步修改 main.rs / OpenAPI
   ↓
Phase 2 ─ Handler 拆分（advanced_handler.rs 1366 → 6 个 + 删 trading_handler）
   ↓
Phase 3 ─ Service 拆分（7 个超大 service → 22 个子领域文件）
   ↓
Phase 4 ─ 横向收尾（4 个 enhanced 合并 + models 孤儿清理 + bin/cli 拆分）
   ↓
Phase 5 ─ 安全增强（4 维度横切）
   ↓
Phase 6 ─ 验证 + 推送
```

### 3.2 目录结构最终态

```
backend/src/
├── handlers/
│   ├── mod.rs                    # 统一导出（< 100 行）
│   ├── auth_handler.rs           # 保留
│   ├── ...
│   ├── advanced/                 # 【新】从 advanced_handler.rs 拆分
│   │   ├── mod.rs
│   │   ├── forecast.rs           # 销售预测
│   │   ├── analytics.rs          # 五维分析
│   │   ├── recommendation.rs     # 智能推荐
│   │   ├── reorder.rs            # 补货建议
│   │   └── decision.rs           # 智能决策
│   └── (trading_handler.rs 删除)
├── services/
│   ├── mod.rs
│   ├── ...
│   ├── report/                   # 【新】从 report_engine_service 拆分
│   │   ├── mod.rs
│   │   ├── template.rs
│   │   ├── datasource.rs
│   │   ├── exporter.rs
│   │   └── scheduler.rs
│   ├── purchase/                 # 【新】
│   │   ├── mod.rs
│   │   ├── order.rs              # 原 purchase_order_service
│   │   ├── contract.rs
│   │   ├── receipt.rs
│   │   ├── price.rs
│   │   └── return.rs
│   ├── sales/                    # 【新】
│   │   ├── mod.rs
│   │   ├── order.rs
│   │   ├── contract.rs
│   │   ├── delivery.rs
│   │   ├── price.rs
│   │   └── return.rs
│   ├── crm/                      # 【新】
│   │   ├── mod.rs
│   │   ├── customer.rs
│   │   ├── lead.rs
│   │   ├── opportunity.rs
│   │   ├── pool.rs
│   │   └── assignment.rs
│   ├── inventory/                # 【新】
│   │   ├── mod.rs
│   │   ├── transfer.rs
│   │   ├── adjustment.rs
│   │   ├── count.rs
│   │   ├── stock.rs
│   │   ├── reservation.rs
│   │   └── batch.rs
│   ├── ai/                       # 【新】
│   │   ├── mod.rs
│   │   ├── forecast.rs
│   │   ├── anomaly.rs
│   │   └── recommendation.rs
│   ├── ar/                       # 【新】
│   │   ├── mod.rs
│   │   ├── reconciliation.rs
│   │   ├── invoice.rs
│   │   ├── payment.rs
│   │   └── verification.rs
│   └── cli/                      # 【新】从 bin/cli.rs 拆分
│       ├── mod.rs
│       ├── admin.rs              # 管理员子命令
│       ├── migrate.rs            # 迁移子命令
│       └── util.rs               # 工具子命令
├── routes/                       # 【重做】从 1 个 2659 行 → 15 个
│   ├── mod.rs                    # < 200 行：仅 nest + Swagger
│   ├── auth_routes.rs
│   ├── identity_routes.rs
│   ├── catalog_routes.rs
│   ├── inventory_routes.rs
│   ├── sales_routes.rs
│   ├── purchase_routes.rs
│   ├── finance_routes.rs
│   ├── production_routes.rs
│   ├── crm_routes.rs
│   ├── analytics_routes.rs
│   ├── system_routes.rs
│   ├── tenant_routes.rs
│   ├── static_assets.rs
│   └── api_v1.rs                 # 统一前缀
├── models/
│   ├── mod.rs
│   ├── ...
│   └── _legacy/                  # 【新】孤儿 models 隔离区
└── ...
```

## 4. 阶段 1：路由拆分（routes/mod.rs）

### 4.1 拆分原则

- **按业务域切分**：每个路由文件 200~500 行
- **不改变路径**：所有 752 个 `.route()` 调用原样迁移
- **统一 nest**：在 `mod.rs` 用 `nest("/api/v1/erp", ...)` 拼接

### 4.2 子任务清单

| 子任务 | 内容 | 验证 |
| --- | --- | --- |
| 1.1 | 创建 14 个 `*_routes.rs` 骨架 | `wc -l` |
| 1.2 | 把原 `mod.rs` 的 86 个 `Router::new()` 块按业务域分发到各子文件 | grep 对比 |
| 1.3 | 静态资源（`/static/*`、`/wasm/*`）迁出到 `static_assets.rs` | `wc -l` 减 200+ |
| 1.4 | 新 `mod.rs` 改为 `pub fn create_router` 调用各子模块 | `cargo check` |
| 1.5 | OpenAPI/Swagger 路径同步 | 启动后访问 swagger UI |

### 4.3 验证指标

- `wc -l src/routes/*.rs` 每个文件 < 500 行
- `grep -c '\.route(' src/routes/*.rs` 累计 = 752（不变）
- `grep -c 'Router::new' src/routes/*.rs` 累计 = 86（不变）

## 5. 阶段 2：Handler 拆分（advanced_handler + trading）

### 5.1 advanced_handler.rs 拆分

原文件 1366 行 / 39 个 pub fn。拆为：

| 新文件 | 行数预估 | 函数归属（按业务领域） |
| --- | --- | --- |
| `advanced/forecast.rs` | ~300 | sales_forecast, demand_forecast, time_series |
| `advanced/analytics.rs` | ~280 | five_dimension, kpi, statistical |
| `advanced/recommendation.rs` | ~260 | product_recommend, customer_recommend, supplier_recommend |
| `advanced/reorder.rs` | ~240 | reorder_point, safety_stock, eoq |
| `advanced/decision.rs` | ~200 | smart_decision, anomaly_detection |

### 5.2 trading_handler.rs 删除

- 在路由层无引用
- 删除前 grep 全代码确认无 `trading_handler::` 引用
- 删除后 `cargo build` 必须通过

## 6. 阶段 3：Service 拆分

### 6.1 拆分原则

- **按子领域**：每个新 service 单一职责
- **接口兼容**：原 `use` 路径可通过 `pub use` 重新导出保兼容
- **测试覆盖**：拆分后必须 `cargo test --lib` 通过

### 6.2 拆分清单

| 原 service | 行数 | 拆为 | 行数预估 |
| --- | --- | --- | --- |
| report_engine_service | 2122 | report/{template, datasource, exporter, scheduler} | 400~600/个 |
| purchase_order_service | 1752 | purchase/{order, contract, receipt, price, return} | 300~500/个 |
| sales_service | 1661 | sales/{order, contract, delivery, price, return} | 300~500/个 |
| crm_service | 1469 | crm/{customer, lead, opportunity, pool, assignment} | 250~400/个 |
| inventory_transfer_service | 1202 | inventory/{transfer, adjustment, count, stock, reservation, batch} | 200~400/个 |
| ai_analysis_service | 1202 | ai/{forecast, anomaly, recommendation} | 350~500/个 |
| ar_reconciliation_service | 1121 | ar/{reconciliation, invoice, payment, verification} | 250~350/个 |

### 6.3 兼容性保证

```rust
// services/mod.rs 示例
pub mod purchase {
    pub mod order;
    pub mod contract;
    // ...
    pub use order::PurchaseOrderService;  // 兼容旧路径
}
```

## 7. 阶段 4：横向收尾

### 7.1 enhanced handler 合并

| enhanced 文件 | 基础版 | 动作 |
| --- | --- | --- |
| `ar_reconciliation_enhanced_handler.rs` | `ar_reconciliation_handler.rs` | 逐函数对比，重复的合并进基础版 |
| `audit_enhanced_handler.rs` | `audit_handler.rs` | 同上 |
| `currency_enhanced_handler.rs` | `currency_handler.rs` | 同上 |
| `report_enhanced_handler.rs` | `report_engine_handler.rs` | 同上 |

合并后预计节省 4 个文件 / 1500+ 行。

### 7.2 models 孤儿清理

- 对照 `migration/` 目录
- 没有对应 migration 的 models 移到 `models/_legacy/`
- 加 `#[deprecated(note = "...")]` 注解
- 不删除，仅隔离

### 7.3 bin/cli.rs 拆分

| 子命令 | 拆为 | 行数预估 |
| --- | --- | --- |
| 管理员（密码重置、用户管理） | `cli/admin.rs` | 350 |
| 迁移相关 | `cli/migrate.rs` | 300 |
| 工具（清理、备份） | `cli/util.rs` | 250 |
| 入口 | `cli/mod.rs` | 200 |

## 8. 阶段 5：安全增强（4 维度）

### 8.1 错误处理与脱敏

**改动**：
1. `utils/error.rs` 增强 `AppError`：
   - 统一错误响应体（`{code, message, trace_id, timestamp}`）
   - 生产环境 `Display` 实现脱敏（不暴露内部错误细节）
   - 开发环境保留详细错误
2. `bin/cli.rs` 中所有 `unwrap_or_else(|_| "39.99.34.194".to_string())` 移除（**已经在上一次修复中处理**）
3. 移除硬编码默认密码（`admin123` 等）

**文件**：`utils/error.rs`、新 `middleware/error_handler.rs`

### 8.2 鉴权与会话

**改动**：
1. JWT JTI 黑名单（新增 `services/auth_service::jti_blacklist()`）
2. Refresh Token 一次性使用（轮换）
3. Argon2id 替换 bcrypt（如果当前用 bcrypt）
4. 强化 `middleware/auth.rs` 校验

**文件**：`middleware/auth.rs`、`services/auth_service.rs`

### 8.3 输入验证与防注入

**改动**：
1. 引入 `validator` crate 并 `derive(Validate)`
2. 全部 Request DTO 加 `#[derive(Validate)]`
3. 新增 `middleware/sql_injection_audit.rs`：白名单 + 危险模式检测
4. 统一错误返回 400 + 详细字段

**文件**：`Cargo.toml`（加 `validator`）、`middleware/validation.rs`

### 8.4 HTTP 安全响应头

**改动**：
1. 新建 `middleware/security_headers.rs`：
   - `Content-Security-Policy: default-src 'self'`
   - `Strict-Transport-Security: max-age=63072000; includeSubDomains; preload`
   - `X-Content-Type-Options: nosniff`
   - `X-Frame-Options: DENY`
   - `Referrer-Policy: no-referrer`
   - `Permissions-Policy: geolocation=(), microphone=()...`
2. CORS 白名单配置化（环境变量 `CORS_ALLOWED_ORIGINS`）
3. 在 `routes/mod.rs::create_router` 末尾 `.layer(security_headers)`

**文件**：新 `middleware/security_headers.rs`

## 9. 阶段 6：验证 + 推送

### 9.1 本地验证

```bash
cd /workspace/backend

# 1. 格式检查
cargo fmt --all

# 2. Clippy
cargo clippy --all-targets -- -D warnings

# 3. 单元测试
cargo test --lib --jobs 2

# 4. 编译
cargo build --release
```

### 9.2 路由数量回归测试

```bash
# 拆分前后路由总数必须一致
grep -c '\.route(' src/routes/*.rs
# 期望：752

grep -c 'pub fn\|pub async fn' src/handlers/**/*.rs
# 期望：拆分后总数 ≥ 拆分前
```

### 9.3 API 契约测试

- 用现有 OpenAPI schema 验证路径不变
- 启动服务后用 curl 抽查 30 个核心 API

### 9.4 推送 CI

```bash
git add -A
git commit -m "refactor(backend): 路由/handler/service 拆分 + 4 维安全增强

- routes/mod.rs 2659 行 → 15 个子文件
- advanced_handler.rs 1366 行 → 5 个子模块
- 7 个超大 service 拆分为 22 个子领域文件
- 4 个 enhanced handler 合并
- models 孤儿清理
- bin/cli.rs 1100 行 → 4 个子命令
- 错误处理统一 + 脱敏
- JWT JTI 黑名单 + Token 轮换
- 引入 validator 派生宏
- HTTP 安全响应头中间件
- CORS 白名单配置化

功能零变化，API 契约零变化，安全性提升。"
git push origin main
```

## 10. AI 并行执行方案

### 10.1 并行分组（独立性分析）

| 阶段 | 并行可行性 | 说明 |
| --- | --- | --- |
| Phase 1（路由拆分） | ✅ 高度并行 | 14 个新文件相互独立 |
| Phase 2（handler 拆分） | ✅ 高度并行 | 5 个新文件 + 1 个删除 |
| Phase 3（service 拆分） | ✅ 高度并行 | 7 个独立子树 |
| Phase 4（横向收尾） | ⚠️ 中度并行 | enhanced 合并、models 清理、cli 拆分可并行 |
| Phase 5（安全增强） | ⚠️ 中度并行 | 4 维度部分独立 |
| Phase 6（验证） | ❌ 串行 | 依赖前面所有阶段 |

### 10.2 subagent 调度策略

```
Round 1（并行，6 subagent）:
  - subagent-A: 路由拆分 Phase 1
  - subagent-B: handler 拆分 Phase 2
  - subagent-C: service 拆分 Phase 3 (purchase + sales)
  - subagent-D: service 拆分 Phase 3 (crm + inventory)
  - subagent-E: service 拆分 Phase 3 (ai + ar + report)
  - subagent-F: 阶段 4 横向收尾（enhanced/models/cli）

Round 2（等 Round 1 完成，并行 4 subagent）:
  - subagent-G: 错误处理脱敏
  - subagent-H: 鉴权与会话
  - subagent-I: 输入验证
  - subagent-J: HTTP 安全头

Round 3（串行，主 agent）:
  - cargo fmt / clippy / test
  - 修复冲突
  - commit + push
```

### 10.3 冲突预防

- 每个 subagent 只能修改指定文件列表
- `mod.rs` 由主 agent 在 Round 1 末尾统一修改
- 共享修改（如 `Cargo.toml`、middleware）放主 agent

## 11. 风险与回滚

| 风险 | 概率 | 影响 | 缓解 |
| --- | --- | --- | --- |
| 拆分引入新 bug | 中 | 高 | 拆分前后 grep 路由总数 / 编译验证 |
| Import 错位 | 高 | 中 | `cargo check` 每 subagent 完成时跑一次 |
| 孤儿 models 误删 | 低 | 高 | 不删，仅隔离到 `_legacy/` |
| 安全增强破坏现有流程 | 中 | 高 | 先跑 `cargo test` 基线，每步回归 |
| CI 工具链 1.94 不全 | 中 | 中 | 复用 `eef6f2d / 588e13b` 的 `#[allow]` 模式 |
| subagent 误操作 | 中 | 高 | 每 subagent 输出变更文件清单，主 agent 审核 |

**回滚**：`git revert HEAD` 一键回退。

## 12. 验收标准

### 12.1 量化指标

- [ ] 后端 426 文件 → 约 480 文件（净增是路由 + 子模块带来的）
- [ ] 单文件最大行数 < 1500（当前 2659）
- [ ] 死代码模块数 = 0
- [ ] routes/*.rs 单文件 < 500 行
- [ ] services/ 子目录平均文件 < 500 行
- [ ] `cargo fmt --check` 0 警告
- [ ] `cargo clippy -- -D warnings` 0 警告
- [ ] `cargo test --lib` 全通过
- [ ] CI/CD 推送后通过

### 12.2 安全指标

- [ ] 错误响应不泄露 stack trace
- [ ] JWT 含 JTI 且可吊销
- [ ] Refresh Token 一次性
- [ ] 全部 Request DTO 有 `#[derive(Validate)]`
- [ ] 响应头含 CSP/HSTS/X-CTO/XFO/Referrer-Policy/Permissions-Policy
- [ ] CORS 白名单配置化

### 12.3 功能指标

- [ ] 810 个 API 端点路径不变
- [ ] OpenAPI schema 与原版完全一致
- [ ] 30 个核心 API 端到端测试通过

## 13. 文档关联

- 实施计划：待 writing-plans 生成
- 上轮修复记录：[docs/PROJECT_HEALTH_REPORT.md](../PROJECT_HEALTH_REPORT.md)
- 上一轮计划：[2026-06-03-comprehensive-bug-fix.md](../plans/2026-06-03-comprehensive-bug-fix.md)
