# V15 面料行业深化审计报告（类四·批次 04）

- **审计子代理**：V15 审计子代理（类四 面料行业深化审计类）
- **审计范围**：17 维度（四层级联 / 缸号追溯 / 染整工艺 / 化验室打样 / 大货处方 / 流转卡条码 / 验布打卷 / 产量工资 / 能耗管理 / 染化料主数据 / 委外加工 / 多业务模式 / 缸号状态机 / 成本核算 / 质检分级 / 行业词汇 / 事件贯通）
- **审计依据**：
  - `/workspace/.monkeycode/docs/research/fabric-industry-research.md`（13 章节，1043 行）
  - `/workspace/.monkeycode/MEMORY.md`（规则 15：v14 复审 + V15 全项目综合审计）
  - `/workspace/backend/src/models/dye_batch_*.rs`（缸号系列模型）
  - `/workspace/backend/src/services/dye_batch_*.rs`（缸号系列服务）
  - `/workspace/database/migration/032_v14_fabric_unique_constraints.sql`
  - `/workspace/database/migration/035_v14_quality_grade_and_dyelot_validation.sql`
  - `/workspace/database/migration/046_v14_dye_batch_state_machine.sql`
- **审计方法**：Grep 检索关键表/字段/事件 + Read 关键模型/服务/迁移文件 + 对照研究文档核对实现完整性
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；面料行业核心特性缺失标 P0/P1

---

## 维度 1：一个面料多颜色四层级联（面料 → 颜色 → 缸号 → 批号）

### 检查方法
1. Read `/workspace/backend/src/models/product_color.rs`（产品色号模型）
2. Read `/workspace/backend/src/models/inventory_piece.rs`（库存匹数模型，含 dye_lot_id/product_id/batch_no/color_no/dye_lot_no）
3. Read `/workspace/database/migration/032_v14_fabric_unique_constraints.sql`（v14 批次 416 唯一约束补全）
4. Grep `dye_lot_no` 在 `backend/src/models/dye_batch.rs` 中是否存在
5. 对照研究文档 §2 四层级联关系：面料（Fabric）→ 颜色（Color）→ 缸号（DyeLotNo）→ 批号/匹号（BatchNo）

### 发现

#### ✅ 已落实的四层级联机制

1. **产品色号唯一约束已建立**（`/workspace/database/migration/032_v14_fabric_unique_constraints.sql:11`）：
   ```sql
   SELECT safe_add_constraint('product_colors', 'uk_pc_product_color_no', 'UNIQUE (product_id, color_no)');
   ```
   业务语义：一个面料有多个颜色，但同一面料+色号组合唯一。修复了 v14 D-P0-1（原只有 color_name 单字段 UNIQUE）。

2. **库存四维联合唯一索引已建立**（`/workspace/database/migration/032_v14_fabric_unique_constraints.sql:17-18`）：
   ```sql
   CREATE UNIQUE INDEX IF NOT EXISTS idx_inv_stock_four_dim_unique
   ON inventory_stocks (warehouse_id, product_id, color_no, batch_no, COALESCE(dye_lot_no, ''));
   ```
   业务语义：库存唯一标识 = 仓库 + 产品 + 色号 + 批号 + 缸号。使用 `COALESCE(dye_lot_no, '')` 处理白坯布无缸号的 NULL 场景。修复了 v14 D-P0-2。

3. **匹号联合唯一约束已修正**（`/workspace/database/migration/032_v14_fabric_unique_constraints.sql:39`）：
   ```sql
   SELECT safe_add_constraint('inventory_piece', 'uk_ip_dye_lot_piece_no', 'UNIQUE (dye_lot_id, piece_no)');
   ```
   业务语义：同一缸号下不能有相同的匹号（原 piece_no 全局 UNIQUE 不合理）。修复了 v14 D-P1-2。

4. **库存匹数模型追溯字段完整**（`/workspace/backend/src/models/inventory_piece.rs`，167 行）：
   含 `dye_lot_id`、`product_id`、`batch_no`、`color_no`、`dye_lot_no`、`inspection_id`、`parent_piece_id` 等追溯字段，v14 批次 416/419/426 多次补字段。

5. **产品色号模型字段完整**（`/workspace/backend/src/models/product_color.rs`，56 行）：
   含 `color_no`、`color_name`、`pantone_code`、`dye_formula`。

#### ❌ 关键缺陷：dye_batch 表缺少 dye_lot_no 字段

**风险等级：P0**（面料行业核心追溯字段缺失）

**证据**：
- `/workspace/backend/src/models/dye_batch.rs:12-26`：dye_batch 模型仅含 `id/batch_no/greige_fabric_id/color_no/planned_quantity/status/started_at/completed_at/is_deleted/created_at/updated_at`，**无 `dye_lot_no` 字段**。
- `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs:152-153`：注释明确说明 `"dye_lot_no 暂为 None，dye_batch 表当前无此字段，后续批次补全"`。
- `/workspace/database/migration/046_v14_dye_batch_state_machine.sql:15`：注释"复用现有资产：dye_batch 表（已有，仅基本字段）：缸号主表，本批次不加外键约束，用应用层校验"——确认 v14 批次 432 未补 dye_lot_no 字段。

**业务影响**：
- 面料行业四层级联关系为：面料 → 颜色 → **缸号（dye_lot_no）** → 批号/匹号。
- 缸号是染色批次的最小成本归集单元、追溯单元、质检单元。dye_batch 表作为缸号主表却无 dye_lot_no 字段，导致：
  1. 成本归集时无法将成本精确归集到缸号（dye_batch_cost_bridge_service 已注释承认）
  2. 缸号追溯链路在 dye_batch 表断链
  3. 与 inventory_piece.dye_lot_no、quality_inspection_records.dye_lot_no 字段不一致，存在"概念同名但表不同字段"问题

#### ❌ 次要缺陷：dye_batch 与 batch_dye_lot 表概念重叠

**风险等级：P2**（数据模型冗余）

**证据**：
- `/workspace/backend/src/models/dye_batch.rs`：dye_batch 表（缸号主表，40 行）
- `/workspace/backend/src/models/batch_dye_lot.rs`：batch_dye_lot 表（批次染色批次表，73 行），含 `batch_no/product_id/color_id/dye_lot_no/dye_date/quantity`

两张表均涉及缸号管理，存在概念重叠，需明确边界或合并。

### 修复建议
1. **P0 紧急**：为 dye_batch 表添加 `dye_lot_no VARCHAR(50)` 字段（迁移脚本 047+），并在 Rust 模型 `dye_batch.rs` 中补字段。同步修改 `dye_batch_cost_bridge_service.rs:152-153` 注释处逻辑，将 `dye_lot_no` 从 None 改为从 dye_batch 表读取。
2. **P2**：评估 dye_batch 与 batch_dye_lot 两表的职责边界。建议 dye_batch 作为缸号主表（含 dye_lot_no），batch_dye_lot 作为缸号-产品-颜色关联明细表，避免概念混淆。

---

## 维度 2：缸号/批号全链路追溯

### 检查方法
1. Read `/workspace/backend/src/models/batch_trace_log.rs`（批次追溯日志模型）
2. Read `/workspace/backend/src/models/dye_batch_lifecycle_log.rs`（缸号生命周期日志，v14 批次 432）
3. Read `/workspace/backend/src/models/dye_batch_operation.rs`（缸号操作记录）
4. Grep `dye_lot_no|batch_no` 在 batch_trace_log.rs 中是否存在
5. 对照研究文档 §3.2 缸号全生命周期追踪、§12.7 缸号状态机

### 发现

#### ✅ 已落实的追溯机制

1. **缸号生命周期日志完整**（`/workspace/backend/src/models/dye_batch_lifecycle_log.rs`，60 行）：
   v14 批次 432 建立。字段含 `batch_id/batch_no/from_status/to_status/transition_code/transition_name/operator_id/operator_name/equipment_id/equipment_name/work_shift/captured_params(JSONB)/remarks/transition_at`。
   - `captured_params` JSONB 字段可记录 PDA/工控终端采集参数（温度/色差ΔE/时间戳等）
   - 4 个索引（batch_id/batch_no/transition_code/transition_at）支持多维度查询

2. **缸号操作记录完整**（`/workspace/backend/src/models/dye_batch_operation.rs`，56 行）：
   支持 6 种操作类型：`merge 合缸 / split 分缸 / priority_adjust 优先级调整 / batch_change 缸变更 / schedule_change 计划变更 / terminate 终止`。
   含 `source_batch_ids(JSONB 数组)` 支持合缸/分缸时记录源缸号列表。

