# 冰溪 ERP 纺织行业专属功能评估报告

> 评估日期：2026-06-16
> 评估范围：5 大纺织行业专属功能
> 评估方法：数据库表结构 + 后端代码 + 前端页面 三层证据链
> 评估原则：只评估不修改，不写代码、不创建 commit、不运行构建

---

## 1. 评估总览

| 功能 | 状态 | 完整度 | 优先级 |
|------|------|--------|--------|
| 面料多色号定价 | ⚠️ 部分实现 | 50% | P1 |
| 色卡仓储管理 | ❌ 未实现 | 0% | P2 |
| 面料仓储管理 | ✅ 完整实现 | 90% | P1 |
| 全链路销售报价单 | ❌ 未实现 | 0% | P0 |
| 定制订单全流程跟踪 | ❌ 未实现 | 0% | P0 |

**整体结论**：项目在"面料行业基础特性（色号/缸号/匹号/批次）"上做了较深适配，但在**面向客户的业务能力**（销售报价、定制订单全流程）和**色卡物理管理**上仍是空白。

---

## 2. 详细评估

### 2.1 面料多色号定价

**状态**：⚠️ 部分实现

#### 现有实现

- **数据模型**：
  - 数据库表 `product_colors`（`001_consolidated_schema.sql:660-685`）已建立：色号、颜色名称、潘通色号、颜色类型（常规/定制）、染色配方、`extra_cost`（特殊色号加价）、`is_active` 状态等字段齐全。
  - 库存表 `inventory_stocks` 增加了 `color_no` 字段（`001_consolidated_schema.sql:221, 242, 753, 775`），并建立了 `idx_inventory_batch_color` 与 `idx_inventory_warehouse_batch` 联合索引（行 799-800）。
  - 销售订单明细 `sales_order_items` 有 `color_no`、`pantone_code`、`color_extra_cost` 字段（`001_consolidated_schema.sql:870, 887, 897`）。
  - 库存事务表 `inventory_transactions` 同步有色号（`001_consolidated_schema.sql:809, 837`）。
  - 数据库迁移 `028_create_dye_batch_greige_fabric.sql:46` 也保留 `color_no`。

- **后端模型**：
  - `backend/src/models/product_color.rs`（存在）
  - `backend/src/models/color_code_mapping.rs`（存在）
  - `backend/src/models/supplier_product_color.rs`（存在）
  - `backend/src/handlers/product_handler.rs:97-410` 提供完整的色号 CRUD 端点：`list_product_colors`、`create_product_color`、`update_product_color`、`delete_product_color`、`batch_create_colors`。
  - `sales_order` service 中有 `color_extra_cost`、`grade_price_diff`、`final_price` 字段（`backend/src/services/so/mod.rs:92-94, 163-165`、`order.rs:137-141, 239-244, 504-505`）。

- **前端 API**：
  - `frontend/src/api/product.ts:21-97` 提供 `ProductColor` 接口（含 `price_adjustment` 字段）及 `getColors`、`createColor`、`updateColor`、`deleteColor`、`batchCreateColors` 五个方法。

#### 关键缺口

1. **没有 `product_color_prices` 表（每色号一价）**：当前采用 `product_colors.extra_cost`（加价）模式，即"产品基础价 + 色号加价"。这不能支持"色号 A 卖 30 元/米，色号 B 卖 38 元/米"这种**基础价差异**场景。
2. **价格表设计简单**：前端 `ProductColor.price_adjustment` 字段仅支持加价幅度（绝对值），未支持**百分比加价、阶梯价、含税价、币种**等纺织行业常见价格策略。
3. **按色号查价格的接口缺失**：未发现 `GET /products/:id/colors/:color_id/price` 这类查询色号当前生效价的端点。
4. **色号定价管理 UI 缺失**：色号管理页面可能与产品管理混杂，无独立的"色号价格簿"管理界面。
5. **无阶梯价表设计**：未发现 `step_prices` 或 `tier_prices` 表，无法实现"采购 1000 米以上 9 折"等阶梯定价。

#### 改进建议

