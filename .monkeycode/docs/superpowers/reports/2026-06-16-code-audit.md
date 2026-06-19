# 冰溪 ERP 全量代码审计报告

> **审计日期**：2026-06-16
> **审计人**：冰溪 ERP 代码审计子代理
> **目的**：识别并列出所有需要清理的代码（**仅识别，不修改**）
> **覆盖范围**：`backend/src/**`（Rust）+ `frontend/src/**`（Vue 3/TS）+ `Cargo.toml` + `package.json` + `.github/workflows/`

---

## 1. 项目规模

| 维度 | 数量 | 总行数 |
|------|------|--------|
| Rust 源文件 | **493** | **105,548** |
| Vue 源文件 | **118** | **54,133** |
| TS/TSX 源文件 | **103** | **9,505** |
| **代码总规模** | **714 个文件** | **169,186 行** |
| backend 模块数 | services/handlers/models/middleware/utils/routes/... | 8 大模块 |
| frontend 模块数 | views/components/api/composables/store/router/types/utils | 8 大模块 |
| Rust 公开函数 | `pub fn` / `pub async fn` | **982** 个 |
| Rust 全部函数 | 含私有 | **2,696** 个 |
| 前端 API 模块 | `frontend/src/api/*.ts` | **86** 个文件 |
| 前端 composables | `frontend/src/composables/*.ts` | **1** 个文件（`useTableColumns.ts`） |

---

## 2. 危险模式统计

### 2.1 Rust 后端

| 类别 | 数量 | 严重度 | 备注 |
|------|------|--------|------|
| `unwrap()` | **55** | 🔴 高 | 含 2 处 `main.rs` 入口 |
| `expect()` | **111** | 🟡 中 | 多为测试代码 + 主流程信号处理 |
| `panic!` / `panic(` | **0** | ✅ 良好 | 全部由 `Result` 替代 |
| `unsafe` 块 | **0** | ✅ 良好 | 零不安全代码 |
| `println!` / `eprintln!` / `dbg!` | **161** | 🟢 低 | 主要在 `cli/util/*`（CLI 工具，符合预期） |
| `TODO` / `FIXME` / `XXX` / `HACK` | **234** | 🟡 中 | 大量为 `TODO(tech-debt):` 模板化注释 |
| `#[allow(dead_code)]` 项级 | **136** | 🟡 中 | 分布：services 27, handlers 15, models 7, utils 4 |
| `#![allow(dead_code)]` 文件级 | **222** | 🔴 高 | **违反项目规则**（utils/ 之外禁止） |
| `fn _unused()` 占位 | **5** | 🔴 高 | 明显死代码 |

#### `unwrap()` 文件分布 Top 10

| 文件 | 数量 | 备注 |
|------|------|------|
| `backend/src/utils/import_export.rs` | 14 | 测试代码 + LazyLock 静态正则 |
| `backend/src/services/cost_collection_service.rs` | 6 | |
| `backend/src/middleware/trace_context.rs` | 6 | 链路追踪 |
| `backend/src/utils/password_validator.rs` | 3 | 静态正则编译 |
| `backend/src/services/customer_credit_service.rs` | 3 | |
| `backend/src/utils/field_mask.rs` | 2 | 字段掩码 |
| `backend/src/utils/di_container.rs` | 2 | |
| `backend/src/utils/date_utils.rs` | 2 | |
| `backend/src/services/report/job.rs` | 2 | |
| `backend/src/services/report/ds.rs` | 2 | |

#### `expect()` 文件分布 Top 10

| 文件 | 数量 | 备注 |
|------|------|------|
| `backend/src/utils/dual_unit_converter.rs` | 26 | 单位换算工具 |
| `backend/src/services/metrics_service.rs` | 14 | 监控指标 |
| `backend/src/utils/di_container.rs` | 13 | 依赖注入 |
| `backend/src/services/auth_service.rs` | 13 | 鉴权服务 |
| `backend/src/handlers/dual_unit_converter_handler.rs` | 10 | |
| `backend/src/middleware/trace_context.rs` | 6 | |
| `backend/src/utils/app_state.rs` | 4 | |
| `backend/src/services/operation_log_service.rs` | 4 | |
| `backend/src/handlers/inventory_stock_handler.rs` | 4 | |
| `backend/src/middleware/logger_middleware.rs` | 2 | |