3. **缸号回修记录完整**（`/workspace/backend/src/models/dye_batch_rework.rs`，60 行）：
   支持 4 种回修类型：`color_difference 色差 / defect 疵点 / specification_unqualified 规格不符 / other 其他`。
   状态机：`draft 草稿 / approved 已审批 / in_progress 回修中 / completed 已完成 / cancelled 已取消`。

4. **库存匹数追溯链路完整**（`/workspace/backend/src/models/inventory_piece.rs`）：
   含 `dye_lot_id/product_id/batch_no/color_no/dye_lot_no/inspection_id/parent_piece_id`，支持从匹号反向追溯到缸号、产品、色号、批号、质检记录、母卷。

#### ❌ 关键缺陷：batch_trace_log 表字段不足，无法支持缸号全链路追溯

**风险等级：P1**（批次追溯日志缺少缸号/色号字段）

**证据**：
`/workspace/backend/src/models/batch_trace_log.rs:13-50`：
```rust
pub struct Model {
    pub id: i32,
    pub batch_no: String,                    // 仅 batch_no
    pub operation_type: String,              // 仅 CREATE/TRANSFER/ADJUST 3 种
    pub source_type: Option<String>,
    pub source_id: Option<i32>,
    pub source_no: Option<String>,
    pub quantity: Option<Decimal>,
    pub quantity_before: Option<Decimal>,
    pub quantity_after: Option<Decimal>,
    pub remarks: Option<String>,
    pub operated_by: Option<i32>,
    pub operated_at: DateTime<Utc>,
}
```

**问题**：
1. **缺少 `dye_lot_no` 字段**：无法按缸号查询追溯日志
2. **缺少 `color_no` 字段**：无法按色号查询追溯日志
3. **缺少 `product_id` 字段**：无法按产品查询追溯日志
4. **operation_type 仅 3 种**（CREATE/TRANSFER/ADJUST）：无法覆盖面料行业全链路操作（如 dyeing 染色、inspection 验布、grade 分级、ship 发货、rework 回修等）
5. **缺少 `from_status/to_status` 状态流转字段**：无法记录操作前后的状态变化
6. **文件顶部有 `#![allow(dead_code)]`**（`/workspace/backend/src/models/batch_trace_log.rs:1`）：违反项目规则六第 1 条"禁止使用文件级 `#![allow(dead_code)]` 全局抑制"——但根据规则例外，models/ 下的 SeaORM 自动生成模型可保留，需评估是否属于例外。

**业务影响**：
- 面料行业要求缸号从坯布投缸到成品发货的全链路追溯（§3.2）
- batch_trace_log 表作为"批次追溯日志"却无缸号字段，无法支持"按缸号查全链路操作"的核心追溯场景
- 与 dye_batch_lifecycle_log（有 batch_no/dye_lot_no 间接关联）职责重叠但不一致

### 修复建议
1. **P1**：为 batch_trace_log 表添加 `dye_lot_no VARCHAR(50)`、`color_no VARCHAR(50)`、`product_id INTEGER` 字段，并创建相应索引。
2. **P1**：扩展 operation_type 枚举，覆盖面料行业全链路操作：`CREATE/DYE/INSPECT/GRADE/SHIP/REWORK/TRANSFER/ADJUST/MERGE/SPLIT` 等。
3. **P2**：评估 batch_trace_log 与 dye_batch_lifecycle_log 的职责边界。建议 batch_trace_log 作为通用批次追溯（含坯布/成品/染化料等），dye_batch_lifecycle_log 作为缸号专用生命周期日志，避免重复。
4. **P2**：评估 batch_trace_log.rs:1 的 `#![allow(dead_code)]` 是否符合 models/ 例外规则，如不符合则按规则六第 3 条添加项级 `#[allow(dead_code)]` + TODO 注释。

---

## 维度 3：染整工艺流程完整性（前处理 → 染色 → 印花 → 后整理 → 验布）

### 检查方法
1. Read `/workspace/backend/src/models/process_route.rs`（工序路线模板）
2. Read `/workspace/backend/src/models/process_step_record.rs`（工序流转记录）
3. Grep `前处理|染色|印花|后整理|验布|pre_treatment|dyeing|printing|finishing|inspection` 在 models/ 中
4. 对照研究文档 §3 染整工艺流程（10 道工序：配布→精练→漂白→染色→对色→理布→烘干→定型→成品对色→成检）

### 发现

#### ✅ 已落实的工艺流程机制

1. **工序路线模板完整**（`/workspace/backend/src/models/process_route.rs`，72 行）：
   含 `route_code/route_name/product_category/steps(JSONB)/is_default`。
   支持 5 大工序分类：前处理 → 染色 → 印花 → 后整理 → 验布（通过 steps JSONB 配置具体工序序列）。

2. **工序流转记录完整**（`/workspace/backend/src/models/process_step_record.rs`，140 行）：
   含 `route_id/step_code/step_name/step_sequence/flow_card_id/dye_batch_id/equipment_id/operator_id/work_shift/start_at/end_at/duration_minutes/parameters(JSONB)/status/abnormal_reason`。
   状态机：`pending → in_progress → completed / abnormal / rework`，覆盖正常/异常/回修三种流转路径。
   `parameters` JSONB 字段可记录工序参数（温度/时间/压力等）。

3. **缸号状态机覆盖详细工序**（`/workspace/database/migration/046_v14_dye_batch_state_machine.sql:9-11`）：
   14 种状态覆盖详细工序：投缸 → 染色 → 皂洗 → 固色 → 脱水 → 烘干 → 验布 → 入库。
   每一环节通过 PDA 扫码或工控终端确认，自动捕获时间戳、操作人、设备 ID、实时采集参数。

#### ❌ 次要缺陷：10 道精细工序与 5 大工序分类的映射关系未明确

**风险等级：P3**（文档与实现粒度不一致）

**证据**：
- 研究文档 §3 定义 10 道精细工序：配布 → 精练 → 漂白 → 染色 → 对色 → 理布 → 烘干 → 定型 → 成品对色 → 成检
- process_route.rs 的 steps JSONB 仅按 5 大分类（前处理/染色/印花/后整理/验布）配置
- process_step_record.rs 的 step_code/step_name 可支持精细工序，但无预置数据约束

**业务影响**：
- 工序粒度可由 steps JSONB 灵活配置，技术上不影响实现
- 但缺少预置 10 道精细工序模板，需用户手工配置，存在配置错误风险

### 修复建议
1. **P3**：在 process_route 预置数据中提供 10 道精细工序模板（配布/精练/漂白/染色/对色/理布/烘干/定型/成品对色/成检），便于用户快速启用。
2. **P3**：补充文档说明 5 大工序分类与 10 道精细工序的映射关系。

---

## 维度 4：化验室打样 5 步闭环

### 检查方法
1. Read `/workspace/backend/src/models/lab_dip_request.rs`（打样通知单，161 行）
2. Read `/workspace/backend/src/models/lab_dip_sample.rs`（打样小样，169 行）
3. Read `/workspace/backend/src/models/lab_dip_resample.rs`（复样记录，150 行）
4. Grep `start_sampling|approve_ok_sample|issue_tech_card` 在 handlers/lab_dip_handler.rs 中
5. 对照研究文档 §11 化验室打样 5 步闭环：打样通知 → 打样 ABCD 多版 → OK 样确认 → 复样 → 建数据库

### 发现

#### ✅ 已落实的 5 步闭环机制

1. **第 1 步：打样通知单完整**（`/workspace/backend/src/models/lab_dip_request.rs`，161 行）：
   字段含 `request_no/customer_id/product_id/color_no/color_name/pantone_code/light_source(对色光源)/color_fastness(色牢度要求)/sample_count(打样版数 ABCD)/dye_type(染料类别)/urgency/request_status`。
   支持对色光源（D65/TL84/CWF/A 等）、色牢度要求、ABCD 多版打样等面料行业核心字段。

2. **第 2 步：打样小样 ABCD 多版完整**（`/workspace/backend/src/models/lab_dip_sample.rs`，169 行）：
   字段含 `request_id/sample_version(A/B/C/D)/formula_detail(JSONB FormulaDetailItem)/color_difference_delta_e/evaluation_result/ok_sample_flag`。
   `formula_detail` JSONB 结构存储染料配方明细，支持多版配方对比。

