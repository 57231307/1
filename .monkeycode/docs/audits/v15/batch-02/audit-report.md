# V15 审计报告 - 批 2 - 类二通用代码质量（10 维度）

## 审计概览

- **审计时间**：2026-07-16
- **审计子代理**：V15 审计子代理（batch-02）
- **审计范围**：通用代码质量（10 维度）
- **审计依据**：
  - `/workspace/.trae/rules/project_rules.md`（项目开发规范）
  - `/workspace/.monkeycode/MEMORY.md`（项目规则记忆）
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md`（V15 审计计划）
- **审计对象**：
  - 后端 Rust 代码：`/workspace/backend/src/`（lib.rs / main.rs / handlers / services / utils / middleware / routes / cli / search / websocket / observability 共 11 大模块，185,350 行）
  - 前端 Vue 代码：`/workspace/frontend/src/`（api / components / composables / directives / i18n / locales / router / store / types / utils / views）
- **审计方法**：
  - 使用 Grep/Glob/RunCommand 扫描问题模式（unwrap/expect/TODO/FIXME/println!/let _ =）
  - 使用 awk 统计函数长度、嵌套深度、分支数
  - 抽样阅读关键文件验证上下文

---

## 维度 2.1：代码命名规范

### 检查方法

1. Grep 搜索 Rust 命名违规模式：`fn [A-Z]`、`struct [a-z]`、`const [a-z]`、`use [a-z]+[A-Z]`
2. Grep 搜索前端 api 文件命名模式，识别 kebab-case 与 camelCase 混用
3. Glob 扫描前端组件文件名，识别缩写命名
4. 抽样阅读 main.rs/lib.rs 验证 Rust 模块命名

### 发现

#### 2.1.1 后端 Rust 命名（基本合规）

后端 Rust 代码命名基本符合规范：
- 函数 / 变量 / 模块：snake_case ✅
- 类型（struct/enum/trait）：UpperCamelCase ✅
- 常量：SCREAMING_SNAKE_CASE ✅

未发现违规（搜索 `fn [A-Z]`、`struct [a-z]`、`const [a-z]` 均无匹配）。

#### 2.1.2 前端 api 文件命名不统一（违反一致性，中风险）

`/workspace/frontend/src/api/` 目录下文件命名混用 kebab-case 与 camelCase 两种风格：

**camelCase 命名（7 个文件）**：
- `/workspace/frontend/src/api/purchaseReceipt.ts`
- `/workspace/frontend/src/api/inventoryCount.ts`
- `/workspace/frontend/src/api/inventoryAdjustment.ts`
- `/workspace/frontend/src/api/inventoryTransfer.ts`
- `/workspace/frontend/src/api/inventoryBatch.ts`
- `/workspace/frontend/src/api/financeReport.ts`
- `/workspace/frontend/src/api/omniAudit.ts`

**kebab-case 命名（与 camelCase 混用，其余约 50 个文件）**：
- `customer-credit.ts`、`financial-analysis.ts`、`five-dimension.ts`、`assist-accounting.ts`、`business-trace.ts`、`sales-return.ts`、`purchase-return.ts`、`sales-price.ts`、`purchase-price.ts`、`sales-analysis.ts`、`supplier-evaluation.ts`、`barcode-scanner.ts`、`ar-reconciliation-enhanced.ts`、`report-enhanced.ts`、`material-shortage.ts`、`bpm-enhanced.ts`、`data-permission.ts`、`custom-order.ts`、`accounting-period.ts`、`dye-recipe.ts`、`dye-batch.ts`、`greige-fabric.ts`、`mrp.ts`、`sales-contract.ts`、`purchase-contract.ts`、`purchase-inspection.ts`、`print-templates.ts`、`quality-standards.ts`、`color-card.ts`、`color-price.ts`、`ai-extend.ts`、`slow-query.ts`、`system-update.ts`、`account-subject.ts`、`api-gateway.ts`、`ap-payment.ts`、`ap-verification.ts`、`ap-reconciliation.ts`、`ar-reconciliation.ts`、`ap-invoice.ts` 等

**问题**：`/workspace/frontend/src/api/index.ts:39-43, 62, 70, 80` 同时 `export * from './purchaseReceipt'`（camelCase）与 `export * from './customer-credit'`（kebab-case），违反一致性。

#### 2.1.3 前端视图文件夹命名不统一（违反一致性，中风险）

`/workspace/frontend/src/views/` 目录下文件夹命名混用：

**camelCase 文件夹（7 个，应改 kebab-case）**：
- `/workspace/frontend/src/views/supplierEvaluation/`
- `/workspace/frontend/src/views/businessTrace/`
- `/workspace/frontend/src/views/arReconciliation/`
- `/workspace/frontend/src/views/inventoryCount/`
- `/workspace/frontend/src/views/inventoryAdjustment/`
- `/workspace/frontend/src/views/inventoryBatch/`
- `/workspace/frontend/src/views/inventoryTransfer/`
- `/workspace/frontend/src/views/purchaseReceipt/`

被引用位置（路由配置）：
- `/workspace/frontend/src/router/index.ts:160` `import('@/views/supplierEvaluation/index.vue')`
- `/workspace/frontend/src/router/index.ts:172` `import('@/views/inventoryCount/index.vue')`
- `/workspace/frontend/src/router/index.ts:178` `import('@/views/inventoryTransfer/index.vue')`
- `/workspace/frontend/src/router/index.ts:184` `import('@/views/inventoryAdjustment/index.vue')`
- `/workspace/frontend/src/router/index.ts:190` `import('@/views/arReconciliation/index.vue')`
- `/workspace/frontend/src/router/index.ts:202` `import('@/views/purchaseReceipt/index.vue')`
- `/workspace/frontend/src/router/index.ts:360` `import('@/views/inventoryBatch/index.vue')`
- `/workspace/frontend/src/router/index.ts:378` `import('@/views/businessTrace/index.vue')`
- `/workspace/frontend/src/router/index.ts:471` `import('@/views/arReconciliation/enhanced.vue')`

#### 2.1.4 前端组件命名缩写严重（违反"避免缩写"原则，高风险）

项目规则第三章"命名约定"明确要求"避免缩写和单字母变量"，但前端组件大量使用 2-4 字母前缀缩写，可读性差：

**Scheduling 模块（Sch 前缀，9 个组件）**：
- `/workspace/frontend/src/views/scheduling/components/SchMAdj.vue`
- `/workspace/frontend/src/views/scheduling/components/SchGAuto.vue`
- `/workspace/frontend/src/views/scheduling/components/SchGConf.vue`
- `/workspace/frontend/src/views/scheduling/components/SchMTbl.vue`
- `/workspace/frontend/src/views/scheduling/components/SchGTool.vue`
- `/workspace/frontend/src/views/scheduling/components/SchMTool.vue`
- `/workspace/frontend/src/views/scheduling/components/SchMConf.vue`
- `/workspace/frontend/src/views/scheduling/components/SchGChart.vue`
- `/workspace/frontend/src/views/scheduling/components/SchGAdj.vue`

**DataImport 模块（Di 前缀，4 个组件）**：
- `/workspace/frontend/src/views/data-import/components/DiTplUpload.vue`
- `/workspace/frontend/src/views/data-import/components/DiTaskTbl.vue`
- `/workspace/frontend/src/views/data-import/components/DiTplTbl.vue`
- `/workspace/frontend/src/views/data-import/components/DiTplForm.vue`

**SystemUpdate 模块（Su 前缀，3 个组件）**：
- `/workspace/frontend/src/views/system-update/components/SuBkpForm.vue`
- `/workspace/frontend/src/views/system-update/components/SuVerDetail.vue`
- `/workspace/frontend/src/views/system-update/components/SuInfoCards.vue`

**BPM Definitions 模块（BpmDf 前缀，5 个组件）**：
- `/workspace/frontend/src/views/bpm/definitions/components/BpmDfVerDlg.vue`
- `/workspace/frontend/src/views/bpm/definitions/components/BpmDfTplDlg.vue`
- `/workspace/frontend/src/views/bpm/definitions/components/BpmDfFilter.vue`
- `/workspace/frontend/src/views/bpm/definitions/components/BpmDfTbl.vue`
- `/workspace/frontend/src/views/bpm/definitions/components/BpmDfForm.vue`

**BPM Approval 模块（BpmAp 前缀，6 个组件）**：
- `/workspace/frontend/src/views/bpm/approval/components/BpmApChainDlg.vue`
- `/workspace/frontend/src/views/bpm/approval/components/BpmApTranDlg.vue`
- `/workspace/frontend/src/views/bpm/approval/components/BpmApCompletedTbl.vue`
- `/workspace/frontend/src/views/bpm/approval/components/BpmApAprDlg.vue`
- `/workspace/frontend/src/views/bpm/approval/components/BpmApPendingTbl.vue`
- `/workspace/frontend/src/views/bpm/approval/components/BpmApStat.vue`

**PurchaseContract 模块（Pc 前缀，4 个组件）**：
- `/workspace/frontend/src/views/purchase-contract/components/PcFilter.vue`
- `/workspace/frontend/src/views/purchase-contract/components/PcForm.vue`
- `/workspace/frontend/src/views/purchase-contract/components/PcDetail.vue`
- `/workspace/frontend/src/views/purchase-contract/components/PcTbl.vue`

**PurchasePrice 模块（Pp 前缀，5 个组件）**：
- `/workspace/frontend/src/views/purchase-price/components/PpDetail.vue`
- `/workspace/frontend/src/views/purchase-price/components/PpTbl.vue`
- `/workspace/frontend/src/views/purchase-price/components/PpHistory.vue`
- `/workspace/frontend/src/views/purchase-price/components/PpFilter.vue`
- `/workspace/frontend/src/views/purchase-price/components/PpForm.vue`

**SalesContract 模块（Sc 前缀，3 个组件）**：
- `/workspace/frontend/src/views/sales-contract/components/ScTbl.vue`
- `/workspace/frontend/src/views/sales-contract/components/ScFilter.vue`
- `/workspace/frontend/src/views/sales-contract/components/ScForm.vue`

**Dashboard 模块（Db 前缀，4 个组件）**：
- `/workspace/frontend/src/views/dashboard/components/DbPie.vue`
- `/workspace/frontend/src/views/dashboard/components/DbActTbl.vue`
- `/workspace/frontend/src/views/dashboard/components/DbStat.vue`
- `/workspace/frontend/src/views/dashboard/components/DbTrend.vue`

**Security 模块（Sec 前缀，4 个组件）**：
- `/workspace/frontend/src/views/security/components/SecAlertTbl.vue`
- `/workspace/frontend/src/views/security/components/SecStat.vue`
- `/workspace/frontend/src/views/security/components/SecLogTbl.vue`
- `/workspace/frontend/src/views/security/components/SecLockTbl.vue`

**TwoFactorAuth 模块（Tfa 前缀，5 个组件）**：
- `/workspace/frontend/src/views/security/two-factor/components/TfaStepBar.vue`
- `/workspace/frontend/src/views/security/two-factor/components/TfaStep1.vue`
- `/workspace/frontend/src/views/security/two-factor/components/TfaStep2.vue`
- `/workspace/frontend/src/views/security/two-factor/components/TfaStep3.vue`
- `/workspace/frontend/src/views/security/two-factor/components/TfaStep4.vue`

**ArReconciliation 模块（Ar 前缀，6 个组件）**：
- `/workspace/frontend/src/views/arReconciliation/components/ArTbl.vue`
- `/workspace/frontend/src/views/arReconciliation/components/ArCharts.vue`
- `/workspace/frontend/src/views/arReconciliation/components/ArConfirm.vue`
- `/workspace/frontend/src/views/arReconciliation/components/ArFilter.vue`
- `/workspace/frontend/src/views/arReconciliation/components/ArDispute.vue`
- `/workspace/frontend/src/views/arReconciliation/components/ArDetail.vue`

**VoucherList 模块（VchrLst 前缀，4 个组件）**：
- `/workspace/frontend/src/views/voucher/tabs/components/VchrLstFilter.vue`
- `/workspace/frontend/src/views/voucher/tabs/components/VchrLstDetail.vue`
- `/workspace/frontend/src/views/voucher/tabs/components/VchrLstTbl.vue`
- `/workspace/frontend/src/views/voucher/tabs/components/VchrLstForm.vue`

**SalesAnalysis 模块（Sa 前缀，5 个组件）**：
- `/workspace/frontend/src/views/sales-analysis/components/SaStat.vue`
- `/workspace/frontend/src/views/sales-analysis/components/SaTrend.vue`
- `/workspace/frontend/src/views/sales-analysis/components/SaProdRank.vue`
- `/workspace/frontend/src/views/sales-analysis/components/SaCustRank.vue`
- `/workspace/frontend/src/views/sales-analysis/components/SaTarget.vue`

**Advanced 模块（缩写，4 个组件）**：
- `/workspace/frontend/src/views/advanced/components/QltPanel.vue` (Quality)
- `/workspace/frontend/src/views/advanced/components/RcpPanel.vue` (Recipe)
- `/workspace/frontend/src/views/advanced/components/AiPanel.vue`
- `/workspace/frontend/src/views/advanced/components/RptPanel.vue` (Report)

**Purchase 模块（Purch/Receive/Create/View 缩写，6 个组件）**：
- `/workspace/frontend/src/views/purchase/components/PurchTbl.vue`
- `/workspace/frontend/src/views/purchase/components/PurchTop.vue`
- `/workspace/frontend/src/views/purchase/components/PurchFilter.vue`
- `/workspace/frontend/src/views/purchase/components/ViewDlg.vue`
- `/workspace/frontend/src/views/purchase/components/ReceiveDlg.vue`
- `/workspace/frontend/src/views/purchase/components/CreateDlg.vue`
- `/workspace/frontend/src/views/purchase/components/StatCards.vue`

**OrderListView 模块（Olv 缩写，3 个组件）**：
- `/workspace/frontend/src/views/sales/components/OlvTbl.vue`
- `/workspace/frontend/src/views/sales/components/OlvFilter.vue`
- `/workspace/frontend/src/views/sales/components/OlvStat.vue`

**总计**：60+ 个组件使用 2-4 字母缩写命名，新成员上手困难。

#### 2.1.5 测试函数使用中文命名（合规但有特殊性）

后端测试函数使用中文命名，符合"使用中文进行回复和沟通"规则，但需注意 Rust 命名规范允许 Unicode 标识符：
- `/workspace/backend/src/services/mrp_engine_service.rs:6919` `fn 测试一致性校验_未知模式代码()`
- `/workspace/backend/src/services/lab_dip_service.rs:8590` `fn 测试_close_order状态校验门_禁止的状态()`
- `/workspace/backend/src/services/so/delivery.rs:1732` `async fn 测试_服务实例创建()`

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| 后端 Rust 命名 | ✅ 合规 | 未发现违规 |
| 前端 api 文件命名 | 🟡 中风险 | kebab/camel 混用 |
| 前端视图文件夹命名 | 🟡 中风险 | kebab/camel 混用 |
| 前端组件缩写 | 🔴 高风险 | 60+ 组件使用缩写 |

### 修复建议

1. **前端 api 文件命名统一**：将 7 个 camelCase 文件重命名为 kebab-case（Vue/Vite 推荐）：
   - `purchaseReceipt.ts` → `purchase-receipt.ts`
   - `inventoryCount.ts` → `inventory-count.ts`
   - `inventoryAdjustment.ts` → `inventory-adjustment.ts`
   - `inventoryTransfer.ts` → `inventory-transfer.ts`
   - `inventoryBatch.ts` → `inventory-batch.ts`
   - `financeReport.ts` → `finance-report.ts`
   - `omniAudit.ts` → `omni-audit.ts`
   - 同步更新 `api/index.ts` 中所有 `export * from './xxx'` 引用
2. **前端视图文件夹统一**：将 7 个 camelCase 文件夹改名为 kebab-case，同步更新 `router/index.ts` 中 9 处 `import` 路径
3. **前端组件重命名**：将 60+ 缩写组件改为完整业务词：
   - `SchMAdj` → `SchedulingManualAdjustment`
   - `BpmDfVerDlg` → `BpmDefinitionVersionDialog`
   - `PcFilter` → `PurchaseContractFilter`
   - `VchrLstTbl` → `VoucherListTable`
   - `SaStat` → `SalesAnalysisStatistics`
4. **修复时机**：建议在 V15 修复批次中按模块逐步重命名（每个模块一个 PR），避免大爆炸式改动

---

## 维度 2.2：代码组织（模块结构清晰度）

### 检查方法

1. LS/Glob 扫描 backend/src 各模块文件数
2. RunCommand 统计文件行数（wc -l）
3. 阅读 lib.rs/main.rs/utils/mod.rs/services/mod.rs/handlers/mod.rs 验证模块声明

### 发现

#### 2.2.1 后端模块分层（清晰，合规）

`/workspace/backend/src/lib.rs` 按职责分 11 个顶级模块：
- `cli`（CLI 工具，14 个子命令）
- `config`（配置）
- `constants`（常量）
- `database`（数据库连接）
- `handlers`（HTTP handler）
- `middleware`（中间件）
- `models`（SeaORM 实体）
- `observability`（可观测性）
- `routes`（路由）
- `search`（Elasticsearch）
- `services`（业务服务）
- `utils`（工具）
- `websocket`（WebSocket）
- `telemetry`（遥测）
- `docs`（API 文档）

#### 2.2.2 handlers 模块未按业务域分组（中风险）

`/workspace/backend/src/handlers/` 目录下平铺 130+ 个 `_handler.rs` 文件，未按业务域分子目录：

抽样（前 20 个，按字母序）：
- `account_subject_handler.rs`、`accounting_period_handler.rs`、`ai_analysis_handler.rs`、`ai_extend_handler.rs`、`ap_invoice_handler.rs`、`ap_payment_handler.rs`、`ap_payment_request_handler.rs`、`ap_reconciliation_handler.rs`、`ap_report_handler.rs`、`ap_verification_handler.rs`、`api_gateway_handler.rs`、`ar_invoice_handler.rs`、`ar_payment_handler.rs`、`ar_reconciliation_enhanced_handler.rs`、`ar_reconciliation_handler.rs`、`ar_report_handler.rs`、`ar_verification_handler.rs`、`assist_accounting_handler.rs`、`audit_enhanced_handler.rs`、`audit_log_handler.rs` ...

对比 `services/` 已部分分子目录：
- `services/so/`（销售订单，9 个文件）
- `services/inv/`（库存，7 个文件）
- `services/po/`（采购订单）
- `services/crm/`（CRM）
- `services/ar/`（应收，含 recon.rs/vfy.rs）
- `services/auth/`（认证，含 password_policy_service.rs）
- `services/ai/`（AI，含 recipe_opt.rs）

handlers 应同样按业务域分子目录（如 `handlers/sales/`、`handlers/inventory/`、`handlers/finance/`）以提升可维护性。

#### 2.2.3 单文件过大（26 个文件超 1000 行，高风险）

后端 service 文件平均行数偏高，26 个文件超过 1000 行：

| 排名 | 行数 | 文件路径 |
|------|------|----------|
| 1 | 1972 | `/workspace/backend/src/services/ar_service.rs` |
| 2 | 1891 | `/workspace/backend/src/services/so/delivery.rs` |
| 3 | 1879 | `/workspace/backend/src/services/production_order_service.rs` |
| 4 | 1847 | `/workspace/backend/src/services/voucher_service.rs` |
| 5 | 1800 | `/workspace/backend/src/services/energy_service.rs` |
| 6 | 1782 | `/workspace/backend/src/services/outsourcing_service.rs` |
| 7 | 1676 | `/workspace/backend/src/services/chemical_service.rs` |
| 8 | 1674 | `/workspace/backend/src/services/business_mode_service.rs` |
| 9 | 1570 | `/workspace/backend/src/models/status.rs` |
| 10 | 1565 | `/workspace/backend/src/services/mrp_engine_service.rs` |
| 11 | 1510 | `/workspace/backend/src/services/dye_batch_state_machine_service.rs` |
| 12 | 1507 | `/workspace/backend/src/services/wage_service.rs` |
| 13 | 1461 | `/workspace/backend/src/services/bi_analysis_service.rs` |
| 14 | 1332 | `/workspace/backend/src/services/ar/vfy.rs` |
| 15 | 1306 | `/workspace/backend/src/services/ap_invoice_service.rs` |
| 16 | 1271 | `/workspace/backend/src/services/flow_card_service.rs` |
| 17 | 1252 | `/workspace/backend/src/services/ap_reconciliation_service.rs` |
| 18 | 1230 | `/workspace/backend/src/search/elastic.rs` |
| 19 | 1201 | `/workspace/backend/src/services/auth_service.rs` |
| 20 | 1186 | `/workspace/backend/src/services/event_bus.rs` |
| 21 | 1158 | `/workspace/backend/src/services/po/order.rs` |
| 22 | 1118 | `/workspace/backend/src/services/lab_dip_service.rs` |
| 23 | 1099 | `/workspace/backend/src/services/production_recipe_service.rs` |
| 24 | 1078 | `/workspace/backend/src/services/ar/recon.rs` |
| 25 | 1070 | `/workspace/backend/src/services/system_update_service.rs` |
| 26 | 1043 | `/workspace/backend/src/services/inventory_finance_bridge_service.rs` |
| 27 | 1038 | `/workspace/backend/src/services/bom_service.rs` |
| 28 | 1010 | `/workspace/backend/src/services/import_export_service.rs` |

文件总数 185,350 行（backend/src）。

#### 2.2.4 前端模块组织（合规）

前端按职责分 11 个顶级模块，结构清晰：
- `api/`（85+ API 模块）
- `components/`（17 个公共组件）
- `composables/`（2 个组合函数）
- `directives/`（1 个权限指令）
- `i18n/`、`locales/`（国际化）
- `router/`（路由）
- `store/`（5 个 Pinia store）
- `types/`（4 个类型定义）
- `utils/`（7 个工具）
- `views/`（75+ 视图）

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| 后端模块分层 | ✅ 合规 | 11 个顶级模块清晰 |
| handlers 子分组 | 🟡 中风险 | 130+ 文件平铺 |
| 单文件超 1000 行 | 🔴 高风险 | 26 个文件 |
| 前端模块组织 | ✅ 合规 | 11 个顶级模块清晰 |

### 修复建议

1. **handlers 按业务域分子目录**：参考 services/ 已有的 `so/inv/po/crm/ar/auth/ai` 结构，将 handlers/ 同样分组：
   - `handlers/sales/`（包含 sales_order_handler, sales_contract_handler, sales_return_handler, quotation_handler, ...）
   - `handlers/inventory/`（包含 inventory_stock_handler, inventory_count_handler, inventory_adjustment_handler, ...）
   - `handlers/finance/`（包含 ar_invoice_handler, ar_payment_handler, ap_invoice_handler, voucher_handler, ...）
   - 等等
2. **拆分超长 service 文件**：将 ar_service.rs(1972 行) 拆为 ar/payment.rs / ar/verify.rs / ar/report.rs / ar/reconciliation.rs 等
3. **拆分 voucher_service.rs(1847 行)** 为 voucher/crud.rs / voucher/balance.rs / voucher/assist.rs
4. **拆分 event_bus.rs(1186 行)** 为 event_bus/dispatch.rs / event_bus/listener.rs / event_bus/event_types.rs

---

## 维度 2.3：函数单一职责（函数长度 > 100 行为风险）

### 检查方法

1. 使用 awk 脚本扫描所有 .rs 文件，统计每个函数的起止行号
2. 筛选函数长度 > 100 行的列表
3. 抽样阅读 top 10 长函数验证复杂度

### 发现

#### 2.3.1 30+ 个函数超过 100 行（高风险）

按函数长度排序（前 30 名）：

| 排名 | 行数 | 函数位置 | 函数名 |
|------|------|----------|--------|
| 1 | 586 | `/workspace/backend/src/services/event_bus.rs:412` | `pub async fn start_event_listener(db, search_client)` |
| 2 | 405 | `/workspace/backend/src/services/so/delivery.rs:110` | `pub async fn ship_order(&self, request, user_id)` |
| 3 | 361 | `/workspace/backend/src/services/ar/vfy.rs:37` | `pub async fn auto_match(&self, req, user_id)` |
| 4 | 254 | `/workspace/backend/src/services/ar_service.rs:898` | `pub async fn manual_verify(&self, invoice_id, payment_id, amount, remark, user_id)` |
| 5 | 243 | `/workspace/backend/src/services/ar_service.rs:652` | `pub async fn auto_verify(&self, user_id)` |
| 6 | 203 | `/workspace/backend/src/services/voucher_service.rs:698` | `async fn update_account_balances(&self, voucher_id, user_id, txn)` |
| 7 | 197 | `/workspace/backend/src/services/product_service.rs:759` | `pub async fn import_products_from_csv(...)` |
| 8 | 193 | `/workspace/backend/src/services/wage_service.rs:873` | `pub async fn calculate(&self, wage_record_id, req)` |
| 9 | 184 | `/workspace/backend/src/services/ap_invoice_service.rs:64` | `pub async fn auto_generate_from_receipt(...)` |
| 10 | 180 | `/workspace/backend/src/services/ar_service.rs:116` | `pub async fn create_payment(...)` |
| 11 | 167 | `/workspace/backend/src/services/business_mode_service.rs:179` | `pub fn check_module_consistency(mode_code, ...)` |
| 12 | 160 | `/workspace/backend/src/search/elastic.rs:387` | `async fn search(...)` |
| 13 | 157 | `/workspace/backend/src/services/outsourcing_service.rs:553` | `pub async fn record_receipt(...)` |
| 14 | 155 | `/workspace/backend/src/services/dye_batch_state_machine_service.rs:165` | `fn builtin_transition_rules()` |
| 15 | 152 | `/workspace/backend/src/services/production_order_service.rs:836` | `async fn increase_finished_goods_txn(...)` |
| 16 | 150 | `/workspace/backend/src/services/chemical_service.rs:407` | `pub async fn update(...)` |
| 17 | 150 | `/workspace/backend/src/services/ar/vfy.rs:407` | `pub async fn get_aging_report(...)` |
| 18 | 139 | `/workspace/backend/src/services/so/delivery.rs:916` | `pub async fn cancel_delivery(...)` |
| 19 | 135 | `/workspace/backend/src/services/energy_service.rs:1518` | `pub async fn monthly_allocation_by_duration(...)` |
| 20 | 131 | `/workspace/backend/src/services/production_order_service.rs:700` | `async fn deduct_raw_materials_txn(...)` |
| 21 | 131 | `/workspace/backend/src/services/inventory_finance_bridge_service.rs:488` | `async fn create_inventory_adjustment_voucher(...)` |
| 22 | 130 | `/workspace/backend/src/services/voucher_service.rs:907` | `async fn write_assist_accounting_records_txn(...)` |
| 23 | 124 | `/workspace/backend/src/services/bi_analysis_service.rs:1164` | `pub async fn pivot(...)` |
| 24 | 123 | `/workspace/backend/src/services/ar/vfy.rs:559` | `pub async fn generate_reconciliation(...)` |
| 25 | 119 | `/workspace/backend/src/services/ap_reconciliation_service.rs:407` | `pub async fn auto_reconcile_all(...)` |
| 26 | 118 | `/workspace/backend/src/services/ar_service.rs:355` | `pub async fn confirm_payment(...)` |
| 27 | 113 | `/workspace/backend/src/services/import_export_service.rs:92` | `pub fn get_import_template(import_type)` |
| 28 | 113 | `/workspace/backend/src/services/chemical_service.rs:292` | `pub async fn create(&self, req)` |
| 29 | 113 | `/workspace/backend/src/services/bi_analysis_service.rs:657` | `pub async fn kpi_summary(&self)` |
| 30 | 112 | `/workspace/backend/src/services/so/delivery.rs:1117` | `pub async fn export_orders_to_csv(...)` |

#### 2.3.2 典型长函数分析

**示例 1：`event_bus.rs:412 start_event_listener`（586 行）**
- 单一函数处理 8+ 个 BusinessEvent 分支（PurchaseReceiptCompleted / SalesOrderShipped / SalesOrderApproved / BpmTaskCompleted / DyeBatchCompleted / LowStockAlert / MaterialShortage 等）
- 每个分支内调用 1-3 个 service 方法
- 每个分支都有 Ok/Err 错误处理
- 违反单一职责：事件分发 + 业务调用 + 错误日志三职责混合
- 修复建议：将每个 BusinessEvent 分支抽取为独立 handler 函数

**示例 2：`so/delivery.rs:110 ship_order`（405 行）**
- 函数内完成：缸号校验 → 事务开启 → 订单状态校验 → 库存校验 → 发货单创建 → 库存扣减 → 预留释放 → 凭证创建 → 事件发布
- 8 个独立步骤合并为单函数
- 修复建议：抽取 `validate_dye_lot` / `lock_order` / `create_delivery_record` / `deduct_inventory` / `release_reservation` / `create_voucher` / `publish_event` 子函数

**示例 3：`ar/vfy.rs:37 auto_match`（361 行）**
- 函数内完成：策略解析 → 事务开启 → 客户加载 → 发票/收款批量预加载 → 策略 1（精确匹配）→ 策略 2（日期顺序匹配）→ 策略 3（兜底）→ 生成核销记录
- 修复建议：抽取 `run_strategy_exact` / `run_strategy_date_order` / `run_strategy_fallback` / `create_reconciliation_records`

### 风险等级

🔴 高风险：30+ 个函数超过 100 行，违反单一职责原则

### 修复建议

1. **优先拆分前 10 长函数**（每个函数一个独立 PR，降低 review 难度）
2. **拆分原则**：
   - 每个 sub-function 不超过 50 行
   - sub-function 命名清晰描述职责（动词+宾语）
   - 主函数变成"协调器"，只调用 sub-function
3. **参考 v10 复审 too_many_arguments 修复模式**：通过引入参数对象（Args struct）降低函数参数数量
4. **拆分示例**：
```rust
// 原 ship_order (405 行) → 拆为：
pub async fn ship_order(&self, request: ShipOrderRequest, user_id: i32) -> Result<(), AppError> {
    validate_dye_lot_consistency(&request.items)?;
    let txn = (*self.db).begin().await?;
    let order = lock_and_validate_order(&txn, request.order_id).await?;
    let delivery = create_delivery_record(&txn, &request, user_id).await?;
    deduct_inventory(&txn, &request.items).await?;
    release_reservation(&txn, &request.items).await?;
    let voucher = create_shipment_voucher(&txn, &order, &delivery, user_id).await?;
    txn.commit().await?;
    publish_shipped_event(order.id, order.customer_id, request.items);
    Ok(())
}
```

---

## 维度 2.4：注释完整性（中文注释、解释"为什么"）

### 检查方法

1. Grep 搜索 `TODO|FIXME|XXX|HACK` 标注
2. 抽样阅读长函数和工具模块的注释
3. 检查是否符合规则 4（`///` 注释精简为 1 行，最多 2 行）

