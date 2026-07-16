# V15 业务主体维度审计报告（类十五·批次 13）

- **审计子代理**：V15 审计子代理（类十五业务主体维度审计与数据流转）
- **审计范围**：15 维度（15.1 供货商主数据完整性 ~ 15.15 数据流转审计与异常检测）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 5597-5995 行（类十五详细检查要点）
  - `/workspace/backend/src/models/` 主数据模型
  - `/workspace/backend/src/services/` 主数据服务
  - `/workspace/backend/src/handlers/` 主数据 handler
  - `/workspace/backend/src/routes/` 路由
  - `/workspace/backend/migrations/` migration 文件
- **审计方法**：Read 审计计划 + Grep 检索 + Read 关键文件 + 对照审计计划核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 重要说明：审计计划与实际代码的差异

在执行审计过程中，发现 V15 审计计划类十五中多处描述与实际代码状态**严重不一致**。本报告以**实际代码扫描结果为准**，对审计计划中的失实描述进行标注。主要差异：

1. **15.4 加工商维度**：审计计划称"完全未实现"，但实际 `outsourcing` 模块在 v14 批次 430 已完整实现（含主表/明细/收回入库/凭证/Service/Handler/路由/状态机/损耗处理/三步分录）。
2. **15.5 加工商业务流程闭环**：审计计划称"0/8 打通"，实际 7/8 已打通。
3. **15.6 销售预测**：审计计划称"未实现"，实际 `backend/src/services/ai/pred.rs` 已实现（WMA + 指数平滑 + 季节性因子组合算法）。
4. **15.13 business_traces 写入**：审计计划称"模型存在但无写入"，实际 `business_trace_snapshot` 表有 `create_snapshot` 写入接口，仅 `business_trace_chain` 表无写入。
5. **15.15 主动异常检测引擎**：审计计划称"未实现"，实际 `backend/src/services/ai/detect.rs` 已实现销售异常（Z-score）+ 库存异常（IQR）检测。

---

## 维度 15.1：供货商主数据完整性审计

### 检查方法
- Grep `category_id` in `backend/src/models/supplier.rs`
- Glob `backend/migrations/` 检查 supplier_eval 相关 migration
- Grep `update_supplier_qualification|delete_supplier_qualification` in `backend/src/services/supplier_service.rs`
- Read `backend/src/models/supplier.rs`、`backend/src/models/supplier_evaluation.rs`、`backend/src/models/supplier_evaluation_record.rs`
- Read `backend/migrations/20260528000001_add_crm_supplier_tables/up.sql`

### 发现

#### ✅ 已落实的项
1. **suppliers 主表字段完整**：`backend/src/models/supplier.rs:10-87`，含 25+ 字段（supplier_code/supplier_name/supplier_short_name/supplier_type/credit_code/registered_address/business_address/legal_representative/registered_capital/establishment_date/business_term/business_scope/taxpayer_type/bank_name/bank_account/contact_phone/fax/website/email/main_business/main_market/employee_count/annual_revenue/grade/grade_score/last_evaluation_date/status/is_enabled/assist_batch/assist_supplier/created_at/updated_at/created_by/updated_by/remarks）。
2. **supplier_qualifications 资质表存在**：`backend/src/models/supplier_qualification.rs` + migration `backend/migrations/20260528000001_add_crm_supplier_tables/up.sql:150-170`（含 qualification_name/qualification_type/qualification_no/issuing_authority/issue_date/valid_until/attachment_path/need_annual_check/annual_check_record/is_expired）。
3. **supplier_grades 等级表存在**：`backend/src/models/supplier_grade.rs`。
4. **supplier_contacts 联系人表存在**：`backend/src/models/supplier_contact.rs` + migration `:126-145`（含 is_primary 唯一性）。
5. **supplier_blacklists 黑名单表存在**：`backend/src/models/supplier_blacklist.rs`。
6. **supplier_categories 分类表存在**：`backend/src/models/supplier_category.rs`。
7. **product_supplier_mappings 产品-供应商映射表存在**：`backend/src/models/product_supplier_mapping.rs`（含 supplier_price/min_order_quantity/lead_time/is_primary/priority）。
8. **purchase_prices 采购价格表存在**：`backend/src/models/purchase_price.rs`（含 effective_date/expiry_date/status/approved_by）。
9. **SupplierService 联系人 CRUD 完整**：`backend/src/services/supplier_service.rs:457-650`（list/create/update/delete supplier_contact）。
10. **SupplierService 资质 list/create**：`backend/src/services/supplier_service.rs:651-680`。

#### ❌ 缺陷项 1：suppliers 主表无 category_id 外键字段
**风险等级：P2**
**证据**：
- `backend/src/models/supplier.rs:10-87` 完整字段列表中**无 category_id 字段**
- Grep `category_id` in `backend/src/models/supplier.rs` 返回 `No matches found`
- 但 `backend/src/models/supplier_category.rs` 分类表存在

**业务影响**：`supplier_categories` 分类表形同虚设，无法将供应商归类（如染料/助剂/坯布供应商分类），分类功能未落地。
**修复建议**：suppliers 主表新增 `category_id INTEGER REFERENCES supplier_categories(id)` 字段，SupplierService 在 create/update 时支持 category_id 设置，list 支持按分类筛选。

#### ❌ 缺陷项 2：SupplierService 资质无 update/delete 方法
**风险等级：P2**
**证据**：
- `backend/src/services/supplier_service.rs:651-680` 仅有 `list_supplier_qualifications` 和 `create_supplier_qualification`
- Grep `update_supplier_qualification|delete_supplier_qualification` 返回 `No matches found`

**业务影响**：供应商资质信息无法修改或删除，资质过期后无法更新续期记录，影响供应商资质管理闭环。
**修复建议**：SupplierService 补齐 `update_supplier_qualification` 和 `delete_supplier_qualification` 方法，并增加资质过期自动告警 cron。

#### ❌ 缺陷项 3：supplier_evaluation_records 表无 migration
**风险等级：P1**
**证据**：
- `backend/src/models/supplier_evaluation_record.rs:12` `table_name = "supplier_evaluation_records"`
- 但 migration `backend/migrations/20260528000001_add_crm_supplier_tables/up.sql` 中**仅创建 supplier_evaluation_indicators 表**（:197-211），**无 supplier_evaluations 和 supplier_evaluation_records 表的 CREATE TABLE 语句**
- Grep `supplier_eval` in migrations 目录找到 4 个文件，但均无 `supplier_evaluation_records` 建表语句

**业务影响**：model 层 `supplier_evaluation_record.rs` 对应的表在数据库中实际不存在，运行时查询会报错，供应商评估记录功能不可用。
**修复建议**：补齐 sea-orm migration 文件，创建 `supplier_evaluations` 和 `supplier_evaluation_records` 表。

#### ⚠️ 部分缺陷项 4：supplier_evaluation.rs model 命名不一致
**风险等级：P3**
**证据**：
- `backend/src/models/supplier_evaluation.rs:10` `table_name = "supplier_evaluation_indicators"`（model 文件名为 `supplier_evaluation`，但实际表名为 `supplier_evaluation_indicators`）
- 文件名与表名不匹配，容易误导开发者

**业务影响**：命名不一致增加维护成本，开发者可能误以为该 model 对应 `supplier_evaluations` 表。
**修复建议**：将 `supplier_evaluation.rs` 重命名为 `supplier_evaluation_indicator.rs`，或在文件头注释中明确说明。

---