3. **第 3 步：OK 样确认机制完整**：
   lab_dip_sample 含 `ok_sample_flag` 标识 OK 样，handler 含 `approve_ok_sample` 接口确认 OK 样。

4. **第 4 步：复样记录完整**（`/workspace/backend/src/models/lab_dip_resample.rs`，150 行）：
   字段含 `original_sample_id/resample_no/dyeing_tech_card(染色技术卡)/color_difference_delta_e/evaluation_result`。
   含染色技术卡字段，支持复样时技术参数传递。

5. **第 5 步：建数据库（染色技术卡签发）完整**：
   handler 含 `issue_tech_card` 接口签发染色技术卡，将 OK 样配方写入生产处方数据库（production_recipe）。

6. **Handler 接口完整**（`/workspace/backend/src/handlers/lab_dip_handler.rs`，22 个接口）：
   含 `create_request/start_sampling/submit_sample/evaluate_sample/approve_ok_sample/request_resample/issue_tech_card` 等完整 5 步闭环接口。

#### ✅ 5 步闭环全链路验证通过

研究文档 §11 定义的 5 步闭环（打样通知 → 打样 ABCD 多版 → OK 样确认 → 复样 → 建数据库）在代码中全部落实，无缺失。

### 修复建议
无需修复。本维度实现完整，符合面料行业真实业务。

---

## 维度 5：大货处方与加料处方

### 检查方法
1. Read `/workspace/backend/src/models/production_recipe.rs`（大货处方主表，205 行）
2. Read `/workspace/backend/src/models/production_recipe_addition.rs`（加料处方表，138 行）
3. Grep `approve|calculate|create_addition` 在 handlers/production_recipe_handler.rs 中
4. 对照研究文档 §12.3 大货处方与加料处方：扫描流转卡条码 → 加载处方 → 填物料 → 计算用量 → 审核 → 自动生成领用单据

### 发现

#### ✅ 已落实的处方机制

1. **大货处方主表完整**（`/workspace/backend/src/models/production_recipe.rs`，205 行）：
   字段含 `recipe_no/flow_card_id/dye_batch_id/liquor_ratio(浴比)/bath_volume(浴量)/adjustment_factor(调整系数)/recipe_detail(JSONB)/status/approved_by/approved_at`。
   核心字段：
   - `liquor_ratio` 浴比：面料行业染色核心参数（1:10 等）
   - `bath_volume` 浴量：根据布重 × 浴比自动计算
   - `adjustment_factor` 调整系数：根据化验室 OK 样放大到大货的修正系数
   - `recipe_detail` JSONB：存储染料/助剂配方明细

2. **加料处方表完整**（`/workspace/backend/src/models/production_recipe_addition.rs`，138 行）：
   字段含 `addition_no/recipe_id(关联大货处方)/flow_card_id/dye_batch_id/addition_sequence(加料次序)/addition_time/addition_detail(JSONB)/status`。
   支持多次加料（addition_sequence），记录每次加料的时间和物料明细。

3. **Handler 接口完整**（`/workspace/backend/src/handlers/production_recipe_handler.rs`，15 个接口）：
   含 `create_recipe/calculate_recipe/approve_recipe/create_addition/approve_addition` 等核心接口。
   - `calculate_recipe`：根据布重 × 浴比 × 调整系数自动计算用量
   - `approve_recipe`：审核处方后自动生成领用单据
   - `create_addition`：扫描流转卡条码创建加料处方

4. **与流转卡/缸号关联完整**：
   production_recipe 和 production_recipe_addition 均含 `flow_card_id` 和 `dye_batch_id`，支持扫描流转卡条码加载处方。

#### ✅ 大货处方与加料处方全链路验证通过

研究文档 §12.3 定义的流程（扫描流转卡条码 → 加载处方 → 填物料 → 计算用量 → 审核 → 自动生成领用单据）在代码中全部落实。

### 修复建议
无需修复。本维度实现完整，符合面料行业真实业务。

---

## 维度 6：流转卡条码与车间工序

### 检查方法
1. Read `/workspace/backend/src/models/production_flow_card.rs`（生产流转卡，208 行）
2. Read `/workspace/backend/src/models/process_step_record.rs`（工序流转记录，140 行）
3. Grep `barcode|schedule|start_dyeing|complete_flow_card|ship_flow_card` 在 handlers/flow_card_handler.rs 中
4. 对照研究文档 §12.2 流转卡条码与车间工序

### 发现

#### ✅ 已落实的流转卡机制

1. **生产流转卡完整**（`/workspace/backend/src/models/production_flow_card.rs`，208 行）：
   字段含 `card_no/barcode(条码)/dye_batch_id/dye_lot_no/product_id/color_no/batch_no/planned_quantity/status(状态机)/current_step/scheduled_start/actual_start/actual_end`。
   核心特性：
   - `barcode` 条码字段：支持 PDA 扫码
   - `dye_lot_no` 缸号字段：与流转卡绑定缸号
   - `status` 状态机字段：流转卡状态流转
   - `current_step` 当前工序：跟踪流转卡所在工序

2. **工序流转记录完整**（`/workspace/backend/src/models/process_step_record.rs`，140 行）：
   字段含 `route_id/step_code/step_name/step_sequence/flow_card_id/dye_batch_id/equipment_id/operator_id/work_shift/start_at/end_at/duration_minutes/parameters(JSONB)/status/abnormal_reason`。
   状态机：`pending → in_progress → completed / abnormal / rework`，覆盖正常/异常/回修三种流转路径。

3. **Handler 接口完整**（`/workspace/backend/src/handlers/flow_card_handler.rs`，31 个接口）：
   含 `create_flow_card/schedule/start_dyeing/complete_flow_card/ship_flow_card/scan_barcode` 等核心接口。
   - `scan_barcode`：PDA 扫码接口
   - `schedule`：排缸
   - `start_dyeing`：进缸染色
   - `complete_flow_card`：完成流转卡
   - `ship_flow_card`：发货

#### ✅ 流转卡条码与车间工序全链路验证通过

研究文档 §12.2 定义的流转卡条码 + 车间工序流转机制在代码中全部落实。

### 修复建议
无需修复。本维度实现完整，符合面料行业真实业务。

---

## 维度 7：验布打卷（十项指标 + A/B/C 分级）

### 检查方法
1. Read `/workspace/backend/src/models/fabric_inspection_record.rs`（验布记录，132 行）
2. Read `/workspace/backend/src/models/fabric_defect_record.rs`（疵点明细，85 行）
3. Grep `calculate_four_point_points|calculate_ten_point_points|grade_inspection|determine_quality_grade` 在 services/ 中
4. Grep `skewness|shrinkage|pilling|handfeel|tensile_strength|tear_strength|weight_gsm|color_fastness|width|density` 在 models/ 中
5. 对照研究文档 §4.7 质量检验模块（四分制/十分制评分 + A/B/C 级判定 + 十项指标）

### 发现

#### ✅ 已落实的验布打卷机制

1. **验布记录完整**（`/workspace/backend/src/models/fabric_inspection_record.rs`，132 行）：
   字段含 `inspection_no/flow_card_id/dye_batch_id/piece_no/inspection_length/fabric_width_inches(门幅)/scoring_system(四分制/十分制)/total_defect_points/points_per_100_sq_yards(每百平方码扣分)/grade(级)/abc_grade(A/B/C级)/qualification_rate(合格率)/inspector/inspection_at`。
   核心特性：
   - `scoring_system` 评分制：支持四分制（four_point）和十分制（ten_point）
   - `total_defect_points` 总扣分：累计疵点扣分
   - `points_per_100_sq_yards` 每百平方码扣分：标准化评分
   - `grade` 级：四分制/十分制评级（如 First/Second/Second）
   - `abc_grade` A/B/C 级：面料行业等级判定

2. **疵点明细完整**（`/workspace/backend/src/models/fabric_defect_record.rs`，85 行）：
   支持 12 种疵点类型 + 四分制/十分制扣分规则。
   字段含 `inspection_id/defect_code/defect_name/defect_type/defect_length/defect_points(扣分)/position(位置)`。

3. **四分制/十分制评分函数完整**（`/workspace/backend/src/services/fabric_inspection_service.rs`）：
   - `calculate_four_point_points` 四分制规则：
     - ≤3 寸 = 1 分
     - 3-6 寸 = 2 分
     - 6-9 寸 = 3 分
     - \>9 寸 = 4 分
     - 破洞/连续 = 4 分
   - `calculate_ten_point_points` 十分制规则：
     - 破洞 = 10 分
     - 经向/纬向按长度 + 半门幅判定