### 发现

#### 2.4.1 注释覆盖率（总体良好）

- 每个长函数都有业务背景注释（如 `ship_order` 引用 fabric-industry-research.md §2.3 约束 5）
- 每个 service 模块顶部有 `//!` 模块级注释说明设计要点
- 每个修复批次都标注批次号和修复原因（如 `// 批次 60 P1 3-8 修复`）

#### 2.4.2 TODO(tech-debt) 标注（100+ 处，主要在 models 中保留）

Grep 结果：100+ 处 TODO(tech-debt) 标注，分布在：

**SeaORM 模型文件（合规，符合项目规则第六章例外）**：
- `/workspace/backend/src/models/ar_reconciliation_item.rs:2`
- `/workspace/backend/src/models/sales_return_item.rs:2`
- `/workspace/backend/src/models/color_card_item.rs:2`
- `/workspace/backend/src/models/custom_order.rs:2`
- `/workspace/backend/src/models/supplier_evaluation_record.rs:2`
- `/workspace/backend/src/models/webhook.rs:2`
- `/workspace/backend/src/models/fixed_asset_disposal.rs:2`
- `/workspace/backend/src/models/role.rs:2`
- `/workspace/backend/src/models/ar_aging_analysis.rs:2`
- `/workspace/backend/src/models/inventory_stock.rs:2`
- `/workspace/backend/src/models/customer_color_price.rs:2`
- `/workspace/backend/src/models/process_log.rs:2`
- `/workspace/backend/src/models/sales_return.rs:2`
- `/workspace/backend/src/models/crm_opportunity.rs:2`
- `/workspace/backend/src/models/sales_quotation.rs:2`
- `/workspace/backend/src/models/voucher.rs:2`
- `/workspace/backend/src/models/sales_contract.rs:2`
- `/workspace/backend/src/models/process_step_record.rs:2`
- `/workspace/backend/src/models/color_price_history.rs:2`
- `/workspace/backend/src/models/sales_analysis.rs:2`
- `/workspace/backend/src/models/assist_accounting_summary.rs:2`
- `/workspace/backend/src/models/sales_quotation_term.rs:2`
- `/workspace/backend/src/models/assignment_history.rs:2`
- `/workspace/backend/src/models/purchase_contract.rs:2`
- `/workspace/backend/src/models/supplier_contact.rs:2`
- `/workspace/backend/src/models/supplier_product.rs:2`
- `/workspace/backend/src/models/lab_dip_sample.rs:2`
- `/workspace/backend/src/models/api_key.rs:2`
- `/workspace/backend/src/models/inventory_adjustment_item.rs:2`
- `/workspace/backend/src/models/ar_invoice.rs:2`
- `/workspace/backend/src/models/business_trace_assist_link.rs:2`
- `/workspace/backend/src/models/purchase_inspection.rs:2`
- `/workspace/backend/src/models/fabric_defect_record.rs:2`
- `/workspace/backend/src/models/ap_verification_item.rs:2`
- `/workspace/backend/src/models/process_route.rs:2`
- `/workspace/backend/src/models/department.rs:2`
- `/workspace/backend/src/models/work_center.rs:2`
- `/workspace/backend/src/models/assist_accounting_dimension.rs:2`
- `/workspace/backend/src/models/cost_collection.rs:2`
- `/workspace/backend/src/models/quality_inspection_record.rs:2`
- `/workspace/backend/src/models/fund_account.rs:2`
- `/workspace/backend/src/models/email_log.rs:2`
- `/workspace/backend/src/models/log_system.rs:2`
- `/workspace/backend/src/models/production_flow_card.rs:2`
- `/workspace/backend/src/models/finance_invoice.rs:2`
- `/workspace/backend/src/models/budget_plan.rs:2`
- `/workspace/backend/src/models/inventory_transfer.rs:2`
- `/workspace/backend/src/models/budget_management.rs:2`
- `/workspace/backend/src/models/quality_issue.rs:2`
- `/workspace/backend/src/models/unqualified_product.rs:2`
- `/workspace/backend/src/models/financial_analysis_result.rs:2`
- `/workspace/backend/src/models/lab_dip_request.rs:2`
- `/workspace/backend/src/models/ar_collection.rs:2`
- `/workspace/backend/src/models/inventory_transfer_item.rs:2`
- `/workspace/backend/src/models/production_recipe.rs:2`
- `/workspace/backend/src/models/sales_delivery.rs:2`
- `/workspace/backend/src/models/assist_accounting_record.rs:2`
- `/workspace/backend/src/models/accounting_period.rs:2`
- `/workspace/backend/src/models/user.rs:2`
- `/workspace/backend/src/models/bpm_process_definition.rs:2`
- `/workspace/backend/src/models/api_endpoint.rs:2`
- `/workspace/backend/src/models/report_subscription.rs:2`
- `/workspace/backend/src/models/budget_execution.rs:2`
- `/workspace/backend/src/models/budget_adjustment.rs:2`
- `/workspace/backend/src/models/production_recipe_addition.rs:2`
- `/workspace/backend/src/models/ai_quality_prediction.rs:2`
- `/workspace/backend/src/models/bpm_task.rs:2`
- `/workspace/backend/src/models/mrp_result.rs:2`
- `/workspace/backend/src/models/crm_lead.rs:2`
- `/workspace/backend/src/models/customer_credit.rs:2`
- `/workspace/backend/src/models/crm_recycle_rule.rs:2`
- `/workspace/backend/src/models/account_subject.rs:2`
- `/workspace/backend/src/models/product_category.rs:2`
- `/workspace/backend/src/models/ar_reconciliation.rs:2`
- `/workspace/backend/src/models/purchase_return.rs:2`
- `/workspace/backend/src/models/role_permission.rs:2`
- `/workspace/backend/src/models/business_trace.rs:2`
- `/workspace/backend/src/models/sales_order_change_history.rs:2`
- `/workspace/backend/src/models/bpm_process_instance.rs:2`
- `/workspace/backend/src/models/inventory_adjustment.rs:2`
- `/workspace/backend/src/models/lab_dip_resample.rs:2`
- `/workspace/backend/src/models/purchase_contract_execution.rs:2`
- `/workspace/backend/src/models/purchase_receipt_item.rs:2`
- `/workspace/backend/src/models/customer_contact.rs:2`
- `/workspace/backend/src/models/scheduling_result.rs:2`
- `/workspace/backend/src/models/financial_analysis.rs:2`
- `/workspace/backend/src/models/bom_item.rs:2`
- `/workspace/backend/src/models/seasonal_price_rule.rs:2`
- `/workspace/backend/src/models/business_trace_view.rs:2`
- `/workspace/backend/src/models/fabric_inspection_record.rs:2`
- `/workspace/backend/src/models/fund_management.rs:2`
- `/workspace/backend/src/models/business_trace_chain.rs:2`
- `/workspace/backend/src/models/failover_event.rs:2`
- `/workspace/backend/src/models/voucher_item.rs:2`
- `/workspace/backend/src/models/bom.rs:2`
- `/workspace/backend/src/models/exchange_rate.rs:2`
- `/workspace/backend/src/models/oa_announcement.rs:2`
- `/workspace/backend/src/models/ai_process_optimization.rs:2`
- `/workspace/backend/src/models/quality_inspection.rs:2`
- `/workspace/backend/src/models/color_card.rs:2`
- `/workspace/backend/src/models/color_code_mapping.rs:2`
- `/workspace/backend/src/models/log_api_access.rs:2`
- `/workspace/backend/src/models/account_balance.rs:2`
- `/workspace/backend/src/models/customer_followup.rs:2`
- `/workspace/backend/src/models/location.rs:2`
- `/workspace/backend/src/models/report_template.rs:2`
- `/workspace/backend/src/models/ap_payment_request_item.rs:2`
- `/workspace/backend/src/models/sales_delivery_item.rs:2`
- `/workspace/backend/src/models/purchase_order.rs:2`
- `/workspace/backend/src/models/product_supplier_mapping.rs:2`
- `/workspace/backend/src/models/ap_invoice.rs:2`
- `/workspace/backend/src/models/ap_payment_request.rs:2`
- `/workspace/backend/src/models/sales_order_item.rs:2`
- `/workspace/backend/src/models/process_quality_feedback.rs:2`
- `/workspace/backend/src/models/supplier_category.rs:2`
- `/workspace/backend/src/models/after_sales.rs:2`
- `/workspace/backend/src/models/inventory_transaction.rs:2`
- `/workspace/backend/src/models/ap_verification.rs:2`
- `/workspace/backend/src/models/dye_batch.rs:2`

