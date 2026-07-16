# V15 胚布拆匹与库存排程审计报告（类二十一+类二十二·批次 18）

- **审计子代理**：V15 审计子代理（类二十一胚布拆匹+类二十二库存排程）
- **审计范围**：11 维度（胚布拆匹 5 维度 + 库存排程 6 维度）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 6465-6601 行
  - `/workspace/backend/src/models/greige_fabric.rs`
  - `/workspace/backend/src/models/piece_mapping.rs`
  - `/workspace/backend/src/models/inventory_piece.rs`
  - `/workspace/backend/src/models/inventory_transfer.rs` / `inventory_transfer_item.rs`
  - `/workspace/backend/src/models/quality_issue.rs` / `unqualified_product.rs`
  - `/workspace/backend/src/models/work_center.rs` / `scheduling_result.rs`
  - `/workspace/backend/src/models/outsourcing_order.rs` / `outsourcing_receipt.rs`
  - `/workspace/backend/src/handlers/greige_fabric_handler.rs`
  - `/workspace/backend/src/handlers/piece_split_handler.rs`
  - `/workspace/backend/src/handlers/inventory_transfer_handler.rs`
  - `/workspace/backend/src/handlers/material_shortage_handler.rs`
  - `/workspace/backend/src/services/inv/inventory_move.rs` / `batch.rs` / `hold.rs`
  - `/workspace/backend/src/services/stock_alert.rs`
  - `/workspace/backend/src/services/material_shortage_service.rs`
  - `/workspace/backend/src/services/scheduling_auto.rs` / `scheduling_manual.rs` / `scheduling_query.rs` / `scheduling_service.rs`
  - `/workspace/backend/src/services/capacity_service.rs`
  - `/workspace/backend/src/services/mrp_engine_service.rs`
  - `/workspace/backend/src/services/quality_inspection_service.rs`
  - `/workspace/backend/src/services/custom_order_quality_service.rs`
  - `/workspace/backend/src/services/inventory_reservation_service.rs`
  - `/workspace/backend/src/services/inventory_stock_query.rs`
  - `/workspace/backend/src/services/inventory_count_service.rs`
  - `/workspace/backend/src/services/outsourcing_service.rs`
  - `/workspace/backend/src/services/fabric_inspection_service.rs`
- **审计方法**：Read 审计计划 + Grep 检索 + Read 关键文件 + 对照审计计划核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 类二十一 维度 1：胚布库存与采购管理审计

### 检查方法
- Grep 检索：`greige|胚布|坯布|拆匹|piece_split|piece_mapping`，发现 `greige_fabric` 模型与 handler。
- Read `/workspace/backend/src/models/greige_fabric.rs` 全文 + `handlers/greige_fabric_handler.rs` 全文。
- Read `/workspace/backend/src/models/dye_batch.rs`（验证 `greige_fabric_id` 外键）。
- Grep 检索 `piece_mapping::`（验证模型业务引用）。
- Grep 检索 `reorder_point|safety_stock|安全库存` 在 `greige_fabric` 模型与 handler 中的覆盖情况。

### 发现

#### ✅ 已落实的项

1. **胚布独立库存模型存在**：`greige_fabric` 表是独立的胚布管理表，含 `fabric_no`/`fabric_name`/`fabric_type`/`color_code`/`width_cm`/`weight_kg`/`length_m`/`supplier_id`/`batch_no`/`warehouse_id`/`location`/`quality_grade`/`status`/`quantity_meters`/`quantity_kg` 等字段，支持按卷/匹/公斤管理（`/workspace/backend/src/models/greige_fabric.rs:11-52`）。
2. **胚布关联供应商与仓库**：`greige_fabric` 外键关联 `product`/`supplier`/`warehouse`（`/workspace/backend/src/models/greige_fabric.rs:54-80`）。
3. **胚布入库与出库接口存在**：`stock_in`/`stock_out` 接口支持累加/扣减库存，校验出库不能大于现有重量/长度（`/workspace/backend/src/handlers/greige_fabric_handler.rs:281-408`）。
4. **胚布软删除与状态校验**：在库胚布不允许删除（`/workspace/backend/src/handlers/greige_fabric_handler.rs:268-270`）。
5. **胚布与染批次关联**：`dye_batch` 表外键 `greige_fabric_id` 关联胚布，可从染缸→胚布追溯（`/workspace/backend/src/models/dye_batch.rs:16,30-37`）。

#### ❌ 缺陷项

**缺陷 1.1：胚布未走采购订单流程，无关联采购订单字段**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/greige_fabric.rs:13-52`：`greige_fabric` 模型仅有 `supplier_id` 字段，无 `purchase_order_id` / `purchase_receipt_id` 字段。
- `/workspace/backend/src/handlers/greige_fabric_handler.rs:151-198`：`create_greige_fabric` 直接由前端传入 `supplier_id` 创建胚布，不关联采购订单与采购入库单，无三单匹配（订单/收货/发票）。
**业务影响**：胚布采购脱离采购流程，无法做采购订单跟踪、应付凭证生成、采购入库三单匹配，财务对账缺链路。
**修复建议**：在 `greige_fabric` 模型增加 `purchase_order_id`/`purchase_receipt_id` 可空外键字段；`stock_in` 接口增加可选 `purchase_receipt_id`，关联采购入库单做库存增加。

**缺陷 1.2：胚布库存无安全库存预警字段**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/greige_fabric.rs:13-52`：模型无 `safety_stock`/`reorder_point`/`max_stock_point`/`reorder_quantity` 字段。
- `/workspace/backend/src/handlers/greige_fabric_handler.rs:281-335`：`stock_in` 接口仅累加库存，无低库存预警判断。
- 对比 `inventory_stocks` 表有完整 `reorder_point`/`max_stock_point`/`reorder_quantity` 字段（`/workspace/backend/src/models/inventory_stock.rs:24-30`），胚布未接入统一库存表。
**业务影响**：胚布作为染整源头，库存预警盲区，可能因胚布短缺导致停产；未接入统一库存告警系统，告警逻辑割裂。
**修复建议**：将胚布库存纳入 `inventory_stocks` 统一管理（fabric_type 区分胚布/成品），或在 `greige_fabric` 模型增加安全库存字段，并接入 `stock_alert` 模块。

**缺陷 1.3：胚布批次追溯字段不全**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/models/greige_fabric.rs:29`：仅有 `batch_no: Option<String>`，无 `color_no`/`dye_lot_no`/`production_date` 与采购批次关联字段，未与 `inventory_piece` 的匹号/缸号体系打通。
- 胚布表无 `dye_lot_no` 字段，导致无法从胚布→染色→成品全链路按缸号追溯。
**业务影响**：胚布作为染整源头，无法做"胚布→染色缸号→成品布"反向追溯，遇质量问题难以定位源头胚布批次。
**修复建议**：在 `greige_fabric` 表增加 `dye_lot_no`/`color_no` 字段，并在 `dye_batch.greige_fabric_id` 关联基础上补 `batch_dye_lot` 双向关联。

---

## 类二十一 维度 2：胚布委托加工流转审计

### 检查方法
- Read `/workspace/backend/src/models/outsourcing_order.rs` 全文。
- Read `/workspace/backend/src/services/outsourcing_service.rs` L1-700。
- Grep `outsourcing|委外` 在 routes 中检索路由暴露。
- 检查胚布出库 → 委外加工 → 成品入库的链路完整性。

### 发现

#### ✅ 已落实的项

1. **委外加工独立模块存在**：`outsourcing_order` 表 + `outsourcing_service` 实现"发料→加工费→入库"三步分录模型（`/workspace/backend/src/services/outsourcing_service.rs:1-23`）。
2. **委外订单状态机完整**：`draft→issued→processing→received→settled→closed→cancelled` 状态机已实现，校验合法状态（`/workspace/backend/src/services/outsourcing_service.rs:169-186`）。
3. **委外损耗核算完整**：实现 `compute_loss_rate`/`compute_total_cost`/`compute_unit_cost`/`compute_standard_loss_rate`/`classify_loss`/`compute_abnormal_loss_amount` 6 个核心纯函数，含染色 0.05 / 织布 0.035 / 后整理 0.03 行业损耗标准（`/workspace/backend/src/services/outsourcing_service.rs:60-148`）。
4. **正常/非正常损耗分类**：`classify_loss` 比较 actual vs standard，超标准损耗计入"营业外支出"（`/workspace/backend/src/services/outsourcing_service.rs:117-123,664-679`）。
5. **加工费凭证自动生成**：`record_receipt` 创建入库凭证 `voucher_no_receipt` + 损耗凭证（`/workspace/backend/src/services/outsourcing_service.rs:640-700`）。
6. **委外订单关联缸号**：`outsourcing_order` 表外键关联 `dye_batch_id`/`color_no`/`dye_lot_no`（`/workspace/backend/src/models/outsourcing_order.rs:33-39`）。

#### ❌ 缺陷项

**缺陷 2.1：委外发料未关联胚布（greige_fabric）**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/outsourcing_order.rs:22-102`：委外订单主表无 `greige_fabric_id` 字段。
- `/workspace/backend/src/services/outsourcing_service.rs:280-450`：创建委外订单只校验 `supplier_id`/`production_order_id`，未引用 `greige_fabric` 表，发料明细 `outsourcing_order_item` 也无 `greige_fabric_id`。
- 委外订单仅靠 `dye_batch_id` 间接关联胚布，无法精确追溯具体哪一卷胚布被发出加工。
**业务影响**：胚布委外加工不能精确到胚布卷/匹级追溯，发生损耗或质量问题时无法定位具体胚布源头。
**修复建议**：在 `outsourcing_order_item` 表增加 `greige_fabric_id` 字段，发料时扣减对应胚布库存并记录映射。