- 建议 1：在 `product_colors` 表中增加 `unit_price`（色号基础单价）字段，或新建 `product_color_prices` 表（按色号 × 币种 × 客户等级 × 有效期四维度建表）。
- 建议 2：增加 `color_price_steps` 表，支持数量阶梯价。
- 建议 3：在 `sales_price_service` 中扩展，按色号 + 客户 + 数量返回最终价。
- 建议 4：新增"色号价格管理"前端 Tab，含价格簿、生效时间、含税/不含税切换。
- 建议 5：在销售订单创建时，自动按所选色号 + 客户等级 + 数量查出"色号生效价 + 加价"。

---

### 2.2 色卡仓储管理

**状态**：❌ 未实现

#### 现有实现

- 无任何相关代码与表结构。

#### 证据

- 数据库迁移文件 0 处命中 `color_card | color_sample | swatch | palette | 色卡 | 色本 | 色板`（`/workspace/database/migration` 全部 30+ 文件）。
- 后端 src 0 处命中（`/workspace/backend/src` 全树）。
- 前端 src 0 处命中（`/workspace/frontend/src` 全树）。
- 前端 view 目录无 card/swatch 相关页面。
- 后端 handler 目录无 card/swatch 相关文件。

#### 关键缺口

1. **数据模型完全缺失**：无 `color_cards`（色卡基本信息表）、`color_card_items`（色卡内色号列表）、`color_card_inventory`（色卡库存表）。
2. **仓储逻辑缺失**：色卡无法作为库存物料进行入库、领用、归还、报废。
3. **业务规则缺失**：色卡编号、版本（如 2024 春夏版）、物理位置、客户借出归还等流程完全没有。
4. **色卡 vs 色号概念混淆**：当前 `product_colors` 表是"色号"（颜色编号），与色卡（物理样本册）属于不同业务对象，但系统未对色卡做物理管理。
5. **UI 完全缺失**：无色卡列表、领用、归还、盘点页面。

#### 改进建议

- 建议 1：新增 `color_cards` 表（id、code、name、version、publish_date、status、location、quantity、remark）。
- 建议 2：新增 `color_card_items` 表（card_id、color_no、position、pantone_code、sample_image_url），实现"色卡包含哪些色号"。
- 建议 3：将色卡纳入 `inventory_stocks` 框架（作为 `material_type = 'COLOR_CARD'` 的特殊物料）。
- 建议 4：新增 `color_card_lending` 表（card_id、customer_id、lend_date、return_date、status、handler），跟踪借出归还。
- 建议 5：前端新增"色卡管理"模块（列表/详情/借出/归还/盘点）。
- 建议 6：在销售员"打色样"动作时，可一键生成色卡领用单。

---

### 2.3 面料仓储管理

**状态**：✅ 完整实现（高度完整）

#### 现有实现

- **四级批次管理**（`001_consolidated_schema.sql:4679-5100`）:
  - `batch_dye_lot`（缸号管理表，行 4687-4737）：含 `dye_lot_no`（内部缸号）、`supplier_dye_lot_no`（供应商缸号）、`product_id`、`color_id`、`supplier_id`、`quality_grade`（A/B/C/D）、`quality_status`、生产日期等。
  - `inventory_piece`（库存匹号表，行 4744-4784）：含 `piece_no`（内部匹号）、`dye_lot_id`、`supplier_piece_no`、长度、重量、质量状态、库存状态。
  - `dye_lot_mapping`（缸号映射表，行 4871-4906）：内部缸号 ↔ 供应商缸号对照。
  - `piece_mapping`（匹号映射表，行 4908-4942）：内部匹号 ↔ 供应商匹号对照。
  - `batch_trace_log`（批次追溯日志表，行 4959-4989）：含 `internal_piece_nos[]`、`supplier_piece_nos[]` 数组字段，记录溯源链路。
  - 触发器与函数：自动生成匹号（行 5039-5044）、缸号总匹数自动更新（行 5021-5038）。