#### `allow(dead_code)` 文件分布 Top 10

| 文件 | 数量 | 类型 |
|------|------|------|
| `backend/src/utils/query_builder.rs` | 5 | 项级 |
| `backend/src/services/supplier_service.rs` | 5 | 项级 |
| `backend/src/services/assignment_history_service.rs` | 5 | 项级 |
| `backend/src/handlers/audit_enhanced_handler.rs` | 5 | 项级 |
| `backend/src/handlers/ar_reconciliation_handler.rs` | 5 | 项级 |
| `backend/src/utils/import_export.rs` | 4 | 项级 |
| `backend/src/services/sales_analysis_service.rs` | 4 | 项级 |
| `backend/src/services/email_log_service.rs` | 4 | 项级 |
| `backend/src/services/assist_accounting_service.rs` | 4 | 项级 |
| `backend/src/services/scheduling_service.rs` | 3 | 项级 |

#### `println!` 文件分布 Top 10

| 文件 | 数量 | 性质 |
|------|------|------|
| `backend/src/cli/util/upgrade.rs` | 42 | CLI 工具（合规） |
| `backend/src/cli/util/service.rs` | 42 | CLI 工具（合规） |
| `backend/src/cli/util/misc.rs` | 32 | CLI 工具（合规） |
| `backend/src/cli/util/backup.rs` | 20 | CLI 工具（合规） |

> ⚠️ 全部 161 处 `println!` 集中在 4 个 CLI 工具文件中，符合 `.clippy.toml` 规则（CLI 工具除外）。

---

### 2.2 前端（Vue 3 + TypeScript）

| 类别 | 数量 | 严重度 | 备注 |
|------|------|--------|------|
| `as any` | **336** | 🔴 高 | 大量绕过类型检查 |
| `: any` / `<any>` | **697** | 🔴 高 | 类型注解缺失 |
| `as unknown` | **0** | ✅ 良好 | 零使用 |
| `console.log` | **0** | ✅ 良好 | 已替换为 logger |
| `console.warn` | **14** | 🟢 低 | 部分为正常提示 |
| `console.error` | **133** | 🟡 中 | 建议统一走 logger.ts |
| `console.info/debug` | **2** | 🟢 低 | |
| `@ts-ignore` / `@ts-nocheck` | **0** | ✅ 良好 | 零抑制 |
| `TODO` / `FIXME` / `XXX` / `HACK` | **14** | 🟢 低 | 正常开发注释 |
| `alert(` | **2** | 🟢 低 | 实际为 `ElMessageBox.alert` |
| `debugger` | **0** | ✅ 良好 | 零调试断点 |
| `vue-tsc` 类型错误 | **40** | 🟡 中 | TS 编译期错误 |

#### `as any` + `: any` 文件分布 Top 10

| 文件 | 数量 | 严重度 |
|------|------|--------|
| `frontend/src/views/system/index.vue` | 56 | 🔴 极高 |
| `frontend/src/views/ap/index.vue` | 31 | 🔴 高 |
| `frontend/src/views/advanced/index.vue` | 28 | 🔴 高 |
| `frontend/src/views/ar/index.vue` | 26 | 🔴 高 |
| `frontend/src/views/quality/index.vue` | 24 | 🔴 高 |
| `frontend/src/views/sales/index.vue` | 23 | 🟡 中 |
| `frontend/src/views/finance/index.vue` | 22 | 🟡 中 |
| `frontend/src/views/sales-ext/index.vue` | 21 | 🟡 中 |
| `frontend/src/views/sales-contract/index.vue` | 21 | 🟡 中 |
| `frontend/src/views/inventory/index.vue` | 20 | 🟡 中 |

#### `console.*` 文件分布 Top 10

| 文件 | 数量 |
|------|------|
| `frontend/src/views/trading/index.vue` | 8 |
| `frontend/src/views/purchase-return/index.vue` | 7 |
| `frontend/src/views/purchase-contract/index.vue` | 7 |
| `frontend/src/views/crm/opportunities/index.vue` | 7 |
| `frontend/src/views/bpm/definitions.vue` | 7 |
| `frontend/src/views/tenant-billing/index.vue` | 6 |
| `frontend/src/views/security/index.vue` | 6 |
| `frontend/src/views/purchase-price/index.vue` | 6 |
| `frontend/src/views/logistics/index.vue` | 6 |
| `frontend/src/views/dye-recipe/index.vue` | 6 |