**缺陷 2.2：委外收回未走质检流程**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/services/outsourcing_service.rs:552-635`：`record_receipt` 接收 `quality_status`/`grade` 字段，由前端传入，未调用 `QualityInspectionService::create_record` 创建质检记录，未走不合格品 `process_unqualified` 流程。
- `/workspace/backend/src/models/outsourcing_receipt.rs` 收回单字段有 `quality_status`/`grade` 但无 `inspection_id` 外键关联质检记录。
**业务影响**：委外收回质量数据仅前端录入，未走独立质检流程，不合格委外成品未触发返工/报废/降级处理，8D 闭环缺环。
**修复建议**：`record_receipt` 接收后自动触发 `QualityInspectionService::create_record` 创建质检记录，质检不合格时走 `process_unqualified`。

**缺陷 2.3：委外加工费未按缸号/匹号核算**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/outsourcing_service.rs:223-238`：`CreateOutsourcingOrderRequest` 仅在订单级有 `dye_lot_no`/`color_no`，发料明细 `outsourcing_order_item` 表无独立缸号/匹号字段。
- `/workspace/backend/src/models/outsourcing_order_item.rs:31-32`：item 级有 `dye_lot_no` 字段但与 `outsourcing_receipt` 中的匹号未直接关联。
- 加工费 `processing_fee` 仅在订单级记录，未按匹号分摊到 `inventory_piece`。
**业务影响**：加工费不能按匹号精确归集到成本，影响单匹成本核算准确性。
**修复建议**：在委外收回单按匹号记录加工费分摊比例，自动写入 `cost_collection` 表按匹号归集成本。

---

## 类二十一 维度 3：拆匹后缸号匹号继承规则审计

### 检查方法
- Read `/workspace/backend/src/handlers/piece_split_handler.rs` 全文。
- Read `/workspace/backend/src/models/inventory_piece.rs` 全文。
- Read `/workspace/backend/src/models/piece_mapping.rs` 全文。
- Grep `piece_mapping::` 验证模型业务引用。
- Grep `dye_lot_no|缸号继承|拆匹.*缸号` 验证缸号继承规则。

### 发现

#### ✅ 已落实的项

1. **拆匹接口存在**：`POST /api/v1/erp/inventory/pieces/split` 调用 `piece_split_handler::split_fabric_piece`（`/workspace/backend/src/routes/inventory.rs:28`、`/workspace/backend/src/handlers/piece_split_handler.rs:33-134`）。
2. **拆匹继承缸号 ID**：拆分时 `dye_lot_id: Set(parent.dye_lot_id)` 继承母卷缸号 ID（`/workspace/backend/src/handlers/piece_split_handler.rs:90`）。
3. **拆匹继承批次号**：`batch_no: Set(parent.batch_no.clone())` 继承母卷批次号（`/workspace/backend/src/handlers/piece_split_handler.rs:91`）。
4. **拆匹记录母卷关联**：`parent_piece_id: Set(Some(parent.id))` 关联母卷，支持反向追溯（`/workspace/backend/src/handlers/piece_split_handler.rs:97`）。
5. **拆匹长度校验**：校验 `parent.length < req.cut_length` 拒绝超切（`/workspace/backend/src/handlers/piece_split_handler.rs:53-58`）。
6. **拆匹状态校验**：已发货/不可用状态布卷不允许拆分（`/workspace/backend/src/handlers/piece_split_handler.rs:46-50`）。
7. **匹号唯一性校验**：通过 `(dye_lot_id, piece_no)` 联合唯一约束保证匹号缸号内唯一（`/workspace/backend/src/models/inventory_piece.rs:22-23`）。
8. **打卷入库匹号序号生成**：`fabric_inspection_service::roll_fabric` 自动生成匹号 `{dye_lot_no}-{seq:03}`，校验缸号内唯一（`/workspace/backend/src/services/fabric_inspection_service.rs:514-541`）。

#### ❌ 缺陷项

**缺陷 3.1：拆匹后子匹未继承 `dye_lot_no` 字符串字段**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/handlers/piece_split_handler.rs:118-119`：拆分创建新布卷时，`color_no: sea_orm::ActiveValue::NotSet` 和 `dye_lot_no: sea_orm::ActiveValue::NotSet`，**未继承母卷的 `color_no`/`dye_lot_no` 字符串字段**，仅继承了 `dye_lot_id`（外键 ID）。
- 后续按 `dye_lot_no` 字符串追溯时，拆匹产生的子卷 `dye_lot_no` 为 NULL，无法参与缸号字符串维度的追溯。
**业务影响**：违反审计计划 21.3"拆匹后子匹必须继承原缸号（dye_lot_no），禁止新建缸号"。子卷 `dye_lot_no` 字符串字段为空，下游按 `dye_lot_no` 字符串过滤的查询（如发货、调拨、追溯）将漏掉拆匹子卷。
**修复建议**：拆匹时显式继承 `color_no: Set(parent.color_no.clone())` 与 `dye_lot_no: Set(parent.dye_lot_no.clone())`。

**缺陷 3.2：拆匹数量之和未做等于原匹数量的强校验**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/handlers/piece_split_handler.rs:60-76`：仅校验剪裁长度不超过母卷长度，但允许"剪裁一次后剩余长度 > 0"的中间状态，未做"所有子匹长度之和必须等于原匹长度"的强校验，差异无自动告警。
- 缺少 `piece_split_history` 表记录完整拆分链路（每次拆分仅修改母卷 + 新建一卷，无累计校验）。
**业务影响**：拆匹累计误差无法及时发现，可能存在多拆/少拆导致账实不符。
**修复建议**：增加"剩余长度 + 所有子卷长度 = 原母卷长度"的定时对账任务，差异自动告警。