- **业务表同步升级**：
  - `inventory_stocks` 增加 `color_no`、`batch_no`、`dye_lot_no`（行 220-222, 751-800）。
  - `inventory_transactions` 增加 `color_no`、`batch_no`、`dye_lot_no`（行 808-810, 836-843）。
  - `sales_order_items` 增加 `color_no`、`pantone_code`、`color_extra_cost`、`dye_lot_requirement`、`allocated_dye_lot_ids[]`、`allocated_piece_ids[]`（行 870-897, 5177-5184）。
  - `sales_delivery_item` 增加 `dye_lot_id`、`dye_lot_no`、`piece_ids[]`、`piece_nos[]`（行 5265-5299）。
  - `purchase_receipt_item` 增加 `internal_dye_lot_id`、`internal_piece_ids[]`、`supplier_dye_lot_no`、`supplier_piece_nos[]`（行 5321-5346）。

- **后端模型**：
  - `backend/src/models/batch_dye_lot.rs`
  - `backend/src/models/inventory_piece.rs`
  - `backend/src/models/dye_lot_mapping.rs`
  - `backend/src/models/piece_mapping.rs`
  - `backend/src/models/batch_trace_log.rs`
  - 均在 `backend/src/models/mod.rs` 注册。

- **后端 handler**：
  - `backend/src/handlers/piece_split_handler.rs`（**母卷剪裁**）：支持 `split_fabric_piece`，从母卷剪裁出新布卷（含长度/重量/条形码），状态检查、长度校验、事务一致性。
  - `backend/src/handlers/barcode_scanner_handler.rs`（条码扫描）。
  - `backend/src/handlers/inventory_batch_handler.rs`（批次管理）。

- **前端 UI**：
  - `frontend/src/views/inventoryBatch/index.vue`：批次管理页面（已实现，header 含批次号/色号/等级三个筛选项，行 12-39）。
  - `frontend/src/views/inventoryAdjustment`、`inventoryTransfer`、`inventoryCount`：库存调整/调拨/盘点页面。
  - `frontend/src/views/warehouse`：仓库管理页面。
  - `frontend/src/views/fiveDimension/index.vue`：5 维度查询（产品 × 色号 × 缸号 × 匹号 × 批次）。
  - `frontend/src/views/barcodeScanner/index.vue`：条码扫描出入库。

#### 关键缺口

1. **母卷剪裁页面缺失**：后端有 `piece_split_handler`，前端 `inventoryBatch` 页面未见剪裁入口。
2. **缸号/匹号追溯图谱 UI 缺失**：数据模型支持，但前端未发现"批次追溯图"或"匹号树状图"页面。
3. **等级管理薄弱**：库存表有 `grade` 字段，但未见专门的"次品/不合格品"入库与转正品流程页面。
4. **跨缸/同缸策略执行**：`sales_order_items.dye_lot_requirement`（同缸/可混缸）数据存在，但前端"分配缸号"交互流程未完全实现。

#### 改进建议

- 建议 1：前端批次管理页面增加"剪裁母卷"按钮，调用 `split_fabric_piece` 端点。
- 建议 2：新增"批次追溯"页面（产品 ↔ 色号 ↔ 缸号 ↔ 匹号 四级树状图）。
- 建议 3：实现销售订单"分配缸号/匹号"前端交互（拖拽式分配）。
- 建议 4：增加等级（正品/次品）调拨流程页面。
- 建议 5：补齐"批次成本归集"与"批次毛利"报表。

---

### 2.4 全链路销售报价单

**状态**：❌ 未实现

#### 现有实现

- **无报价单实体**：
  - 数据库 0 处命中 `sales_quotation | quotation | quote_header | quote_item`。
  - 唯一命中在 `001_consolidated_schema.sql:6771` 的 CRM 销售漏斗 `{"stage": "proposal", "name": "方案报价", "probability": 50}`，是销售机会阶段枚举，不是报价单实体。
- **后端无报价单服务与 handler**：
  - `backend/src/handlers/` 无 `quote_handler.rs` 或 `quotation_handler.rs`。
  - `backend/src/services/` 无 `sales_quotation_service.rs` 或 `quote_service.rs`。