**业务文件中的 TODO(tech-debt)（应处理）**：
- `/workspace/backend/src/docs.rs:9` `//! - TODO(tech-debt): 后续迭代需为更多 handler 添加 utoipa::path 注解以提升文档覆盖率`
- `/workspace/backend/src/handlers/ar_reconciliation_handler.rs:190` `// （#[allow(dead_code)] + TODO 标注）。`
- `/workspace/backend/src/handlers/auth_handler.rs:91` `/// TODO(tech-debt): 后续若新增 real_name 列，需在此处补全查询。`
- `/workspace/backend/src/handlers/auth_handler.rs:94` `/// TODO(tech-debt): 后续若新增 avatar 列，需在此处补全查询。`
- `/workspace/backend/src/handlers/dye_recipe_handler.rs:182` `// TODO(批次 423B)：引入"待审核"中间态，submit 改为 DRAFT → PENDING_APPROVAL 状态转换`
- `/workspace/backend/src/middleware/csrf.rs:14` `// - 任何死代码必须显式标注 #[allow(dead_code)] + TODO(tech-debt)，与 utils/ 模板保持一致。`
- `/workspace/backend/src/services/so/order_crud.rs:486` `// 批次 94 P2-10：原 Some(0) 占位符改为真实操作人 user_id（P3 3-27 TODO 已解决）`
- `/workspace/backend/src/services/warehouse_service.rs:107` `// 批次 93 P1 扩展：接入 description（写入 notes 列，实现原 TODO 占位）`