**缺陷 3.3：`piece_mapping` 表存在但无业务代码引用**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/piece_mapping.rs:1-66`：定义了 `piece_mapping` 模型，含 `batch_no`/`product_id`/`piece_no`/`length`/`weight`/`status` 字段。
- Grep 检索 `piece_mapping::` 在 `/workspace/backend/src` 中**零业务调用**（仅 `models/mod.rs:250` 模块声明 + 模型定义本身）。
- 拆匹流程实际使用 `inventory_piece.parent_piece_id` 自关联做映射，`piece_mapping` 表成"死表"。
**业务影响**：审计计划 21.3"拆匹必须记录原匹→子匹映射（piece_mapping），支持反向追溯"未通过 `piece_mapping` 表实现，而是依赖 `inventory_piece.parent_piece_id`。虽然能反向追溯，但 `piece_mapping` 表存在却未使用，违反死代码规范第 6.1 节"真正未使用的项应显式删除"。
**修复建议**：二选一：(a) 删除 `piece_mapping` 模型与迁移，明确以 `inventory_piece.parent_piece_id` 作为唯一映射机制；(b) 在拆匹流程中同步写入 `piece_mapping` 表，建立独立映射层。

**缺陷 3.4：拆匹未生成新匹号而用 `{parent.piece_no}-CUT-{timestamp}`**
**风险等级：P3**
**证据**：
- `/workspace/backend/src/handlers/piece_split_handler.rs:79-85`：新匹号格式 `{parent.piece_no}-CUT-{Utc::now().timestamp_subsec_millis()}`，**不是缸号内有序序号**。
- 对比 `fabric_inspection_service::roll_fabric` 用 `{dye_lot_no}-{seq:03}`（缸号内有序）。
- 审计计划 21.3"拆匹后子匹必须有新匹号（batch_no），匹号在缸号内唯一"——当前实现满足唯一但**不满足缸号内有序**。
**业务影响**：拆匹产生的匹号无序，前端按匹号排序展示混乱，不利于人工核对。
**修复建议**：拆匹时复用 `roll_fabric` 的序号生成逻辑，按缸号下最大 `piece_seq + 1` 生成有序匹号。

---

## 类二十一 维度 4：质量问题 8D 处理流程审计

### 检查方法
- Read `/workspace/backend/src/models/quality_issue.rs` 全文。
- Read `/workspace/backend/src/services/custom_order_quality_service.rs` 全文。
- Grep `8D|5Why|five_why|fishbone|根因|纠正预防|corrective_action|permanent_action` 在 backend/src 中检索。
- Grep `quality_issues` 检索所有引用方。

### 发现

#### ✅ 已落实的项

1. **质量异常表存在**：`quality_issues` 表记录 `issue_type`/`severity`/`description`/`discovered_at`/`resolved_at`/`resolution`/`status` 字段（`/workspace/backend/src/models/quality_issue.rs:11-25`）。
2. **质量异常上报接口存在**：`report_issue` 校验严重度（low/medium/high/critical）+ GB/T 26377 色差 ΔE + ISO 105 色牢度等级（`/workspace/backend/src/services/custom_order_quality_service.rs:51-104`）。
3. **质量异常解决接口存在**：`resolve_issue` 走事务 + 行锁 + 审计日志，状态门拒绝已关闭异常重复解决（`/workspace/backend/src/services/custom_order_quality_service.rs:109-146`）。
4. **质量异常列表查询**：`list_by_order` 接入统一分页（`/workspace/backend/src/services/custom_order_quality_service.rs:153-168`）。

#### ❌ 缺陷项

**缺陷 4.1：质量异常未走 8D 流程，仅 open/resolved/closed 三态**
**风险等级：P0**
**证据**：
- `/workspace/backend/src/models/quality_issue.rs:11-25`：`quality_issues` 表字段仅有 `issue_type`/`severity`/`description`/`discovered_at`/`resolved_at`/`resolution`/`status`，**无 8D 各阶段独立字段**（无 D1 团队成员 / D3 临时措施 / D4 根因分析 / D5 永久措施 / D6 验证结果 / D7 预防措施 / D8 闭环总结）。
- `/workspace/backend/src/services/custom_order_quality_service.rs:109-146`：`resolve_issue` 仅将 status 改为 `resolved`，无 8D 流转节点，根因分析与永久措施仅靠 `resolution` 单字段文本记录。
- Grep `8D|5Why|five_why|fishbone|根因|纠正预防|corrective_action|permanent_action` 在整个 `backend/src` 中**零结果**。
- 审计计划 21.4"质量问题必须走 8D 流程：D1 团队→D2 描述→D3 临时措施→D4 根因→D5 永久措施→D6 验证→D7 预防→D8 闭环"完全未实现。
**业务影响**：质量问题处理流于形式，仅做"上报→解决"两步，无根因分析、无永久措施、无预防机制，同类质量问题会反复发生，无法形成闭环管理。这是面料行业质量管理的核心能力缺失。
**修复建议**：扩展 `quality_issues` 表，新增 `d1_team`/`d3_interim_action`/`d4_root_cause`/`d5_permanent_action`/`d6_verification`/`d7_prevention`/`d8_closure_summary` 字段；新增 8D 流转服务，按阶段状态机推进，每阶段必须填写才能进入下一阶段。

**缺陷 4.2：未使用 5Why/鱼骨图等根因分析方法**
**风险等级：P1**
**证据**：
- Grep `5Why|five_why|fishbone|根因分析` 在 `backend/src` 中零结果。
- `/workspace/backend/src/models/quality_issue.rs:11-25`：无根因分析方法字段，无分析过程记录字段。
**业务影响**：根因分析缺乏结构化工具，依赖人工文本描述，分析质量参差不齐。
**修复建议**：在 8D D4 阶段增加 `root_cause_method`（5why/fishbone/其他）字段 + `root_cause_detail` JSON 字段结构化存储分析过程。

**缺陷 4.3：纠正预防措施无责任人和完成日期跟踪**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/quality_issue.rs:11-25`：无 `permanent_action_owner`/`permanent_action_due_date`/`permanent_action_completed_at` 字段。
- `/workspace/backend/src/services/custom_order_quality_service.rs:109-146`：`resolve_issue` 仅记录 `resolution` 文本，无超期告警机制。
**业务影响**：永久措施落实无跟踪，可能成为空头承诺，超期无告警。
**修复建议**：增加责任人 + 完成日期字段，定时任务扫描超期未完成的永久措施并告警通知。

**缺陷 4.4：无 8D 月报能力**
**风险等级：P2**
**证据**：
- Grep `8D 月报|8D 报表|8D report|quality_issue.*monthly` 零结果。
- 现有 `quality_issues` 列表查询无按月汇总（问题数/关闭率/平均关闭周期）能力。
**业务影响**：管理层无法掌握质量问题整体趋势与处理效率。
**修复建议**：增加 8D 月报查询接口，按月汇总问题数/关闭数/关闭率/平均关闭周期，支持导出。

---

## 类二十一 维度 5：不合格品降级返工报废流程审计

### 检查方法
- Read `/workspace/backend/src/models/unqualified_product.rs` 全文。
- Read `/workspace/backend/src/services/quality_inspection_service.rs:1-200` 与 `:300-415`。
- Grep `HANDLING_DOWNGRADE_SALE|HANDLING_REWORK|HANDLING_SCRAP` 验证处理方式常量。
- Grep `rework.*production_order|返工工单` 验证返工工单流程。

### 发现

#### ✅ 已落实的项

1. **不合格品分类常量存在**：`HANDLING_DOWNGRADE_SALE`（降级销售）/ `HANDLING_REWORK`（返工）/ `HANDLING_SCRAP`（报废）三类常量定义（`/workspace/backend/src/services/quality_inspection_service.rs:39-41`）。
2. **A/B/C 等级分级判定**：`determine_quality_grade` 按 95%/80% 阈值判 A/B/C 级，C 级必须返工或报废（`/workspace/backend/src/services/quality_inspection_service.rs:51-103`）。
3. **处理方式与等级匹配校验**：`validate_handling_method_by_grade` 强制 B 级降级销售、C 级返工或报废（`/workspace/backend/src/services/quality_inspection_service.rs:65-103`）。
4. **不合格品记录字段完整**：`unqualified_product` 表含 `grade`/`handling_method`/`handling_status`/`handling_by`/`handling_at`/`handling_result` 字段（`/workspace/backend/src/models/unqualified_product.rs:11-44`）。
5. **处理结果记录**：`handling_result` 字段记录降级销售单价/返工工时/报废损失金额（`/workspace/backend/src/models/unqualified_product.rs:38-39`）。

#### ❌ 缺陷项