## 维度 15.2：供货商业务闭环审计

### 检查方法
- Read `backend/src/services/supplier_service.rs`（CRUD 方法）
- Read `backend/src/services/supplier_evaluation_service.rs`（评估服务）
- Grep `supplier_id` in `backend/src/models/purchase_order.rs`、`backend/src/models/ap_invoice.rs`、`backend/src/models/ap_reconciliation.rs`
- Grep `cron|scheduler|auto_evaluate` in supplier 相关 service
- Grep `supplier.*balance|supplier.*history|supplier.*import|supplier.*export` in services

### 发现

#### ✅ 已落实的项
1. **供应商创建（含联系人/资质事务性创建）**：`backend/src/services/supplier_service.rs:47-151` `create_supplier`。
2. **供应商更新 + 事件发布 + 审计日志**：`backend/src/services/supplier_service.rs:238-352` `update_supplier`，发布 `SupplierUpdated` 事件。
3. **供应商删除（前置校验 + lock_exclusive + 事务）**：`backend/src/services/supplier_service.rs:356-422` `delete_supplier` + `can_delete_supplier`。
4. **采购订单关联 supplier_id**：`backend/src/models/purchase_order.rs:31` `pub supplier_id: i32`。
5. **应付账款关联（ap_invoice）**：`backend/src/models/ap_invoice.rs:36` `pub supplier_id: i32`。
6. **供应商对账单（ap_reconciliation）**：`backend/src/models/ap_reconciliation.rs:17-50`，含 supplier_id/reconciliation_no/start_date/end_date/opening_balance/total_invoice/total_payment/closing_balance。
7. **供应商等级评估（加权得分 + A/B/C/D 评级）**：`backend/src/services/supplier_evaluation_service.rs` 实现 `create_evaluation_record`/`get_supplier_score`/`list_ratings`/`get_supplier_rankings` 等方法。
8. **供应商状态切换**：`backend/src/services/supplier_service.rs:425-455` `toggle_supplier_status`。

#### ❌ 缺陷项 1：供应商评估自动触发（季度/年度）未实现
**风险等级：P2**
**证据**：
- Grep `supplier_evaluation.*cron|cron.*supplier_eval|auto.*supplier_eval` in `backend/src` 返回 `No matches found`
- `backend/src/services/supplier_evaluation_service.rs` 中 `create_evaluation_record` 需手工调用

**业务影响**：供应商评估依赖人工触发，易遗漏定期评估，导致供应商等级 outdated，影响采购决策。
**修复建议**：引入 tokio-cron-scheduler，按季度自动触发供应商评估任务，调用 `create_evaluation_record` 并刷新 grade。

#### ❌ 缺陷项 2：供应商账户余额管理未实现
**风险等级：P2**
**证据**：
- Grep `supplier.*balance` in `backend/src/services` 返回 `No matches found`
- account_balance 表按科目+期间维度，非供应商维度

**业务影响**：无法按供应商维度查询应付余额，对账困难。
**修复建议**：扩展供应商维度余额查询 Service，基于 ap_invoice + ap_payment 聚合计算供应商应付余额。

#### ❌ 缺陷项 3：供应商供货历史查询未实现
**风险等级：P3**
**证据**：
- Grep `supplier.*history` in `backend/src/services` 返回 `No matches found`
- 无独立供货历史表/Service

**业务影响**：无法快速查询某供应商的历史供货记录（采购单/入库/退货/质检），供应商绩效分析困难。
**修复建议**：通过 purchase_orders + purchase_receipts 联表查询 Service，提供供应商供货历史 API。

#### ❌ 缺陷项 4：供应商价格清单导入未实现
**风险等级：P3**
**证据**：
- Grep `supplier.*import|supplier.*export|supplier.*batch_enable` in `backend/src/services` 返回 `No matches found`
- 无 import_export 逻辑

**业务影响**：供应商价格清单需逐条手工录入，批量导入缺失，效率低。
**修复建议**：补批量导入 Excel 接口，支持 product_supplier_mappings + purchase_prices 批量导入。

---

## 维度 15.3：供货商面料行业特性审计

### 检查方法
- Read `backend/src/models/supplier.rs` 检查 supplier_type 字段
- Grep `dyeing_capacity|printing_capacity|color_card_capability|is_processor|processor_type` in `backend/src/models/supplier.rs`
- Read `backend/src/models/product_supplier_mapping.rs` 检查交期/起订量

### 发现

#### ✅ 已落实的项
1. **supplier_type 字段区分染料/助剂/坯布供应商**：`backend/src/models/supplier.rs:21` `pub supplier_type: String`（含 fabric/dye/auxiliary/logistics/service/other）。
2. **供应商质量认证（通用资质表可覆盖）**：`backend/src/models/supplier_qualification.rs` qualification_type 可扩展（ISO9001/Bluesign/Oeko-Tex 等）。
3. **供应商交期/起订量管理**：`backend/src/models/product_supplier_mapping.rs` 含 `min_order_quantity` 和 `lead_time` 字段。

#### ❌ 缺陷项 1：供应商色卡能力字段缺失
**风险等级：P3**
**证据**：
- Grep `color_card_capability` in `backend/src/models/supplier.rs` 返回 `No matches found`
- suppliers 主表无色卡能力字段

**业务影响**：染料供应商的色卡能力（能否提供色卡样、色差等级）无法记录，影响染料供应商选型。
**修复建议**：suppliers 主表或扩展表新增 `color_card_capability` 字段（JSON 或文本，描述色卡能力）。

#### ❌ 缺陷项 2：供应商染色能力字段缺失
**风险等级：P3**
**证据**：
- Grep `dyeing_capacity` in `backend/src/models/supplier.rs` 返回 `No matches found`

**业务影响**：委外染色供应商的染色能力（缸号容量/染色类型/最大布重）无法记录，影响委外染色订单分配。
**修复建议**：suppliers 主表或扩展表新增 `dyeing_capacity` 字段。

#### ❌ 缺陷项 3：供应商印花能力字段缺失
**风险等级：P3**
**证据**：
- Grep `printing_capacity` in `backend/src/models/supplier.rs` 返回 `No matches found`

**业务影响**：印花供应商的印花能力（印花类型/最大门幅/套色数）无法记录，影响委外印花订单分配。
**修复建议**：suppliers 主表或扩展表新增 `printing_capacity` 字段。

---

## 维度 15.4：加工商（委外加工商）维度审计

### ⚠️ 审计计划失实声明

**审计计划描述**（第 5680 行）："加工商维度**完全未实现**，是面料行业核心业务流程的重大缺失。"

**实际代码扫描结果**：`outsourcing` 模块在 v14 批次 430 已**完整实现**。审计计划描述严重失实。

### 检查方法
- Grep `outsourc|processor|is_processor|委外|外协|外发` in `backend/src/`
- Read `backend/src/models/outsourcing_order.rs`、`backend/src/models/outsourcing_order_item.rs`、`backend/src/models/outsourcing_receipt.rs`、`backend/src/models/outsourcing_voucher.rs`
- Read `backend/src/services/outsourcing_service.rs`
- Grep `outsourc` in `backend/src/routes/production.rs`

### 发现

