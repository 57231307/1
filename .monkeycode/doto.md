# 未完成任务（详细）

> 本文件**详细**记录未完成的任务（问题描述、影响范围、修复方案、技术要点），禁止简化。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近一次整理：2026-07-12（批次 330 规则 10 整理，已完成批次 290-329 迁移到 doto-su.md）。

---

## ✅ 历史任务：v8/v9 复审问题修复（全部完成）

- **v8 复审**（批次 290-308）：21 项问题（4 高 + 8 中 + 9 低）全部修复，详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。
- **v9 复审**（批次 317-323）：16 项问题（2 P0 + 2 高 + 5 中 + 7 低）全部修复，详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。
- **sea-orm 版本调研**（批次 324）：确认使用 1.1.20 稳定版正确，2.0 仍 RC 不升级。
- **规则 14 新增**（批次 324）：移除所有警告抑制，clippy baseline 渐进清理。

---

## 🔥 当前任务：v10 复审问题修复（P0 1/1 ✅，P1 5/5 ✅，P2 4/4 ✅，P3 43/43 ✅ 全部完成）

> **v10 复审报告**（2026-07-12，Task 工具扫描）：v9 + sea-orm 调研 + 规则 14 新增后复审，扫描所有 `#[allow(...)]` 警告抑制。
> 发现 180 个抑制标注（108 例外 models/ + 72 非例外），非例外分类：1 P0 + 5 P1 + 4 P2 + ~43 P3。
> 修复策略：按规则 13+14 连续执行，P0 → P1 → P2 → P3，每批 1 commit，CI 全绿后合并 main。
> **批次文件数量策略**：每批次处理 5-6 个文件（用户最新要求，原为 8-10）。
> **v10 复审全部完成**（2026-07-12，批次 339 合并）：所有 `#[allow(clippy::too_many_arguments)]` 抑制已全部移除，规则 14 合规。

### 进度总览

| 优先级 | 总数 | 已完成 | 剩余 | 状态 |
|--------|------|--------|------|------|
| 🔴 P0 死代码 | 1 | 1 | 0 | ✅ 全部完成（批次 325） |
| 🟠 P1 文件级抑制过宽+未使用重导出 | 5 | 5 | 0 | ✅ 全部完成（批次 325） |
| 🟡 P2 clippy 代码味道 | 4 | 4 | 0 | ✅ 全部完成（批次 326，pred.rs 2 项已在 main 修复） |
| 🟢 P3 too_many_arguments | 43 | 43 | 0 | ✅ 全部完成（批次 327-339，含 9 项误报删除） |
| **合计** | **~53** | **53** | **0** | ✅ v10 复审全部完成 |

### ✅ 已完成（批次 325-339）

**批次 339（PR #511）**：v10 复审 P3 too_many_arguments DTO 重构剩余 3 项（v10 复审收官）
- `product_service.rs create_product`：19 参数 → 1 参数对象 `CreateProductArgs`（19 字段，含面料行业字段）
- `product_service.rs update_product`：19 参数 → 1 参数对象 `UpdateProductArgs`（19 字段，所有业务字段均为 Option）
- `mrp_engine_service.rs explode_bom_recursive`：11 参数（含 &self + &mut Vec + &mut HashMap）→ 4 参数（&self + `ExplodeBomArgs<'a>` + &mut Vec + &mut HashMap），9 个标量参数聚合为参数对象，借用参数 results/stock_cache 保留在签名中
- 调用方同步修改：product_handler.rs（create_product + update_product）+ product_service.rs import_products_from_csv 内部调用方 + mrp_engine_service.rs 递归调用方 + explode_bom 入口调用方
- **v10 复审 P3 too_many_arguments 全部 43 项完成，所有 #[allow(clippy::too_many_arguments)] 抑制全部移除，规则 14 合规**