**缺陷 5.1：降级处理未联动库存等级与价格调整**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/services/quality_inspection_service.rs:351-391`：`process_unqualified` 仅创建 `unqualified_product` 记录，**不更新 `inventory_stocks.grade` 字段**，不调用价格调整服务。
- `inventory_stocks.grade` 字段（`/workspace/backend/src/models/inventory_stock.rs:45`）独立维护，与 `unqualified_product.grade` 不联动。
- 无"按等级价差表自动调整价格"的逻辑。
**业务影响**：降级后库存等级与账面等级不一致，销售开单时可能仍按 A 级定价，造成定价错误。
**修复建议**：`process_unqualified` 触发后，同步更新 `inventory_stocks.grade` 字段，并按等级价差表调用价格服务调整销售价。

**缺陷 5.2：返工未走生产订单（返工工单）**
**风险等级：P0**
**证据**：
- `/workspace/backend/src/services/quality_inspection_service.rs:351-391`：`process_unqualified` 当 `handling_method = rework` 时，仅记录文本 `handling_result`，**不创建 `production_order` 返工工单**。
- Grep `rework.*production_order|返工工单|rework_order` 零业务结果（仅 `dye_batch_rework.rs` 模型存在，但是染色批次返工，非通用返工工单）。
- 审计计划 21.5"返工必须走生产订单（返工工单），返工成本归集到原缸号"完全未实现。
**业务影响**：返工无工单跟踪，返工成本无法归集到原缸号，返工过程无进度管理，返工完成也无质检闭环。这是生产管理的核心能力缺失。
**修复建议**：扩展 `production_order` 增加 `order_type=rework` + `original_order_id` + `original_dye_lot_no` 字段；`process_unqualified` 当 handling_method=rework 时自动创建返工生产订单，返工完成走质检闭环。

**缺陷 5.3：报废未走二级审批（财务+总经理）**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/services/quality_inspection_service.rs:351-391`：`process_unqualified` 当 `handling_method = scrap` 时，无审批流程，直接创建记录。
- Grep `scrap.*审批|scrap_approval|报废审批` 零业务结果。
- 审计计划 21.5"报废必须走审批（财务+总经理），报废损失自动计入成本"未实现。
**业务影响**：报废无审批管控，存在舞弊风险（如将合格品报废私分）；报废损失未自动计入成本，财务核算缺环。
**修复建议**：`process_unqualified` 当 handling_method=scrap 时进入审批流（接入 BPM 服务），需财务 + 总经理二级审批通过后才生效；审批通过后自动生成报废损失凭证。

**缺陷 5.4：不合格品分类无审批**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/quality_inspection_service.rs:351-391`：分类（A/B/C）+ 处理方式（降级/返工/报废）由 `process_unqualified` 调用方直接传入，仅校验等级与处理方式匹配，无独立审批节点。
**业务影响**：分类决策无复核，可能存在错分类（如将 C 级判为 B 级降级销售规避返工成本）。
**修复建议**：增加分类复核审批节点，由独立质检员复核等级判定。

---

## 类二十二 维度 1：库存调拨跨库位跨缸号审计

### 检查方法
- Read `/workspace/backend/src/models/inventory_transfer.rs` / `inventory_transfer_item.rs` 全文。
- Read `/workspace/backend/src/services/inv/inventory_move.rs` / `batch.rs` 全文。
- Read `/workspace/backend/src/handlers/inventory_transfer_handler.rs` 全文。
- 检查调拨流程闭环、跨库位、跨缸号、分级审批。

### 发现

#### ✅ 已落实的项

1. **调拨流程闭环完整**：状态机 `pending → approved → shipped → completed`，含申请/审批/出库/在途/入库/确认全流程（`/workspace/backend/src/services/inv/inventory_move.rs:371-424`、`batch.rs:29-253`、`batch.rs:256-600`）。
2. **跨仓库调拨支持**：`from_warehouse_id`/`to_warehouse_id` 字段（`/workspace/backend/src/models/inventory_transfer.rs:15-16`）。
3. **在途库存独立核算**：发出时扣减源仓库 `quantity_on_hand`/`quantity_available`，接收时增加目标仓库，期间库存处于"在途"状态（status=shipped），目标仓库 `quantity_incoming` 字段可用于在途独立核算（`/workspace/backend/src/services/inv/batch.rs:73-220` 出库流程）。
4. **面料行业追溯字段**：`inventory_transfer_item` 含 `color_no`/`dye_lot_no`/`batch_no` 三字段（`/workspace/backend/src/models/inventory_transfer_item.rs:25-31`）。
5. **乐观锁防并发**：发出/接收用 `version` 字段做乐观锁，并发冲突时回滚（`/workspace/backend/src/services/inv/batch.rs:111-150`、`:345-384`）。
6. **事务完整性**：发出/接收全程事务包裹，失败回滚（`/workspace/backend/src/services/inv/batch.rs:35-36`、`:262-263`）。
7. **审批状态门**：只有 PENDING 状态可审批，已审核可发出，已发出可接收（`/workspace/backend/src/services/inv/inventory_move.rs:391-395`、`batch.rs:50-55`、`batch.rs:274-279`）。
8. **删除限制**：仅 pending/rejected 状态可删除，已审核/已发出/已完成不可删（`/workspace/backend/src/services/inv/inventory_move.rs:446-454`）。
9. **事件总线集成**：发出/接收后发布 `InventoryTransactionCreated` 事件触发财务凭证生成（`/workspace/backend/src/services/inv/batch.rs:180-249`、`:551-596`）。
10. **审计日志完整**：所有状态变更走 `update_with_audit` 记录操作人 user_id（`/workspace/backend/src/services/inv/inventory_move.rs:249-256` 等）。

#### ❌ 缺陷项

**缺陷 6.1：调拨审批无金额/数量分级**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/services/inv/inventory_move.rs:371-424`：`approve_transfer` 任何 user_id 都能审批任意金额/数量的调拨单，无"金额 > 1 万需经理审批"的分级审批逻辑。
- `inventory_transfer` 表无 `approval_level`/`approved_by_role` 字段记录审批层级。
- 审计计划 22.1"调拨必须按金额/数量分级审批（如 >1 万需经理审批）"未实现。
**业务影响**：大额调拨无上级复核，存在内部舞弊风险（如虚假调拨转移库存）。
**修复建议**：增加调拨金额计算（数量 × unit_cost），按金额阈值路由到不同审批层级；接入 BPM 服务做分级审批。

**缺陷 6.2：调拨明细未强制要求缸号（dye_lot_no 可空）**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/inventory_transfer_item.rs:28`：`dye_lot_no: Option<String>`（注释"白坯布调拨时为 NULL"）。
- `/workspace/backend/src/services/inv/inventory_move.rs:215-242`：创建调拨明细时 `dye_lot_no: NotSet`，**未从请求中接收 `dye_lot_no`**，DB 默认值处理。
- `/workspace/backend/src/services/inv/mod.rs:80-85`：`InventoryTransferItemRequest` DTO 无 `dye_lot_no`/`color_no`/`batch_no` 字段。
- 创建调拨明细时不强制要求缸号，可能发生混合缸号调拨。
**业务影响**：违反审计计划 22.1"调拨必须支持按缸号/匹号明细调拨，禁止混合缸号调拨"。混合缸号调拨会导致染色批次追溯断链，库存按缸号维度不可信。
**修复建议**：`InventoryTransferItemRequest` DTO 增加 `color_no`/`dye_lot_no`/`batch_no` 必填字段（白坯布除外），创建明细时强制写入，避免 DB 默认值 NULL。

**缺陷 6.3：调拨在途库存未独立核算**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/inv/batch.rs:73-220`：发出时直接扣减源仓库 `quantity_on_hand`/`quantity_available`，但**未增加源仓库的 `quantity_shipped` 或目标仓库的 `quantity_incoming`**。
- `inventory_stocks.quantity_incoming` 字段存在（`/workspace/backend/src/models/inventory_stock.rs:23`）但调拨发出流程未更新此字段。
- "在途库存"在两个仓库都不体现，调拨途中库存"消失"，账实不符。
**业务影响**：审计计划 22.1"调拨必须支持跨仓库/跨库位，在途库存独立核算"未完整实现。在途库存对账困难，盘点时无法解释差异。
**修复建议**：发出时增加目标仓库 `quantity_incoming`（在途量），接收时将 `quantity_incoming` 转 `quantity_on_hand`；或独立建 `inventory_in_transit` 表记录在途。

---

## 类二十二 维度 2：库存告警安全库存补货策略审计

### 检查方法
- Read `/workspace/backend/src/services/stock_alert.rs` 全文。
- Read `/workspace/backend/src/services/inventory_stock_query.rs:1-100` + `:350-411`。
- Grep `reorder_point|safety_stock|max_stock_point|reorder_quantity|补货策略|EOQ|订货点法` 全局检索。

### 发现

#### ✅ 已落实的项