- **前端无报价单页面**：
  - `frontend/src/views/` 无 `quote`、`报价`、`quotation` 目录。
  - 仅 `purchase-price`（采购价格）与 `sales-price`（销售价格）是价格管理页面，**不是报价单**。

#### 现有相关（但不等同）

- `sales_price_service.rs`（`backend/src/services/sales_price_service.rs`）：管理**销售价格策略**（产品级定价），含 `get_prices_list`、`create_price`、`approve_price`、`activate_price`、`list_strategies`、`get_current_price` 等。
- `purchase_price_service.rs`：采购价格策略。
- 前端 `frontend/src/views/sales-price/index.vue`：销售价格维护。

#### 关键缺口

1. **数据模型完全缺失**：
   - 无 `sales_quotations`（报价单主表）：缺 `quote_no`、`customer_id`、`salesperson_id`、`quote_date`、`valid_until`、`currency`、`tax_rate`、`status`（draft/pending/approved/rejected/expired/converted）、`total_amount`、`approval_id`。
   - 无 `sales_quotation_items`（报价单明细）：缺 `product_id`、`color_id`、`quantity`、`unit`、`unit_price`、`discount`、`amount`、`delivery_date`。
   - 无 `sales_quotation_terms`（报价条款）：含付款方式、交货期、备注。
2. **价格策略能力缺失**：阶梯价、含税/不含税、多币种、客户等级价、有效期失效均无数据模型支撑。
3. **审批流未集成**：未发现 `sales_quotation_approval` 表或 BPM 流程模板。
4. **报价转订单未贯通**：无"报价单 → 销售订单"一键转换的接口或前端按钮。
5. **UI 完全缺失**：无报价单列表/创建/详情/审批/打印/导出页面。
6. **历史与版本缺失**：无报价单修订历史、版本号、变更记录。

#### 改进建议

- 建议 1：新增 `sales_quotations` + `sales_quotation_items` + `sales_quotation_terms` 三表（建议放在 `database/migration/031_create_sales_quotation.sql`）。
- 建议 2：在 `sales_quotations` 中增加 `currency`、`exchange_rate`、`tax_rate`、`is_tax_inclusive` 字段。
- 建议 3：复用现有 BPM 引擎（`bpm_service.rs`），为报价单配置 `sales_quotation_approval` 流程模板。
- 建议 4：新增 `SalesQuotationService`，支持阶梯价计算（依赖"色号定价"功能）。
- 建议 5：新增 `sales_quotation_handler.rs`，含 8 个端点：列表、详情、创建、更新、提交审批、审批、拒绝、转销售订单、打印。
- 建议 6：前端新增 `views/sales-quotation/` 模块：列表（按状态/客户/销售员筛选）、创建向导（选客户→选面料→选色号→自动算价）、详情（含审批轨迹、打印预览）。
- 建议 7：报价单到期前 N 天自动提醒销售员跟进。

---

### 2.5 定制订单全流程跟踪

**状态**：❌ 未实现

#### 现有实现

- **无定制订单实体**：
  - 数据库 0 处命中 `custom_order | process_track | process_node | production_node`。
  - 仅有 4 处"定制"相关：`001_consolidated_schema.sql:652`（`finish` 字段"后整理"）、`679`（`color_type` 字段"定制色"）、`945-946`（种子数据"定制色"）。
- **后端无定制订单服务与 handler**：
  - `backend/src/handlers/` 无 `custom_order_handler.rs`。
  - `backend/src/services/` 无 `custom_order_service.rs` 或 `custom_process_service.rs`。
- **现有 production 模块不覆盖定制场景**：
  - `backend/src/services/production_order_service.rs`：基于销售订单的**标准化生产**（状态机：created → released → in_progress → completed），不含客户定制工艺。
  - `backend/src/handlers/production_order_handler.rs`：同上。
  - 前端 `frontend/src/views/production/index.vue`：生产管理（计划、排程、报工），不含定制订单全链路。