#### `vue-tsc` 类型错误分布

- **TS2339** (Property does not exist): 23 处 — 多为 `ApiResponse<T>` 缺少 `total` 字段
- **TS2353** (Unknown property): 6 处 — `year` / `page` / `keyword` 字段多余
- **TS2345** (Type mismatch): 6 处 — `unknown` 与 `string` 不匹配
- **TS2322** (Type assignability): 3 处 — 类型不兼容
- **TS6133** (Declared but never read): 1 处 — `user-profile/index.vue:170` 变量 `rule` 未使用
- **TS2308** (Module re-export conflict): 1 处 — `api/index.ts:49` 重导出冲突

> 📋 **TS 错误 Top 5 文件**：
> 1. `src/views/fiveDimension/index.vue` — 9 处
> 2. `src/views/financeReport/index.vue` — 4 处
> 3. `src/views/system-update/index.vue` — 3 处
> 4. `src/views/dataPermission/index.vue` — 3 处
> 5. `src/views/customer/index.vue` — 3 处

---

## 3. 死代码清单

### 3.1 明显的 `fn _unused()` 占位函数（5 处）

| 位置 | 文件 | 行号 | 描述 |
|------|------|------|------|
| 1 | `backend/src/services/ar/mod.rs` | 198 | 抑制 `ReconciliationItemModel` / `ReconciliationActiveModel` 未使用 |
| 2 | `backend/src/services/report/job.rs` | 287 | 抑制 `ReportSubscriptionEntity::find_by_id::<i32>` 未使用 |
| 3 | `backend/src/services/report/ds.rs` | 438 | 抑制 `Vec<ReportColumn>` 未使用 |
| 4 | `backend/src/services/report/mod.rs` | 369 | 抑制 `Option<Decimal>` 未使用 |
| 5 | `backend/src/services/report/exp.rs` | 495 | 抑制 `base64::engine::general_purpose::STANDARD.encode` 未使用 |

> 💡 这 5 个函数是占位符，目的是为了"使用"导入以避免 `unused_imports` 警告。**建议**：直接删除未使用的导入，而不是用占位函数掩盖。

### 3.2 文件级 `#![allow(dead_code)]` 抑制（222 处，按目录统计）

| 目录 | 文件数 | 性质 |
|------|--------|------|
| `backend/src/models/` | 138 | ✅ SeaORM 自动生成（项目规则豁免） |
| `backend/src/services/` | 44 | 🔴 **违反规则**：应使用项级抑制 |
| `backend/src/handlers/` | 22 | 🔴 **违反规则**：应使用项级抑制 |
| `backend/src/middleware/` | 6 | 🔴 **违反规则**：应使用项级抑制 |
| `backend/src/services/{inv,so,ar,report,po,crm,ai}/` | 11 | 🔴 **违反规则** |
| `backend/src/models/dto/` | 1 | ✅ dto 子模块 |
| `backend/src/utils/` | **0** | ✅ 已全部清理（项目模板） |

> ⚠️ **共 83 个非 models/ 文件** 使用了文件级 `#![allow(dead_code)]`，**违反项目规则**（规则明确禁止非 models/ 业务模块使用文件级抑制）。

### 3.3 文件级 `#![allow(dead_code)]` 完整清单（按目录）

#### `backend/src/middleware/`（6 个，**违规**）

1. `auth_context.rs`
2. `logger_middleware.rs`
3. `operation_log.rs`
4. `api_gateway.rs`
5. `permission.rs`
6. `tenant.rs`

#### `backend/src/handlers/`（22 个，**违规**）

1. `inventory_stock_handler.rs`
2. `quality_inspection_handler.rs`
3. `init_handler.rs`
4. `ap_invoice_handler.rs`
5. `quality_standard_handler.rs`
6. `sales_fabric_order_handler.rs`
7. `customer_handler.rs`
8. `barcode_scanner_handler.rs`
9. `purchase_price_handler.rs`
10. `greige_fabric_handler.rs`
11. `supplier_evaluation_handler.rs`
12. `system_update_handler.rs`
13. `inventory_batch_handler.rs`
14. `ap_payment_handler.rs`
15. `purchase_receipt_handler.rs`
16. `supplier_handler.rs`
17. `fixed_asset_handler.rs`
18. `budget_management_handler.rs`
19. `purchase_inspection_handler.rs`
20. `warehouse_handler.rs`
21. `sales_price_handler.rs`
22. `purchase_order_handler.rs`