1. **安全库存字段存在**：`inventory_stocks` 表有 `reorder_point`/`max_stock_point`/`reorder_quantity` 字段（`/workspace/backend/src/models/inventory_stock.rs:24-30`）。
2. **告警类型完整**：`AlertType` 枚举含 OutOfStock/LowStock/OverStock/Expiring/SlowMoving/Discrepancy 6 类（`/workspace/backend/src/services/stock_alert.rs:28-46`）。
3. **告警派生计算**：`compute_alert_type` 按优先级派生告警类型（`/workspace/backend/src/services/inventory_stock_query.rs:36-77`）。
4. **告警接口存在**：`GET /api/v1/erp/inventory/stock/alerts` 返回告警列表（`/workspace/backend/src/services/inventory_stock_query.rs:357-411`）。
5. **滞销告警**：90 天无库存变动视为滞销（`SLOW_MOVING_THRESHOLD_DAYS = 90`，`/workspace/backend/src/services/stock_alert.rs:88-94`）。
6. **即将过期告警**：30 天内过期视为即将过期（`EXPIRING_THRESHOLD_DAYS = 30`，`/workspace/backend/src/services/stock_alert.rs:82-86`）。

#### ❌ 缺陷项

**缺陷 7.1：仅一种"订货点法"补货策略，无 EOQ/MRP 多策略**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/inventory_stock.rs:24-30`：仅 `reorder_point`（订货点）+ `reorder_quantity`（固定补货量）字段，无 `replenishment_strategy` 字段配置策略类型。
- Grep `EOQ|economic_order_quantity|订货点法` 在 `backend/src` 中零业务结果。
- 审计计划 22.2"必须支持订货点法/EOQ/MRP 三种补货策略，按产品配置"未实现。
- MRP 引擎（`mrp_engine_service`）存在但与库存告警无自动联动，需手动调用。
**业务影响**：单一补货策略无法适应不同物料特性（如高价物料用 EOQ、低值物料用订货点法、关键物料用 MRP），可能造成库存积压或短缺。
**修复建议**：在 `inventory_stocks` 增加 `replenishment_strategy` 字段（enum: reorder_point/eoq/mrp），按策略实现不同补货建议算法。

**缺陷 7.2：告警无通知机制（站内信+邮件）**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/services/inventory_stock_query.rs:357-411`：`get_stock_alerts` 仅返回告警列表供前端查询，**无主动推送**（站内信/邮件）。
- Grep `stock_alert.*notification|inventory_alert.*notify|告警.*邮件` 零业务结果。
- `notification_service` 模块存在但未与库存告警集成。
- 审计计划 22.2"库存告警必须通知到采购员/计划员（站内信+邮件）"未实现。
**业务影响**：告警仅被动查询，无主动通知，告警可能被忽略导致补货延误。
**修复建议**：定时任务扫描告警，通过 `notification_service` 推送站内信 + 邮件给对应采购员/计划员。

**缺陷 7.3：告警无去重机制**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/inventory_stock_query.rs:357-411`：每次调用 `get_stock_alerts` 都返回全部告警，无 24h 去重。
- Grep `alert.*dedup|alert.*24h|告警去重` 零业务结果。
- 审计计划 22.2"同一产品 24h 内只告警一次，避免告警轰炸"未实现。
**业务影响**：告警可能反复推送造成骚扰，且每次查询全量告警性能差。
**修复建议**：建立 `inventory_alert_log` 表记录告警推送时间，定时任务扫描时跳过 24h 内已告警的产品。

**缺陷 7.4：告警无按仓库/缸号维度配置**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/models/inventory_stock.rs:24-30`：`reorder_point` 是库存记录级字段，按 (warehouse_id, product_id, batch_no, color_no, dye_lot_no) 组合设置。
- 缺少按"仓库+产品"维度配置安全库存的独立配置表（同一产品在不同仓库可有不同安全库存）。
- 审计计划 22.2"每个产品必须设置安全库存（按仓库/缸号）"——当前可按缸号设置但缺少统一配置管理界面。
**业务影响**：新增库存记录时需重复设置安全库存，配置维护成本高。
**修复建议**：建立 `safety_stock_config` 独立配置表（warehouse_id + product_id + 可选 color_no/dye_lot_no），新建库存记录时自动继承配置。

---

## 类二十二 维度 3：物料短缺预警闭环审计

### 检查方法
- Read `/workspace/backend/src/handlers/material_shortage_handler.rs` 全文。
- Read `/workspace/backend/src/services/material_shortage_service.rs:1-500`。
- 检查物料短缺识别、分级、处理闭环、报表。

### 发现

#### ✅ 已落实的项

1. **物料短缺识别完整**：`detect_shortages` 基于生产订单（SCHEDULED/IN_PROGRESS）+ 默认 BOM + 库存自动识别短缺（`/workspace/backend/src/services/material_shortage_service.rs:126-348`）。
2. **短缺分级完整**：`ShortageLevel` 枚举含 Critical（库存为 0）/ Severe（缺口 > 50%）/ Warning（缺口 ≤ 50%）/ Normal 四级（`/workspace/backend/src/services/material_shortage_service.rs:23-47`）。
3. **缺料汇总接口存在**：`get_shortage_summary` 按产品/日期范围汇总缺料（`/workspace/backend/src/handlers/material_shortage_handler.rs:97-131`）。
4. **缺料预警事件发布**：检测到缺料时发布 `MaterialShortageAlert` 事件（`/workspace/backend/src/services/material_shortage_service.rs:300-311`）。
5. **补货建议生成**：`generate_replenishment_suggestions` 按缺口 × 1.2 倍生成建议采购量，按严重程度排序（`/workspace/backend/src/services/material_shortage_service.rs:455-496`）。
6. **阈值配置接口**：`save_threshold_config`/`load_threshold_config` 支持配置安全倍率与严重/紧急阈值（`/workspace/backend/src/handlers/material_shortage_handler.rs:140-184`）。
7. **状态更新接口**：`update_shortage_status` 支持将状态从 pending → notified → resolved（`/workspace/backend/src/handlers/material_shortage_handler.rs:229-270`）。
8. **受影响订单追溯**：缺料项关联受影响生产订单列表（`AffectedOrder`，`/workspace/backend/src/services/material_shortage_service.rs:64-71`）。

#### ❌ 缺陷项

**缺陷 8.1：缺料预警状态不持久化，无法形成处理闭环**
**风险等级：P0**
**证据**：
- `/workspace/backend/src/services/material_shortage_service.rs:438-452`：`save_threshold_config` 注释"租户配置表已删除，配置不再持久化"，`load_threshold_config` 直接返回默认值。
- `/workspace/backend/src/services/material_shortage_service.rs:498+`：`update_status` 注释"租户配置表已删除，状态不再持久化，仅返回严重程度"。
- `/workspace/backend/src/handlers/material_shortage_handler.rs:229-270`：`update_shortage_status` 调用 service 的 `update_status`，但 service 不持久化，**仅返回 severity 字符串**，状态变更无效。
- 缺料检测每次都从生产订单 + BOM + 库存实时计算，无持久化缺料单据，无"识别→采购申请→采购订单→入库→解除"闭环。
- 审计计划 22.3"短缺必须走处理流程：识别→采购申请→采购订单→入库→解除"未实现。
**业务影响**：缺料预警是"一次性查询"而非"工单流转"，无法跟踪缺料处理进度，无法统计平均处理周期，无法关联采购订单验证解除。这是供应链闭环的核心能力缺失。
**修复建议**：建立 `material_shortage_order` 持久化表，记录缺料单号/物料/缺口/状态/关联采购申请 ID/关联采购订单 ID/解除时间；状态机推进 `identified → purchase_request → purchase_order → received → resolved`。

**缺陷 8.2：严重短缺无立即通知机制**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/services/material_shortage_service.rs:300-311`：仅发布 `MaterialShortageAlert` 事件，无主动通知 Critical 级别的逻辑。
- 事件订阅方是否调用 `notification_service` 推送站内信/邮件未在缺料服务中体现。
- 审计计划 22.3"严重短缺立即通知"未明确实现。
**业务影响**：Critical 级别缺料（库存为 0）若不及时处理会导致停产，无立即通知则响应延误。
**修复建议**：检测到 Critical 短缺时，同步调用 `notification_service` 推送站内信 + 邮件 + 短信给采购员与计划员。

**缺陷 8.3：无缺料月报能力**
**风险等级：P2**
**证据**：
- Grep `shortage.*monthly|缺料月报|shortage_report` 零业务结果。
- 缺料检测是实时计算，无按月汇总存储。
- 审计计划 22.3"必须有短缺月报（短缺次数/处理周期/影响生产），支持导出"未实现。
**业务影响**：管理层无法评估缺料频次与处理效率，无法识别高频缺料物料做战略备货。
**修复建议**：定时任务按月汇总缺料数据写入 `material_shortage_monthly_report` 表，提供查询与导出接口。

**缺陷 8.4：缺料检测未考虑在途采购入库**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/material_shortage_service.rs:393-414`：`get_material_stock_map` 仅查询 `quantity_available`，**未考虑 `quantity_incoming`（在途采购量）**。
- 对比 `mrp_engine_service.rs` 有 `consider_in_transit` 参数（`/workspace/backend/src/services/mrp_engine_service.rs:38`），缺料检测无此选项。
**业务影响**：已下单未入库的采购量未纳入可用量计算，可能产生"虚假缺料"导致重复采购。
**修复建议**：`get_material_stock_map` 增加 `consider_in_transit` 参数，可用量 = `quantity_available + quantity_incoming`。