- **缺失的工艺节点**：
  - 无"纱线采购"专门模块（采购是通用 PO）。
  - 无"染整工艺"专门模块（仅有 `dye-batch`、`dye-recipe` 染**批**管理，不是订单级染整跟踪）。
  - 无"后整理"模块（防水/防油/阻燃等无专属工艺跟踪）。
  - 无"交付物流"模块（仅有 `logistics` 视图，未与定制订单关联）。
  - 无"售后"模块（仅有 `sales_return_service` 通用退货）。

#### 关键缺口

1. **数据模型完全缺失**：
   - 无 `custom_orders`（定制订单主表）：缺 `custom_no`、`customer_id`、`salesperson_id`、`product_spec`、`customization_type`、`expected_delivery_date`、`current_node`、`status`。
   - 无 `custom_process_nodes`（定制工艺节点表）：缺 `order_id`、`node_name`（yarn_procurement/dyeing/printing/finishing/delivery/after_sales）、`node_order`、`planned_start/end`、`actual_start/end`、`status`、`operator_id`、`remark`。
   - 无 `custom_process_logs`（节点日志）：记录每个节点的状态变更、操作人、附件、异常。
   - 无 `custom_quality_issues`（质量异常）：异常记录与处理流程。
2. **业务流程未贯通**：
   - 纱线采购 → 染整 → 后整理 → 交付 → 售后 无串联机制。
   - 各节点之间无"前一节点完成才能进入下一节点"的工作流引擎（虽 BPM 引擎存在，但未配置对应模板）。
3. **进度可视化缺失**：
   - 无定制订单大屏（节点甘特图、流水线进度图）。
   - 无节点预警（延期/异常自动通知）。
4. **异常处理缺失**：
   - 无"工艺异常上报 → 工艺师处理 → 重启节点"流程。
   - 无客户"投诉 → 售后 → 退换"闭环。
5. **UI 完全缺失**：无定制订单创建/详情/进度跟踪/异常上报页面。

#### 改进建议

- 建议 1：新增 `custom_orders` + `custom_process_nodes` + `custom_process_logs` + `custom_quality_issues` 四表（建议放在 `database/migration/032_create_custom_order.sql`）。
- 建议 2：基于 BPM 引擎配置 `custom_order_process` 流程模板（节点：yarn_procurement → dyeing → printing → finishing → qc → delivery → after_sales）。
- 建议 3：新增 `CustomOrderService`，提供"推进到下一节点"、"上报异常"、"延期预警"等核心方法。
- 建议 4：新增 `custom_order_handler.rs`，含端点：列表、详情、创建、推进节点、上报异常、解决异常、关闭。
- 建议 5：前端新增 `views/custom-order/` 模块：
  - 列表（按节点状态筛选、按客户/销售员筛选）
  - 详情（含甘特图、节点时间轴、附件、异常列表）
  - 创建向导（选客户 → 选工艺路线 → 选面料 → 设交付日期）
  - 大屏视图（按节点分组的卡片流）
- 建议 6：与"面料多色号定价"联动：定制色号自动加价。
- 建议 7：与"面料仓储管理"联动：定制订单消耗的缸号/匹号可追溯。
- 建议 8：与"销售报价单"联动：报价单中"定制色"标记 → 转定制订单时自动带工艺路线。

---

## 3. 缺口汇总

### 3.1 数据模型层

| 缺失表 | 所属功能 | 优先级 |
|--------|----------|--------|
| `product_color_prices`（色号价格簿） | 面料多色号定价 | P1 |
| `color_price_steps`（色号阶梯价） | 面料多色号定价 | P1 |
| `color_cards`（色卡主表） | 色卡仓储管理 | P2 |
| `color_card_items`（色卡色号） | 色卡仓储管理 | P2 |
| `color_card_lending`（色卡借出归还） | 色卡仓储管理 | P2 |
| `sales_quotations`（销售报价单主表） | 销售报价单 | P0 |
| `sales_quotation_items`（报价单明细） | 销售报价单 | P0 |
| `sales_quotation_terms`（报价条款） | 销售报价单 | P0 |
| `sales_quotation_history`（报价历史版本） | 销售报价单 | P1 |
| `custom_orders`（定制订单主表） | 定制订单跟踪 | P0 |
| `custom_process_nodes`（定制工艺节点） | 定制订单跟踪 | P0 |
| `custom_process_logs`（节点日志） | 定制订单跟踪 | P0 |
| `custom_quality_issues`（质量异常） | 定制订单跟踪 | P1 |