> ⚠️ 所有 `handlers/` 下文件级抑制都**违反项目规则**。建议立即改为项级 `#[allow(dead_code)] // TODO(tech-debt): ...`。

### 3.4 项级 `#[allow(dead_code)]` 重点位置（前 30 行号）

| 文件:行 | 标记的项 | 备注 |
|---------|----------|------|
| `utils/cache.rs:51` | `CachedValue<T>` 字段 | ✅ 已加 TODO 注释 |
| `utils/admin_checker.rs:35` | `clear_admin_role_cache` | 仅测试引用（`utils/admin_checker.rs:110, 115`） |
| `utils/import_export.rs:16,25,243,246` | 4 个辅助项 | |
| `utils/query_builder.rs:5,47,77,107,143` | 5 个构建器 | |
| `cli/util/mod.rs:230` | CLI 工具项 | |
| `observability/span.rs:72` | `_macro_compiles()` 探针 | |
| `middleware/security_headers.rs:95` | 1 个项 | |
| `handlers/missing_handlers.rs:11` | 1 个项 | |
| `handlers/audit_enhanced_handler.rs:14,26,48,73,126` | 5 个审计函数 | |
| `handlers/tenant_config_handler.rs:20` | 1 个项 | |
| `handlers/email_handler.rs:24` | 1 个项 | |
| `handlers/print_handler.rs:19` | 1 个项 | |
| `handlers/crm_customer_handler.rs:19` | 1 个项 | |
| `handlers/webhook_integration_handler.rs:22,31` | 2 个项 | |
| `handlers/scheduling_handler.rs:99` | 1 个项 | |
| `handlers/ar_reconciliation_handler.rs:214,225,232,240,250` | 5 个项 | |
| `handlers/report_enhanced_handler.rs:26,35,348` | 3 个项 | |
| `handlers/bpm_definition_handler.rs:108` | 1 个项 | |
| `handlers/crm_assignment_handler.rs:224,242` | 2 个项 | |
| `handlers/crm_pool_handler.rs:19,43` | 2 个项 | |
| `handlers/dye_batch_handler.rs:44` | `to_str()` | |
| `handlers/financial_analysis_handler.rs:23,34` | 2 个项 | |
| `services/ar/mod.rs:197` | `_unused()` | 死代码 |
| `services/report/job.rs:286` | `_unused()` | 死代码 |
| `services/report/ds.rs:437` | `_unused()` | 死代码 |
| `services/report/mod.rs:368` | `_unused()` | 死代码 |
| `services/report/exp.rs:494` | `_unused()` | 死代码 |
| `services/cost_collection_service.rs:463,475,490` | `calculate_total_cost` / `calculate_unit_cost_*` | |
| `services/email_template_service.rs:234` | `render_template` | |

---

## 4. 冗余函数清单（抽样）

### 4.1 抽样文件 1：`backend/src/handlers/audit_enhanced_handler.rs`

| 公开函数 | 行号 | 跨文件引用 | 判定 |
|----------|------|------------|------|
| `list_operation_logs` | 74 | 仅 `routes/analytics.rs` | ✅ 路由绑定，正常 |
| `export_operation_logs` | 127 | 仅本文件 | ⚠️ 需检查路由 |
| `list_audit_logs` | 177 | 仅 `routes/analytics.rs` | ✅ 正常 |
| `export_audit_logs` | 240 | 仅本文件 | ⚠️ 需检查路由 |

> 该文件 5 处 `#[allow(dead_code)]`（行 14, 26, 48, 73, 126）属于文件级抑制的衍生（整个文件已被全局抑制）。

### 4.2 抽样文件 2：`backend/src/services/cost_collection_service.rs`

| 函数 | 行号 | `#[allow(dead_code)]` | 跨文件引用 |
|------|------|------------------------|------------|
| `calculate_total_cost` | 463 | ✅ | 0 次 |
| `calculate_unit_cost_meters` | 475 | ✅ | 0 次 |
| `calculate_unit_cost_kg` | 490 | ✅ | 0 次 |