---

## 类二十二 维度 4：自动排程算法合理性审计

### 检查方法
- Read `/workspace/backend/src/services/scheduling_auto.rs:1-300`。
- Read `/workspace/backend/src/services/scheduling_query.rs:1-200`。
- 检查排程算法、冲突检测、可视化、与生产订单集成。

### 发现

#### ✅ 已落实的项

1. **多排程策略支持**：`SchedulingAlgo` 枚举含 Fifo/Priority/Spt/Edd 四种策略（`/workspace/backend/src/services/scheduling_auto.rs:28-46`），`auto_schedule` 按策略排序（priority/fifo/earliest_due，`/workspace/backend/src/services/scheduling_auto.rs:90-99`）。
2. **产能约束检测**：检查工作中心可用产能是否充足，不足时记录 `CAPACITY_INSUFFICIENT` 冲突（`/workspace/backend/src/services/scheduling_auto.rs:170-190`）。
3. **时间重叠检测**：检测同工作中心时间重叠，记录 `TIME_OVERLAP` 冲突（`/workspace/backend/src/services/scheduling_auto.rs:209-228`）。
4. **最早可用时间槽查找**：`find_earliest_slot` 自动避开已排程订单（`/workspace/backend/src/services/scheduling_auto.rs:462-510`）。
5. **冲突检测接口存在**：`detect_conflicts` 检测 TIME_OVERLAP/MISSING_DATES/INVALID_DATES 三类冲突（`/workspace/backend/src/services/scheduling_auto.rs:268-354`）。
6. **排程结果持久化**：`save_schedule_result` 保存到 `scheduling_result` 表（`/workspace/backend/src/services/scheduling_auto.rs:356-429`）。
7. **甘特图数据生成**：`build_gantt_data` 生成 GanttData 含 items/work_centers/date_range（`/workspace/backend/src/services/scheduling_query.rs:299-366`）。
8. **排程确认集成生产订单**：`confirm_schedule_result` 将排程明细应用到生产订单，更新 `planned_start_date`/`planned_end_date`/`work_center_id`，状态从 DRAFT 升级为 SCHEDULED（`/workspace/backend/src/services/scheduling_query.rs:231-297`）。

#### ❌ 缺陷项

**缺陷 9.1：排程未基于缸号批量约束**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/services/scheduling_auto.rs:53-100`：`auto_schedule` 加载 `pending_orders` 后按 priority/created_at/due_date 排序，**未按缸号（dye_lot_no）分组**，未考虑"同一缸号必须连续排产"的批量约束。
- `ProductionOrderModel` 无 `dye_lot_no` 字段参与排程序号。
- 审计计划 22.4"排程算法必须基于产能约束 + 订单优先级 + 交期 + 缸号批量，支持自动排程"——缸号批量约束未实现。
**业务影响**：染色订单必须按缸号批量排产（同缸号合并染色降低能耗），当前按订单单独排程会导致同缸号订单分散到不同时段，增加换缸成本。
**修复建议**：排程前按缸号聚合订单，同缸号订单合并为一个排程单元分配工作中心与时间槽。

**缺陷 9.2：排程冲突无自动告警通知**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/scheduling_auto.rs:139-251`：检测到冲突仅记录到 `conflicts` 列表返回，无主动通知计划员。
- Grep `schedule_conflict.*notify|排程冲突.*通知` 零业务结果。
- 审计计划 22.4"排程必须检测冲突（如同一缸号同时排两单），冲突自动告警"未完整实现。
**业务影响**：冲突需人工查询才能发现，响应不及时可能导致排程延误。
**修复建议**：检测到 HIGH 严重度冲突时，通过 `notification_service` 推送站内信给计划员。

**缺陷 9.3：甘特图无拖拽调整后端接口**
**风险等级：P3**
**证据**：
- `/workspace/backend/src/services/scheduling_query.rs:39-122`：`get_gantt_data` 仅返回数据供前端展示。
- `/workspace/backend/src/services/scheduling_manual.rs:47-98`：`adjust_schedule` 接口存在但仅支持单订单字段更新（work_center_id/start_date/end_date/priority），无拖拽产生的"批量时间偏移"接口。
- 审计计划 22.4"排程必须支持甘特图展示，支持拖拽调整"——展示已实现，拖拽后端接口不完整。
**业务影响**：前端拖拽需多次调用 `adjust_schedule` 单订单接口，性能差且无原子性保证。
**修复建议**：增加 `batch_adjust_schedule` 接口支持批量更新多订单时间。

**缺陷 9.4：排程与生产订单集成存在重复录入风险**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/scheduling_auto.rs:53-88`：`auto_schedule` 加载 `pending_orders` 来自 `production_orders` 表（status=DRAFT）。
- 排程确认后 `confirm_schedule_result` 将 DRAFT 升级为 SCHEDULED（`/workspace/backend/src/services/scheduling_query.rs:277-279`）。
- 但**未禁止人工直接创建 SCHEDULED 状态生产订单**，存在绕过排程直接录入的风险。
- 审计计划 22.4"排程结果必须自动生成生产订单，禁止手工重复录入"未完整保障。
**业务影响**：人工创建 SCHEDULED 订单绕过排程，可能导致产能超载或时间冲突未被检测。
**修复建议**：生产订单创建接口强制初始状态为 DRAFT，必须经排程确认才能升级为 SCHEDULED。

---

## 类二十二 维度 5：产能规划与瓶颈识别审计

### 检查方法
- Read `/workspace/backend/src/services/capacity_service.rs:1-500`。
- 检查产能模型、负荷计算、瓶颈识别、报表。

### 发现

#### ✅ 已落实的项

1. **产能模型存在**：`work_centers` 表含 `daily_capacity`/`capacity_unit`/`status`/`work_center_type` 字段（`/workspace/backend/src/models/work_center.rs:30-64`）。
2. **产能负荷计算**：`load_analysis` 计算各工作中心 `planned_quantity`（SCHEDULED）+ `in_progress_quantity`（IN_PROGRESS），`load_rate = total_demand / daily_capacity * 100`（`/workspace/backend/src/services/capacity_service.rs:170-264`）。
3. **负荷分级**：load_rate > 100% → OVERLOADED，> 80% → HIGH，> 20% → NORMAL，≤ 20% → IDLE（`/workspace/backend/src/services/capacity_service.rs:237-245`）。
4. **瓶颈识别**：`overview` 识别 load_rate > 80% 的工作中心为瓶颈（`/workspace/backend/src/services/capacity_service.rs:289-294`）。
5. **产能概览**：`overview` 返回总工作中心数/活跃数/总日产能/总计划需求/整体负荷率/瓶颈列表/超载数/闲置数（`/workspace/backend/src/services/capacity_service.rs:267-313`）。
6. **班次配置**：`default_shifts_for_type` 按 CONTINUOUS/STANDARD 配置不同班次（`/workspace/backend/src/services/capacity_service.rs:316-339`）。
7. **产能预测**：`forecast_capacity` 基于历史数据预测未来产能负荷（`/workspace/backend/src/services/capacity_service.rs:434-490`）。

#### ❌ 缺陷项

**缺陷 10.1：产能模型缺少标准工时/设备数/人员数字段**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/work_center.rs:30-64`：`work_centers` 表仅有 `daily_capacity`/`capacity_unit` 字段，**无 `standard_hours_per_unit`（标准工时）/ `equipment_count`（设备数）/ `worker_count`（人员数）/ `shift_hours`（班次工时）字段**。
- `daily_capacity` 是综合产能指标，无法拆解到工时/设备/人员维度。
- 审计计划 22.5"必须按工作中心建模产能（标准工时/班次/设备数）"未完整实现。
**业务影响**：无法做精细产能分析（如某设备故障影响多少产能、人员不足是否瓶颈），产能规划粗放。
**修复建议**：扩展 `work_centers` 表增加 `standard_hours_per_unit`/`equipment_count`/`worker_count`/`shift_hours` 字段，`daily_capacity` 由这些字段派生计算。