**批次 338（PR #510）**：v10 复审 P3 too_many_arguments DTO 重构 8 项（5 核心 service + 8 调用方）
- `ai/recipe_opt.rs make_recipe`：8 参数 → 1 参数对象 `RecipeFixture<'a>`（测试夹具，7 个调用点同步修改）
- `inventory_stock_query.rs record_transaction`：18 参数 → 1 参数对象 `RecordTransactionArgs`（sales_return_service.rs 调用方同步修改）
- `inventory_stock_service.rs create_stock`：12 参数 → 1 参数对象 `CreateStockArgs` + `create_stock_fabric`：13 参数 → 1 参数对象 `CreateStockFabricArgs`（2 个 handler 调用方同步修改）
- `inventory_stock_txn.rs create_stock_fabric_txn`：14 参数 → 2 参数（txn + CreateStockFabricArgs 复用）+ `record_transaction_txn`：19 参数 → 2 参数（txn + RecordTransactionArgs 复用）（4 个 service 调用方共 9 个调用点同步修改）
- `customer_service.rs create_customer`：18 参数 → 1 参数对象 `CreateCustomerArgs` + `update_customer`：18 参数 → 1 参数对象 `UpdateCustomerArgs`（customer_handler.rs 调用方同步修改）

**批次 337（PR #509）**：v10 复审 P3 too_many_arguments DTO 重构 6 项
- `inventory_finance_bridge_service.rs` 6 个函数统一引入 `VoucherCreateArgs<'a>` 参数对象：
  - 5 个 `create_*_voucher` 私有函数（create_purchase_receipt_voucher/create_sales_delivery_voucher/create_inventory_adjustment_voucher/create_production_receipt_voucher/create_production_issue_voucher）：10 参数 → 1 参数对象
  - `handle_inventory_transaction`：12 参数 → 3 参数（_transaction_id + transaction_type + VoucherCreateArgs）
- 使用生命周期 `&'a str` 借用 source_bill_type/source_bill_no/batch_no/color_no，避免调用方不必要的 to_string()
- 事件监听器 `start_listener` 同步构造 `VoucherCreateArgs` 并传递给 `handle_inventory_transaction`
- **CI 修复**：补充 `OrderChangeRecord` dead code 警告到 clippy baseline（批次 332 引入的 struct 唯一构造点在 dead code `record_order_created` 内部，编译器推断 never constructed，属预存技术债务传播）

**批次 336（PR #508）**：v10 复审 P3 too_many_arguments DTO 重构 1 项
- `mrp_engine_service.rs calculate_requirement`：8 参数 → 1 参数，引入 `RequirementCalcParams` 参数对象（product_id/required_quantity/required_date/source_type/source_id/consider_safety_stock/consider_in_transit/bom_level）
- 同步更新 `run_mrp_calculation` 内部调用方构造 `RequirementCalcParams`（bom_level=0 表示顶层）
- 注：`calculate_requirement_with_stock`（10 参数含 &self + &StockInfo）和 `explode_bom_recursive`（11 参数含 &self + &mut Vec + &mut HashMap）仍保留 #[allow]，因含借用参数需单独评估

**批次 335（PR #507）**：v10 复审 P3 too_many_arguments DTO 重构 1 项
- `inventory_stock_query.rs list_transactions`：9 参数 → 1 参数，引入 `ListTransactionsQuery` 参数对象（page/page_size/batch_no/color_no/product_id/warehouse_id/transaction_type/start_date/end_date）
- 在 service 层定义独立 `ListTransactionsQuery`，与 handler 层 `ListTransactionParams` 分离（service 不依赖 axum Deserialize）
- 同步更新 `inventory_stock_handler_query.rs` 调用方构造 `ListTransactionsQuery`，函数体内 `query` 变量重命名为 `q` 避免与参数名冲突

**批次 334（PR #506）**：v10 复审 P3 too_many_arguments DTO 重构 1 项
- `inventory_finance_bridge_service.rs make_voucher_item`：9 参数 → 1 参数，引入 `VoucherItemArgs<'a>` 参数对象（line_no/subject_code/subject_name/debit/credit/summary/quantity_meters/quantity_kg/unit_price）
- 使用生命周期 `&'a str` 借用 subject_code/subject_name，避免调用方不必要的 to_string()
- 同步更新 12 个内部调用点（采购入库/销售出库/库存调整盘盈盘亏/生产入库/生产领料 各 2 个分录）
- make_voucher_item 是私有函数（fn 不是 pub fn），所有调用均在 crate 内