### 3.2 业务逻辑层

| 缺失功能 | 所属功能 | 优先级 |
|----------|----------|--------|
| 色号独立定价（不仅是加价） | 面料多色号定价 | P1 |
| 阶梯价计算引擎 | 面料多色号定价 | P1 |
| 多币种 + 汇率 + 含税/不含税价格计算 | 面料多色号定价 | P1 |
| 色卡入库/领用/归还/盘点 | 色卡仓储管理 | P2 |
| 报价单创建/审批/转订单 | 销售报价单 | P0 |
| 报价单阶梯价 / 客户等级价 | 销售报价单 | P0 |
| 报价单有效期失效与提醒 | 销售报价单 | P1 |
| 定制订单工艺路线编排 | 定制订单跟踪 | P0 |
| 纱线 → 染整 → 后整理 → 交付 → 售后 工作流 | 定制订单跟踪 | P0 |
| 工艺异常上报与处理 | 定制订单跟踪 | P1 |
| 节点延期预警 | 定制订单跟踪 | P1 |

### 3.3 UI 层

| 缺失页面 | 所属功能 | 优先级 |
|----------|----------|--------|
| 色号价格管理 Tab | 面料多色号定价 | P1 |
| 色号定价查询（按客户/数量） | 面料多色号定价 | P1 |
| 色卡列表 / 详情 / 借出 / 归还 / 盘点 | 色卡仓储管理 | P2 |
| 销售报价单列表 / 创建 / 详情 / 审批 / 打印 | 销售报价单 | P0 |
| 报价单转销售订单向导 | 销售报价单 | P0 |
| 定制订单创建向导 | 定制订单跟踪 | P0 |
| 定制订单详情（含甘特图/时间轴/附件/异常） | 定制订单跟踪 | P0 |
| 定制订单大屏（按节点分组） | 定制订单跟踪 | P0 |
| 工艺异常上报与处理页面 | 定制订单跟踪 | P1 |

---

## 4. 实施优先级

| 优先级 | 功能 | 工作量 | ROI | 理由 |
|--------|------|--------|-----|------|
| P0 | 销售报价单 | 3 周 | 极高 | 直接面向客户转化率与营收；行业 ERP 必备 |
| P0 | 定制订单全流程跟踪 | 4 周 | 极高 | 纺织行业核心差异化能力；客户黏性关键 |
| P1 | 面料多色号定价 | 2 周 | 高 | 数据模型基础已具备，只需补"色号价格簿" |
| P1 | 面料仓储管理（补全） | 2 周 | 中 | 后端能力已具备，需补前端剪裁与追溯图 |
| P2 | 色卡仓储管理 | 1.5 周 | 中 | 销售辅助工具，提升客户服务体验 |

**推荐实施顺序**：
1. 第一阶段（5 周）：销售报价单 + 定制订单全流程跟踪（最大缺口，最高 ROI）
2. 第二阶段（2 周）：面料多色号定价（充分利用现有 product_colors 表）
3. 第三阶段（2 周）：面料仓储管理补全（剪裁 UI、追溯图）
4. 第四阶段（1.5 周）：色卡仓储管理（增量模块）

---

## 5. 附录

### 5.1 关键命令清单

```bash
# 数据库迁移
ls /workspace/database/migration/                  # 33 个 SQL 文件
grep -r "color_price\|product_color" /workspace/database/migration/

# 后端
ls /workspace/backend/src/handlers/ | wc -l        # 106 个 handler
ls /workspace/backend/src/services/ | wc -l        # 110+ 个 service
ls /workspace/backend/src/models/ | wc -l          # 60+ 个 model

# 前端
ls /workspace/frontend/src/views/ | wc -l          # 80 个 view
ls /workspace/frontend/src/api/ | wc -l            # 90+ 个 API 模块
```

### 5.2 已确认存在的相关文件