**缺陷 10.2：负荷 > 80% 无自动告警**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/capacity_service.rs:237-245`：仅将 load_rate > 80% 标记为 HIGH 状态，**无主动告警推送**。
- `CapacityOverloadAlert` 结构体定义存在（`/workspace/backend/src/services/capacity_service.rs:39-50`）但 Grep 显示无业务调用方发布告警事件。
- 审计计划 22.5"负荷 > 80% 告警"未实现主动通知。
**业务影响**：产能瓶颈需人工查询才能发现，响应延误可能导致订单延误。
**修复建议**：定时任务扫描 HIGH/OVERLOADED 工作中心，发布 `CapacityOverloadAlert` 事件并通知计划员。

**缺陷 10.3：瓶颈识别仅按负荷率，无扩产/外包建议**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/capacity_service.rs:289-294`：瓶颈识别仅过滤 load_rate > 80%，**无扩产/外包建议生成**。
- 审计计划 22.5"必须自动识别瓶颈工作中心（负荷最高），建议扩产/外包"未完整实现。
**业务影响**：识别瓶颈后无后续行动建议，瓶颈长期存在无法缓解。
**修复建议**：瓶颈工作中心生成建议（如"建议外包 X 单位产能"或"建议增加 Y 台设备"），写入 `capacity_suggestion` 表。

**缺陷 10.4：无产能月报能力**
**风险等级：P2**
**证据**：
- Grep `capacity.*monthly|产能月报|capacity_report` 零业务结果。
- 产能负荷是实时计算，无按月汇总存储。
- 审计计划 22.5"必须有产能月报（各工作中心负荷/利用率/瓶颈），支持导出"未实现。
**业务影响**：无法做产能长期趋势分析，无法识别季节性瓶颈。
**修复建议**：定时任务按月汇总产能数据写入 `capacity_monthly_report` 表，提供查询与导出接口。

---

## 类二十二 维度 6：工作中心调度与排程集成审计

### 检查方法
- Read `/workspace/backend/src/models/work_center.rs` 全文。
- Read `/workspace/backend/src/services/scheduling_manual.rs` 全文。
- Read `/workspace/backend/src/services/capacity_service.rs:100-170`。
- 检查工作中心模型、调度规则、排程下发、异常处理。

### 发现

#### ✅ 已落实的项

1. **工作中心模型存在**：`work_centers` 表含 `code`/`name`/`work_center_type`/`daily_capacity`/`capacity_unit`/`status`/`remarks`（`/workspace/backend/src/models/work_center.rs:30-64`）。
2. **工作中心 CRUD 完整**：`list_work_centers`/`get_work_center`/`create_work_center`/`update_work_center`/`delete_work_center`（软删除改 status=INACTIVE，`/workspace/backend/src/services/capacity_service.rs:122-431`）。
3. **工作中心状态枚举**：Active/Maintenance/Inactive 三态（`/workspace/backend/src/models/work_center.rs:13-25`）。
4. **手动调整接口存在**：`adjust_schedule` 支持调整 work_center_id/start_date/end_date/priority（`/workspace/backend/src/services/scheduling_manual.rs:47-98`）。
5. **调整类型枚举**：MoveUp/MoveDown/MoveTop/MoveBottom/Lock/Unlock（`/workspace/backend/src/services/scheduling_manual.rs:20-41`）。
6. **排程结果自动下发工作中心**：`confirm_schedule_result` 将排程明细的 `work_center_id` 写入生产订单（`/workspace/backend/src/services/scheduling_query.rs:273`）。

#### ❌ 缺陷项

**缺陷 11.1：工作中心未关联设备/人员/班次实体**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/models/work_center.rs:30-64`：`work_centers` 表无 `equipment_ids`/`worker_ids`/`shift_ids` 字段，无关联实体表。
- `default_shifts_for_type`（`/workspace/backend/src/services/capacity_service.rs:316-339`）返回硬编码班次配置（CONTINUOUS→早晚班，其他→白班），**无班次实体表**。
- `ShiftInfo` 结构体存在但无持久化（`/workspace/backend/src/services/capacity_service.rs:66-72`）。
- 无 `work_center_equipment`/`work_center_worker`/`work_center_shift` 关联表。
- 审计计划 22.6"工作中心必须关联设备/人员/班次，支持多技能人员"未实现。
**业务影响**：无法精确定位产能瓶颈是设备、人员还是班次问题，无法做人员技能矩阵管理，调度优化受限。
**修复建议**：建立 `work_center_equipment`/`work_center_worker`/`work_center_shift` 三张关联表，`work_center_worker` 增加 `skills` 字段支持多技能。

**缺陷 11.2：调度规则未实现 FIFO/SPT/EDD 全覆盖**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/scheduling_manual.rs:20-41`：`AdjustType` 仅含 MoveUp/MoveDown/MoveTop/MoveBottom/Lock/Unlock，**无 FIFO/SPT/EDD 调度规则应用**。
- `/workspace/backend/src/services/scheduling_auto.rs:91-99`：`auto_schedule` 支持策略字符串 `priority`/`fifo`/`earliest_due`，但**无 `spt`（最短加工时间）**策略分支（虽然 `SchedulingAlgo::Spt` 枚举存在但 auto_schedule match 中无 "spt" 分支）。
- 调度规则不可配置（仅通过请求参数 algo 字符串选择，无工作中心级规则配置）。
- 审计计划 22.6"调度必须按规则（FIFO/SPT/EDD/优先级），规则可配置"未完整实现。
**业务影响**：SPT 策略缺失，无法按最短加工时间优先优化吞吐；规则不可在工作中心级配置，不同工作中心只能用同一规则。
**修复建议**：补全 `spt` 策略分支（按 planned_quantity / daily_capacity 升序）；在 `work_centers` 表增加 `default_scheduling_rule` 字段，工作中心级配置默认规则。

**缺陷 11.3：调度异常无自动重排**
**风险等级：P1**
**证据**：
- Grep `reschedule|重排|设备故障.*重排|人员请假.*重排` 零业务结果。
- `/workspace/backend/src/services/scheduling_manual.rs:47-98`：`adjust_schedule` 仅手动调整单订单，无设备故障/人员请假触发的自动重排。
- 工作中心状态改为 Maintenance 时无关联生产订单自动重排逻辑。
- 审计计划 22.6"调度异常（设备故障/人员请假）必须自动重排，通知计划员"未实现。
**业务影响**：设备故障时受影响订单需人工逐个调整，响应慢导致订单延误；无自动重排可能造成故障设备继续被排产。
**修复建议**：工作中心状态变更为 Maintenance 时，自动调用 `auto_schedule` 重排该工作中心的所有 SCHEDULED 订单到其他可用工作中心，并通知计划员。