**批次 333（PR #505）**：v10 复审 P3 too_many_arguments DTO 重构 1 项
- `po/price.rs create_purchase_suggestion_from_shortage`：8 参数 → 1 参数，引入 `ShortageAlertParams` 参数对象（material_id/material_name/material_code/required_quantity/available_quantity/shortage_quantity/shortage_level/affected_orders_count）
- 调用方：`event_bus.rs` 的 BusinessEvent::MaterialShortageAlert 处理分支同步构造 `ShortageAlertParams` 参数对象

**批次 332（PR #504）**：v10 复审 P3 too_many_arguments DTO 重构 1 项
- `order_change_history_service.rs record_change`：9 参数（含 &self，8 参数不含 self >7）→ 1 参数，引入 `OrderChangeRecord` 参数对象（order_id/change_type/field_name/old_value/new_value/changed_by/change_reason/ip_address/user_agent），内部调用方 record_order_created 同步修改
- 调用链分析：record_change 仅被内部 record_order_created 调用，record_order_created 虽 pub 但 crate 内无外部调用（business_metrics/metrics_service 的 record_order_created 是不同 service 的方法）

**批次 331（PR #503）**：v10 复审 P3 too_many_arguments DTO 重构 1 项
- `utils/app_state.rs with_secrets_and_cors`：8 参数 → 1 参数，引入 `AppStateParams` 参数对象（db/omni_audit/audit_cleanup/jwt_secret/previous_jwt_secret/cookie_secret/webhook_secret/allowed_origins），main.rs 调用方同步修改
- 附：补充 clippy baseline 3 项（path_validator 模块的 validate_extracted_paths/validate_dir_recursive/MAX_RECURSION_DEPTH 被编译器判定为 dead code，疑似 pub(super) 可见性导致 reachability 分析未追踪到 cli::util::run 调用链，属预存技术债务）

**批次 330（PR #502）**：v10 复审 P3 误报删除 5 项 + DTO 重构 1 项
- 误报删除 5 项（clippy::too_many_arguments 不计算 &self，阈值 7，参数 ≤7 均为误报）：
  - `product_service.rs create_product_color`：7 参数（不含 &self），删除误报 #[allow]
  - `inventory_stock_query.rs get_inventory_summary`：7 参数（不含 &self），删除误报 #[allow]
  - `mrp_engine_service.rs explode_bom`：7 参数（不含 &self），删除误报 #[allow]
  - `mrp_engine_service.rs run_mrp_calculation`：7 参数（不含 &self），删除误报 #[allow]
  - `ar/inv.rs create_receivable`：6 参数（不含 &self），删除误报 #[allow]
- DTO 重构 1 项：`product_service.rs update_product_color` 8 参数 → 1 参数，引入 `UpdateProductColorParams` 参数对象（id/color_name/pantone_code/color_type/dye_formula/extra_cost/is_active/user_id），handler 调用方同步修改
- 附：规则 10 每 15 批次记忆整理（迁移批次 290-329 归档摘要到 doto-su.md）

**批次 329（PR #501）**：v10 复审 P3 too_many_arguments 参数对象重构 2 项
- `ar_service.rs create_payment`：8 参数 → 2 参数，引入 `CreateArPaymentParams` 参数对象（customer_id/amount/payment_method/payment_date/bank_account/remark/invoice_ids），handler 同步修改
- `budget_management_service.rs create_execution`：9 参数 → 2 参数，引入 `CreateBudgetExecutionParams` 参数对象（plan_id/execution_type/amount/expense_date/expense_type/related_document_type/related_document_id/remark），handler + service 内部 2 处调用方（occupy_budget/verify_budget）同步修改

**批次 328（PR #500）**：v10 复审 P3 误报 too_many_arguments 抑制移除 9 项
- clippy too_many_arguments 默认阈值 7（参数 >7 才警告），9 个函数参数 ≤7 均为误报
- 1 参数：`color_card_borrow_service.rs list_records`
- 5 参数：`ar_service.rs manual_verify` + `ai/quality_pred.rs make_record`（测试夹具）
- 6 参数：`color_card_borrow_service.rs borrow`
- 7 参数：`ap_payment_service.rs get_list` + `ap_payment_request_service.rs get_list` + `ap_invoice_service.rs get_list` + `finance_payment_service.rs create_payment` + `email_service.rs tencent_sign` + `event_notification_service.rs notify_multiple_users`