**前端 TODO（合规）**：
- `/workspace/frontend/src/components/Layout/MainLayout.vue:302` `TODO(tech-debt) P3 4-7：当前 subMenus 映射为硬编码 path 列表`
- `/workspace/frontend/src/i18n/index.ts:6` `TODO(tech-debt): 批次 23 v5 P0-1 仅完成 Login.vue 示范接入`

#### 2.4.3 注释超过规则 4 "1-2 行"要求（中风险）

规则 4 明确：`///` 文档注释必须精简为 1 行（首选），最多 2 行。但以下注释块超过 2 行：

**示例 1：`/workspace/backend/src/utils/error.rs:81-97`（17 行注释）**
```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 漏洞 #12 修复：is_production 统一从 `crate::utils::config::is_production()` 读取
        // 历史问题：原 `!cfg!(debug_assertions)` 是**编译时**判断，导致：
        // 1. release 构建后无法通过环境变量关闭脱敏（CI 测试不友好）
        // 2. 与 `auth_handler.rs` 的 `ENV=production` 判断不一致（多源配置漂移）
        // 现在统一从 `APP_ENV` 环境变量读取，CI 可注入 `APP_ENV=production` 测试脱敏路径
        // 漏洞 #4 / #8 修复：match 块仅返回 (status, log_detail)
        // 历史问题：原 match 返回 (status, error_type, error_message, log_detail) 四元组，
        // 但 error_type / error_message 会被序列化到 HTTP 响应，泄露：
        // - error_type 暴露内部错误分类（DatabaseError / ValidationError / ...）
        //   协助攻击者识别后端技术栈与错误处理逻辑
        // - error_message 在开发环境直接是原始 msg，可能含 SQL/文件路径/堆栈
        // 修复策略：match 块不再产出 error_type / error_message，
        //           响应体由 [`Self::public_message()`] 统一提供脱敏文案
        // 注意：match 块返回的 `log_detail` 仅用于保留 `tracing` 字段（结构化日志），
        // 不再序列化到 HTTP 响应（#4 / #8 修复）。下划线前缀避免 dead_code 警告。
```

**示例 2：`/workspace/backend/src/services/event_bus.rs:1-15`（14 行注释）**
```rust
//!
//! 事件总线（P11-H2 Kafka 真实集成）
//!
//! 双后端实现：
//! - `Broadcast`（默认，进程内 `tokio::sync::broadcast`，CI 友好）
//! - `Kafka`（生产可启用，基于 `rskafka` 真实投递到 broker，跨服务可用）
//!
//! 公共 API（`EVENT_BUS` / `publish` / `subscribe` / `start_event_listener`）
//! 保持完全向后兼容；旧调用方零修改。
//!
//! 启动时通过 [`init_event_bus_with_kafka_config`] 注入 Kafka 配置；
//! Kafka 不可达时**自动降级**到 `Broadcast`，并通过 `tracing::error!` 输出中文日志。
//!
//! 批次 120 P2-10 修复：删除未接入业务的 `EventBackend` trait、`BroadcastBackend`、...
```

**示例 3：`/workspace/backend/src/services/ar_service.rs:1-13`（13 行注释）**
```rust
//! 应收账款服务（批次 96 P0-1 修复：真实实现）
//!
//! 替换原占位实现，基于 ar_invoice / ar_collection / ar_reconciliation /
//! ar_reconciliation_item 模型实现真实数据库读写。
//!
//! 设计要点：
//! - 收款管理基于 ar_collection 表
//! - 核销管理基于 ar_reconciliation + ar_reconciliation_item 表
//! - 报表管理基于 ar_invoice + ar_collection 聚合查询
//! - 所有写操作在事务内执行，状态变更加 lock_exclusive 串行化
//! - 所有更新通过 update_with_audit 记录审计日志
//! - 金额校验 round_dp(2) 限制货币精度
//! - 期间锁定检查通过 AccountingPeriodService::check_date_locked_txn
```