**缺陷 11.4：排程与调度集成存在手工转移风险**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/scheduling_query.rs:231-297`：`confirm_schedule_result` 将排程结果应用到生产订单，但**未禁止人工直接修改生产订单的 `work_center_id`/`planned_start_date`/`planned_end_date`**绕过排程。
- `/workspace/backend/src/services/scheduling_manual.rs:47-98`：`adjust_schedule` 是手动调整接口，与排程结果无版本关联，可能覆盖排程结果而无审计。
- 审计计划 22.6"排程结果必须自动下发到工作中心，禁止手工转移"未完整保障。
**业务影响**：人工修改订单工作中心绕过排程，可能导致工作中心产能超载未被发现。
**修复建议**：生产订单 `work_center_id`/`planned_start_date`/`planned_end_date` 修改必须经 `adjust_schedule` 接口走审计日志，禁止直接 update。

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 21.1 胚布库存与采购管理 | 0 | 2 | 1 | 0 | 5 | 8 |
| 21.2 胚布委托加工流转 | 0 | 2 | 1 | 0 | 6 | 9 |
| 21.3 拆匹后缸号匹号继承规则 | 0 | 2 | 1 | 1 | 8 | 12 |
| 21.4 质量问题 8D 处理流程 | 1 | 2 | 1 | 0 | 4 | 8 |
| 21.5 不合格品降级返工报废流程 | 1 | 2 | 1 | 0 | 5 | 9 |
| 22.1 库存调拨跨库位跨缸号 | 0 | 2 | 1 | 0 | 10 | 13 |
| 22.2 库存告警安全库存补货策略 | 0 | 2 | 2 | 0 | 6 | 10 |
| 22.3 物料短缺预警闭环 | 1 | 1 | 2 | 0 | 8 | 12 |
| 22.4 自动排程算法合理性 | 0 | 1 | 2 | 1 | 8 | 12 |
| 22.5 产能规划与瓶颈识别 | 0 | 1 | 3 | 0 | 7 | 11 |
| 22.6 工作中心调度与排程集成 | 0 | 2 | 2 | 0 | 6 | 10 |
| **合计** | **3** | **19** | **15** | **2** | **73** | **114** |

---

## 修复优先级队列

### P0（阻塞级，3 项）

1. **缺陷 4.1**：质量异常未走 8D 流程，仅 open/resolved/closed 三态 — `/workspace/backend/src/models/quality_issue.rs:11-25` + `/workspace/backend/src/services/custom_order_quality_service.rs:109-146`
2. **缺陷 5.2**：返工未走生产订单（返工工单）— `/workspace/backend/src/services/quality_inspection_service.rs:351-391`
3. **缺陷 8.1**：缺料预警状态不持久化，无法形成处理闭环 — `/workspace/backend/src/services/material_shortage_service.rs:438-452,498+` + `/workspace/backend/src/handlers/material_shortage_handler.rs:229-270`

### P1（高级，19 项）

4. **缺陷 1.1**：胚布未走采购订单流程，无关联采购订单字段 — `/workspace/backend/src/models/greige_fabric.rs:13-52`
5. **缺陷 1.2**：胚布库存无安全库存预警字段 — `/workspace/backend/src/models/greige_fabric.rs:13-52`
6. **缺陷 2.1**：委外发料未关联胚布（greige_fabric） — `/workspace/backend/src/models/outsourcing_order.rs:22-102`
7. **缺陷 2.2**：委外收回未走质检流程 — `/workspace/backend/src/services/outsourcing_service.rs:552-635`
8. **缺陷 3.1**：拆匹后子匹未继承 `dye_lot_no` 字符串字段 — `/workspace/backend/src/handlers/piece_split_handler.rs:118-119`
9. **缺陷 3.3**：`piece_mapping` 表存在但无业务代码引用 — `/workspace/backend/src/models/piece_mapping.rs:1-66`
10. **缺陷 4.2**：未使用 5Why/鱼骨图等根因分析方法 — 全局缺失
11. **缺陷 4.3**：纠正预防措施无责任人和完成日期跟踪 — `/workspace/backend/src/models/quality_issue.rs:11-25`
12. **缺陷 5.1**：降级处理未联动库存等级与价格调整 — `/workspace/backend/src/services/quality_inspection_service.rs:351-391`
13. **缺陷 5.3**：报废未走二级审批（财务+总经理） — `/workspace/backend/src/services/quality_inspection_service.rs:351-391`
14. **缺陷 6.1**：调拨审批无金额/数量分级 — `/workspace/backend/src/services/inv/inventory_move.rs:371-424`
15. **缺陷 6.2**：调拨明细未强制要求缸号（dye_lot_no 可空） — `/workspace/backend/src/models/inventory_transfer_item.rs:28` + `/workspace/backend/src/services/inv/mod.rs:80-85`
16. **缺陷 7.1**：仅一种"订货点法"补货策略，无 EOQ/MRP 多策略 — `/workspace/backend/src/models/inventory_stock.rs:24-30`
17. **缺陷 7.2**：告警无通知机制（站内信+邮件） — `/workspace/backend/src/services/inventory_stock_query.rs:357-411`
18. **缺陷 8.2**：严重短缺无立即通知机制 — `/workspace/backend/src/services/material_shortage_service.rs:300-311`
19. **缺陷 9.1**：排程未基于缸号批量约束 — `/workspace/backend/src/services/scheduling_auto.rs:53-100`
20. **缺陷 10.1**：产能模型缺少标准工时/设备数/人员数字段 — `/workspace/backend/src/models/work_center.rs:30-64`
21. **缺陷 11.1**：工作中心未关联设备/人员/班次实体 — `/workspace/backend/src/models/work_center.rs:30-64`
22. **缺陷 11.3**：调度异常无自动重排 — 全局缺失

### P2（中级，15 项）

23. **缺陷 1.3**：胚布批次追溯字段不全 — `/workspace/backend/src/models/greige_fabric.rs:29`
24. **缺陷 2.3**：委外加工费未按缸号/匹号核算 — `/workspace/backend/src/services/outsourcing_service.rs:223-238`
25. **缺陷 3.2**：拆匹数量之和未做等于原匹数量的强校验 — `/workspace/backend/src/handlers/piece_split_handler.rs:60-76`
26. **缺陷 4.4**：无 8D 月报能力 — 全局缺失
27. **缺陷 5.4**：不合格品分类无审批 — `/workspace/backend/src/services/quality_inspection_service.rs:351-391`
28. **缺陷 6.3**：调拨在途库存未独立核算 — `/workspace/backend/src/services/inv/batch.rs:73-220`
29. **缺陷 7.3**：告警无去重机制 — `/workspace/backend/src/services/inventory_stock_query.rs:357-411`
30. **缺陷 7.4**：告警无按仓库/缸号维度配置 — `/workspace/backend/src/models/inventory_stock.rs:24-30`
31. **缺陷 8.3**：无缺料月报能力 — 全局缺失
32. **缺陷 8.4**：缺料检测未考虑在途采购入库 — `/workspace/backend/src/services/material_shortage_service.rs:393-414`
33. **缺陷 9.2**：排程冲突无自动告警通知 — `/workspace/backend/src/services/scheduling_auto.rs:139-251`
34. **缺陷 9.4**：排程与生产订单集成存在重复录入风险 — `/workspace/backend/src/services/scheduling_query.rs:231-297`
35. **缺陷 10.2**：负荷 > 80% 无自动告警 — `/workspace/backend/src/services/capacity_service.rs:237-245`
36. **缺陷 10.3**：瓶颈识别仅按负荷率，无扩产/外包建议 — `/workspace/backend/src/services/capacity_service.rs:289-294`
37. **缺陷 10.4**：无产能月报能力 — 全局缺失
38. **缺陷 11.2**：调度规则未实现 FIFO/SPT/EDD 全覆盖 — `/workspace/backend/src/services/scheduling_auto.rs:91-99`
39. **缺陷 11.4**：排程与调度集成存在手工转移风险 — `/workspace/backend/src/services/scheduling_query.rs:231-297`

### P3（低级，2 项）

40. **缺陷 3.4**：拆匹未生成新匹号而用 `{parent.piece_no}-CUT-{timestamp}` — `/workspace/backend/src/handlers/piece_split_handler.rs:79-85`
41. **缺陷 9.3**：甘特图无拖拽调整后端接口 — `/workspace/backend/src/services/scheduling_manual.rs:47-98`

---

## 审计结论

本次审计覆盖类二十一（胚布拆匹与质量处理）5 维度 + 类二十二（库存排程物料）6 维度，共 11 维度 114 检查项，发现 **3 项 P0 阻塞缺陷、19 项 P1 高优先级缺陷、15 项 P2 中级缺陷、2 项 P3 低级缺陷**，已落实 73 项。

**核心能力缺失**：
1. **8D 质量管理流程完全缺失**（缺陷 4.1）：质量异常仅做"上报→解决"两步，无根因分析、无永久措施、无预防机制、无 8D 月报，是面料行业质量管理的核心短板。
2. **返工工单流程完全缺失**（缺陷 5.2）：C 级不合格品返工仅记录文本，无工单跟踪、无成本归集、无质检闭环。
3. **缺料预警闭环完全缺失**（缺陷 8.1）：缺料检测是"一次性查询"而非"工单流转"，无持久化、无状态机、无处理跟踪，租户配置表已删除导致状态不持久化。

**追溯链路断裂**：
- 胚布未走采购订单流程（缺陷 1.1）
- 委外发料未关联胚布（缺陷 2.1）
- 拆匹未继承 dye_lot_no 字符串（缺陷 3.1）
- 调拨明细未强制缸号（缺陷 6.2）
- piece_mapping 表死代码（缺陷 3.3）

**建议优先处理 P0 缺陷**，这三项是行业核心能力缺失，影响业务闭环完整性。