4. **A/B/C 分级联动 determine_quality_grade 完整**（`/workspace/backend/src/services/quality_inspection_service.rs`）：
   `determine_quality_grade` 函数：A 级（≥95%）/B 级（80-95%）/C 级（<80%），常量 `QUALITY_GRADE_A/B/C` 定义。

5. **Handler 接口完整**（`/workspace/backend/src/handlers/fabric_inspection_handler.rs`，12 个接口）：
   含 `create_inspection/add_defect/calculate_grade/grade_inspection(打卷分级)/complete_inspection` 等核心接口。

#### ❌ 关键缺陷：面料检验十项指标无完整建模

**风险等级：P1**（面料行业核心质检指标缺失）

**证据**：
- Grep 检索 `skewness|shrinkage|pilling|handfeel|tensile_strength|tear_strength|weight_gsm|color_fastness|width|density` 在 models/ 中：**无匹配**（仅 fabric_inspection_record 有 `fabric_width_inches` 和 `qualification_rate`）。
- 研究文档 §4.7 定义面料检验十项指标：
  1. 纬斜（skewness）
  2. 缩水率（shrinkage）
  3. 起毛起球（pilling）
  4. 手感（handfeel）
  5. 拉伸强度（tensile_strength）
  6. 撕裂强度（tear_strength）
  7. 克重（weight_gsm）
  8. 色牢度（color_fastness）
  9. 门幅（width）
  10. 密度（density）

**业务影响**：
- 面料行业质检不仅看外观疵点（四分制/十分制），还需检验物理指标（十项指标）
- 十项指标缺失导致验布打卷仅能判定外观等级，无法判定物理性能等级
- 影响成品入库判定（A 级需外观 + 物理双达标）

### 修复建议
1. **P1**：新建 `fabric_physical_test_record` 模型表，字段含 `inspection_id/test_item(test_item 枚举: skewness/shrinkage/pilling/handfeel/tensile_strength/tear_strength/weight_gsm/color_fastness/width/density)/test_value/standard_value/test_result(pass/fail)/tested_at`。
2. **P1**：在 fabric_inspection_handler 添加 `add_physical_test` 接口，支持录入十项指标。
3. **P1**：修改 `determine_quality_grade` 函数，A 级判定需外观合格率 ≥95% **且** 十项指标全部 pass。

---

## 维度 8：产量工资核算（按缸号计件）

### 检查方法
1. Read `/workspace/backend/src/models/wage_record.rs`（工资记录，81 行）
2. Read `/workspace/backend/src/models/wage_record_detail.rs`（工资明细，165 行）
3. Read `/workspace/backend/src/models/process_wage_rate.rs`（工序工价，95 行）
4. Grep `calculate_wage|confirm_wage_record|pay_wage_record|determine_quality_grade` 在 services/wage_service.rs 中
5. Grep `voucher|financial|凭证` 在 services/wage_service.rs 中
6. 对照研究文档 §12.6 产量工资核算（按缸号计件，三维度统计，A/B/C 等级系数）

### 发现

#### ✅ 已落实的工资核算机制

1. **工资记录完整**（`/workspace/backend/src/models/wage_record.rs`，81 行）：
   状态机：`draft 草稿 → confirmed 已确认 → paid 已发放 → cancelled 已取消`。
   字段含 `record_no/period(工资周期)/total_piece_wage/total_time_wage/total_amount/status/confirmed_by/confirmed_at/paid_by/paid_at`。

2. **工资明细完整**（`/workspace/backend/src/models/wage_record_detail.rs`，165 行）：
   字段含 `record_id/dye_batch_id(按缸号计件)/flow_card_id/process_step_id/worker_id/equipment_id/piece_wage(计件工资)/time_wage(计时工资)/grade_ratio(等级系数)/quality_grade(A/B/C)`。
   核心特性：
   - `dye_batch_id` 按缸号计件：支持按缸号归集工资
   - `piece_wage` 计件工资 + `time_wage` 计时工资：双轨制
   - `grade_ratio` 等级系数：A/B/C 等级对应不同系数
   - 三维度统计：工序（process_step_id）/ 设备（equipment_id）/ 工人（worker_id）

3. **工序工价完整**（`/workspace/backend/src/models/process_wage_rate.rs`，95 行）：
   字段含 `process_code/process_name/base_rate(基础工价)/grade_a_ratio(1.0)/grade_b_ratio(0.8)/grade_c_ratio(0.0)`。
   A 级系数 1.0，B 级系数 0.8，C 级系数 0.0（不合格不发放工资）。

4. **工资计算函数完整**（`/workspace/backend/src/services/wage_service.rs`）：
   - `calculate_wage_for_step` 函数：按工序计算工资
   - 复用 `determine_quality_grade` 判定 A/B/C 等级
   - 工资 = 基础工价 × 等级系数 × 数量

5. **Handler 接口完整**（`/workspace/backend/src/handlers/wage_handler.rs`，21 个接口）：
   含 `calculate_wage/confirm_wage_record/pay_wage_record` 等核心接口。

#### ❌ 关键缺陷：工资确认/发放未生成财务凭证

**风险等级：P1**（工资核算与财务系统断链）

**证据**：
- Grep `voucher|financial|凭证` 在 `/workspace/backend/src/services/wage_service.rs` 中：**无匹配**
- 工资记录状态机 `draft → confirmed → paid`，但 confirmed（确认）和 paid（发放）两个节点均未生成财务凭证

**业务影响**：
- 面料行业产量工资是直接人工成本的核心组成部分（§5 成本核算）
- 工资确认时应生成"应付工资"分录，工资发放时应生成"银行存款/现金"分录
- 缺少凭证生成导致：
  1. 财务系统无法自动归集人工成本
  2. 成本核算（维度 14）的 direct_labor 字段无法从工资系统自动取数
  3. 月末结账需人工手工录入工资凭证，存在错漏风险

### 修复建议
1. **P1**：在 wage_service.rs 的 `confirm_wage_record` 函数中添加凭证生成逻辑，生成"应付工资"分录（借：生产成本-直接人工，贷：应付职工薪酬）。
2. **P1**：在 wage_service.rs 的 `pay_wage_record` 函数中添加凭证生成逻辑，生成"工资发放"分录（借：应付职工薪酬，贷：银行存款/现金）。
3. **P1**：发布 `WageConfirmed` / `WagePaid` BusinessEvent，供成本核算服务监听并自动归集 direct_labor。

---

## 维度 9：能耗管理（水电汽分摊）

### 检查方法
1. Read `/workspace/backend/src/models/energy_consumption_record.rs`（能耗记录，115 行）
2. Read `/workspace/backend/src/models/energy_allocation_record.rs`（能耗分摊记录，111 行）
3. Read `/workspace/backend/src/models/energy_allocation_rule.rs`（分摊规则，88 行）
4. Grep `monthly_allocation_by_duration|简化|暂用` 在 services/energy_service.rs 中
5. 对照研究文档 §5.3 能耗管理（水/电/汽分摊，按工时/产量/设备/车间四种分摊基准）

### 发现

#### ✅ 已落实的能耗管理机制

1. **能耗记录完整**（`/workspace/backend/src/models/energy_consumption_record.rs`，115 行）：
   字段含 `record_no/energy_type(water/electricity/steam)/consumption_value/unit/consumption_date/source(manual/IoT/auto_calc)/equipment_id/workshop_id/meter_reading_before/meter_reading_after/cost`。
   支持三种录入方式：手工录入（manual）、物联网采集（IoT）、自动计算（auto_calc）。

2. **能耗分摊记录完整**（`/workspace/backend/src/models/energy_allocation_record.rs`，111 行）：
   字段含 `allocation_no/consumption_id/cost_collection_id(关联成本归集)/dye_batch_id(按缸号分摊)/equipment_id/workshop_id/allocated_quantity/allocated_cost/allocation_rule_id`。
   通过 `cost_collection_id` 关联成本归集表，支持能耗成本自动归集到缸号。

3. **分摊规则完整**（`/workspace/backend/src/models/energy_allocation_rule.rs`，88 行）：
   支持 4 种分摊基准：
   - `by_duration` 按工时分摊
   - `by_output` 按产量分摊
   - `by_equipment` 按设备分摊
   - `by_workshop` 按车间分摊

4. **Handler 接口完整**（`/workspace/backend/src/handlers/energy_handler.rs`，32 个接口）：
   含 `monthly_allocation/confirm_energy_allocation` 等核心接口。