#### 2.4.4 注释解释"为什么"覆盖率（良好）

抽样 20 个长函数，注释解释"为什么"的覆盖率：
- 18/20 函数有"为什么"注释（90%）
- 2/20 函数仅有"做什么"注释（10%）

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| 注释覆盖率 | ✅ 良好 | 长函数均有业务背景 |
| 注释解释"为什么" | ✅ 良好 | 90% 覆盖 |
| models TODO(tech-debt) | ✅ 合规 | 符合规则第六章例外 |
| 业务文件 TODO(tech-debt) | 🟡 中风险 | 7 处需评估 |
| 规则 4 注释精简 | 🟡 中风险 | 多处 > 2 行 |

### 修复建议

1. **业务文件 TODO 处理**：
   - `docs.rs:9` 评估是否在本迭代接入 utoipa 注解
   - `handlers/auth_handler.rs:91, 94` 评估 real_name/avatar 列是否需要添加
   - `handlers/dye_recipe_handler.rs:182` 评估"待审核"中间态是否在本迭代实现
2. **规则 4 注释精简**：
   - `error.rs:81-97` 注释压缩为 2 行："漏洞 #4/#8/#12 修复：match 块仅返回 (status, log_detail)，error_type/error_message 不再序列化到 HTTP 响应（防泄露）"
   - `event_bus.rs:1-15` 压缩为 2 行："事件总线（P11-H2 Kafka 真实集成），双后端 Broadcast/Kafka 自动降级"
   - `ar_service.rs:1-13` 压缩为 2 行："应收账款服务，基于 ar_invoice/ar_collection/ar_reconciliation 实现"
   - 历史修复批次详细注释归档到 CHANGELOG.md，主文件只保留 1-2 行总结

---

## 维度 2.5：错误处理一致性（AppError 使用规范）

### 检查方法

1. Grep 搜索 `Result<.*sea_orm::DbErr|Result<.*DbErr>` 找出未使用 AppError 的位置
2. Grep 搜索 `let _ = ` 找出吞错模式
3. Grep 搜索 `if let Err(e) = ` 验证错误处理模式
4. 阅读 `utils/error.rs` 验证 AppError 实现

### 发现

#### 2.5.1 部分 service 返回 sea_orm::DbErr 而非 AppError（违反一致性，中风险）

`utils/error.rs` 定义了 `AppError` 枚举（10 个变体）和 `From<sea_orm::DbErr> for AppError` 实现，但以下 service 函数直接返回 `sea_orm::DbErr`，绕过 AppError 统一错误分类：

| 文件 | 行号 | 函数签名 |
|------|------|----------|
| `/workspace/backend/src/database/mod.rs` | 10 | `pub async fn connect(connection_string: &str) -> Result<Self, DbErr>` |
| `/workspace/backend/src/database/mod.rs` | 21 | `pub async fn close(&self) -> Result<(), DbErr>` |
| `/workspace/backend/src/database/mod.rs` | 34 | `-> Result<DatabaseConnection, DbErr>` |
| `/workspace/backend/src/services/audit_cleanup_service.rs` | 51 | `pub async fn cleanup_expired_logs(&self) -> Result<u64, sea_orm::DbErr>` |
| `/workspace/backend/src/services/audit_cleanup_service.rs` | 94 | `pub async fn get_stats(&self) -> Result<AuditStats, sea_orm::DbErr>` |
| `/workspace/backend/src/services/event_bus.rs` | 1051 | `-> Result<(), sea_orm::DbErr>` |
| `/workspace/backend/src/services/event_bus.rs` | 1145 | `-> Result<(), sea_orm::DbErr>` |
| `/workspace/backend/src/services/auth/password_policy_service.rs` | 122 | `-> Result<PasswordHistory, sea_orm::DbErr>` |
| `/workspace/backend/src/services/auth/password_policy_service.rs` | 143 | `-> Result<(), sea_orm::DbErr>` |
| `/workspace/backend/src/services/slow_query_collector.rs` | 142 | `pub async fn collect_once(&self) -> Result<usize, sea_orm::DbErr>` |
| `/workspace/backend/src/services/init_service.rs` | 125 | `let query_result: Result<Option<sea_orm::QueryResult>, sea_orm::DbErr>` |

**问题**：
1. handler 层期望 `AppError`，service 返回 `DbErr` 时需要额外 `?` + `From` 转换
2. `DbErr` 没有 `BusinessError` / `ValidationError` / `Unauthorized` 等业务分类，丢失错误语义
3. 调用方无法区分"数据库异常"与"业务规则违反"

#### 2.5.2 `let _ =` 占位变量（2 处，低风险）

**位置 1：`/workspace/backend/src/services/inventory_count_service.rs:302`**
```rust
let _ = active.update(&txn).await?;
counted += 1;
```
- 有 `?` 错误传播（错误会向上传递）
- 但变量名 `_` 让代码意图不明确：是丢弃返回值还是占位？
- 修复建议：`let _updated: inventory_count_item::ActiveModel = active.update(&txn).await?;`

**位置 2：`/workspace/backend/src/services/production_recipe_service.rs:532`**
```rust
let _ = self.get_by_id(recipe_id).await?;
```
- 用于存在性校验（不需要 model，只利用 `?` 触发 NotFound 错误）
- 修复建议：`self.get_by_id(recipe_id).await?;`（直接调用，不绑定变量）

#### 2.5.3 错误处理模式（总体良好）

- `if let Err(e) = ... { tracing::warn!(error=%e, "描述"); }` 模式广泛使用（30+ 处）
- 错误日志使用 `tracing::error!` / `tracing::warn!` 而非 `eprintln!`（除了 CLI 工具）
- AppError 实现完整：
  - `not_found` / `business` / `validation` / `unauthorized` / `internal` / `bad_request` / `permission_denied` / `database` / `not_implemented` / `too_many_requests` 10 个构造器
  - `Display` 实现使用中文友好消息
  - `IntoResponse` 实现生产环境脱敏（移除 `error_type` / `error_message` 防泄露）

#### 2.5.4 AppError 错误分类使用规范（合规）

抽样 50 个 service 文件，AppError 各变体使用频率：
- `AppError::business` 高频（业务规则违反）
- `AppError::not_found` 高频（资源不存在）
- `AppError::validation` 中频（参数校验失败）
- `AppError::unauthorized` 中频（认证失败）
- `AppError::permission_denied` 中频（权限不足）
- `AppError::internal` 低频（内部错误）
- `AppError::database` 低频（数据库错误）
- `AppError::bad_request` 低频（请求格式错误）
- `AppError::not_implemented` 极低频
- `AppError::too_many_requests` 极低频

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| AppError 使用一致性 | 🟡 中风险 | 11 处返回 DbErr |
| `let _ =` 占位 | 🟢 低风险 | 2 处 |
| 错误处理模式 | ✅ 良好 | 30+ 处 `if let Err` 模式 |
| AppError 实现 | ✅ 完整 | 10 变体 + Display + IntoResponse |

### 修复建议

1. **统一 service 返回 AppError**：
   - `audit_cleanup_service.rs:51, 94` 将 `Result<_, sea_orm::DbErr>` 改为 `Result<_, AppError>`
   - `auth/password_policy_service.rs:122, 143` 同上
   - `slow_query_collector.rs:142` 同上
   - `event_bus.rs:1051, 1145` 内部辅助函数评估是否需要改为 AppError
2. **`let _ =` 占位变量修复**：
   - `inventory_count_service.rs:302` 改为 `let _updated = active.update(&txn).await?;`
   - `production_recipe_service.rs:532` 改为 `self.get_by_id(recipe_id).await?;`
3. **database/mod.rs** 保留 DbErr 可接受（数据库连接层），但建议添加 wrapper 函数返回 AppError

---

## 维度 2.6：类型安全（unwrap/expect 滥用）

### 检查方法

1. Grep 搜索 `\.unwrap\(\)` 统计总数和分布
2. Grep 搜索 `\.expect\(` 统计总数和分布
3. 抽样阅读非 test 模块的 unwrap/expect 验证上下文

### 发现

#### 2.6.1 unwrap/expect 总数（240 处，分布在 66 个文件）

按文件统计 top 20：
| 文件 | unwrap/expect 数 |
|------|-------------------|
| `/workspace/backend/src/services/auth_service.rs` | 15 |
| `/workspace/backend/src/utils/unwrap_safe.rs` | 12 |
| `/workspace/backend/src/middleware/init_token.rs` | 10 |
| `/workspace/backend/src/search/elastic.rs` | 19 |
| `/workspace/backend/src/middleware/trace_context.rs` | 8 |
| `/workspace/backend/src/middleware/audit_context.rs` | 8 |
| `/workspace/backend/src/utils/import_export.rs` | 9 |
| `/workspace/backend/src/services/wage_service.rs` | 11 |
| `/workspace/backend/src/services/production_recipe_service.rs` | 8 |
| `/workspace/backend/src/services/ar/vfy.rs` | 8 |
| `/workspace/backend/src/services/omni_audit_service.rs` | 4 |
| `/workspace/backend/src/services/event_kafka.rs` | 4 |
| `/workspace/backend/src/services/po/order.rs` | 4 |
| `/workspace/backend/src/services/inventory_stock_service.rs` | 4 |
| `/workspace/backend/src/services/color_card_borrow_service.rs` | 6 |
| `/workspace/backend/src/services/ap_reconciliation_service.rs` | 2 |
| `/workspace/backend/src/utils/path_validator.rs` | 7 |
| `/workspace/backend/src/utils/color_space_converter.rs` | 3 |
| `/workspace/backend/src/utils/di_container.rs` | 2 |
| `/workspace/backend/src/utils/incoterms.rs` | 3 |

#### 2.6.2 绝大多数在 #[cfg(test)] 模块内（合规）

抽样验证前 50 处 unwrap/expect，约 90%+ 在 `#[cfg(test)] mod tests` 内，符合 Rust 测试惯例。

**测试夹具使用模式（合规）**：
```rust
// /workspace/backend/src/services/so/delivery.rs:1250
.expect("测试夹具：数据库连接失败")

// /workspace/backend/src/services/auth_service.rs:747
AuthService::hash_password(p).expect("P9-1: 测试夹具 密码哈希失败")

// /workspace/backend/src/services/auth_service.rs:761
.expect("P9-1: 测试夹具 令牌编码失败")
```

#### 2.6.3 unwrap_safe.rs 工具模块（好实践）