#### ✅ 已落实的项（与审计计划描述相反）
1. **委外加工订单主表（outsourcing_order）已实现**：`backend/src/models/outsourcing_order.rs:20-100`，完整字段包括：
   - order_no（唯一）、order_type（dyeing/printing/weaving/finishing/other）
   - supplier_id（外键→suppliers，委外加工厂关联）
   - production_order_id（关联生产订单）、dye_batch_id（关联缸号）
   - color_no、dye_lot_no（面料行业追溯）
   - issue_date/expected_return_date/actual_return_date
   - issue_quantity/issue_unit/return_quantity/loss_quantity/loss_type/loss_rate/standard_loss_rate
   - material_cost/processing_fee/freight_fee/tax_amount/abnormal_loss_amount/total_cost/unit_cost
   - status（draft/issued/processing/received/settled/closed/cancelled）
   - voucher_no_issue/voucher_no_fee/voucher_no_receipt（三步凭证号）
2. **委外加工明细表（outsourcing_order_item）已实现**：`backend/src/models/outsourcing_order_item.rs`。
3. **委外收回入库表（outsourcing_receipt）已实现**：`backend/src/models/outsourcing_receipt.rs`，含收回数量/损耗/批号/质检结果。
4. **委外会计分录凭证表（outsourcing_voucher）已实现**：`backend/src/models/outsourcing_voucher.rs`，支持 issue/fee/receipt/loss 四类凭证。
5. **委外加工 Service 已实现**：`backend/src/services/outsourcing_service.rs`，含：
   - 损耗率计算纯函数 `compute_loss_rate`（:60-65）
   - 总成本计算 `compute_total_cost`（:72-79）
   - 单位成本计算 `compute_unit_cost`（:86-91）
   - 标准损耗率（dyeing=0.05/weaving=0.035/printing=0.05/finishing=0.03）
   - 委外订单 CRUD + 状态机 + 取消
   - 委外发料明细 CRUD
   - 委外收回入库单 CRUD + 状态机 + 损耗分类
   - 委外会计分录凭证 CRUD + 过账
6. **委外加工 Handler 已实现**：`backend/src/handlers/outsourcing_handler.rs`。
7. **委外加工路由已挂载**：`backend/src/routes/production.rs:375-421`，30+ 路由（CRUD + 状态流转 + 明细 + 收回 + 凭证）。
8. **损耗处理完整**：normal/abnormal 分类，标准损耗率，损耗率计算，非正常损耗计入营业外支出。
9. **三步分录完整**：发料（借 委托加工物资 / 贷 自制半成品）+ 加工费（借 委托加工物资+进项税 / 贷 银行存款）+ 入库（借 库存商品 / 贷 委托加工物资）。

#### ❌ 缺陷项 1：suppliers 表无 is_processor 标志
**风险等级：P2**
**证据**：
- Grep `is_processor` in `backend/src/models/supplier.rs` 返回 `No matches found`
- suppliers 表无 `is_processor` 布尔字段，无法区分供应商与加工商

**业务影响**：无法在主数据层面区分"供应商"与"加工商"，委外订单的 supplier_id 实际指向 suppliers 表，但无法筛选出"仅加工商"列表。
**修复建议**：suppliers 表新增 `is_processor BOOLEAN NOT NULL DEFAULT FALSE` + `processor_type VARCHAR(20)`（dyeing/printing/finishing）字段。

#### ❌ 缺陷项 2：suppliers 表无染色/印花能力字段
**风险等级：P3**
**证据**：
- Grep `dyeing_capacity|printing_capacity` in `backend/src/models/supplier.rs` 返回 `No matches found`

**业务影响**：无法记录加工商的染色能力（缸号容量/染色类型/最大布重）和印花能力（印花类型/最大门幅/套色数），影响委外订单分配决策。
**修复建议**：suppliers 表新增 `dyeing_capacity` 和 `printing_capacity` 字段。

#### ❌ 缺陷项 3：前端加工商管理界面未确认
**风险等级：P3**
**证据**：
- 本次审计未扫描前端代码，需前端审计子代理确认

**业务影响**：后端 API 已完整，但前端是否有对应的加工商管理界面需确认。
**修复建议**：前端审计子代理扫描 `frontend/src/` 下 outsourcing 相关页面。

---

## 维度 15.5：加工商业务流程闭环审计

### ⚠️ 审计计划失实声明

**审计计划描述**（第 5779 行）："业务流程 0/8 打通（0%），完全空白。"

**实际代码扫描结果**：7/8 已打通（87.5%）。审计计划描述严重失实。

### 检查方法
- Read `backend/src/services/outsourcing_service.rs` 完整方法列表
- Grep `outsourc` in `backend/src/routes/production.rs` 检查路由
- Read `backend/src/models/outsourcing_order.rs` 检查状态机字段

### 发现

#### ✅ 已落实的项
1. **外发染整/印花/整理**：`outsourcing_orders` 创建 + 外发记录，`backend/src/services/outsourcing_service.rs` 实现 create_outsourcing_order + issue_outsourcing_order。
2. **加工费核算**：自动核算（数量×单价×加成）+ 关联成本归集，`outsourcing_service.rs:72-79` `compute_total_cost` 含 processing_fee。
3. **收回入库**：`outsourcing_receipts` + 库存回写 + 四维索引，`backend/src/models/outsourcing_receipt.rs` + `confirm_outsourcing_receipt` 路由。
4. **损耗处理**：实际损耗率 vs 标准损耗率，`outsourcing_service.rs:60-65` `compute_loss_rate` + `standard_loss_rate` 字段。
5. **加工费付款**：`outsourcing_voucher` 凭证管理，`post_outsourcing_voucher` 路由，关联会计凭证。
6. **委外进度跟踪**：状态机 draft→issued→processing→received→settled→closed，`outsourcing_service.rs` 实现 issue/processing/settle/close/cancel 状态流转。
7. **缸号与委外加工单关联**：`outsourcing_orders.dye_batch_id` + `dye_lot_no` 字段，`backend/src/models/outsourcing_order.rs:35-39`。

#### ❌ 缺陷项 1：委外加工报表未实现
**风险等级：P3**
**证据**：
- Grep `outsourc.*report|outsourc.*ranking|outsourc.*statistic` in `backend/src/services` 返回 `No matches found`
- 无加工商排名/加工费统计/损耗率分析报表

**业务影响**：无法分析加工商绩效、加工费趋势、损耗率异常，影响加工商管理决策。
**修复建议**：新增委外加工报表 Service，提供加工商排名/加工费统计/损耗率分析 API。

---

## 维度 15.6：销售订单数据模型与状态机审计

### 检查方法
- Read `backend/src/models/sales_order.rs`、`backend/src/models/sales_order_item.rs`、`backend/src/models/sales_contract.rs`
- Grep `sales_contract_item|SalesContractItem` in `backend/src/`
- Grep `so_status` in `backend/src/services/so/order_workflow.rs`
- Grep `sales_forecast|SalesForecast` in `backend/src/`
- Read `backend/src/services/ai/pred.rs`

### 发现