#### 数据库迁移
- `/workspace/database/migration/001_consolidated_schema.sql`（行 660-685：product_colors、行 4687-5100：四级批次管理）
- `/workspace/database/migration/028_create_dye_batch_greige_fabric.sql`（行 17, 44, 46：dye_batch）
- `/workspace/database/migration/026_fix_missing_tables_and_columns.sql`（行 24：dye_recipe.color_no）
- `/workspace/database/migration/027_create_missing_tables.sql`（行 30, 55）

#### 后端模型
- `backend/src/models/product_color.rs`
- `backend/src/models/color_code_mapping.rs`
- `backend/src/models/supplier_product_color.rs`
- `backend/src/models/batch_dye_lot.rs`
- `backend/src/models/inventory_piece.rs`
- `backend/src/models/dye_lot_mapping.rs`
- `backend/src/models/piece_mapping.rs`
- `backend/src/models/batch_trace_log.rs`

#### 后端 handler / service
- `backend/src/handlers/product_handler.rs`（色号 CRUD，行 97-410）
- `backend/src/handlers/piece_split_handler.rs`（母卷剪裁）
- `backend/src/handlers/inventory_batch_handler.rs`
- `backend/src/handlers/barcode_scanner_handler.rs`
- `backend/src/services/sales_price_service.rs`（销售价格策略，非报价单）
- `backend/src/services/purchase_price_service.rs`
- `backend/src/services/production_order_service.rs`（标准化生产，非定制）
- `backend/src/services/dye_recipe_service.rs`（染色配方）
- `backend/src/services/sales_return_service.rs`（通用退货）

#### 前端
- `frontend/src/api/product.ts`（行 21-97：ProductColor 接口与 5 个方法）
- `frontend/src/views/inventoryBatch/index.vue`（批次管理，行 12-39 含批次号/色号/等级筛选）
- `frontend/src/views/inventoryAdjustment/index.vue`
- `frontend/src/views/inventoryTransfer/index.vue`
- `frontend/src/views/inventoryCount/index.vue`
- `frontend/src/views/warehouse/index.vue`
- `frontend/src/views/fiveDimension/index.vue`（5 维度查询）
- `frontend/src/views/barcodeScanner/index.vue`
- `frontend/src/views/sales-price/index.vue`（销售价格管理，非报价单）
- `frontend/src/views/purchase-price/index.vue`（采购价格管理）
- `frontend/src/views/dye-batch/index.vue`（染色批次）
- `frontend/src/views/dye-recipe/index.vue`（染色配方）
- `frontend/src/views/fabric/index.vue`（染色批次 + 坯布管理）
- `frontend/src/views/greige-fabrics/index.vue`（坯布）
- `frontend/src/views/production/index.vue`（标准化生产）

### 5.3 已确认不存在的相关代码

| 检索词 | 数据库命中 | 后端命中 | 前端命中 |
|--------|------------|----------|----------|
| `color_card` / `color_sample` / `色卡` / `色本` / `色板` / `swatch` / `palette` | 0 | 0 | 0 |
| `sales_quotation` / `quotation` / `报价单` / `报价` | 0（仅 CRM 阶段枚举 1 处） | 0 | 0 |
| `custom_order` / `定制订单` / `process_track` / `process_node` | 0（仅字段注释 4 处） | 0 | 0（仅 bpm 模板 1 处） |

### 5.4 评估局限性说明

1. 本次评估**仅基于静态代码与表结构扫描**，未运行后端服务验证 API 实际行为。
2. 未读取 BPM 引擎现有模板，未确认是否已有 `sales_quotation_approval` / `custom_order_process` 模板。
3. 未深入查看前端路由配置（router/index.ts），部分页面可能已配置但 view 文件待补。
4. 评估时间约 8 分钟（480 秒），未做代码质量与性能维度的进一步审查。

### 5.5 后续建议

- 下次迭代建议针对 P0 功能（销售报价单、定制订单跟踪）开展详细需求调研。
- 在实施前召开需求评审会，明确"色号独立价格" vs "色号加价"两种模式的选择。
- 建议先做 spike（技术验证），评估将 BPM 引擎用于定制订单工作流的技术可行性。