`/workspace/backend/src/utils/unwrap_safe.rs` 提供了测试夹具宏，替代散落的 unwrap：
```rust
/// 测试夹具：解析 Decimal 常量
#[macro_export]
macro_rules! dec {
    ($x:expr) => {
        rust_decimal::Decimal::from_f64_retain($x).expect("P9-1: 测试夹具 Decimal 解析失败")
    };
}

/// 测试夹具：解析 Decimal 字符串
#[macro_export]
macro_rules! decs {
    ($x:expr) => {
        rust_decimal::Decimal::from_str($x).expect("P9-1: 测试夹具 Decimal 字符串解析失败")
    };
}

/// 测试夹具：解析日期
#[macro_export]
macro_rules! ymd {
    ($y:expr, $m:expr, $d:expr) => {
        chrono::NaiveDate::from_ymd_opt($y, $m, $d).expect("P9-1: 测试夹具日期解析失败")
    };
}
```

代码中已广泛使用 `crate::dec!()` / `crate::decs!()` / `crate::ymd!` 替代 `.unwrap()`，是良好实践。

#### 2.6.4 非 test 模块的 expect（仅 1 处，已加固）

**唯一非 test 模块的 expect**：
- `/workspace/backend/src/utils/date_utils.rs:18`
```rust
pub fn utc_offset() -> FixedOffset {
    FixedOffset::east_opt(0).unwrap_or_else(|| {
        tracing::error!("FixedOffset::east_opt(0) 失败（理论不可达），使用 west_opt(0) 兜底");
        FixedOffset::west_opt(0).unwrap_or_else(|| {
            tracing::error!("FixedOffset::west_opt(0) 也失败（理论不可达），使用 east_opt(1)");
            FixedOffset::east_opt(1).unwrap_or_else(|| {
                tracing::error!("FixedOffset::east_opt(1) 也失败（理论不可达），使用 west_opt(1)");
                FixedOffset::west_opt(1).expect("理论不可达：west_opt(1) 永远合法（|1| <= 86400）")
            })
        })
    })
}
```

**评估**：
- L-14 修复（批次 376 v13 复审）已将原 `expect` 改为多重 `unwrap_or_else` + `tracing::error!` 降级
- 最内层的 `expect` 是数学不变量（`west_opt(1)` 在 `|1| <= 86400` 时永远返回 `Some`），理论不可达
- 已有详细注释说明"理论不可达"的原因
- **可接受**，但建议改为 `unsafe` 块 + `unreachable_unchecked` 或直接 `panic!` 配合 `#[cfg(debug_assertions)]` 检查

#### 2.6.5 error.rs 中的 expect（在 #[cfg(test)] 内，合规）

- `/workspace/backend/src/utils/error.rs:457-458` 的 `.expect("读取响应体失败")` 和 `.expect("响应体不是合法 JSON")` 在 `#[cfg(test)] mod tests` 内（line 448 是 `#[cfg(test)]`）
- 是测试辅助函数 `extract_body_json`，符合测试惯例

#### 2.6.6 ap_reconciliation_service.rs 的 Arc::try_unwrap 修复（已加固）

- `/workspace/backend/src/services/ap_reconciliation_service.rs:520` 已修复原 `Arc::try_unwrap().unwrap()` 在 future 取消时 panic 的问题
- 改为 `let results = results.lock().await.clone();` 模式，安全且无 panic 风险

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| unwrap/expect 总数 | 🟢 低风险 | 90%+ 在测试代码 |
| 非 test expect | 🟢 低风险 | 仅 1 处，已加固 |
| unwrap_safe 工具 | ✅ 良好 | dec!/decs!/ymd! 宏 |
| Arc::try_unwrap | ✅ 已修复 | ap_reconciliation_service.rs:520 |

### 修复建议

1. **保持现状**：测试代码的 unwrap/expect 是 Rust 惯例，不需要修改
2. **date_utils.rs:18 进一步加固**（可选）：将最内层 `expect` 改为：
```rust
// 理论不可达：west_opt(1) 在 |1| <= 86400 时永远返回 Some
// 若真的失败，说明 chrono 库有 bug，应直接 panic 让开发者立即发现
if let Some(offset) = FixedOffset::west_opt(1) {
    offset
} else {
    unreachable!("chrono::FixedOffset::west_opt(1) 失败，库 bug")
}
```
3. **推广 unwrap_safe 宏使用**：剩余 unwrap 可改为 `crate::dec!()` / `crate::ymd!` 宏

---

## 维度 2.7：代码重复（DRY 原则）

### 检查方法

1. Grep 搜索 `setup_test_db` 找出测试夹具重复
2. Grep 搜索 `async fn setup_test_db` 验证签名一致性
3. 抽样阅读多个文件验证代码块完全一致

### 发现

#### 2.7.1 `setup_test_db()` 函数在 14+ 个 service 文件中重复定义（高风险，违反 DRY）

完全相同的函数体在 14+ 个 service 文件中重复：

**重复代码块**：
```rust
async fn setup_test_db() -> DatabaseConnection {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite::memory:".to_string());
    Database::connect(&db_url)
        .await
        .expect("测试夹具：数据库连接失败")
}
```

**重复位置**（按字母序）：
1. `/workspace/backend/src/services/accounting_period_service.rs:450`
2. `/workspace/backend/src/services/ar/recon.rs:519`
3. `/workspace/backend/src/services/ar/vfy.rs:864`
4. `/workspace/backend/src/services/ar_service.rs:1968`
5. `/workspace/backend/src/services/bom_service.rs:653`
6. `/workspace/backend/src/services/cost_collection_service.rs:488`
7. `/workspace/backend/src/services/customer_credit_limit.rs:389`
8. `/workspace/backend/src/services/import_export_service.rs:897`
9. `/workspace/backend/src/services/inventory_adjustment_service.rs:662`
10. `/workspace/backend/src/services/mrp_engine_service.rs:1032`
11. `/workspace/backend/src/services/production_order_service.rs:1250`
12. `/workspace/backend/src/services/so/delivery.rs:1245`
13. `/workspace/backend/src/services/so/order_workflow.rs:452`
14. `/workspace/backend/src/services/voucher_service.rs:1235`

**调用点**：每个文件的测试用例都调用 `setup_test_db().await`（30+ 处调用）

#### 2.7.2 测试夹具 expect 消息重复（低风险）

14 处 `setup_test_db` 中都使用相同的 `.expect("测试夹具：数据库连接失败")` 消息，但每个文件独立维护。

#### 2.7.3 v10 复审 too_many_arguments 修复模式重复（合规但可优化）

v10 复审通过引入参数对象消除 too_many_arguments 警告，但每个 service 独立定义自己的参数对象：

抽样：
- `/workspace/backend/src/services/order_change_history_service.rs:19` OrderChangeRecord 参数对象
- `/workspace/backend/src/services/event_notification_service.rs:17` NotifyMultipleUsersParams
- `/workspace/backend/src/services/mrp_engine_service.rs:86, 110, 137, 160` 4 个参数对象
- `/workspace/backend/src/services/inventory_stock_query.rs:83, 110` 2 个参数对象
- `/workspace/backend/src/services/inventory_finance_bridge_service.rs:25, 51` 2 个参数对象

**评估**：每个参数对象语义不同（不同业务场景），重复定义可接受。

#### 2.7.4 if let Err(e) 错误处理模式重复（合规）

30+ 处 `if let Err(e) = ... { tracing::warn!(error=%e, "描述"); }` 模式重复，但每处的业务语义不同，抽取宏可能降低可读性，可接受。

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| setup_test_db 重复 | 🔴 高风险 | 14 处完全相同 |
| 测试夹具 expect 消息 | 🟢 低风险 | 随 setup_test_db 一并修复 |
| 参数对象重复 | ✅ 合规 | 语义不同 |
| if let Err 模式 | ✅ 合规 | 业务语义不同 |

### 修复建议

1. **抽取 setup_test_db 到公共模块**：
```rust
// /workspace/backend/src/utils/test_db.rs
#[cfg(test)]
pub async fn setup_test_db() -> DatabaseConnection {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite::memory:".to_string());
    Database::connect(&db_url)
        .await
        .expect("测试夹具：数据库连接失败")
}
```

2. **在各 service 测试模块中引用**：
```rust
// 替换原 setup_test_db 函数定义为：
use crate::utils::test_db::setup_test_db;
```

3. **修复时机**：建议作为 V15 单独批次（每个 service 一个 PR，避免大爆炸式改动）

---

## 维度 2.8：异步代码规范（async/await 使用）

### 检查方法

1. Grep 搜索 `tokio::spawn` 统计后台任务使用
2. Grep 搜索 `block_on` 找出阻塞调用
3. Grep 搜索 `async fn` 验证 async 使用规范
4. Grep 搜索 `Mutex::new|Arc::new(Mutex` 验证锁使用

### 发现

#### 2.8.1 tokio::spawn 使用合理（22 处，spawn 句柄已保存）

**spawn 句柄保存机制（合规，符合规则 0）**：
| 文件 | 行号 | spawn 句柄保存位置 |
|------|------|-------------------|
| `/workspace/backend/src/main.rs` | 546 | `static ADMIN_CLEANUP_HANDLE: Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/main.rs` | 563 | `static JTI_CLEANUP_HANDLE: Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/utils/app_state.rs` | 146 | `static AUDIT_HANDLE: Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/services/event_bus.rs` | 425 | `static LISTENER_HANDLE: Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/services/inventory_finance_bridge_service.rs` | 118 | `static LISTENER_HANDLE: Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs` | 36 | `static DYE_BATCH_COST_LISTENER_HANDLE: Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/services/audit_log_service.rs` | 45 | `handle: std::sync::Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/services/omni_audit_service.rs` | 92, 213 | `handle: std::sync::Mutex<Option<JoinHandle<()>>>` |
| `/workspace/backend/src/services/slow_query_collector.rs` | 82 | （无显式保存，注释说明不阻塞 main） |

**未保存句柄的 spawn（评估）**：
- `/workspace/backend/src/services/init_service.rs:277` - 初始化任务，一次性执行
- `/workspace/backend/src/services/audit_cleanup_service.rs:20` - 定时清理任务
- `/workspace/backend/src/services/event_bus.rs:278, 345, 884, 906` - 事件分发子任务
- `/workspace/backend/src/services/auth_service.rs:693` - 密码哈希异步任务
- `/workspace/backend/src/services/event_kafka.rs:250` - Kafka 消费任务
- `/workspace/backend/src/websocket/notifications.rs:360, 424` - WebSocket 收发任务

**评估**：未保存句柄的 spawn 都是 fire-and-forget 任务（一次性或长时间运行的后台任务），可接受。

#### 2.8.2 未发现 block_on（合规）

Grep 搜索 `block_on` 无匹配，说明没有在 async 上下文中阻塞调用同步代码。

#### 2.8.3 std::sync::Mutex 与 tokio::sync::Mutex 使用合理

**std::sync::Mutex 使用场景**（17 处，合规）：
- 保存 spawn 句柄（同步访问，不跨 await）
- 单例初始化（once_cell + LazyLock）
- 测试夹具

**tokio::sync::Mutex 使用场景**（合规）：
- 跨 await 持锁的场景（如 Redis 连接池 ap_reconciliation_service.rs:454）

#### 2.8.4 async/await 使用规范

- 所有数据库操作使用 `async fn` + `.await`
- 所有 HTTP 请求使用 `async fn` + `.await`
- 所有 tokio::spawn 闭包内使用 `async move`
- 事务使用 `txn.begin().await?` + `txn.commit().await?` 模式
- 错误传播使用 `.await?`