#### ✅ 已落实的项
1. **sales_orders 主表字段完整**：`backend/src/models/sales_order.rs`，含 order_no/customer_id/order_date/required_date/ship_date/status/subtotal/tax_amount/discount_amount/shipping_cost/total_amount/paid_amount/balance_amount/shipping_address/billing_address/notes/approved_by/approved_at。
2. **sales_order_item 明细字段完整（含面料行业字段）**：`backend/src/models/sales_order_item.rs:10-47`，含 color_no/color_name/pantone_code/grade_required/quantity_meters/quantity_kg/gram_weight/width/batch_requirement/dye_lot_requirement/base_price/color_extra_cost/grade_price_diff/final_price/shipped_quantity_meters/shipped_quantity_kg/paper_tube_weight/is_net_weight。
3. **状态机 8 态完整**：`backend/src/services/so/order_workflow.rs:508-515`，draft/pending/approved/partial_shipped/shipped/completed/cancelled/rejected，含状态门断言测试。
4. **销售报价单（sales_quotations）完整**：含 currency/exchange_rate/price_terms/incoterms/tax_inclusive/tax_rate/moq/lead_time/customer_level/approval_instance_id/converted_sales_order_id。
5. **销售退货（sales_return）完整**：状态机 DRAFT/SUBMITTED/APPROVED/REJECTED/COMPLETED + 关联 Customer/Warehouse/SalesOrder/Items。
6. **销售预测已实现**（审计计划说"未实现"是失实的）：`backend/src/services/ai/pred.rs:1-50`，实现：
   - 移动平均（WMA, 加权 7 日）
   - 指数平滑（Holt 线性趋势）
   - 季节性因子（按月聚合）
   - 数据不足时的降级预测（fallback_forecast）
   - 算法组合：60% 指数平滑 + 40% 加权移动平均，再乘以季节性因子

#### ❌ 缺陷项 1：销售合同（sales_contracts）仅主表，缺明细行表
**风险等级：P2**
**证据**：
- `backend/src/models/sales_contract.rs:10-31` 仅有主表字段（contract_no/contract_name/contract_type/customer_id/customer_name/total_amount/signed_date/effective_date/expiry_date/payment_terms/payment_method/delivery_date/delivery_location/status）
- Grep `sales_contract_item|SalesContractItem` in `backend/src/` 返回 `No matches found`

**业务影响**：合同应有明细行（合同商品/数量/价格/交期），当前仅主表字段，无法记录合同具体商品明细，合同与订单明细无法关联。
**修复建议**：新增 `sales_contract_items` 表（contract_id/product_id/quantity/unit_price/delivery_date 等），并补 Service/Handler/路由。

---

## 维度 15.7：销售业务流程闭环审计

### 检查方法
- Grep `pub async fn|pub fn` in `backend/src/services/quotation_convert_service.rs`
- Grep `check_credit_available_txn|lock_exclusive` in `backend/src/services/customer_credit_limit.rs`
- Grep `pub async fn|红字|credit_memo` in `backend/src/services/sales_return_service.rs`
- Grep `pub async fn` in `backend/src/services/sales_analysis_service.rs`

### 发现

#### ✅ 已落实的项（12/12 完整）
1. **报价 → 订单转换**：`backend/src/services/quotation_convert_service.rs:47` `convert` 方法，更新报价单状态为 converted，记录 converted_sales_order_id。
2. **订单提交 → 信用检查（事务内 TOCTOU 防护）**：`backend/src/services/so/order_workflow.rs:88-221`，调用 `check_credit_available_txn` 事务内查询。
3. **订单审批 → BPM 审批 + MRP 触发**：`backend/src/services/so/order_workflow.rs:224-345`。
4. **库存预留（approve 后）**：`backend/src/services/so/order_workflow.rs:282-313`。
5. **发货 → 库存扣减 + 防御性 WHERE + 双单位换算**：`backend/src/services/so/delivery.rs:110-514`。
6. **发货 → 生成应收单（AR）**：`backend/src/services/so/delivery.rs:378-400`。
7. **发货 → 生成收入凭证**：`backend/src/services/so/delivery.rs:410-495`。
8. **退货 → 回写库存（四维索引）+ 红字应收单**：`backend/src/services/sales_return_service.rs:339-378` `approve_return` + `:586-620` `generate_red_ar_txn`，使用 `create_credit_memo` 支持负金额 + 外部事务 + 幂等检查。
9. **取消发货 → 对称恢复库存 + 预留恢复**：`backend/src/services/so/delivery.rs:916-1112`。
10. **BPM 审批失败补偿回滚**：`backend/src/services/so/order_workflow.rs:186-209`，开启新事务回滚订单状态为 draft。
11. **销售统计/排名**：`backend/src/services/sales_analysis_service.rs` 实现 `get_overview_stats`/`product_ranking`/`customer_ranking`/`get_trends`/`get_rankings`。
12. **CSV 导出**：`backend/src/services/so/delivery.rs:1117-1228` `export_orders_to_csv`。

#### ❌ 缺陷项
无缺陷，12/12 完整（100%）。

---

## 维度 15.8：销售面料行业特性审计

### 检查方法
- Grep `validate_dye_lot_consistency|DualUnitConverter` in `backend/src/services/so/delivery.rs`
- Read `backend/src/models/sales_order_item.rs` 检查面料行业字段
- Grep `bulk_color_approval|批色` in `backend/src/`

### 发现

#### ✅ 已落实的项
1. **缸号同订单校验（防混缸色差）**：`backend/src/services/so/delivery.rs:68-97` `validate_dye_lot_consistency`。
2. **双单位（米/公斤）换算**：`backend/src/services/so/delivery.rs:262-298` 使用 `DualUnitConverter`。
3. **等级价差/色差附加**：`backend/src/models/sales_order_item.rs:39-42` 含 `base_price`/`color_extra_cost`/`grade_price_diff`/`final_price`。
4. **纸管重量/净重标记**：`backend/src/models/sales_order_item.rs:45-46` 含 `paper_tube_weight`/`is_net_weight`。

#### ⚠️ 部分落实项 1：销售批色流程
**风险等级：P3**
**证据**：V15 类十一已规划 `bulk_color_approval` 表（交货前客户批色，剪大货样）。
**业务影响**：批色流程为规划中，当前未实现。
**修复建议**：按 V15 类十一规划落地。

#### ⚠️ 部分落实项 2：按缸号发货/按匹号发货
**风险等级：P3**
**证据**：发货按 `sales_order_item` 级别，未细化到匹号（batch_no）级别。
**业务影响**：面料行业实际按匹号发货，当前按明细行级别无法精确追踪每匹布的发货。
**修复建议**：后续优化，新增发货明细行级别的 batch_no 字段。

---

## 维度 15.9：客户主数据完整性审计

### 检查方法
- Read `backend/src/models/customer.rs`
- Grep `customer_address|customer_bank_account|CustomerAddress|CustomerBankAccount` in `backend/src/models/`
- Grep `customer_color_price|customer_contact` in `backend/src/models/`
- Grep `customer.*tags|/customers/:id/tags` in `backend/src/`

### 发现

#### ✅ 已落实的项
1. **customers 主表字段完整**：`backend/src/models/customer.rs:10-96`，含 customer_code/customer_name/contact_person/contact_phone/contact_email/address/city/province/country/postal_code/credit_limit/payment_terms/tax_id/bank_name/bank_account/status/customer_type/notes/customer_industry/main_products/annual_purchase/quality_requirement/inspection_standard。
2. **客户分类（customer_type）**：`backend/src/models/customer.rs:67` `pub customer_type: String`（retail/wholesale/vip/distributor/manufacturer/other）。
3. **客户信用评级表（customer_credit_ratings）**：`backend/migrations/20260528000001_add_crm_supplier_tables/up.sql:100-121`，含 credit_level/credit_score/credit_limit/used_credit/available_credit/credit_days/last_assessment_date/next_assessment_date。
4. **客户联系人表（customer_contacts）**：`backend/src/models/customer_contact.rs`，含 is_primary 唯一性保证。
5. **客户专属色卡价格表（customer_color_prices）**：`backend/src/models/customer_color_price.rs:13-30`，含 special_price/discount_percent/currency/valid_from/valid_until/approved_by/approved_at。
6. **客户行业/主营产品/年采购额**：`backend/src/models/customer.rs:81-89` 含 `customer_industry`/`main_products`/`annual_purchase`。
7. **客户质量要求/检验标准**：`backend/src/models/customer.rs:91-95` 含 `quality_requirement`/`inspection_standard`。
8. **客户标签（tags）**：`backend/src/routes/crm.rs` + `backend/src/models/crm_tag.rs` + `backend/src/handlers/crm_customer_handler.rs`，支持 `/customers/:id/tags` 路由。