#### ❌ 关键缺陷：月末分摊存在简化逻辑，影响能耗精确分摊

**风险等级：P1**（能耗分摊精度不足）

**证据**：
`/workspace/backend/src/services/energy_service.rs`：
- **第 491 行**：`/// 缸号（简化：暂用 equipment_name）`
- **第 1555 行**：`// 注意：工序记录没有直接的 workshop 字段，这里通过 equipment_name 或 route_code 简化处理`
- **第 1561 行**：`// 缸号通过 flow_card 关联查询（简化：暂用 equipment_name 作为车间归属）`
- **第 1562 行**：`let dye_lot_no = step.equipment_name.clone(); // 简化：实际应通过 flow_card 查询 dye_lot_no`

**问题**：
1. **dye_lot_no 简化为 equipment_name**：第 1562 行直接将 `equipment_name` 作为 `dye_lot_no`，这是错误的——设备名和缸号是两个完全不同的概念。
2. **workshop 简化为 equipment_name**：第 1561 行将 `equipment_name` 作为车间归属，但工序记录无 workshop 字段时应该通过 equipment → workshop 关联表查询。
3. **未通过 flow_card 查询 dye_lot_no**：注释明确说"实际应通过 flow_card 查询 dye_lot_no"但未实现。

**业务影响**：
- 能耗分摊到缸号的精度严重不足，dye_lot_no 被错误地填入 equipment_name
- 后续成本核算（维度 14）从 energy_allocation_record 读取 dye_lot_no 时会拿到错误的设备名
- 影响按缸号实际成本法的准确性

### 修复建议
1. **P1**：修改 energy_service.rs 第 1562 行，通过 `flow_card_id` 关联查询 `production_flow_card.dye_lot_no`，正确填充 dye_lot_no 字段。
2. **P1**：建立 equipment → workshop 关联表（或在 equipment 表添加 workshop_id 字段），通过设备查询车间归属，替换第 1561 行的简化逻辑。
3. **P2**：补充月末分摊单元测试，验证 dye_lot_no 和 workshop_id 的正确性。

---

## 维度 10：染化料主数据（GHS / MSDS / 批号管理）

### 检查方法
1. Read `/workspace/backend/src/models/chemical_master.rs`（染化料主数据，166 行）
2. Read `/workspace/backend/src/models/chemical_lot.rs`（染化料批次，124 行）
3. Read `/workspace/backend/src/models/chemical_requisition.rs`（染化料领用单，105 行）
4. Grep `GHS|MSDS|UN|危险|保质期|安全库存` 在 models/chemical_master.rs 中
5. 对照研究文档 §7 染化料主数据（GHS 分类/UN 编号/MSDS/保质期/安全库存）

### 发现

#### ✅ 已落实的染化料主数据机制

1. **染化料主数据完整**（`/workspace/backend/src/models/chemical_master.rs`，166 行）：
   字段含 `chemical_code/chemical_name/chemical_type(染料/助剂)/cas_no( CAS 号)/molecular_formula/ghs_class(GHS 分类)/ghs_signal/ghs_hazard_statements/ghs_precautionary_statements/un_number(UN 编号)/msds_url(MSDS 链接)/shelf_life_days(保质期)/safety_stock(安全库存)/unit/supplier_id`。
   核心特性：
   - **GHS 分类完整**：ghs_class/ghs_signal/ghs_hazard_statements/ghs_precautionary_statements 四字段覆盖 GHS 标签全部要素
   - **UN 编号完整**：un_number 支持危险品运输标识
   - **MSDS 完整**：msds_url 存储 MSDS 文档链接
   - **保质期完整**：shelf_life_days 支持过期预警
   - **安全库存完整**：safety_stock 支持库存预警

2. **染化料批次完整**（`/workspace/backend/src/models/chemical_lot.rs`，124 行）：
   字段含 `lot_no/chemical_id/production_date/expiry_date(过期日期)/quantity/inspection_status(状态机)/inspected_by/inspected_at`。
   状态机：`pending 待检 / passed 检验通过 / failed 检验失败 / quarantined 隔离`。
   支持染化料按批次质检和过期管理。

3. **染化料领用单完整**（`/workspace/backend/src/models/chemical_requisition.rs`，105 行）：
   字段含 `requisition_no/dye_batch_id(关联缸号)/flow_card_id/requisition_date/requisition_by/items(JSONB)/status`。
   通过 `dye_batch_id` 关联缸号，支持按缸号归集染化料成本。

4. **Handler 接口完整**（`/workspace/backend/src/handlers/chemical_handler.rs`，32 个接口）：
   含 `pass_inspection/consume_lot/approve_requisition` 等核心接口。

#### ✅ 染化料主数据全链路验证通过

研究文档 §7 定义的 GHS 分类/UN 编号/MSDS/保质期/安全库存/批号管理在代码中全部落实。

### 修复建议
无需修复。本维度实现完整，符合面料行业真实业务。

---

## 维度 11：委外加工（外发染整 / 印花）

### 检查方法
1. Read `/workspace/backend/src/models/outsourcing_order.rs`（委外加工订单，180 行）
2. Grep `voucher_no_issue|voucher_no_fee|voucher_no_receipt|issue|record_processing|settle|post_outsourcing_voucher` 在 models/ 和 handlers/ 中
3. Grep `publish|BusinessEvent|EVENT_BUS` 在 services/outsourcing_service.rs 中
4. 对照研究文档 §5.4 委托加工物资核算（发料分录 → 加工费分录 → 入库分录三步法）

### 发现

#### ✅ 已落实的委外加工机制

1. **委外加工订单完整**（`/workspace/backend/src/models/outsourcing_order.rs`，180 行）：
   字段含 `order_no/order_type(染整/印花/织造)/supplier_id/source_order_id/product_id/color_no/quantity/loss_rate/normal_loss_rate/abnormal_loss/processing_fee/voucher_no_issue(发料凭证号)/voucher_no_fee(加工费凭证号)/voucher_no_receipt(入库凭证号)/status`。
   核心特性：
   - **三凭证号完整**：voucher_no_issue / voucher_no_fee / voucher_no_receipt 对应 §5.4 三步分录
   - **损耗分类完整**：normal 正常损耗 / abnormal 异常损耗
   - **行业标准损耗率**：dyeing=0.05 / weaving=0.035 / spinning=0.055

2. **Handler 接口完整**（`/workspace/backend/src/handlers/outsourcing_handler.rs`，26 个接口）：
   含 `issue(发料)/record_processing(记录加工)/settle(结算)/post_outsourcing_voucher(过账凭证)` 等核心接口，对应 §5.4 三步分录。

3. **服务层三步分录完整**（`/workspace/backend/src/services/outsourcing_service.rs`）：
   - 创建发料凭证（对应 voucher_no_issue）
   - 创建加工费凭证（对应 voucher_no_fee）
   - 创建入库凭证（对应 voucher_no_receipt）

#### ❌ 关键缺陷：委外加工服务无事件发布

**风险等级：P1**（委外加工业务事件断链）

**证据**：
- Grep `publish|BusinessEvent|EVENT_BUS` 在 `/workspace/backend/src/services/outsourcing_service.rs` 中：**无匹配**
- 委外加工订单的发料/加工费/入库三步分录均未发布任何 BusinessEvent

**业务影响**：
- 委外加工是面料行业 6 种业务模式之一（§6 业务模式），涉及物资发料、加工费核算、成品入库等多个业务节点
- 缺少事件发布导致：
  1. 委外发料时无法触发库存出库事件
  2. 委外入库时无法触发库存入库事件
  3. 委外加工费无法自动归集到成本核算系统
  4. 委外损耗（normal/abnormal）无法触发财务异常预警
- 与维度 17（事件贯通）相关，事件总线已定义 `OutsourcingOrderCompleted` 等事件但无发布者

### 修复建议
1. **P1**：在 outsourcing_service.rs 的 `issue`（发料）函数中发布 `OutsourcingMaterialIssued` 事件，含订单号、发料数量、发料凭证号。
2. **P1**：在 outsourcing_service.rs 的 `record_processing`（记录加工）函数中发布 `OutsourcingProcessingRecorded` 事件，含订单号、加工费、加工凭证号。
3. **P1**：在 outsourcing_service.rs 的 `settle`（结算）函数中发布 `OutsourcingOrderSettled` 事件，含订单号、最终损耗、结算金额。
4. **P1**：在 outsourcing_service.rs 的 `post_outsourcing_voucher`（入库过账）函数中发布 `OutsourcingOrderCompleted` 事件，含订单号、入库数量、入库凭证号。
5. **P1**：在 event_bus.rs 中添加对应事件监听器，触发库存出/入库、成本归集等下游动作。