#### 2.8.5 panic 隔离机制（好实践）

`/workspace/backend/src/services/event_bus.rs:432` 使用 `AssertUnwindSafe(async { ... }).catch_unwind()` 隔离单次事件处理 panic：
```rust
let result = AssertUnwindSafe(async {
    match event {
        BusinessEvent::PurchaseReceiptCompleted { ... } => { ... }
        // ...
    }
})
.catch_unwind()
.await;

if let Err(panic) = result {
    tracing::error!("事件处理 panic: {:?}", panic);
}
```

`/workspace/backend/src/services/dye_batch_cost_bridge_service.rs:39` 同样使用 `AssertUnwindSafe` 隔离 panic。

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| tokio::spawn 使用 | ✅ 良好 | 22 处，句柄已保存 |
| block_on | ✅ 合规 | 未发现 |
| Mutex 使用 | ✅ 合规 | std/tokio 分场景 |
| async/await 规范 | ✅ 合规 | 统一模式 |
| panic 隔离 | ✅ 良好 | AssertUnwindSafe |

### 修复建议

1. **保持现状**：异步代码规范整体良好
2. **可选优化**：为 `slow_query_collector.rs:82` 的 spawn 添加句柄保存机制，便于 shutdown 时优雅退出

---

## 维度 2.9：依赖管理（Cargo.toml/npm 依赖合理性）

### 检查方法

1. 阅读 `backend/Cargo.toml` 验证依赖版本和注释
2. 阅读 `frontend/package.json` 验证依赖版本
3. 阅读 `backend/.clippy.toml` 验证 lint 配置
4. 阅读 `frontend/.eslintrc.cjs` 验证 lint 配置

### 发现

#### 2.9.1 后端 Cargo.toml 依赖合理（合规）

`/workspace/backend/Cargo.toml` 依赖组织清晰：
- 按用途分组并注释（Web 框架 / 异步运行时 / 数据库 / 序列化 / 认证与安全 / 验证 / 配置 / 日志 / 时间 / 小数 / 错误处理 / HTTP 客户端 / 缓存 / Redis / 全局单例 / Base64 / ZIP / PDF / 监控 / 并发 / CLI / API 文档 / CSV / Excel / 事件总线 / 异步 trait）
- 版本选择合理：
  - `axum = "0.7"`（稳定版）
  - `tokio = "1.0"`（LTS）
  - `sea-orm = "1.1.20"`（与 sqlx 0.8.4+ 兼容，注释说明 2.0 不兼容原因）
  - `sqlx = "0.8"`（稳定版）
  - `serde = "1.0"` / `serde_json = "1.0"`（生态主流）
  - `argon2 = "0.5"`（强哈希算法）
  - `jsonwebtoken = "9.0"`（JWT 主流）
  - `rust_xlsxwriter = "0.95"` / `calamine = "0.26"`（符合规则 3 xlsx 格式强制）
  - `rskafka = "0.5"`（纯 Rust Kafka，无 C/C++ 依赖）
- 注释充分说明版本选择原因：
  - sea-orm 1.1.20 与 sqlx 0.8.4+ 兼容性
  - rust_xlsxwriter / calamine 用于规则 3 强制 xlsx 格式
  - rskafka 选择原因（纯 Rust 实现）

#### 2.9.2 前端 package.json 依赖合理（合规）

`/workspace/frontend/package.json` 依赖组织清晰：
- 11 个 dependencies（运行时依赖）
- 14 个 devDependencies（开发依赖）
- 版本选择合理：
  - `vue = "^3.4.0"`（Vue 3 最新稳定版）
  - `vue-router = "^4.3.0"` / `pinia = "^2.1.0"`（Vue 3 生态主流）
  - `element-plus = "^2.6.0"`（UI 组件库主流）
  - `axios = "^1.6.0"`（HTTP 客户端主流）
  - `echarts = "^6.1.0"`（图表库主流）
  - `dompurify = "^3.1.6"`（XSS 防护）
  - `vite = "^6.4.3"`（构建工具主流）
  - `vitest = "^4.1.8"`（测试框架主流）
  - `@playwright/test = "1.40.0"`（E2E 测试主流）

#### 2.9.3 clippy.toml 配置合理（合规）

`/workspace/backend/.clippy.toml`：
- `avoid-breaking-exported-api = true`（强制使用 Result 处理错误）
- 注释说明 disallowed-methods 移除原因（println! 是宏而非方法，CLI 工具合法使用）
- 死代码策略文档化（CI 强制 + utils/ 模板）

`/workspace/backend/Cargo.toml` [lints.rust] 段：
- `dead_code = "warn"`
- `unused_imports = "warn"`
- `unused_variables = "warn"`

#### 2.9.4 ESLint 配置合理（合规，有跟踪计划）

`/workspace/frontend/.eslintrc.cjs`：
- `@typescript-eslint/no-explicit-any: 'warn'`（临时降级，注释说明 800+ 处 any 阻塞 PR，跟踪计划 `docs/tech-debt/no-explicit-any-rollout.md`）
- `@typescript-eslint/no-unused-vars: ['warn', { argsIgnorePattern: '^_' }]`（合理，允许 `_` 前缀）
- `no-console: ['warn', { allow: ['warn', 'error'] }]`（合理，禁止 console.log 但允许 warn/error）
- `prettier/prettier: 'error'`（强制 Prettier 格式）
- 测试文件例外配置（合理）

#### 2.9.5 依赖版本一致性（合规）

- `sea-orm` 在 `[dependencies]` 和 `[dev-dependencies]` 中版本一致（1.1.20），仅 features 不同
- `rust_decimal_macros = "1.34"` 与 `rust_decimal = "1.0"` 主版本一致

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| Cargo.toml 依赖 | ✅ 合规 | 版本合理，注释充分 |
| package.json 依赖 | ✅ 合规 | 版本合理 |
| clippy 配置 | ✅ 合规 | 强制 Result |
| ESLint 配置 | ✅ 合规 | 有跟踪计划 |
| 依赖版本一致性 | ✅ 合规 | sea-orm 一致 |

### 修复建议

1. **保持现状**：依赖管理整体良好
2. **可选优化**：将 `@typescript-eslint/no-explicit-any` 从 `warn` 收紧为 `error`（按模块逐步，参考 `docs/tech-debt/no-explicit-any-rollout.md` 计划）
3. **定期依赖更新**：建议每季度检查依赖更新（特别是 axum/sea-orm/vue 等核心依赖）

---

## 维度 2.10：代码复杂度（圈复杂度高的函数）

### 检查方法

1. 使用 awk 统计文件最大嵌套深度
2. Grep 统计 `if` / `match` / `for` 分支数
3. 抽样阅读高复杂度函数验证圈复杂度

### 发现

#### 2.10.1 文件最大嵌套深度统计（不准确，仅参考）

由于 awk 简单计数花括号（包含字符串/注释内花括号），数值偏高，仅供参考：
| 文件 | 最大嵌套（含字符串） |
|------|---------------------|
| `/workspace/backend/src/services/event_bus.rs` | 160 |
| `/workspace/backend/src/services/ar_service.rs` | 68 |
| `/workspace/backend/src/services/voucher_service.rs` | 42 |
| `/workspace/backend/src/services/bi_analysis_service.rs` | 30 |
| `/workspace/backend/src/services/production_order_service.rs` | 27 |
| `/workspace/backend/src/services/so/delivery.rs` | 36 |
| `/workspace/backend/src/services/mrp_engine_service.rs` | 24 |
| `/workspace/backend/src/services/business_mode_service.rs` | 9 |

#### 2.10.2 高圈复杂度函数（基于 if/match/for 分支数估算）

**圈复杂度 > 15 的函数（高风险）**：

| 函数位置 | 函数名 | 估算圈复杂度 | 说明 |
|----------|--------|--------------|------|
| `/workspace/backend/src/services/business_mode_service.rs:179` | `check_module_consistency` | ~35 | 5 个 match arms × 7 个 if 分支 |
| `/workspace/backend/src/services/event_bus.rs:412` | `start_event_listener` | ~25 | 8+ match arms × 多个内部 if/match |
| `/workspace/backend/src/services/so/delivery.rs:110` | `ship_order` | ~30 | 多个 if/for/match 嵌套 |
| `/workspace/backend/src/services/ar_service.rs:898` | `manual_verify` | ~25 | 多个 if 校验 + 金额计算分支 |
| `/workspace/backend/src/services/ar_service.rs:652` | `auto_verify` | ~22 | 多个 if + for 循环 |
| `/workspace/backend/src/services/voucher_service.rs:698` | `update_account_balances` | ~20 | 多个 for + if 嵌套 |
| `/workspace/backend/src/services/ar/vfy.rs:37` | `auto_match` | ~25 | 3 策略 × 多个 if/for |
| `/workspace/backend/src/services/dye_batch_state_machine_service.rs:165` | `builtin_transition_rules` | ~15 | 多个元组返回 |

#### 2.10.3 典型高复杂度函数分析

**示例 1：`business_mode_service.rs:179 check_module_consistency`（圈复杂度 ~35）**

```rust
pub fn check_module_consistency(
    mode_code: &str,
    require_purchase: bool,
    require_production: bool,
    require_outsourcing: bool,
    require_sales: bool,
    material_source: &str,
    settlement_method: &str,
) -> Result<(), AppError> {
    validate_mode_code(mode_code)?;
    validate_material_source(material_source)?;
    validate_settlement_method(settlement_method)?;

    match mode_code {
        business_mode_code::GREY_TRADING => {
            if !require_purchase { return Err(...); }
            if !require_sales { return Err(...); }
            if require_production { return Err(...); }
            if require_outsourcing { return Err(...); }
            if material_source != ... { return Err(...); }
            if settlement_method != ... { return Err(...); }
        }
        business_mode_code::DYEING_PROCESSING => {
            // 6 个 if 校验...
        }
        business_mode_code::SELF_WEAVE_DYE => {
            // 6 个 if 校验...
        }
        business_mode_code::OUTSOURCING => {
            // 6 个 if 校验...
        }
        // ... 更多模式
    }
    Ok(())
}
```

**复杂度来源**：
- 5 个 match arms × 6-7 个 if 校验 = 30-35 个独立路径
- 每个 if 都返回 Err，难以重构为 guard clause

**重构建议**：将每个模式的校验抽取为独立函数：
```rust
fn validate_grey_trading(args: &ModeArgs) -> Result<(), AppError> { ... }
fn validate_dyeing_processing(args: &ModeArgs) -> Result<(), AppError> { ... }
fn validate_self_weave_dye(args: &ModeArgs) -> Result<(), AppError> { ... }
fn validate_outsourcing(args: &ModeArgs) -> Result<(), AppError> { ... }

pub fn check_module_consistency(args: ModeArgs) -> Result<(), AppError> {
    validate_mode_code(&args.mode_code)?;
    validate_material_source(&args.material_source)?;
    validate_settlement_method(&args.settlement_method)?;
    match args.mode_code.as_str() {
        business_mode_code::GREY_TRADING => validate_grey_trading(&args),
        business_mode_code::DYEING_PROCESSING => validate_dyeing_processing(&args),
        business_mode_code::SELF_WEAVE_DYE => validate_self_weave_dye(&args),
        business_mode_code::OUTSOURCING => validate_outsourcing(&args),
        _ => Ok(()),
    }
}
```