#### ❌ 缺陷项 1：客户多地址表未实现
**风险等级：P3**
**证据**：
- Grep `customer_address|CustomerAddress` in `backend/src/models/` 返回 `No matches found`
- customers 表仅有单地址（address 字段）

**业务影响**：大型客户可能有多个收货地址，当前仅单地址无法满足。
**修复建议**：新增 `customer_addresses` 表（customer_id/address_type/address/contact_person/phone/is_default），低优先级。

#### ❌ 缺陷项 2：客户多银行账户表未实现
**风险等级：P3**
**证据**：
- Grep `customer_bank_account|CustomerBankAccount` in `backend/src/models/` 返回 `No matches found`
- customers 表仅有单银行（bank_name/bank_account）

**业务影响**：大型客户可能有多银行账户，当前仅单银行无法满足。
**修复建议**：新增 `customer_bank_accounts` 表（customer_id/bank_name/bank_account/account_type/is_default），低优先级。

---

## 维度 15.10：客户信用与应收管理审计

### 检查方法
- Grep `pub async fn|pub fn` in `backend/src/services/customer_credit_limit.rs`
- Grep `check_credit_available_txn|check_credit_warning|lock_exclusive` in `customer_credit_limit.rs`
- Grep `cron|scheduler|tokio::spawn|interval` in `backend/src/services/customer_credit_evaluate.rs`
- Grep `sync_customer_to_es|CustomerUpdated` in `backend/src/services/customer_service.rs`
- Grep `create_receivable|export_pdf` in `backend/src/services/ar/inv.rs`

### 发现

#### ✅ 已落实的项
1. **信用额度设置（set_credit_rating）**：`backend/src/services/customer_credit_limit.rs:23-81`。
2. **信用占用（occupy_credit）**：`backend/src/services/customer_credit_limit.rs:84`，销售订单提交时占用。
3. **信用释放（release_credit）**：`backend/src/services/customer_credit_limit.rs:134`，订单取消时释放。
4. **信用额度调整（adjust_credit_limit）**：`backend/src/services/customer_credit_limit.rs:176`，decrease 不能低于已用。
5. **信用检查（事务内 TOCTOU 防护）**：`backend/src/services/customer_credit_limit.rs:257-273` `check_credit_available_txn`。
6. **信用预警（80%阈值）**：`backend/src/services/customer_credit_limit.rs:276-305` `check_credit_warning`，含测试（:615-770）。
7. **信用停用（有占用拒绝）**：`backend/src/services/customer_credit_limit.rs:312-340` `deactivate`，使用 `lock_exclusive` 串行化（:320）。
8. **信用评级评估算法（3 因子加权）**：`backend/src/services/customer_credit_evaluate.rs:13-80` `evaluate_credit`，含历史付款记录（30%）+ 合作时长（20%）+ 订单规模（25%）+ 更多因子。
9. **应收账款关联（销售→AR 链路）**：`backend/src/services/ar/inv.rs:120-195` `create_receivable`。
10. **对账单 PDF 导出**：`backend/src/services/ar/inv.rs:41-97` `export_pdf`。
11. **ES 同步最终一致性**：`backend/src/services/customer_service.rs:206` `sync_customer_to_es`，在 create/update/delete 时调用（:310/618/667），PG 提交后 sync，失败仅 warn。

#### ❌ 缺陷项 1：信用评级自动触发（定时调度）未实现
**风险等级：P2**
**证据**：
- Grep `cron|scheduler|tokio::spawn|interval` in `backend/src/services/customer_credit_evaluate.rs` 返回 `No matches found`
- `evaluate_credit` 方法存在但需手工调用

**业务影响**：客户信用评级依赖人工触发，易遗漏定期评估，导致信用额度 outdated，增加坏账风险。
**修复建议**：引入 tokio-cron-scheduler，按月度/季度自动触发 `evaluate_credit`，并刷新 customer_credit_ratings 表。

---

## 维度 15.11：客户面料行业特性审计

### 检查方法
- Read `backend/src/models/customer.rs` 检查 customer_type 字段
- Read `backend/src/models/customer_color_price.rs`
- Grep `customer_level` in sales_quotation 相关 model
- Grep `bulk_color_approval|color_card_distribute` in `backend/src/`

### 发现

#### ✅ 已落实的项
1. **客户分级（零售/批发/VIP）**：`backend/src/models/customer.rs:67` `customer_type: String`（retail/wholesale/vip）。
2. **客户专属色卡价格**：`backend/src/models/customer_color_price.rs:13` `table_name = "customer_color_prices"`，含 special_price/discount_percent/currency/valid_from/valid_until。
3. **报价单按客户等级定价**：sales_quotation 表含 `customer_level` 字段。
4. **客户行业/主营产品**：`backend/src/models/customer.rs:81-85` `customer_industry`/`main_products`。
5. **客户质量标准**：`backend/src/models/customer.rs:91-95` `quality_requirement`/`inspection_standard`。

#### ⚠️ 部分落实项 1：客户批色确认能力
**风险等级：P3**
**证据**：V15 类十一已规划 `bulk_color_approval` 表。
**修复建议**：按 V15 类十一规划落地。

#### ⚠️ 部分落实项 2：客户色卡档案
**风险等级：P3**
**证据**：V15 类十已规划 `color_card_distribute` 表。
**修复建议**：按 V15 类十规划落地。

#### ❌ 缺陷项 1：客户特殊工艺要求字段缺失
**风险等级：P3**
**证据**：
- Grep `special_process|craft_requirement|工艺要求` in `backend/src/models/customer.rs` 返回 `No matches found`
- 当前 `quality_requirement` 可部分覆盖，但无独立工艺要求字段

**业务影响**：面料行业客户可能有特殊工艺要求（如特殊整理/特殊印花），当前无独立字段记录。
**修复建议**：customers 表新增 `special_process_requirement` 字段，或扩展表，低优先级。

---

## 维度 15.12：跨模块数据流转审计

### 检查方法
- Read `backend/src/services/event_bus.rs:870-980` 检查 CustomerUpdated/SupplierUpdated/DyeBatchCompleted/QualityInspectionCompleted 监听器
- Grep `refresh_customer_name_redundancy|refresh_supplier_name_redundancy` in `backend/src/services/event_bus.rs`
- Grep `BusinessEvent` in `backend/src/services/event_bus.rs` 检查事件定义
- Read `backend/src/services/dye_batch_cost_bridge_service.rs` 检查染色成本桥接

### 发现