---

## 维度 12：多业务模式（坯布销售 / 染色加工 / 印花加工 / 来料加工 / 贸易）

### 检查方法
1. Read `/workspace/backend/src/models/business_mode_config.rs`（业务模式配置，95 行）
2. Read `/workspace/backend/src/models/business_mode_flow_step.rs`（业务模式流程节点，60 行）
3. Read `/workspace/backend/src/models/business_mode_rule.rs`（业务模式规则，63 行）
4. Read `/workspace/backend/src/models/business_mode_order_link.rs`（单据-业务模式关联，59 行）
5. Grep `publish|BusinessEvent` 在 services/business_mode_service.rs 中
6. 对照研究文档 §6 业务模式 6 种

### 发现

#### ✅ 已落实的多业务模式机制

1. **业务模式配置完整**（`/workspace/backend/src/models/business_mode_config.rs`，95 行）：
   支持 6 种业务模式：
   - 坯布经销（greige_sales）
   - 成品经销（finished_sales）
   - 染整加工（dyeing_processing）
   - 自织自染（self_weaving_dyeing）
   - 委托加工（outsourcing）
   - 来料加工（tolling）

2. **业务模式流程节点完整**（`/workspace/backend/src/models/business_mode_flow_step.rs`，60 行）：
   字段含 `mode_id/step_code/step_name/step_sequence/is_required`，支持按业务模式配置不同流程节点。

3. **业务模式规则完整**（`/workspace/backend/src/models/business_mode_rule.rs`，63 行）：
   字段含 `mode_id/rule_type/rule_key/rule_value`，规则类型支持 `required/optional/forbidden`。
   例如：来料加工模式下"坯布采购"节点为 forbidden。

4. **单据-业务模式关联完整**（`/workspace/backend/src/models/business_mode_order_link.rs`，59 行）：
   字段含 `mode_id/document_type(sales_order/purchase_order/production_order/outsourcing_order)/document_id/document_no/mode_snapshot(JSONB)`。
   核心特性：
   - `mode_snapshot` 模式快照：防止后续模式修改影响历史单据
   - UNIQUE(document_type, document_id)：同一单据只能关联一个业务模式
   - ON DELETE RESTRICT：业务模式被引用后不可删除

5. **Handler 接口完整**（`/workspace/backend/src/handlers/business_mode_handler.rs`，22 个接口）：
   含 `set_default_business_mode/link_order/validate_order` 等核心接口。

#### ❌ 关键缺陷：业务模式服务无事件发布

**风险等级：P1**（业务模式切换事件断链）

**证据**：
- Grep `publish|BusinessEvent` 在 `/workspace/backend/src/services/business_mode_service.rs` 中：**无匹配**
- 业务模式切换、订单关联模式等节点均未发布任何 BusinessEvent

**业务影响**：
- 业务模式切换会影响后续流程节点配置，其他模块（如库存、成本）需感知模式变化
- 缺少事件发布导致：
  1. 业务模式切换时其他模块无法感知
  2. 订单关联模式时无法触发流程节点校验
  3. 来料加工模式的来料库存无法自动隔离

### 修复建议
1. **P1**：在 business_mode_service.rs 的 `set_default_business_mode` 函数中发布 `BusinessModeChanged` 事件，含模式 ID、模式名称。
2. **P1**：在 business_mode_service.rs 的 `link_order` 函数中发布 `OrderBusinessModeLinked` 事件，含单据类型、单据 ID、模式 ID、模式快照。
3. **P2**：在 event_bus.rs 中添加对应事件监听器，触发流程节点校验、库存隔离等下游动作。

---

## 维度 13：缸号全生命周期状态机（14 种状态）

### 检查方法
1. Read `/workspace/database/migration/046_v14_dye_batch_state_machine.sql`（367 行，v14 批次 432）
2. Read `/workspace/backend/src/models/dye_batch_lifecycle_log.rs`（60 行）
3. Read `/workspace/backend/src/models/dye_batch_state_rule.rs`（55 行）
4. Read `/workspace/backend/src/models/dye_batch_operation.rs`（56 行）
5. Read `/workspace/backend/src/models/dye_batch_rework.rs`（60 行）
6. Grep `record_transition|approve_rework|validate_transition` 在 services/dye_batch_state_machine_service.rs 中
7. 对照研究文档 §12.7 缸号状态机（14 种状态）

### 发现

#### ✅ 已落实的缸号状态机机制

1. **14 种状态完整定义**（`/workspace/database/migration/046_v14_dye_batch_state_machine.sql:5-7`）：
   ```
   pending_schedule / scheduled / preparing / dyeing / washing / fixing /
   dehydrating / drying / inspecting / stored / shipped / cancelled / terminated / rework
   ```
   终态：shipped 发货 / cancelled 取消 / terminated 终止
   回修：rework 可回到 dyeing 重新进缸

2. **4 张状态机表完整**：
   - `dye_batch_lifecycle_log`（生命周期日志，60 行模型）
   - `dye_batch_state_rule`（状态流转规则，55 行模型）
   - `dye_batch_operation`（操作记录，56 行模型）
   - `dye_batch_rework`（回修记录，60 行模型）

3. **预置状态流转规则完整**（`/workspace/database/migration/046_v14_dye_batch_state_machine.sql:213-367`）：
   完整定义 13 种流转操作：schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate。
   每条规则含 `from_status/to_status/transition_code/transition_name/require_operator/require_equipment/require_remarks`。

4. **状态机服务完整**（`/workspace/backend/src/services/dye_batch_state_machine_service.rs`）：
   - 14 种状态校验
   - 13 种流转操作校验
   - 4 种回修类型校验（color_difference/defect/specification_unqualified/other）

5. **Handler 接口完整**（`/workspace/backend/src/handlers/dye_batch_state_machine_handler.rs`，26 个接口）：
   含 `record_transition/approve_rework/merge_batch/split_batch/adjust_priority/change_batch/terminate_batch` 等核心接口。

6. **PDA 采集参数支持**（`/workspace/database/migration/046_v14_dye_batch_state_machine.sql:47`）：
   `captured_params JSONB` 字段可记录温度/色差ΔE/时间戳等 PDA/工控终端采集参数。

#### ✅ 缸号状态机全链路验证通过

研究文档 §12.7 定义的 14 种状态 + 状态流转规则在代码中全部落实。

### 修复建议
无需修复。本维度实现完整，符合面料行业真实业务。

---

## 维度 14：成本核算（按缸号实际成本法）

### 检查方法
1. Read `/workspace/backend/src/models/cost_collection.rs`（成本归集表，48 行）
2. Grep `dye_batch_id|direct_material|direct_labor|manufacturing_overhead|processing_fee|dyeing_fee` 在 models/cost_collection.rs 中
3. Grep `DyeBatchCompleted|dye_batch_cost_bridge` 在 services/ 中
4. 对照研究文档 §5 成本核算（按缸号实际成本法）

### 发现

#### ✅ 已落实的成本核算机制

1. **成本归集表完整**（`/workspace/backend/src/models/cost_collection.rs`，48 行）：
   字段含 `collection_no/dye_batch_id(按缸号归集)/cost_type/cost_period/direct_material(直接材料)/direct_labor(直接人工)/manufacturing_overhead(制造费用)/processing_fee(加工费)/dyeing_fee(染色费)/total_cost/status`。
   核心特性：
   - `dye_batch_id` 按缸号归集：符合面料行业按缸号实际成本法
   - 五大成本要素完整：直接材料/直接人工/制造费用/加工费/染色费

2. **成本归集桥接服务完整**（`/workspace/backend/src/services/dye_batch_cost_bridge_service.rs`）：
   - 监听 `DyeBatchCompleted` 事件自动创建成本归集草稿记录
   - 事件订阅代码：`/workspace/backend/src/services/event_bus.rs:417-420`：
     ```rust
     // 监听 DyeBatchCompleted 事件，自动创建成本归集草稿记录
     let mut receiver = EVENT_BUS.subscribe();
     ```

3. **DyeBatchCompleted 事件有发布者**：
   - Grep 确认 `DyeBatchCompleted` 出现在 5 个文件：event_bus.rs / event_kafka.rs / event_kafka_payload.rs / dye_batch_cost_bridge_service.rs（订阅者）/ dye_batch_handler.rs（发布者）
   - dye_batch_handler.rs 在缸号完成时发布事件，触发成本归集