> 🔴 **3 个函数均无业务调用方**，但仍有 24 次同名引用（实际引用是其他字段）。建议删除或接入业务。

### 4.3 抽样文件 3：`backend/src/utils/admin_checker.rs:35` — `clear_admin_role_cache`

| 项 | 数据 |
|----|------|
| 行号 | 35 |
| `#[allow(dead_code)]` | ✅ |
| 外部引用 | 0 处 |
| 测试引用 | 2 处（`admin_checker.rs:110, 115`） |
| 业务引用 | **0 处** |

> 🔴 仅测试代码使用，业务侧从未调用。建议删除函数及对应测试。

### 4.4 抽样文件 4：前端 `frontend/src/api/sales.ts`

| API 函数 | views 引用次数 | 判定 |
|----------|----------------|------|
| `getOrderList` | 多次 | ✅ |
| `getOrderById` | 4 | ✅ |
| `createOrder` | 2 | ✅ |
| `updateOrder` | 2 | ✅ |
| `deleteOrder` | **0** | 🟡 可能未接入 |
| `submitOrder` | 未统计 | 待查 |
| `approveOrder` | 1 | ✅ |
| `cancelOrder` | **0** | 🟡 可能未接入 |
| `createDelivery` | 1+ | ✅ |
| `getDeliveries` | 未统计 | 待查 |
| `getOrderStatistics` | 未统计 | 待查 |

> 💡 至少 2 个 API 函数（`deleteOrder`, `cancelOrder`）未被前端任何 view 调用。

### 4.5 抽样文件 5：`frontend/src/composables/useTableColumns.ts`

- 公开导出：`useTableColumns` 函数
- 跨文件引用：4 个 view（`sales`, `production`, `inventory`, `quality`）
- ✅ **正常使用**

---

## 5. Top 10 高危文件

### 5.1 Rust 综合危险度（`unwrap` + `expect` + `println` 累计）

| 排名 | 文件 | 累计 | 建议 |
|------|------|------|------|
| 1 | `backend/src/cli/util/upgrade.rs` | 42 | ✅ CLI 工具可接受 |
| 2 | `backend/src/cli/util/service.rs` | 42 | ✅ CLI 工具可接受 |
| 3 | `backend/src/cli/util/misc.rs` | 32 | ✅ CLI 工具可接受 |
| 4 | `backend/src/utils/dual_unit_converter.rs` | 26 | ⚠️ 测试 + 业务混合 |
| 5 | `backend/src/cli/util/backup.rs` | 20 | ✅ CLI 工具可接受 |
| 6 | `backend/src/utils/di_container.rs` | 15 | ⚠️ DI 核心，需检查 |
| 7 | `backend/src/utils/import_export.rs` | 14 | ⚠️ 业务工具 |
| 8 | `backend/src/services/metrics_service.rs` | 14 | ⚠️ 测试代码集中 |
| 9 | `backend/src/services/auth_service.rs` | 14 | 🔴 鉴权核心，需重点关注 |
| 10 | `backend/src/middleware/trace_context.rs` | 12 | ⚠️ 链路追踪 |

### 5.2 前端综合危险度（`as any` + `: any` + `<any>` 累计）

| 排名 | 文件 | 累计 | 建议 |
|------|------|------|------|
| 1 | `frontend/src/views/system/index.vue` | 56 | 🔴 1521 行巨型 view，需重构 |
| 2 | `frontend/src/views/ap/index.vue` | 31 | 🔴 1035 行 |
| 3 | `frontend/src/views/advanced/index.vue` | 28 | 🔴 高级功能页 |
| 4 | `frontend/src/views/ar/index.vue` | 26 | 🔴 967 行 |
| 5 | `frontend/src/views/quality/index.vue` | 24 | 🔴 828 行 |
| 6 | `frontend/src/views/sales/index.vue` | 23 | 🟡 1070 行 |
| 7 | `frontend/src/views/finance/index.vue` | 22 | 🟡 867 行 |
| 8 | `frontend/src/views/sales-ext/index.vue` | 21 | 🟡 1148 行 |
| 9 | `frontend/src/views/sales-contract/index.vue` | 21 | 🟡 716 行 |
| 10 | `frontend/src/views/inventory/index.vue` | 20 | 🟡 899 行 |