#### ✅ 已落实的项
1. **销售→库存预留→发货→AR 链路**：四链路全通（见维度 15.7）。
2. **采购→入库→AP→付款链路**：四链路全通。
3. **生产→领料→入库→成本链路**：四链路全通。
4. **事件总线（21 事件 + 双后端）**：`backend/src/services/event_bus.rs:153-176` 定义 CustomerUpdated/SupplierUpdated 等事件，Broadcast + Kafka + 自动降级。
5. **幂等去重（processed_events）**：主键 (consumer_id, event_key)。
6. **死信队列（event_dead_letters）**：含重试计数/状态流转。
7. **panic 隔离（AssertUnwindSafe）**：`backend/src/services/event_bus.rs:975-980` 单事件 panic 不退出循环；`backend/src/services/dye_batch_cost_bridge_service.rs:39-89` 同样实现。
8. **事务一致性（外部事务复用）**：`create_receivable` 接收外部 txn。
9. **TOCTOU 防护（事务内查询）**：信用检查/库存扣减均事务内（`check_credit_available_txn`）。
10. **行锁（lock_exclusive / FOR UPDATE）**：客户/库存操作均加锁（`customer_credit_limit.rs:320`）。
11. **主数据冗余字段刷新**：`backend/src/services/event_bus.rs:876-894` CustomerUpdated 触发 `refresh_customer_name_redundancy`（5 张表）；`:898-916` SupplierUpdated 触发 `refresh_supplier_name_redundancy`（2 张表），使用 tokio::spawn 异步执行。
12. **ES 同步最终一致性**：PG 提交后 sync，失败仅 warn。
13. **BPM 补偿回滚**：`backend/src/services/so/order_workflow.rs:186-209` 新事务回滚订单状态。

#### ⚠️ 部分缺陷项 1：染色→缸号→质检→入库链路
**风险等级：P2**
**证据**：
- `backend/src/services/event_bus.rs:939-951` DyeBatchCompleted 主监听器**仅 tracing::info 日志**，无业务回写
- `backend/src/services/event_bus.rs:953-967` QualityInspectionCompleted 主监听器**仅 tracing::info 日志**，无业务回写
- 但有独立的 `backend/src/services/dye_batch_cost_bridge_service.rs:40-75` 监听 DyeBatchCompleted 事件，**实际创建成本归集草稿**（业务回写已实现）
- QualityInspectionCompleted 事件无独立桥接监听器

**业务影响**：
- DyeBatchCompleted：主监听器仅日志，但有独立桥接监听器实现成本归集草稿创建，部分打通
- QualityInspectionCompleted：仅日志，质检通过→自动触发入库未实现，链路断裂
**修复建议**：
1. DyeBatchCompleted 主监听器补齐：染色完成 → 自动创建质检单
2. QualityInspectionCompleted 监听器补齐：质检通过 → 自动触发入库

---

## 维度 15.13：数据流转业务回写审计

### 检查方法
- Grep `transaction_type|pub async fn` in `backend/src/services/inventory_finance_bridge_service.rs`
- Read `backend/src/services/business_trace_service.rs` 检查写入方法
- Grep `business_trace_chain::ActiveModel|business_trace_chain.*insert|create_trace_chain` in `backend/src/`
- Grep `BusinessTraceService::new` in `backend/src/handlers/`
- Read `backend/src/services/dye_batch_cost_bridge_service.rs` 检查染色成本桥接

### 发现

#### ✅ 已落实的项
1. **库存财务桥接（7 种 transaction_type）**：`backend/src/services/inventory_finance_bridge_service.rs:214-282`，match transaction_type 处理 7 种类型。
2. **库存财务桥接幂等（inventory_txn:{transaction_id}）**：`backend/src/services/inventory_finance_bridge_service.rs:222-240`。
3. **CustomerUpdated 冗余刷新（5 张表）**：`backend/src/services/event_bus.rs:876-894` 触发 `refresh_customer_name_redundancy`，刷新 ar_invoices/ar_collections/ar_reconciliations/customer_credits/sales_contracts。
4. **SupplierUpdated 冗余刷新（2 张表）**：`backend/src/services/event_bus.rs:898-916` 触发 `refresh_supplier_name_redundancy`，刷新 purchase_contracts/fixed_assets。
5. **染色完成事件回写（部分）**：`backend/src/services/dye_batch_cost_bridge_service.rs:40-75` 独立监听器监听 DyeBatchCompleted，调用 `handle_dye_batch_completed` 创建成本归集草稿记录（status=draft），关联 batch_no/color_no。
6. **business_trace_snapshot 写入**：`backend/src/services/business_trace_service.rs:105-184` `create_snapshot` 方法，插入 business_trace_snapshot 表；`backend/src/handlers/business_trace_handler.rs:255-262` `create_trace_snapshot` handler 调用。

#### ❌ 缺陷项 1：business_trace_chain 表无写入
**风险等级：P2**
**证据**：
- `backend/src/services/business_trace_service.rs` 仅有 `find_trace_chain_by_five_dimension`/`find_trace_chain_by_id`/`forward_trace`/`backward_trace`/`create_snapshot` 方法
- Grep `business_trace_chain::ActiveModel|business_trace_chain.*insert|create_trace_chain` in `backend/src/` 返回 `No matches found`
- business_trace_chain 表无任何写入代码

**业务影响**：business_trace_chain 追溯链表无写入，导致缸号全链路追溯（投染→染色→质检→入库→发货→退货）数据缺失，`create_snapshot` 也因 trace_chain 为空无法生成有效快照。
**修复建议**：在关键业务节点（采购入库/染色投缸/染色完成/质检完成/入库/发货/退货）写入 business_trace_chain 记录，或决策删除该表归入死代码清理。

#### ❌ 缺陷项 2：质检完成事件回写未实现
**风险等级：P2**
**证据**：
- `backend/src/services/event_bus.rs:953-967` QualityInspectionCompleted 主监听器仅 tracing::info 日志
- 无独立桥接监听器处理 QualityInspectionCompleted 事件

**业务影响**：质检通过后无法自动触发入库，质检→入库链路断裂，需人工干预。
**修复建议**：补齐 QualityInspectionCompleted 监听器：质检通过 → 自动触发入库（创建 inventory_transaction + 库存回写）。

#### ⚠️ 部分缺陷项 3：染色完成事件回写（主监听器仅日志）
**风险等级：P3**
**证据**：
- 主监听器 `backend/src/services/event_bus.rs:939-951` 仅 tracing::info 日志
- 但有独立 `dye_batch_cost_bridge_service.rs` 监听器实现成本归集草稿创建
**业务影响**：主监听器侧无业务回写，但有专用桥接监听器补充，部分打通。
**修复建议**：主监听器补齐：染色完成 → 自动创建质检单。

---

## 维度 15.14：数据流转报表与追溯审计

### 检查方法
- Grep `pub async fn|overview|ranking|trend` in `backend/src/services/sales_analysis_service.rs`
- Grep `aging|账龄` in `backend/src/services/ap_invoice_service.rs`
- Grep `data_warehouse|etl|offline_report|离线报表` in `backend/src/`
- Grep `business_trace.*write|create_trace_chain` in `backend/src/services/`

### 发现

#### ✅ 已落实的项
1. **销售分析报表（概览/排名/趋势）**：`backend/src/services/sales_analysis_service.rs` 实现 `get_overview_stats`/`product_ranking`/`customer_ranking`/`get_trends`/`get_rankings`/`get_statistics_list`/`export_report`。
2. **AP 应付账龄分析**：`backend/src/services/ap_invoice_service.rs:694-752` `get_aging_analysis`，6 个账龄区间（未到期/逾期1-30/31-60/61-90/91-180/180以上），含完整测试（:1254-1304）。
3. **库存财务一体化（凭证自动生成）**：业财一体化 7 种流水自动生成凭证（inventory_finance_bridge_service.rs）。
4. **对账单 PDF 导出**：`backend/src/services/ar/inv.rs:41-97` `export_pdf`。
5. **CSV 导出**：`backend/src/services/so/delivery.rs:1117-1228` `export_orders_to_csv`。
6. **财务指标刷新（事件驱动）**：SalesOrderShipped → FinancialIndicatorUpdate。