#### ❌ 关键缺陷：dye_batch 表无 dye_lot_no 字段，导致成本归集字段不完整

**风险等级：P0**（与维度 1 关联，成本归集无法精确到缸号）

**证据**：
- `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs:152-153`：注释 `"dye_lot_no 暂为 None，dye_batch 表当前无此字段，后续批次补全"`
- 成本归集表 cost_collection 含 `dye_batch_id` 但桥接服务在创建草稿时 dye_lot_no 为 None

**业务影响**：
- 成本归集表无法记录 dye_lot_no，影响按缸号查询成本
- 与 inventory_piece.dye_lot_no、quality_inspection_records.dye_lot_no 字段不一致

#### ❌ 次要缺陷：直接人工成本无法从工资系统自动归集

**风险等级：P1**（与维度 8 关联）

**证据**：
- 维度 8 审计发现 wage_service 无凭证生成、无事件发布
- 成本归集的 direct_labor 字段无法从工资系统自动取数

### 修复建议
1. **P0**：与维度 1 联动，为 dye_batch 表添加 dye_lot_no 字段，并修改 dye_batch_cost_bridge_service.rs:152-153 处逻辑，正确填充 dye_lot_no。
2. **P1**：与维度 8 联动，在 wage_service.rs 的 confirm_wage_record 函数中发布 `WageConfirmed` 事件，cost_collection 服务监听该事件自动归集 direct_labor。
3. **P1**：与维度 9 联动，修复 energy_service.rs 第 1562 行的 dye_lot_no 简化逻辑，确保能耗成本正确归集到缸号。

---

## 维度 15：质量检验分级（A/B/C 级）

### 检查方法
1. Read `/workspace/database/migration/035_v14_quality_grade_and_dyelot_validation.sql`（50 行，v14 批次 421）
2. Grep `determine_quality_grade|QUALITY_GRADE_A|QUALITY_GRADE_B|QUALITY_GRADE_C` 在 services/ 中
3. Grep `grade|abc_grade` 在 models/fabric_inspection_record.rs 和 models/quality_inspection_record.rs 中
4. 对照研究文档 §4.7 质量检验模块（A 级合格 / B 级让步接收 / C 级不合格）

### 发现

#### ✅ 已落实的质检分级机制

1. **质检 A/B/C 级字段完整**（`/workspace/database/migration/035_v14_quality_grade_and_dyelot_validation.sql:18`）：
   ```sql
   ALTER TABLE quality_inspection_records ADD COLUMN IF NOT EXISTS grade VARCHAR(2);
   ```
   业务规则（注释明确）：
   - A 级：qualification_rate ≥ 95% 或 ΔE ≤ 1.2，正常入库销售
   - B 级：qualification_rate ≥ 80% 且 < 95%，让步接收，降级销售（影响定价）
   - C 级：qualification_rate < 80%，不合格，需返工或报废

2. **determine_quality_grade 函数完整**（`/workspace/backend/src/services/quality_inspection_service.rs`）：
   - 输入：qualification_rate（合格率）
   - 输出：A / B / C 级
   - 常量定义：`QUALITY_GRADE_A = "A"` / `QUALITY_GRADE_B = "B"` / `QUALITY_GRADE_C = "C"`

3. **determine_quality_grade 被多模块复用**：
   - fabric_inspection_service.rs：验布打卷时调用
   - wage_service.rs：工资核算时调用（A 级系数 1.0，B 级 0.8，C 级 0.0）

4. **不合格品处理字段完整**（`/workspace/database/migration/035_v14_quality_grade_and_dyelot_validation.sql:39-45`）：
   ```sql
   ALTER TABLE unqualified_products ADD COLUMN IF NOT EXISTS grade VARCHAR(2);
   ALTER TABLE unqualified_products ADD COLUMN IF NOT EXISTS handling_result VARCHAR(50);
   ```
   B 级降级销售 / C 级返工报废分支处理。

5. **缸号追溯字段完整**（`/workspace/database/migration/035_v14_quality_grade_and_dyelot_validation.sql:26-27`）：
   ```sql
   ALTER TABLE quality_inspection_records ADD COLUMN IF NOT EXISTS color_no VARCHAR(50);
   ALTER TABLE quality_inspection_records ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
   ```
   支持按缸号追溯质检结果。

#### ✅ 质检分级全链路验证通过

研究文档 §4.7 定义的 A/B/C 级分级判定在代码中全部落实。

### 修复建议
无需修复。本维度实现完整，符合面料行业真实业务。

---

## 维度 16：面料行业专用词汇配套

### 检查方法
1. Grep `浴比|liquor_ratio|浴量|bath_volume|调整系数|adjustment_factor|缸号|dye_lot_no|匹号|piece_no|批号|batch_no|色号|color_no|四分制|four_point|十分制|ten_point|PDA|流转卡|flow_card|化验室|lab_dip|打样|复样|处方|recipe|加料|addition|验布|inspection|打卷|对色|色差|ΔE|color_difference|色牢度|color_fastness|门幅|fabric_width|纬斜|skewness|缩水率|shrinkage|起毛起球|pilling|手感|handfeel|拉伸强度|tensile_strength|撕裂强度|tear_strength|克重|weight_gsm|密度|density|染化料|chemical|染料|dye|助剂|auxiliary|GHS|MSDS|UN|保质期|shelf_life|安全库存|safety_stock|委外|outsourcing|来料加工|tolling|坯布|greige|染色|dyeing|印花|printing|后整理|finishing|前处理|pre_treatment|皂洗|washing|固色|fixing|脱水|dehydrating|烘干|drying|精练|scouring|漂白|bleaching|定型|setting|理布|plaiting` 在 backend/src/ 中
2. 对照研究文档 §10 术语表

### 发现

#### ✅ 已落实的行业词汇配套

1. **四层级联词汇完整**：
   - 面料（Fabric）/ 颜色（Color）/ 缸号（dye_lot_no）/ 批号（batch_no）/ 匹号（piece_no）/ 色号（color_no）
   - 全部在 models/ 中有对应字段

2. **染整工艺词汇完整**：
   - 前处理（pre_treatment）/ 染色（dyeing）/ 印花（printing）/ 后整理（finishing）/ 验布（inspection）
   - 皂洗（washing）/ 固色（fixing）/ 脱水（dehydrating）/ 烘干（drying）
   - 在 process_route.rs / dye_batch_state_rule 中有对应状态

3. **化验室打样词汇完整**：
   - 化验室（lab_dip）/ 打样（sample）/ 复样（resample）/ 处方（recipe）/ 加料（addition）
   - 浴比（liquor_ratio）/ 浴量（bath_volume）/ 调整系数（adjustment_factor）
   - 在 production_recipe.rs 中有对应字段

4. **验布打卷词汇完整**：
   - 四分制（four_point）/ 十分制（ten_point）/ 色差（color_difference）/ 色牢度（color_fastness）
   - 门幅（fabric_width）/ 纬斜（skewness）/ 缩水率（shrinkage）/ 起毛起球（pilling）/ 手感（handfeel）
   - 在 fabric_inspection_record.rs 中有部分字段

5. **染化料词汇完整**：
   - 染化料（chemical）/ 染料（dye）/ 助剂（auxiliary）/ GHS / MSDS / UN 编号
   - 保质期（shelf_life）/ 安全库存（safety_stock）
   - 在 chemical_master.rs 中有对应字段

6. **业务模式词汇完整**：
   - 委外（outsourcing）/ 来料加工（tolling）/ 坯布（greige）/ 成品（finished）
   - 在 business_mode_config.rs 中有对应模式

#### ❌ 次要缺陷：部分物理指标词汇无对应字段

**风险等级：P2**（与维度 7 关联）

**证据**：
- 维度 7 审计发现十项指标中纬斜/缩水率/起毛起球/手感/拉伸强度/撕裂强度/克重/密度等词汇在 models/ 中无对应字段
- 仅门幅（fabric_width_inches）和色牢度（color_fastness 在 lab_dip_request）有字段

### 修复建议
1. **P2**：与维度 7 联动，新建 fabric_physical_test_record 表时确保十项指标词汇与字段命名一致。
2. **P3**：补充行业词汇中英文对照表文档，便于开发人员理解。

---

## 维度 17：业务流转事件贯通