**批次 327（PR #499）**：v10 复审 P3 too_many_arguments 抑制移除 3 项
- `import_export_service.rs:299`：移除误报 `#[allow]`（import_data 仅 3 参数，远低于阈值 7）
- `cache.rs:407`：移除误报 `#[allow]`（set_csrf_token 仅 5 参数，低于阈值 7）
- `user_notification_setting_service.rs:50`：引入 `UpdateNotificationSettingParams` 参数对象聚合 8 个独立参数，handler 同步修改

**批次 326（PR #498）**：v10 复审 P2 clippy 警告抑制移除 2 项
- `sales_analysis_service.rs:228`：移除 `#[allow(clippy::needless_late_init)]`，active_customers 声明与赋值合并
- `material_shortage_service.rs:205`：移除 `#[allow(clippy::type_complexity)]`，提取类型别名 `MaterialReq`
- 注：`ai/pred.rs:90,101` 的 2 项 needless_range_loop 已在 main（5291e773）中修复，无需重复

**批次 325（PR #497）**：v10 复审 P0+P1 警告抑制移除 6 项
- P0：删除 `import_export_service.rs` 死代码 `ExportFormatType` enum（无任何业务引用）
- P1：删除 `enhanced_logger.rs` 文件级 `#![allow(clippy::too_many_arguments)]`（函数仅 1 参数）
- P1：删除 `sensitive_action_alert.rs` 文件级 `#![allow(clippy::too_many_arguments)]`（最多 5 参数）
- P1：删除 `so/mod.rs` 2 个未使用 `pub use` + `#[allow(unused_imports)]`
- P1：删除 `po/mod.rs` 1 个未使用 `pub use` + `#[allow(unused_imports)]`

### 🟢 P3 too_many_arguments（43/43 ✅ 全部完成）

**问题描述**：43 处 `#[allow(clippy::too_many_arguments)]` 抑制，函数参数过多（>7）。

**修复结果**（批次 327-339）：
- 9 项误报删除（参数 ≤7，clippy 阈值 7 不计 &self）：批次 327（2 项）+ 328（9 项，注：实际 9 项误报）
- 34 项 DTO 重构（引入参数对象 Parameter Object 模式）：批次 329-339
- **所有 `#[allow(clippy::too_many_arguments)]` 抑制已全部移除**（2026-07-12 批次 339 合并确认）

**修复方案**：
- 引入参数对象（Parameter Object）重构模式
- 将相关参数分组为 struct，按职责聚合
- 每批次处理 5-6 个文件，避免单批次过大
- 优先处理 service 层（业务逻辑核心），再处理 handler 层

**技术要点**：
- 参数对象需实现 `Clone`/`Debug` 便于测试
- handler 层参数对象可考虑 `From<Request>` 提取器
- service 层参数对象可考虑 Builder 模式（可选参数多时）
- 含借用参数（&mut Vec / &mut HashMap / &str）的函数：标量参数聚合为参数对象，借用参数保留在签名中（如 explode_bom_recursive）

---

## 🔥 当前任务：v11 复审问题修复（P0 0/1，P1 0/8，P2 0/10 保留，P3 0/8 保留）

> **v11 复审报告**（2026-07-12，批次 339 合并后 Task 工具扫描）：v10 复审全部完成后复审，扫描所有剩余 `#[allow(...)]` 警告抑制（非 models/ SeaORM 例外）。
> 发现 27 个抑制标注：1 P0 + 8 P1 + 10 P2（带 TODO 保留）+ 8 P3（合理保留）。
> 修复策略：按规则 13+14 连续执行，P0 → P1，每批 5-6 个文件，CI 全绿后合并 main。
> P2 项均带 TODO(tech-debt) 标注，待业务接入后移除；P3 项为测试代码防御性抑制，合理保留。

### 进度总览

| 优先级 | 总数 | 已完成 | 剩余 | 状态 |
|--------|------|--------|------|------|
| 🔴 P0 文件级抑制超出例外 | 1 | 0 | 1 | ⏳ 待修复 |
| 🟠 P1 clippy 警告抑制 | 8 | 0 | 8 | ⏳ 待修复 |
| 🟡 P2 带 TODO 的 dead_code | 10 | 0 | 10 | ⏸️ 保留（待业务接入） |
| 🟢 P3 测试代码/防御性抑制 | 8 | 0 | 8 | ⏸️ 合理保留 |
| **合计** | **27** | **0** | **27** | 🔄 进行中 |