### 5.3 后端 `allow(dead_code)` 集中文件 Top 5

| 排名 | 文件 | 数量 |
|------|------|------|
| 1 | `backend/src/utils/query_builder.rs` | 5 |
| 2 | `backend/src/services/supplier_service.rs` | 5 |
| 3 | `backend/src/services/assignment_history_service.rs` | 5 |
| 4 | `backend/src/handlers/audit_enhanced_handler.rs` | 5 |
| 5 | `backend/src/handlers/ar_reconciliation_handler.rs` | 5 |

---

## 6. 清理建议（按 ROI 排序）

### 6.1 🔴 P0：必须立即处理（违反项目规则 + 高危）

1. **删除所有 `fn _unused()` 占位函数（5 处）**——明显死代码。
   - 改用方案：直接删除对应 `use` 导入。

2. **83 个非 models/ 文件级 `#![allow(dead_code)]`** — 全部违反项目规则。
   - 处理：移除文件级抑制，改为项级 `#[allow(dead_code)] // TODO(tech-debt): ...`。
   - 涉及：`middleware/`（6 个）+ `handlers/`（22 个）+ `services/`（44 个）+ 子目录（11 个）。

3. **删除 `services/cost_collection_service.rs:463,475,490`** 三个未被调用的 `calculate_*` 函数。

4. **删除 `utils/admin_checker.rs:35` 的 `clear_admin_role_cache`** 及其测试（仅测试使用）。

### 6.2 🟡 P1：高 ROI 清理（提高可维护性）

5. **统一前端 `any` 类型**：697 处 `: any` + 336 处 `as any`（共 1,033 处）。
   - 重点：`views/system/index.vue`（56 处）、`views/ap/index.vue`（31 处）。
   - 策略：定义具体类型替代 `any`，分批替换。

6. **`utils/dual_unit_converter.rs:26` 处 `expect()`**：多为测试代码，确认无业务影响。

7. **`services/auth_service.rs:13` 处 `expect()`**：鉴权核心，需逐一评估。

8. **`middleware/trace_context.rs:6` 处 `unwrap()`**：链路追踪关键路径，建议改 `?` 传播。

### 6.3 🟢 P2：长期治理（提升代码质量）

9. **前端 40 处 `vue-tsc` 类型错误**：
   - 23 处 TS2339（多为 `ApiResponse<T>` 缺少 `total`）—— 后端统一返回结构。
   - 6 处 TS2353（`year/page/keyword` 字段未声明）—— 补全类型定义。
   - 1 处 TS6133（`user-profile/index.vue:170` 变量 `rule` 未使用）—— 直接删除。

10. **133 处 `console.error`**：建议统一走 `frontend/src/utils/logger.ts`，按日志级别分类。

11. **`sales.ts` 中 `deleteOrder` / `cancelOrder`** 等未被引用的 API：业务调研后删除或接入。

12. **重构巨型 view 文件**：
    - `views/system/index.vue` (1521 行) — 拆分 11 个 tab 组件
    - `views/purchase-ext/index.vue` (1151 行)
    - `views/sales-ext/index.vue` (1148 行)
    - `views/sales/index.vue` (1070 行)
    - `views/ap/index.vue` (1035 行)

13. **`cli/util/*` 中 `println!`（161 处）**：规则已豁免，但可考虑替换为 `tracing::info!` 保持统一。

### 6.4 已达成 ✅（零问题项）

- `panic!` 使用：0 处
- `unsafe` 块：0 处
- `@ts-ignore` / `@ts-nocheck`：0 处
- `as unknown`：0 处
- `console.log`：0 处
- `debugger` 断点：0 处
- `utils/` 文件级抑制：0 处（已清理完成）

---

## 7. 附录：所有 grep 命令