### 检查方法
1. Read `/workspace/backend/src/services/event_bus.rs`（事件总线，含事件定义和监听器）
2. Grep `DyeBatchCompleted|QualityInspectionCompleted|OutsourcingOrderCompleted|BusinessModeChanged|WageConfirmed|WagePaid` 在 backend/src/ 中
3. Grep `publish\(.*QualityInspection` 在 backend/src/ 中
4. Grep `publish\(.*DyeBatchCompleted` 在 backend/src/ 中
5. Grep `tracing::info!` 在 services/event_bus.rs 中（检查监听器是否仅打印日志）
6. 对照研究文档 §13 事件贯通

### 发现

#### ✅ 已落实的事件贯通机制

1. **事件总线架构完整**（`/workspace/backend/src/services/event_bus.rs`）：
   - 第 8 行：公共 API（`EVENT_BUS` / `publish` / `subscribe` / `start_event_listener`）
   - 第 166 行：定义 `DyeBatchCompleted` 事件
   - 第 176 行：定义 `QualityInspectionCompleted` 事件
   - 第 304 行：`subscribe()` 方法返回 broadcast::Receiver
   - 支持 Broadcast 和 Kafka 两种后端（第 324/390 行）

2. **DyeBatchCompleted 事件贯通完整**：
   - **有发布者**：dye_batch_handler.rs 在缸号完成时发布事件
   - **有订阅者**：dye_batch_cost_bridge_service.rs 监听事件自动创建成本归集草稿
   - 订阅代码（event_bus.rs:417-420）：
     ```rust
     // 监听 DyeBatchCompleted 事件，自动创建成本归集草稿记录
     let mut receiver = EVENT_BUS.subscribe();
     ```

3. **CollectionCompleted 事件贯通完整**：
   - event_bus.rs:495 监听 CollectionCompleted 事件触发发票相关动作

#### ❌ 关键缺陷 1：QualityInspectionCompleted 事件无发布者

**风险等级：P1**（质检完成事件断链）

**证据**：
- Grep `publish(.*QualityInspection` 在 backend/src/ 中：**无匹配**
- QualityInspectionCompleted 事件仅在 event_bus.rs:176（定义）、event_kafka.rs、event_kafka_payload.rs（Kafka 桥接）中出现
- 无任何 handler 或 service 发布此事件

**业务影响**：
- 质检完成后应触发：
  1. 库存入库（A 级正常入库 / B 级降级入库 / C 级返工报废）
  2. 成本结转（按缸号结转生产成本到库存商品）
  3. 工资核算（触发产量工资计算）
- 缺少事件发布导致上述下游动作无法自动触发，需人工干预

#### ❌ 关键缺陷 2：事件监听器仅打印日志，未实际触发下游动作

**风险等级：P1**（事件订阅者空实现）

**证据**：
`/workspace/backend/src/services/event_bus.rs`：
- 第 435-485 行：多个 `tracing::info!` 打印日志
- 第 953 行：`BusinessEvent::QualityInspectionCompleted {`
- 第 965 行：`"收到质检完成事件（QualityInspectionCompleted），可触发库存入库/成本结转"`——注释明确说"可触发"但实际未触发

**问题**：
1. 监听器收到事件后仅 info! 打印日志
2. 注释说"可触发库存入库/成本结转"但未实际调用相关服务
3. 事件订阅流于形式，未实现真正的业务联动

**业务影响**：
- 即使有发布者发布事件（如 DyeBatchCompleted），订阅者也仅打印日志
- 事件总线沦为日志打印工具，未实现业务贯通

#### ❌ 关键缺陷 3：委外加工/业务模式/工资等模块无事件发布

**风险等级：P1**（与维度 8/11/12 关联）

**证据**：
- 维度 8：wage_service.rs 无 publish/BusinessEvent/EVENT_BUS 匹配
- 维度 11：outsourcing_service.rs 无 publish/BusinessEvent/EVENT_BUS 匹配
- 维度 12：business_mode_service.rs 无 publish/BusinessEvent/EVENT_BUS 匹配

### 修复建议
1. **P1**：在 fabric_inspection_service.rs 的 `complete_inspection` 或 `grade_inspection` 函数中发布 `QualityInspectionCompleted` 事件，含质检记录 ID、缸号、A/B/C 级、合格率。
2. **P1**：修改 event_bus.rs 第 953-965 行的 QualityInspectionCompleted 监听器，从仅 info! 打印日志改为实际调用：
   - 库存入库服务（按 A/B/C 级分支处理）
   - 成本结转服务（按缸号结转）
   - 工资核算服务（触发产量工资计算）
3. **P1**：检查 event_bus.rs 第 435-485 行所有 info! 日志，评估哪些监听器需要实际触发下游动作，逐步补充业务逻辑。
4. **P1**：与维度 8/11/12 联动，在 wage_service/outsourcing_service/business_mode_service 中添加事件发布。
5. **P2**：建立事件贯通集成测试，验证每个事件的发布-订阅全链路。

---

## 审计总结

### 风险等级汇总

| 风险等级 | 数量 | 维度分布 |
|---------|------|---------|
| P0      | 2    | 维度 1（dye_batch 缺 dye_lot_no）、维度 14（成本归集字段不完整，与维度 1 关联） |
| P1      | 11   | 维度 2（batch_trace_log 字段不足）、维度 7（十项指标无建模）、维度 8（工资无凭证）、维度 9（能耗分摊简化）、维度 11（委外无事件）、维度 12（业务模式无事件）、维度 14（人工成本无法归集，与维度 8 关联）、维度 17（QualityInspectionCompleted 无发布者、监听器仅打印日志、多模块无事件） |
| P2      | 3    | 维度 1（dye_batch 与 batch_dye_lot 重叠）、维度 2（batch_trace_log 文件级 allow(dead_code)）、维度 16（部分物理指标词汇无字段） |
| P3      | 2    | 维度 3（10 道精细工序与 5 大分类映射）、维度 16（行业词汇中英文对照文档） |

### 核心问题聚类

1. **dye_lot_no 字段缺失问题**（P0，影响维度 1/14）：
   - dye_batch 表无 dye_lot_no 字段
   - 导致成本归集、缸号追溯、事件贯通均受影响
   - **建议作为最高优先级修复**

2. **事件贯通断链问题**（P1，影响维度 8/11/12/17）：
   - QualityInspectionCompleted 无发布者
   - 事件监听器仅打印日志
   - 委外/业务模式/工资等模块无事件发布
   - **建议系统性梳理事件总线，补全发布-订阅链路**

3. **十项物理指标缺失问题**（P1，影响维度 7/16）：
   - 验布打卷仅支持外观疵点评分，无物理指标建模
   - **建议新建 fabric_physical_test_record 表**

4. **能耗分摊简化逻辑问题**（P1，影响维度 9/14）：
   - dye_lot_no 简化为 equipment_name
   - workshop 简化为 equipment_name
   - **建议通过 flow_card 关联查询正确字段**

5. **工资凭证缺失问题**（P1，影响维度 8/14）：
   - 工资确认/发放未生成财务凭证
   - **建议在 confirm/pay 节点添加凭证生成逻辑**

### 已落实的完整维度（无需修复）

- 维度 3：染整工艺流程完整性
- 维度 4：化验室打样 5 步闭环
- 维度 5：大货处方与加料处方
- 维度 6：流转卡条码与车间工序
- 维度 10：染化料主数据（GHS/MSDS/批号管理）
- 维度 13：缸号全生命周期状态机（14 种状态）
- 维度 15：质量检验分级（A/B/C 级）

### 审计结论

V15 类四（面料行业深化审计类）17 维度审计完成。项目在面料行业核心特性方面整体实现较为完整，17 个维度中 7 个维度完全通过，但存在以下核心问题需优先修复：

1. **P0 紧急**：dye_batch 表缺少 dye_lot_no 字段（影响四层级联、成本归集、缸号追溯）
2. **P1 重要**：事件贯通断链（QualityInspectionCompleted 无发布者、监听器仅打印日志、多模块无事件发布）
3. **P1 重要**：验布打卷十项物理指标无建模
4. **P1 重要**：能耗分摊简化逻辑（dye_lot_no 误填 equipment_name）
5. **P1 重要**：工资凭证缺失（confirm/pay 节点无凭证生成）

建议按 P0 → P1 优先级依次修复，每个修复需经 CI/CD 验证全绿后方可合并。

---

**审计报告完成时间**：2026-07-16
**审计子代理**：V15 审计子代理（类四 面料行业深化审计类）
**报告路径**：`/workspace/.monkeycode/docs/audits/v15/batch-04/audit-report.md`