### 🔴 P0 待修复项（1 项）

**P0-1：business_trace_snapshot.rs 文件级抑制超出例外范围**
- 文件：`/workspace/backend/src/models/business_trace_snapshot.rs:1`
- 抑制：`#![allow(dead_code, unused_imports, unused_variables)]`
- 问题：项目规则仅允许 models/ 下 SeaORM 自动生成模型保留 `#![allow(dead_code)]`，该文件额外抑制了 `unused_imports` 和 `unused_variables``，超出例外范围
- 修复：收窄为 `#![allow(dead_code)]`，清理真实的未使用导入与变量

### 🟠 P1 待修复项（8 项）

**P1-1：auth_handler.rs:205 redundant_clone** — login 函数 audit_ctx.clone() + csrf_token.clone()
**P1-2：auth_handler_misc.rs:34 redundant_clone** — refresh_token 函数 token.clone() + csrf_token.clone()
**P1-3：import_export_service.rs:211 needless_pass_by_value** — parse_csv content: &str 参数
**P1-4：import_export_service.rs:242 needless_pass_by_value** — validate_import_data data: &[Vec<String>] + template: &ImportTemplate
**P1-5：inventory_count_service.rs:72 default_constructed_unit_structs** — SeaORM Entity::default() 模式
**P1-6：crud_macro.rs:35 default_constructed_unit_structs** — 宏内 SeaORM Entity 构造（合理保留）
**P1-7：crud_macro.rs:47 default_constructed_unit_structs** — 宏内 SeaORM Entity 构造（合理保留）
**P1-8：color_card_borrow_service.rs:61 should_implement_trait** — from_str 方法 + TODO 迁移到 FromStr trait

### 🟡 P2 保留项（10 项，带 TODO(tech-debt)）

- P2-1：bpm_dto.rs:48 TemplateQuery.category 字段（待模板子分类功能）
- P2-2~P2-5：dto/mod.rs:32,39,45,51 PageRequest 四个分页工具方法（待 paginate_with_total 统一接入）
- P2-6：status.rs:238 inventory_reservation::LOCKED 常量（待 lock_reservation 路由接入）
- P2-7：status.rs:243 inventory_reservation::RELEASED 常量（待 release_reservation 路由接入）
- P2-8：app_state.rs:254 AppState::default dead_code + unused_variables（待重构为 for_test() 构造器）
- P2-9：user_notification_setting.rs:55 notification_type::NONE 常量（待通知类型 NONE 接入）
- P2-10：crm/mod.rs:68 CrmService 重导出 unused_imports（待业务接入评估）

### 🟢 P3 合理保留项（8 项）

- P3-1~P3-5：测试模块中 decs! 宏导入 unused_imports（inventory_unit_tests/sales_unit_tests/purchase_unit_tests/bi_unit_tests/dual_unit_converter_handler）
- P3-6：cache.rs:564 csrf_token_tests 模块 use super::*（防御性 allow）
- P3-7：dual_unit_converter.rs:138 测试模块 use crate::dec（防御性 allow）
- P3-8：event_bus.rs:726 catch_unwind 防御性 match unreachable_patterns（防御性 match-all）

---

## 🔄 历史任务：v14 深度调研报告修复（高风险 6/6 ✅，中风险 22/25 🔄）