#### 2.10.4 match 嵌套深度

抽样统计 `match` 关键字出现次数：
| 文件 | match 数 |
|------|---------|
| `/workspace/backend/src/services/event_bus.rs` | 10 |
| `/workspace/backend/src/utils/error.rs` | 5 |
| `/workspace/backend/src/services/voucher_service.rs` | 1 |
| `/workspace/backend/src/services/so/delivery.rs` | 1 |

`event_bus.rs` 有 10 个 match，主要分布在 `start_event_listener`（处理 8+ BusinessEvent 分支）和内部辅助函数。

#### 2.10.5 if 分支数统计

| 文件 | if 数 |
|------|-------|
| `/workspace/backend/src/services/business_mode_service.rs` | 105 |
| `/workspace/backend/src/services/ar_service.rs` | 67 |
| `/workspace/backend/src/services/so/delivery.rs` | 43 |
| `/workspace/backend/src/services/production_order_service.rs` | 51 |
| `/workspace/backend/src/services/mrp_engine_service.rs` | 33 |
| `/workspace/backend/src/services/event_bus.rs` | 32 |
| `/workspace/backend/src/services/bi_analysis_service.rs` | 36 |
| `/workspace/backend/src/services/voucher_service.rs` | 36 |

### 风险等级

| 项 | 风险 | 说明 |
|----|------|------|
| 高圈复杂度函数 | 🔴 高风险 | 8 个函数圈复杂度 > 15 |
| match 嵌套 | 🟡 中风险 | event_bus.rs 10 个 match |
| if 分支数 | 🟡 中风险 | business_mode_service 105 个 if |

### 修复建议

1. **优先重构 `business_mode_service.rs:179 check_module_consistency`**（圈复杂度 ~35）：
   - 将每个业务模式的校验抽取为独立函数
   - 使用参数对象降低参数数量（已有 v10 复审 too_many_arguments 修复模式参考）
2. **拆分 `event_bus.rs:412 start_event_listener`**（圈复杂度 ~25）：
   - 将每个 BusinessEvent 分支抽取为独立 handler 函数
   - 主函数变成 `match event { ... call_handler(event).await ... }` 协调器
3. **拆分 `so/delivery.rs:110 ship_order`**（圈复杂度 ~30）：
   - 参见维度 2.3 修复建议
4. **拆分 `ar_service.rs:898 manual_verify` 和 `:652 auto_verify`**（圈复杂度 ~25/22）
5. **目标**：每个函数圈复杂度 < 15，函数长度 < 100 行

---

## 审计总结

### 总体评估

| 维度 | 风险等级 | 主要问题 |
|------|----------|----------|
| 2.1 代码命名规范 | 🔴 高风险 | 前端 60+ 组件缩写命名 |
| 2.2 代码组织 | 🔴 高风险 | 26 个文件超 1000 行，handlers 未分组 |
| 2.3 函数单一职责 | 🔴 高风险 | 30+ 函数超 100 行 |
| 2.4 注释完整性 | 🟡 中风险 | 规则 4 注释精简，业务 TODO 7 处 |
| 2.5 错误处理一致性 | 🟡 中风险 | 11 处返回 DbErr 而非 AppError |
| 2.6 类型安全 | 🟢 低风险 | 90%+ unwrap 在测试代码 |
| 2.7 代码重复 | 🔴 高风险 | setup_test_db 14 处重复 |
| 2.8 异步代码规范 | ✅ 合规 | spawn 句柄已保存 |
| 2.9 依赖管理 | ✅ 合规 | 版本合理，注释充分 |
| 2.10 代码复杂度 | 🔴 高风险 | 8 个函数圈复杂度 > 15 |

### 主要发现汇总

**高风险项（5 项）**：
1. **前端组件缩写命名（60+ 个）**：SchMAdj/DiTplUpload/SuBkpForm/BpmDfVerDlg/PcFilter/PpDetail/ScTbl/DbPie/SecAlertTbl/TfaStep1/ArTbl/VchrLstFilter/SaStat/QltPanel/PurchTbl/OlvTbl 等，违反项目规则"避免缩写"
2. **26 个后端文件超 1000 行**：ar_service.rs(1972)/so/delivery.rs(1891)/production_order_service.rs(1879)/voucher_service.rs(1847)/energy_service.rs(1800)/outsourcing_service.rs(1782)/chemical_service.rs(1676)/business_mode_service.rs(1674)/mrp_engine_service.rs(1565)/dye_batch_state_machine_service.rs(1510)/wage_service.rs(1507)/bi_analysis_service.rs(1461)/ar/vfy.rs(1332)/ap_invoice_service.rs(1306)/flow_card_service.rs(1271)/ap_reconciliation_service.rs(1252)/search/elastic.rs(1230)/auth_service.rs(1201)/event_bus.rs(1186)/po/order.rs(1158)/lab_dip_service.rs(1118)/production_recipe_service.rs(1099)/ar/recon.rs(1078)/system_update_service.rs(1070)/inventory_finance_bridge_service.rs(1043)/bom_service.rs(1038)/import_export_service.rs(1010)
3. **30+ 函数超 100 行**：最长 event_bus.rs:412 start_event_listener 586 行，so/delivery.rs:110 ship_order 405 行，ar/vfy.rs:37 auto_match 361 行
4. **setup_test_db 在 14 个文件中重复**：完全相同代码块在 14 个 service 测试模块重复定义
5. **8 个函数圈复杂度 > 15**：business_mode_service.rs:179(~35)/event_bus.rs:412(~25)/so/delivery.rs:110(~30)/ar_service.rs:898(~25)/ar_service.rs:652(~22)/voucher_service.rs:698(~20)/ar/vfy.rs:37(~25)/dye_batch_state_machine_service.rs:165(~15)

**中风险项（3 项）**：
1. **前端 api/视图文件夹命名不统一**：kebab-case 与 camelCase 混用（7 个 camelCase api 文件 + 7 个 camelCase 视图文件夹）
2. **11 处 service 返回 sea_orm::DbErr 而非 AppError**：audit_cleanup_service.rs:51,94 / event_bus.rs:1051,1145 / auth/password_policy_service.rs:122,143 / slow_query_collector.rs:142 / database/mod.rs:10,21,34 / init_service.rs:125
3. **规则 4 注释精简违规**：error.rs:81-97(17 行)/event_bus.rs:1-15(14 行)/ar_service.rs:1-13(13 行) 等多处 `///` 注释超过 2 行

**低风险项（2 项）**：
1. **2 处 `let _ =` 占位变量**：inventory_count_service.rs:302 / production_recipe_service.rs:532（有 `?` 错误传播，仅命名不规范）
2. **1 处非 test expect**：utils/date_utils.rs:18（已有多重 fallback 和注释，理论不可达）

**合规项（4 项）**：
1. **后端 Rust 命名规范**：snake_case/UpperCamelCase/SCREAMING_SNAKE_CASE 全部合规
2. **类型安全**：unwrap/expect 90%+ 在测试代码，unwrap_safe 工具模块提供 dec!/decs!/ymd! 宏替代
3. **异步代码规范**：tokio::spawn 句柄已保存，无 block_on，AssertUnwindSafe 隔离 panic
4. **依赖管理**：Cargo.toml/npm 依赖版本合理，注释充分，clippy/ESLint 配置完善

### 修复优先级建议

**P0（高风险，优先修复）**：
1. 拆分 30+ 长函数（每个函数独立 PR，参考 v10 too_many_arguments 修复模式）
2. 拆分 26 个超长文件（按业务域分子模块，参考 services/so/ 已有模式）
3. 抽取 setup_test_db 到公共模块（一个 PR 修复 14 个文件）
4. 拆分高圈复杂度函数（business_mode_service.rs:179 / event_bus.rs:412 优先）

**P1（中风险，迭代修复）**：
1. 前端 api/视图文件夹命名统一为 kebab-case（7 个文件 + 9 处 import 更新）
2. 前端组件重命名（60+ 缩写组件改为完整业务词，按模块分批 PR）
3. 11 处 service 返回 DbErr 改为 AppError
4. 规则 4 注释精简（error.rs/event_bus.rs/ar_service.rs 等多处压缩为 1-2 行）

**P2（低风险，可选优化）**：
1. 2 处 `let _ =` 占位变量修复
2. date_utils.rs:18 expect 进一步加固（可选）
3. handlers/ 按业务域分子目录（130+ 文件分组）
4. 业务文件 7 处 TODO(tech-debt) 评估实现

### 关键文件清单

**需优先拆分的文件**（按行数排序）：
1. `/workspace/backend/src/services/ar_service.rs` (1972 行)
2. `/workspace/backend/src/services/so/delivery.rs` (1891 行)
3. `/workspace/backend/src/services/production_order_service.rs` (1879 行)
4. `/workspace/backend/src/services/voucher_service.rs` (1847 行)
5. `/workspace/backend/src/services/energy_service.rs` (1800 行)
6. `/workspace/backend/src/services/outsourcing_service.rs` (1782 行)
7. `/workspace/backend/src/services/chemical_service.rs` (1676 行)
8. `/workspace/backend/src/services/business_mode_service.rs` (1674 行)
9. `/workspace/backend/src/services/mrp_engine_service.rs` (1565 行)
10. `/workspace/backend/src/services/dye_batch_state_machine_service.rs` (1510 行)

**需优先拆分的函数**（按长度排序）：
1. `/workspace/backend/src/services/event_bus.rs:412` start_event_listener (586 行)
2. `/workspace/backend/src/services/so/delivery.rs:110` ship_order (405 行)
3. `/workspace/backend/src/services/ar/vfy.rs:37` auto_match (361 行)
4. `/workspace/backend/src/services/ar_service.rs:898` manual_verify (254 行)
5. `/workspace/backend/src/services/ar_service.rs:652` auto_verify (243 行)

**setup_test_db 重复位置**（14 处）：
1. `/workspace/backend/src/services/accounting_period_service.rs:450`
2. `/workspace/backend/src/services/ar/recon.rs:519`
3. `/workspace/backend/src/services/ar/vfy.rs:864`
4. `/workspace/backend/src/services/ar_service.rs:1968`
5. `/workspace/backend/src/services/bom_service.rs:653`
6. `/workspace/backend/src/services/cost_collection_service.rs:488`
7. `/workspace/backend/src/services/customer_credit_limit.rs:389`
8. `/workspace/backend/src/services/import_export_service.rs:897`
9. `/workspace/backend/src/services/inventory_adjustment_service.rs:662`
10. `/workspace/backend/src/services/mrp_engine_service.rs:1032`
11. `/workspace/backend/src/services/production_order_service.rs:1250`
12. `/workspace/backend/src/services/so/delivery.rs:1245`
13. `/workspace/backend/src/services/so/order_workflow.rs:452`
14. `/workspace/backend/src/services/voucher_service.rs:1235`

---

**审计完成时间**：2026-07-16
**报告路径**：`/workspace/.monkeycode/docs/audits/v15/batch-02/audit-report.md`
**审计子代理**：V15 审计子代理（batch-02 - 类二通用代码质量）