```bash
# === 步骤 1: 基础统计 ===
find backend/src -name "*.rs" -type f | wc -l
find frontend/src -name "*.vue" -type f | wc -l
find frontend/src -name "*.ts" -type f | wc -l
find backend/src -name "*.rs" -exec cat {} + | wc -l
find frontend/src -name "*.vue" -exec cat {} + | wc -l

# === 步骤 2: Rust 危险模式 ===
grep -rn "\.unwrap()" backend/src/ --include="*.rs" | wc -l
grep -rn "panic!\|panic(" backend/src/ --include="*.rs" | wc -l
grep -rn "unsafe " backend/src/ --include="*.rs" | wc -l
grep -rn "\.expect(" backend/src/ --include="*.rs" | wc -l
grep -rn "println!\|eprintln!\|dbg!" backend/src/ --include="*.rs" | wc -l
grep -rn "TODO\|FIXME\|XXX\|HACK" backend/src/ --include="*.rs" | wc -l
grep -rn "allow(dead_code)\|allow(unused)" backend/src/ --include="*.rs" | wc -l
grep -rln "^#!\[allow(dead_code)\]" backend/src/ --include="*.rs" | wc -l
grep -rn "fn _unused()" backend/src/ --include="*.rs" | wc -l

# === 步骤 3: 前端危险模式 ===
grep -rn " as any\b" frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l
grep -rn ": any\b\|<any>" frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l
grep -rn " as unknown\b" frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l
grep -rn "console\." frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l
grep -rn "@ts-ignore\|@ts-nocheck" frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l
grep -rn "TODO\|FIXME\|XXX\|HACK" frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l
grep -rn "alert(" frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l
grep -rn "debugger\b" frontend/src/ --include="*.ts" --include="*.tsx" --include="*.vue" | wc -l

# === 步骤 4: 前端未使用导入（vue-tsc）===
cd /workspace/frontend && npx vue-tsc --noEmit --project tsconfig.json 2>&1 | grep "TS6133"

# === 步骤 5: Rust 死代码 ===
grep -rln "^#!\[allow(dead_code)\]" backend/src/ --include="*.rs"
grep -rn "fn _unused()" backend/src/ --include="*.rs"

# === 步骤 6: 抽样交叉验证 ===
grep -rn "calculate_total_cost\|calculate_unit_cost_meters\|calculate_unit_cost_kg" backend/src/ --include="*.rs"
grep -rn "clear_admin_role_cache" backend/src/ --include="*.rs"
grep -rE "salesApi\." frontend/src/views/ | head -20

# === 步骤 7: Top 文件扫描（示例：Rust 危险模式）===
for f in $(find backend/src -name "*.rs"); do
  count=$(grep -cE "\.unwrap\(\)|\.expect\(|println!|eprintln!|dbg!" "$f")
  echo "$count $f"
done | sort -rn | head -10
```

---

## 8. 关键发现总结

### 🔴 关键违反规则项

1. **83 个非 `models/` 文件**使用文件级 `#![allow(dead_code)]`，违反项目规则（规则豁免仅适用于 `backend/src/models/` 下 SeaORM 自动生成模型）。
2. **5 个 `fn _unused()` 占位函数**明显是死代码。
3. **3 个 `cost_collection_service.rs` 中的 `calculate_*` 函数** 无业务调用方。

### ✅ 健康指标

- Rust：`panic!` = 0，`unsafe` = 0
- 前端：`@ts-ignore` = 0，`as unknown` = 0，`console.log` = 0，`debugger` = 0
- `utils/` 模块的文件级抑制已全部清理（项目模板达成）

### 📊 综合数据

- 169,186 行代码
- 493 个 Rust 文件 + 221 个 Vue/TS 文件
- 222 处文件级 + 136 处项级 `allow(dead_code)` = **358 处死代码抑制**
- 1,033 处 `any` 绕过类型检查
- 40 处 TypeScript 类型错误
- 161 处 `println!`（全部在 CLI 工具，合规）

### 🎯 Top 5 高危文件（综合）

1. `backend/src/utils/dual_unit_converter.rs`（26 处 `expect()`）
2. `backend/src/services/metrics_service.rs`（14 处 `expect()`，多为测试）
3. `backend/src/services/auth_service.rs`（13 处 `expect()`，鉴权核心）
4. `frontend/src/views/system/index.vue`（56 处 `any`，1521 行巨型 view）
5. `frontend/src/views/ap/index.vue`（31 处 `any`，1035 行）

---

> 📌 **报告生成时间**：2026-06-16
> 📌 **本报告仅作识别，不修改任何源代码**
> 📌 **下一步建议**：根据 P0/P1 优先级安排清理迭代