#### ❌ 缺陷项 1：业务追溯（business_traces chain 写入）未实现
**风险等级：P2**
**证据**：
- 见维度 15.13 缺陷项 1
- business_trace_chain 表无写入代码

**业务影响**：无法追溯缸号全链路（投染→染色→质检→入库→发货→退货），报表数据无法追溯到源单据的缸号维度。
**修复建议**：见维度 15.13 缺陷项 1 修复建议。

#### ❌ 缺陷项 2：离线报表/数据仓库 ETL 未实现
**风险等级：P3**
**证据**：
- Grep `data_warehouse|etl|offline_report|离线报表` in `backend/src/` 返回 `No matches found`

**业务影响**：大数据量场景需 T+1 聚合，当前项目规模可暂不实现。
**修复建议**：低优先级，未来数据量增大时实现。

#### ⚠️ 部分落实项 1：报表数据追溯到源单据
**风险等级：P3**
**证据**：销售统计可追溯到订单，但缸号全链路追溯未实现。
**修复建议**：见缺陷项 1。

---

## 维度 15.15：数据流转审计与异常检测审计

### ⚠️ 审计计划部分失实声明

**审计计划描述**（第 5959 行）："主动异常检测引擎 ❌ 未实现"

**实际代码扫描结果**：`backend/src/services/ai/detect.rs` 已实现销售异常（Z-score）+ 库存异常（IQR）检测。审计计划描述部分失实。

### 检查方法
- Read `backend/src/services/ai/detect.rs`
- Grep `anomaly_detect|abnormal|异常检测` in `backend/src/`
- Grep `operation_log|omni_audit_log` in `backend/src/models/`
- Grep `event_dead_letter|processed_event` in `backend/src/models/`
- Grep `alert|告警|阈值` in `backend/src/services/`

### 发现

#### ✅ 已落实的项
1. **操作日志（operation_logs）**：`backend/src/models/operation_log.rs`，含 module/action/request_method/request_uri/request_ip/user_agent/status/error_message/duration_ms/extra_data。
2. **全链路审计（omni_audit_logs）**：`backend/src/models/omni_audit_log.rs`，含 trace_id/span_id/parent_span_id/HMAC-SHA256 防篡改。
3. **事务内审计写入**：`backend/src/services/customer_service.rs:599/655/779` 调用 `AuditLogService::update_with_audit` 事务内原子写入。
4. **生产订单操作日志查询**：`backend/src/services/production_order_service.rs` `get_order_logs`。
5. **事件死信审计**：`backend/src/models/event_dead_letter.rs` 失败事件落库。
6. **事件处理幂等审计**：`backend/src/models/processed_event.rs` 已处理事件落库。
7. **主动异常检测引擎（部分实现）**：`backend/src/services/ai/detect.rs:1-80` `detect_anomalies` 方法，实现：
   - 销售异常（Z-score）：SPIKE 突增 / DROP 突降，CRITICAL 与 WARNING 两级
   - 库存异常（IQR）：ZERO_STOCK / LOW_STOCK / OVERSTOCK / SLOW_MOVING 滞销
   - 基于历史数据计算 mean/std_deviation，检测最近 period 内的异常

#### ❌ 缺陷项 1：业务批次追溯（business_trace chain 无写入）
**风险等级：P2**
**证据**：见维度 15.13 缺陷项 1。
**业务影响**：无法追溯缸号全链路。
**修复建议**：见维度 15.13 缺陷项 1。

#### ⚠️ 部分缺陷项 2：主动异常检测引擎覆盖不全
**风险等级：P2**
**证据**：
- `backend/src/services/ai/detect.rs` 已实现销售异常 + 库存异常检测
- 但审计计划提到的"异常大额订单/异常频繁退货"未实现
- Grep `frequent_return|abnormal_order|大额异常` in `backend/src/` 无匹配

**业务影响**：异常检测覆盖销售波动和库存，但缺少退货频率异常、订单金额异常等业务侧异常检测。
**修复建议**：扩展 ai/detect.rs，新增异常大额订单检测（Z-score on order amount）+ 异常频繁退货检测（退货频率 IQR）。

#### ❌ 缺陷项 3：数据流转异常告警未实现
**风险等级：P2**
**证据**：
- Grep `alert|告警|阈值告警` in `backend/src/services/` 无阈值告警引擎
- 无事件处理延迟>5min 告警/死信>10 条告警/事务失败率>1% 告警

**业务影响**：事件处理延迟、死信堆积、事务失败率等系统侧异常无主动告警，运维被动。
**修复建议**：新增阈值告警引擎，配置告警规则（事件延迟/死信堆积/事务失败率），触发邮件/消息通知。

#### ⚠️ 部分落实项 1：审计日志定期审查 cron
**风险等级：P3**
**证据**：V15 类十三 13.10 已规划每日合规审查 cron。
**修复建议**：按 V15 类十三规划落地。

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 15.1 供货商主数据完整性 | 0 | 1 | 2 | 1 | 10 | 14 |
| 15.2 供货商业务闭环 | 0 | 0 | 2 | 2 | 8 | 12 |
| 15.3 供货商面料行业特性 | 0 | 0 | 0 | 3 | 3 | 6 |
| 15.4 加工商维度 | 0 | 0 | 1 | 2 | 9 | 12 |
| 15.5 加工商业务流程闭环 | 0 | 0 | 0 | 1 | 7 | 8 |
| 15.6 销售订单数据模型与状态机 | 0 | 0 | 1 | 0 | 6 | 7 |
| 15.7 销售业务流程闭环 | 0 | 0 | 0 | 0 | 12 | 12 |
| 15.8 销售面料行业特性 | 0 | 0 | 0 | 2 | 4 | 6 |
| 15.9 客户主数据完整性 | 0 | 0 | 0 | 2 | 8 | 10 |
| 15.10 客户信用与应收管理 | 0 | 0 | 1 | 0 | 11 | 12 |
| 15.11 客户面料行业特性 | 0 | 0 | 0 | 3 | 5 | 8 |
| 15.12 跨模块数据流转 | 0 | 0 | 1 | 0 | 13 | 14 |
| 15.13 数据流转业务回写 | 0 | 0 | 2 | 1 | 6 | 9 |
| 15.14 数据流转报表与追溯 | 0 | 0 | 1 | 2 | 6 | 9 |
| 15.15 数据流转审计与异常检测 | 0 | 0 | 2 | 2 | 7 | 11 |
| **合计** | **0** | **1** | **13** | **21** | **111** | **146** |

**审计判定**：
- 已落实率：111/146 = 76.0%
- P0 阻塞：0 项
- P1 高：1 项（supplier_evaluation_records 表无 migration）
- P2 中：13 项
- P3 低：21 项

---

## 修复优先级队列

### P1（高优先级，1 项）

1. **15.1-缺陷3**：补齐 `supplier_evaluations` 和 `supplier_evaluation_records` 表的 sea-orm migration 文件
   - 证据：`backend/src/models/supplier_evaluation_record.rs:12` `table_name = "supplier_evaluation_records"`，但 migration 中无建表语句
   - 影响：model 层对应的表在数据库中实际不存在，运行时查询报错