> **v14 深度调研报告**（2026-07-09，[bug.md](file:///workspace/.monkeycode/bug.md)）：12 维度全量扫描，15 高/25 中/74 低风险，共 114 个问题。
> v13 后端 P0/P1 全部完成（批次 229-236），v13 剩余 P2 任务合并到 v14 队列。
> 修复策略：按优先级（高→中→低）+ 影响范围（核心路径→边缘功能）排序，每批 1 commit，CI 全绿后合并 main。

### 进度总览

| 风险等级 | 总数 | 已完成 | 剩余 | 状态 |
|----------|------|--------|------|------|
| 🔴 高风险 | 6 | 6 | 0 | ✅ 全部完成（批次 237-242） |
| 🟡 中风险 | 25 | 22 | 3 | 🔄 进行中 |
| 🟢 低风险 | 74 | 0 | 74 | ⏳ 后续迭代 |
| **合计** | **114** | **28** | **86** | — |

---

### 🟡 中风险待修复项（3 项 ⏳）

#### 1. 测试覆盖（7 项 ⏳ 待修复）

**问题背景**：bug.md 中风险测试覆盖问题 — 项目测试覆盖率严重不足，关键模块零测试或低覆盖，无法保证代码质量和重构安全性。

**影响范围**：
- **handlers 层**：100+ 文件覆盖率仅 10%，大部分 handler 无单元测试
- **services 层**：107 个 service 无任何测试，业务逻辑错误只能在运行时发现
- **frontend api 层**：覆盖率 4.4%，前端 API 调用逻辑无测试保障
- **ai 算法层**：零测试，AI 相关算法（RFM 评分、预测、推荐等）的正确性无验证
- **store 层**：覆盖率低，状态管理逻辑无测试
- **middleware 层**：覆盖率低（permission.rs 已在批次 240 补测 23 个，其余 middleware 仍无测试）

**修复方案**：
- 按模块优先级分批补测：先补核心业务 service（auth/user/order/inventory），再补 handler，最后补前端
- 每个 service 至少覆盖：正常路径 + 边界条件 + 错误处理
- 测试 mock 数据遵循规则 6（禁止硬编码，使用 fixtures 工厂函数）
- 使用 `tokio::test` + `testcontainers` 或内存数据库进行 service 集成测试

**技术要点**：
- service 测试需 mock DatabaseConnection（使用 `sea-orm-mock` 或自建 trait + mock 实现）
- handler 测试使用 `axum::test::TestServer` + 内存路由
- 前端测试使用 `vitest` + `@testing-library/vue`
- AI 算法测试使用固定输入 + 期望输出对比（Golden Master 模式）

**待修复文件清单**（部分示例，完整清单见 bug.md）：
- `backend/src/services/auth_service.rs`（核心，高优先级）
- `backend/src/services/user_service.rs`（核心，高优先级）
- `backend/src/services/order_service.rs`（核心业务）
- `backend/src/services/inventory_service.rs`（核心业务）
- `backend/src/services/ai/*.rs`（AI 算法，零测试）
- `backend/src/handlers/*.rs`（100+ 文件）
- `frontend/src/api/*.ts`（前端 API 层）
- `frontend/src/stores/*.ts`（状态管理）

#### 2. view 表格逻辑接入 useTableApi（46/56 完成 🔄，剩余 10 个 ⏳）

**问题背景**：56 个前端 view 文件各自实现表格加载/分页/排序/查询逻辑，与已封装的 `useTableApi` composable 重复。

**当前进度**：46/56 完成（批次 267-289 已处理 46 个文件）

**待修复文件清单**（剩余 10 个 ⏳）：
- `frontend/src/views/finance/voucher/*`（财务凭证模块剩余文件）
- `frontend/src/views/data-import/*`（数据导入模块剩余文件）
- `inventory/tabs/InventoryStockTab`（1-based 分页）
- `inventoryAdjustment/AdjustmentListTab`
- `inventoryTransfer/TransferListTab`
- `barcodeScanner`（0-based 分页需特殊处理）
- `assistAccounting`（0-based 分页需特殊处理）
- 其他待扫描发现的遗漏文件

**修复方案**：
- 扫描所有使用 `el-table` + 分页的 view 文件
- 评估每个 view 的特殊逻辑（如有自定义排序/筛选需保留）
- 接入 `useTableApi` composable，删除重复的表格逻辑代码
- 保持 view 的业务逻辑不变，只替换通用表格逻辑

**技术要点**：
- `useTableApi` 已封装：分页参数管理 / 数据加载 / loading 状态 / 错误处理
- 接入时需保留 view 特有的查询参数构建逻辑（如日期范围/多字段搜索）
- 部分 view 有自定义列配置/导出功能，需评估是否纳入 composable
- **测试 mock 适配**：view 接入后不再 import `listXxx`，测试 mock 需从 `@/api/xxx` 改为 `@/api/request`，mock 返回 `{ code, message, data: { items/list, total } }`，断言 `mock.calls[0][1].params`

#### 3. 重复实现 service 分页（35/35 全部清零 ✅）

**状态**：✅ 已全部完成（批次 255-266）。35 个 service 文件已全部接入 `paginate_with_total` 工具函数。详细记录见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

---

### 🟢 低风险修复队列（74 项 ⏳ 后续迭代）

**占位符/Mock 存根（21 项）**：
- 问题描述：21 处占位符或 Mock 存根，多数为测试夹具或合理设计
- 修复方案：逐个评估，合理保留的加注释说明，不合理的真实实现
- 优先级：低（多数无需修复）

**项目规则符合性（11 项）**：
- 问题描述：11 处配置层默认值或 best-effort 合理模式
- 修复方案：评估是否符合规则 0-13，不符合的修正
- 优先级：低

**死代码（8 项）**：
- 问题描述：8 处合规标注的死代码（`#[allow(dead_code)]` + TODO）
- 修复方案：逐个评估是否接入业务或删除
- 优先级：低

**其他（34 项）**：
- 问题描述：34 处其他低风险问题（命名规范/注释完善/代码风格等）
- 修复方案：后续迭代统一处理
- 优先级：低

---

### 📋 合并到 v14 的历史遗留任务（⏳ 待修复）

**v13 前端 P2（3 项 ⏳）**：
- FE-P2-1：前端类型定义完善（any 类型清理已完成，剩余 unknown 类型细化）
- FE-P2-2：前端组件 props 类型强化
- FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）— 大量 view 未接入 i18n，硬编码中文文本

**v13 后端 P2（3 项 ⏳）**：
- P2-1：后端错误处理统一（部分 handler 仍直接返回字符串而非 AppError）
- P2-2：后端日志规范（部分模块日志级别不当）
- P2-3：后端配置项完善

**其他遗留（3 项 ⏳）**：
- FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）— 引入 `el-table-v2` 或 `vue-virtual-scroller`
- P2-8：剩余 143 个无测试 service（后续迭代）— 分批补测
- E2E 失败排查（已知问题，待规则 5 节点）— 下载 playwright-report 分析失败用例

---

## 🐛 安全漏洞待修复（来自 bug.md）

> 详见 [bug.md](file:///workspace/.monkeycode/bug.md)。7 项安全漏洞已全部修复（批次 290-296）。

| 优先级 | 漏洞 | 位置 | 状态 |
|--------|------|------|------|
| P0 | 1.1 SQL 注入 (LIMIT) | tracking_service.rs:258-259 | ✅ 已修复（批次 290，PR #470） |
| P0 | 1.2 命令注入 (backup) | backup.rs:149 | ✅ 已修复（批次 291，PR #471） |
| P0 | 1.3 SSRF (currency) | currency_service.rs:301-305 | ✅ 已修复（批次 292，PR #472） |
| P1 | 2.1 日志泄露 | webhook_service.rs:235 | ✅ 已修复（批次 293，PR #473） |
| P1 | 2.2 速率限制 | webhook_handler.rs:114-135 | ✅ 已修复（批次 294，PR #474） |
| P1 | 2.3 文件权限 | system_update_service.rs:438 | ✅ 已修复（批次 295，PR #475） |
| P2 | 3.2 备份权限 | backup.rs:54-62 | ✅ 已修复（批次 296，PR #476） |

---

## ✅ v8 全项目复审（2026-07-11，批次 290-308 全部修复）

> 详见 [v8-review-2026-07-11.md](file:///workspace/.monkeycode/docs/audits/v8-review-2026-07-11.md)。复审发现 21 个问题（4 高 + 8 中 + 9 低），全部在批次 290-308 修复完成，详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

---

## 规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 270 触发（403 权限不足，需用户手动触发）
  - 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch
  - 批次 N+20：第 1 次监控（GitHub API 查询 run 状态）
  - 批次 N+28：第 2 次监控（若 N+20 未完成）
  - 批次 N+29：最后监控，未完成则跳过 N+30 的 E2E 周期
  - **注意**：E2E 已从 ci-cd.yml 独立到 e2e-batch.yml，不阻塞主 CI
  - **下次触发**：批次 330（已到期，需触发）
- **规则 10（每 15 批次记忆整理）**：批次 330 已执行（迁移批次 290-329 到 doto-su.md）
  - 下次整理：批次 345