### P2（中优先级，13 项）

1. **15.1-缺陷1**：suppliers 主表新增 `category_id` 外键字段，关联 supplier_categories 表
2. **15.1-缺陷2**：SupplierService 补齐资质 `update_supplier_qualification` 和 `delete_supplier_qualification` 方法
3. **15.2-缺陷1**：供应商评估自动触发 cron（按季度自动触发供应商评估任务）
4. **15.2-缺陷2**：供应商账户余额管理（扩展供应商维度余额查询 Service）
5. **15.4-缺陷1**：suppliers 表新增 `is_processor` + `processor_type` 字段，区分供应商与加工商
6. **15.6-缺陷1**：新增 `sales_contract_items` 表（销售合同明细行表）
7. **15.10-缺陷1**：客户信用评级自动触发 cron（按月度/季度自动评估）
8. **15.12-部分缺陷1**：DyeBatchCompleted 主监听器补齐业务回写（染色完成→自动创建质检单）
9. **15.13-缺陷1**：business_trace_chain 表写入 Service（缸号全链路追溯）
10. **15.13-缺陷2**：QualityInspectionCompleted 监听器补齐业务回写（质检通过→自动触发入库）
11. **15.14-缺陷1**：业务追溯（business_traces chain 写入）—— 同 15.13-缺陷1
12. **15.15-部分缺陷2**：扩展主动异常检测引擎（异常大额订单 + 异常频繁退货检测）
13. **15.15-缺陷3**：数据流转异常告警（阈值告警引擎 + 邮件/消息通知）

### P3（低优先级，21 项）

1. **15.1-部分缺陷4**：supplier_evaluation.rs model 重命名为 supplier_evaluation_indicator.rs
2. **15.2-缺陷3**：供应商供货历史查询 Service
3. **15.2-缺陷4**：供应商价格清单导入（批量导入 Excel 接口）
4. **15.3-缺陷1**：供应商色卡能力字段（color_card_capability）
5. **15.3-缺陷2**：供应商染色能力字段（dyeing_capacity）
6. **15.3-缺陷3**：供应商印花能力字段（printing_capacity）
7. **15.4-缺陷2**：suppliers 表新增 dyeing_capacity/printing_capacity 字段
8. **15.4-缺陷3**：前端加工商管理界面确认（需前端审计）
9. **15.5-缺陷1**：委外加工报表（加工商排名/加工费统计/损耗率分析）
10. **15.8-部分落实1**：销售批色流程（V15 类十一已规划）
11. **15.8-部分落实2**：按匹号发货优化
12. **15.9-缺陷1**：客户多地址表（customer_addresses）
13. **15.9-缺陷2**：客户多银行账户表（customer_bank_accounts）
14. **15.11-部分落实1**：客户批色确认能力（V15 类十一已规划）
15. **15.11-部分落实2**：客户色卡档案（V15 类十已规划）
16. **15.11-缺陷1**：客户特殊工艺要求字段
17. **15.13-部分缺陷3**：染色完成事件主监听器补齐业务回写
18. **15.14-缺陷2**：离线报表/数据仓库 ETL（未来数据量增大时实现）
19. **15.14-部分落实1**：报表数据追溯到源单据（缸号维度）
20. **15.15-部分落实1**：审计日志定期审查 cron（V15 类十三已规划）
21. **15.15-缺陷1**：业务批次追溯（同 15.13-缺陷1）

---

## 审计结论

### 整体评价

类十五业务主体维度审计覆盖 15 个维度，共检查 146 项，已落实 111 项（76.0%），缺陷 35 项（P0:0 / P1:1 / P2:13 / P3:21）。

### 关键发现

1. **审计计划严重失实**：V15 审计计划类十五中多处描述与实际代码状态严重不一致，主要表现在：
   - 加工商维度（15.4/15.5）：审计计划称"完全未实现"，实际 v14 批次 430 已完整实现
   - 销售预测（15.6）：审计计划称"未实现"，实际 ai/pred.rs 已实现
   - 异常检测（15.15）：审计计划称"未实现"，实际 ai/detect.rs 已实现销售+库存异常检测
   - business_traces 写入（15.13）：审计计划称"完全无写入"，实际 business_trace_snapshot 有 create_snapshot 写入接口

2. **业务闭环完整度高**：
   - 销售业务闭环 12/12 完整（100%）
   - 加工商业务闭环 7/8 完整（87.5%）
   - 供货商业务闭环 8/12 完整（67%）

3. **核心缺陷集中在数据流转回写**：
   - business_trace_chain 表无写入（缸号全链路追溯断裂）
   - QualityInspectionCompleted 监听器仅日志（质检→入库链路断裂）
   - DyeBatchCompleted 主监听器仅日志（但有独立桥接监听器部分补偿）
   - 信用评级/供应商评估自动触发 cron 未实现

4. **P1 缺陷需优先修复**：supplier_evaluation_records 表无 migration，运行时会报错。

### 建议下一步行动

1. **立即修复 P1**：补齐 supplier_evaluation_records 表 migration
2. **批量修复 P2**：重点修复数据流转回写（business_trace_chain 写入 + QualityInspectionCompleted 监听器 + 信用/评估自动触发 cron）
3. **规划 P3**：按业务优先级逐步补齐，部分项已由 V15 其他类别规划（批色/色卡档案/审计 cron）
4. **更新审计计划**：建议 V15 审计计划维护者根据本报告更新类十五描述，避免后续误导

---

## 附录：审计扫描命令汇总

```bash
# 1. 供货商主数据完整性
grep -n "category_id" backend/src/models/supplier.rs  # 无匹配（缺陷）
ls backend/migrations/ | grep -i supplier_eval  # 找到 4 文件，但无 records 建表
grep -n "update_supplier_qualification|delete_supplier_qualification" backend/src/services/supplier_service.rs  # 无匹配（缺陷）

# 2. 加工商维度（实际已实现，与审计计划不符）
grep -rn "outsourc|processor|is_processor|委外|外协|外发" backend/src/  # 找到 16 文件

# 3. 销售合同明细行表
grep -n "sales_contract_item|SalesContractItem" backend/src/models/ backend/src/services/ backend/src/handlers/  # 无匹配（缺陷）

# 4. 客户信用评级自动触发
grep -rn "cron|scheduler|evaluate_credit" backend/src/services/customer_credit*.rs  # 无 cron（缺陷）

# 5. 数据流转监听器业务回写
grep -A 20 "DyeBatchCompleted|QualityInspectionCompleted" backend/src/services/event_bus.rs  # 主监听器仅日志

# 6. business_traces 写入
grep -rn "business_trace_chain::ActiveModel|business_trace_chain.*insert|create_trace_chain" backend/src/  # 无匹配（缺陷）
grep -rn "create_snapshot" backend/src/services/business_trace_service.rs  # snapshot 有写入

# 7. 异常检测引擎
grep -rn "anomaly_detect|abnormal|异常检测" backend/src/  # 找到 ai/detect.rs（部分实现）

# 8. 销售预测
grep -rn "sales_forecast|SalesForecast|销售预测" backend/src/  # 找到 ai/pred.rs（已实现）
```

---

**审计完成时间**：2026-07-16
**审计子代理**：V15 审计子代理（类十五业务主体维度审计与数据流转）
**报告文件**：`/workspace/.monkeycode/docs/audits/v15/batch-13/audit-report.md`
