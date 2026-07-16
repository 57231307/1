# V15 运行逻辑闭环深化审计报告（类五·批次 05）

- **审计子代理**：V15 审计子代理（类五 运行逻辑闭环深化审计类）
- **审计范围**：7 维度（业务流程闭环 / 异常路径闭环 / 状态机闭环 / 资源生命周期闭环 / 配置依赖闭环 / 事件闭环 / 业财一致性闭环）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15/batch-04/audit-report.md`（批次 04 报告格式参考）
  - `/workspace/backend/src/services/dye_batch_state_machine_service.rs`（缸号状态机服务）
  - `/workspace/backend/src/services/event_bus.rs`（业务事件总线）
  - `/workspace/backend/src/services/event_retry_service.rs`（事件重试与死信队列）
  - `/workspace/backend/src/services/inventory_finance_bridge_service.rs`（库存财务桥接）
  - `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs`（染色成本桥接）
  - `/workspace/backend/src/services/flow_card_service.rs`（流转卡扫码服务）
  - `/workspace/backend/src/services/lab_dip_service.rs`（化验室打样服务）
  - `/workspace/backend/src/services/so/order_workflow.rs`（销售订单工作流）
  - `/workspace/backend/src/services/so/delivery.rs`（销售出库服务）
  - `/workspace/backend/src/services/production_order_service.rs`（生产订单服务）
  - `/workspace/backend/src/services/audit_log_service.rs`（审计日志 mpsc channel）
  - `/workspace/backend/src/services/energy_service.rs`（能耗月末分摊）
  - `/workspace/backend/src/utils/app_state.rs`（应用全局状态）
  - `/workspace/backend/src/main.rs`（启动期 fail-fast 与后台任务管理）
  - `/workspace/backend/.env.example`（环境变量示例）
  - `/workspace/backend/config.yaml.example`（配置文件示例）
  - `/workspace/deploy/deploy-latest.sh`（部署脚本 config.yaml 注入）
  - `/workspace/backend/src/models/status.rs`（状态常量定义）
- **审计方法**：Grep 检索关键代码模式 + Read 关键文件 + 对照审计计划核对实现完整性
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；面料行业核心特性缺失标 P0/P1；证据充分附文件路径:行号

---

## 维度 1：业务流程闭环（5.1）

### 检查方法
1. Read `/workspace/backend/src/models/process_route.rs`（工序路线模板）
2. Read `/workspace/backend/src/services/lab_dip_service.rs`（化验室打样 5 步闭环）
3. Read `/workspace/backend/src/services/flow_card_service.rs`（流转卡扫码上报）
4. Read `/workspace/backend/src/services/so/order_workflow.rs`（销售订单审批→库存预留）
5. Read `/workspace/backend/src/services/so/delivery.rs`（销售出库→凭证）
6. Read `/workspace/backend/src/services/production_order_service.rs`（生产订单完成→成本归集）
7. Grep `配布|精练|漂白|染色|对色|理布|烘干|定型|成检` 在 backend/src 中
8. Grep `重染|补染|降级|返工|报废` 在 backend/src/services 中
9. Grep `drill_down|穿透追溯` 在 backend/src/services 中

### 发现

#### ✅ 已落实的项

1. **染整 10 道工序闭环（工序路线模板自定义）**：
   - `/workspace/backend/src/models/process_route.rs:17-19`：标准流程 前处理(PRE_TREAT) → 染色(DYE) → 印花(PRINT) → 后整理(FINISH) → 验布(INSPECT)，工序路线在后台自定义可根据车间布局增删调整顺序。
   - `/workspace/backend/src/services/flow_card_service.rs:14`：工序流转扫码（扫描流转卡条码 → 登记工人 → 开始/结束工序 → 自动统计产量），实现扫码→工序流转→进度跟踪闭环。
   - `/workspace/backend/src/services/dye_batch_state_machine_service.rs:11`：缸号状态机 14 种状态含 pending_schedule→scheduled→preparing→dyeing→washing→fixing→dehydrating→drying→inspecting→stored→shipped 全链路，覆盖染整核心工序。

2. **化验室打样 5 步闭环完整实现**：
   - `/workspace/backend/src/services/lab_dip_service.rs:1-12`：注释明确"打样通知单 → 打样（ABCD 多版样）→ 色样确认（OK 样）→ 复样 → 建数据库"5 步闭环。
   - 打样通知单状态机：pending → sampling → submitted → approved/rejected → completed（`lab_dip_service.rs:8`）。
   - ABCD 多版样管理 + 对色结果记录（`lab_dip_service.rs:697` record_matching_result）。
   - OK 样确认（客户从多版中选 1 版，状态 → selected）（`lab_dip_service.rs:367`）。
   - 复样记录 CRUD + 复样结果判定（passed/failed/adjusted）（`status.rs:770-782`）。
   - 染色技术卡开具（复样通过后由研发组长开卡）（`lab_dip_handler.rs:319-329`）。

3. **流转卡扫码上报闭环**：
   - `/workspace/backend/src/services/flow_card_service.rs:714-746`：扫码开始工序（自动创建 pending 记录并切换到 in_progress）+ 扫码结束工序（in_progress → completed）。
   - `/workspace/backend/src/services/wage_service.rs:6`：工序流转扫码 → process_step_record 自动记录工人 IDs + 实际产量 + 合格产量，工资按产量自动计算。

4. **销售订单审批→库存预留/锁定闭环**：
   - `/workspace/backend/src/services/so/order_workflow.rs:272-313`：approve_order commit 成功后调用 InventoryReservationService::create_reservation 为每个明细创建库存预留记录。批次 356 v13 复审 B-P0-1 修复，原实现不调用导致超卖风险。
   - `/workspace/backend/src/services/inventory_reservation_service.rs:22-115`：库存预留状态机 PENDING→LOCKED→RELEASED 完整闭环，含 lock_reservation/release_reservation。

5. **销售出库→库存凭证+收入凭证+成本凭证闭环**：
   - `/workspace/backend/src/services/so/delivery.rs:405-495`：F-P0-3+F-P0-6 修复，每次发货生成收入确认凭证（借：应收账款 / 贷：主营业务收入 + 应交税费-销项税额），调用 voucher_service.create_and_post。
   - `/workspace/backend/src/services/so/delivery.rs:497-499`：commit 后统一发布 SALES_DELIVERY 库存流水事件，触发 inventory_finance_bridge 自动生成销售出库凭证（借：主营业务成本 / 贷：库存商品）。

6. **生产订单完成→成本归集闭环**：
   - `/workspace/backend/src/services/production_order_service.rs:588-627`：批次 356 v13 复审 B-P0-3 修复，complete_production_order commit 成功后调用 CostCollectionService::create 做成本归集。原实现不调用导致生产成本无法归集。

7. **订单→生产→染整→入库→发货→售后→报表 全链路**：
   - 销售退货：`/workspace/backend/src/services/sales_return_service.rs`（退货单 CRUD + 状态流转）。
   - 报表穿透追溯：`/workspace/backend/src/services/finance_report_service.rs:808-877` drill_down_by_subject_prefix / drill_down_by_period_and_subject 实现试算平衡表穿透。

#### ❌ 缺陷项 1：染整 10 道工序闭环与行业工序类型映射不完整

**风险等级：P3**（业务可通过 process_route 自定义，但缺少行业标准化映射）

**证据**：
- `/workspace/backend/src/models/process_route.rs:32`：工序类型仅支持 `pretreat(前处理)/dye(染色)/print(印花)/finish(后整理)/inspect(验布)/other` 6 种。
- 审计计划要求"染整 10 道工序闭环：配布→精练→漂白→染色→对色→理布→烘干→定型→成品对色→成检→反馈工艺优化"，缺少精练/漂白/对色/理布/定型/成品对色/成检等细分工序类型的显式枚举。

**业务影响**：
- 6 种工序类型可覆盖主要场景，但面料行业 10 道工序的精细化管理（如精练与漂白分离、对色与成检分离）需通过 process_route 表自定义 route_name 实现，缺乏枚举层面的标准化约束。

**修复建议**：
- 在 process_type 枚举中补充 `scour(精练)/bleach(漂白)/color_match(对色)/cloth_arrange(理布)/set(定型)/final_color_match(成品对色)/final_inspect(成检)` 等行业细分类型，或在文档中明确"通过 route_name 自定义 + process_type=other 兜底"的设计意图。

#### ❌ 缺陷项 2：缺少"反馈工艺优化"与"反馈配方优化"的显式闭环

**风险等级：P2**（业务闭环最后一步缺失，影响持续改进能力）

**证据**：
- `/workspace/backend/src/services/lab_dip_service.rs`：复样通过后开具染色技术卡，但未自动将 OK 样配方反馈到 dye_recipe 配方库优化（无 dye_recipe_service 调用链路）。
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs`：缸号状态机流转到 shipped/cancelled/terminated 终态后，未自动触发工艺优化反馈事件（无 BusinessEvent 发布）。

**业务影响**：
- 化验室打样 5 步闭环的"反馈配方优化"和染整 10 道工序闭环的"反馈工艺优化"是面料行业持续改进的核心环节，缺失导致历史生产数据无法反哺配方优化和工艺改进。

**修复建议**：
- 在 lab_dip_resample PASSED 后自动调用 dye_recipe_service 创建/更新配方库记录。
- 在 DyeBatchCompleted 或 QualityInspectionCompleted 事件处理中增加工艺优化反馈逻辑（如更新 process_route 默认工时、记录工艺偏差）。

---

## 维度 2：异常路径闭环（5.2）

### 检查方法
1. Grep `let _ =` 在 backend/src/services 中（吞错检测）
2. Grep `\.unwrap\(\)|\.expect\(` 在 backend/src/services 中
3. Grep `dead_letter|死信|DLQ` 在 backend/src 中
4. Grep `锁中毒|poisoned|into_inner|is_poisoned` 在 backend/src 中
5. Grep `Drop|impl Drop|File::open|File::create` 在 backend/src 中
6. Read `/workspace/backend/src/services/event_retry_service.rs`（事件重试与死信队列）
7. Read `/workspace/backend/src/services/quality_inspection_service.rs`（验布不合格处理路径）
8. Grep `重染|补染|降级|返工|报废|rework|scrap|downgrade` 在 backend/src/services 中

### 发现

#### ✅ 已落实的项

1. **let _ = 吞错已大量修复**：
   - 批次 80/94/98/113/377 等多处修复，event_notification_service.rs:117、purchase_price_service.rs:139、bpm_service.rs:910 等均注释说明"原 let _ = 静默吞错"已改为 ? 传播或显式断言。
   - 仅剩的 `let _ =` 多为测试代码（cache_service.rs:294-296 测试缓存命中）或显式丢弃 ActiveModel 返回值（已注释说明）。

2. **验布不合格有处理路径（降级/返工/报废）**：
   - `/workspace/backend/src/services/quality_inspection_service.rs:22-41`：B 级（让步接收）→ 降级销售（downgrade_sale）；C 级（不合格）→ 返工（rework）或报废（scrap）。
   - `/workspace/backend/src/services/quality_inspection_service.rs:64-93`：validate_handling_method 强制校验 B 级品必须降级销售、C 级品必须返工或报废，违反则返回业务错误。

3. **事件处理失败有重试（指数退避）+死信队列+告警完整实现**：
   - `/workspace/backend/src/services/event_retry_service.rs:17`：MAX_RETRY_COUNT=5。
   - `/workspace/backend/src/services/event_retry_service.rs:85-91`：calculate_backoff_delay 指数退避公式 delay = BASE_DELAY * 2^retry_count，上限 60 秒。
   - `/workspace/backend/src/services/event_retry_service.rs:50-67`：超过最大重试次数入死信队列（event_dead_letters 表）+ tracing::error! 告警。
   - `/workspace/backend/src/models/event_dead_letter.rs:24`：死信状态 PENDING/DEAD/RESOLVED 完整闭环。

4. **数据库事务 commit/rollback 路径完备**：
   - `/workspace/backend/src/services/so/order_workflow.rs:36-74`：cancel_order 使用 txn = begin() + lock_exclusive + update_with_audit + txn.commit()，? 传播错误时 DatabaseTransaction Drop 自动 rollback。
   - `/workspace/backend/src/services/ar/inv.rs:297`：注释明确"事务回滚由调用方 txn 的 Drop 实现"。
   - `/workspace/backend/src/services/po/order.rs:289`：注释明确"事务将在 ? 传播 Err 时由 DatabaseTransaction 的 Drop 自动回滚"。

5. **Arc<Mutex<T>> 锁中毒统一降级处理**：
   - `/workspace/backend/src/utils/di_container.rs:11-17`：注释明确"互斥锁中毒通常意味着有线程 panic，状态已不可信。安全修复：改为优雅降级（e.into_inner() 恢复数据继续运行）"。
   - `/workspace/backend/src/services/audit_log_service.rs:94-98`：shutdown 路径使用 `unwrap_or_else(|e| e.into_inner())` 安全访问 poisoned lock。
   - `/workspace/backend/src/services/omni_audit_service.rs:255-258`：同上模式。
   - `/workspace/backend/src/main.rs:106-112`：setup 初始化标志锁中毒降级。
   - `/workspace/backend/src/services/event_bus.rs:236-238`：lock_event_bus_state 锁中毒降级。
   - `/workspace/backend/src/utils/app_state.rs:389-395`：APP_STATE_BACKGROUND_TASKS 锁中毒降级。

6. **文件句柄 Drop 闭环（RAII 自动 Drop）**：
   - `/workspace/backend/src/services/system_update_service.rs:400,737,830`：File::open/File::create 使用 ? 传播错误，Rust RAII 保证文件句柄离开作用域自动 Drop。
   - `/workspace/backend/src/telemetry.rs:297`：impl Drop for TelemetryGuard 显式实现 Drop 闭环。

#### ❌ 缺陷项 1：染整工序失败缺少显式"重染/补染"恢复路径

**风险等级：P2**（异常恢复路径不完整）

**证据**：
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:99-113`：rework_type 仅支持 `color_difference(色差)/defect(疵点)/specification_unqualified(规格不合格)/other` 4 种类型。
- 审计计划要求"染整工序失败有恢复路径（重染/补染/降级）"，但 rework_type 枚举未明确包含"重染(re_dye)/补染(replenish_dye)"类型。
- 缸号状态机有 rework 状态（inspecting/stored → rework → dyeing），但回修类型无法区分"重染"与"补染"的业务语义差异。

**业务影响**：
- 面料行业染整失败场景中，"重染"（整缸重新染色，成本高）与"补染"（局部补色，成本低）是两种不同的恢复策略，缺少枚举区分导致业务统计和成本核算无法精确分类。

**修复建议**：
- 在 dye_batch_rework_type 枚举中补充 `re_dye(重染)/replenish_dye(补染)` 类型，并在 dye_batch_rework 表记录对应的成本字段（重染成本/补染成本）。

#### ❌ 缺陷项 2：缸号状态机异常缺少告警+死信队列机制

**风险等级：P2**（异常态无告警可能导致生产停滞未及时发现）

**证据**：
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:372-378`：validate_transition_with_rule 失败仅返回 AppError::business，未触发告警或写入死信队列。
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:152-160`：is_terminal_status 仅判断 shipped/cancelled/terminated，无异常态告警逻辑。
- 缺少缸号状态机异常态的告警通知机制（如通知生产调度人员、写入告警表）。

**业务影响**：
- 缸号状态机流转异常（如非法状态跳转、终态后尝试流转）仅返回错误给调用方，未触发生产调度告警，可能导致异常缸号长期未处理影响交付。

**修复建议**：
- 在 validate_transition_with_rule 失败时增加 tracing::warn! 告警 + 可选的告警事件发布（BusinessEvent::DyeBatchStateException）。
- 对于关键状态流转异常（如 dyeing → terminated），增加死信队列写入以便人工复核。

#### ❌ 缺陷项 3：生产订单成本归集失败仅 warn 不重试

**风险等级：P3**（失败后有 warn 日志但无自动重试机制）

**证据**：
- `/workspace/backend/src/services/production_order_service.rs:621-627`：cost_service.create 失败时仅 tracing::warn!("批次 356 B-P0-3: 生产订单成本归集失败，请人工检查")，未接入 event_retry_service 重试机制。

**业务影响**：
- 成本归集失败后需人工检查，缺少自动重试可能导致成本数据缺失。

**修复建议**：
- 将成本归集失败接入 event_retry_service 重试机制，超过重试次数后入死信队列。

---

## 维度 3：状态机闭环（5.3）

### 检查方法
1. Read `/workspace/backend/src/services/dye_batch_state_machine_service.rs`（缸号状态机）
2. Read `/workspace/backend/src/utils/process_state_machine.rs`（定制订单工艺流程状态机）
3. Read `/workspace/backend/src/services/init_service.rs:20-40`（InitTaskStatus）
4. Read `/workspace/backend/src/models/status.rs:740-810,1118-1133`（色卡/对色结果状态）
5. Grep `color_card.*status|ColorCardStatus|MatchStatus|Disputed` 在 backend/src 中
6. Grep `Failed|OnHold|DyeBatchStatus` 在 backend/src/services/dye_batch_state_machine_service.rs 中

### 发现

#### ✅ 已落实的项

1. **缸号状态机 14 种状态 + 3 终态 + rework 恢复路径**：
   - `/workspace/backend/src/services/dye_batch_state_machine_service.rs:11`：14 种状态 pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored/shipped(终态)/cancelled(终态)/terminated(终态)/rework。
   - `/workspace/backend/src/services/dye_batch_state_machine_service.rs:152-160`：is_terminal_status 判断 shipped/cancelled/terminated 为终态。
   - `/workspace/backend/src/services/dye_batch_state_machine_service.rs:270-317`：rework 恢复路径（inspecting/stored → rework → dyeing 重新进缸染色）。
   - `/workspace/backend/src/services/dye_batch_state_machine_service.rs:382-394`：check_rework_eligibility 校验只有 inspecting/stored 状态可回修。
   - 内置流转规则表 33 条规则（`builtin_transition_rules` L165-318），完整覆盖所有合法流转。

2. **染整工单状态机（流转卡）**：
   - `/workspace/backend/src/models/production_flow_card.rs:9`：状态机 pending(待排缸) → scheduled(已排缸) → preparing(备布中) → dyeing(染色中) → dyed(已出缸) → inspecting(验布中) → completed(已完成) → shipped(已发货) / terminated(已终止)。

3. **BorrowStatus 完整终态**：
   - `/workspace/backend/src/services/color_card_borrow_service.rs:437-441`：borrowed → returned/lost/damaged/cancelled 4 个终态完整。

4. **process_state_machine.rs 完整闭环**：
   - `/workspace/backend/src/utils/process_state_machine.rs:13-30`：CustomOrderStatus 8 种状态（Draft/YarnPurchasing/Dyeing/Finishing/Delivery/AfterSales/Completed/Cancelled）。
   - `/workspace/backend/src/utils/process_state_machine.rs:48-50`：is_terminal 判断 Completed/Cancelled 为终态。
   - `/workspace/backend/src/utils/process_state_machine.rs:91-99`：next_status 状态机推进 + 终态保护。

5. **InitTaskStatus 失败后恢复路径文档化**：
   - `/workspace/backend/src/services/init_service.rs:21-31`：状态机 Running → Completed | Failed（终态）；注释明确"Failed 后需重新调用 initialize 创建新 task_id 恢复"。

#### ❌ 缺陷项 1：缸号状态机缺少 Failed + OnHold 异常态

**风险等级：P1**（异常态缺失影响生产异常处理）

**证据**：
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:46-69`：validate_lifecycle_status 校验 14 种状态，无 Failed/OnHold 状态。
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:165-318`：builtin_transition_rules 33 条规则中无 Failed/OnHold 相关流转。
- 审计计划要求"缸号状态机：异常态 Failed+OnHold+恢复路径"和"DyeBatchStatus：增加 Failed+OnHold"。

**业务影响**：
- 染整过程中设备故障、染料异常、停电等场景需要"OnHold(暂停)"状态临时挂起缸号，待恢复后继续流转；"Failed(失败)"状态标识彻底失败的缸号（需返工或报废）。
- 当前仅有 cancelled/terminated 终态，无法区分"临时暂停"与"彻底终止"，导致生产调度无法精准识别可恢复的异常缸号。

**修复建议**：
- 在 dye_batch_lifecycle_status 枚举中增加 `on_hold(暂停)` 和 `failed(失败)` 状态。
- 增加流转规则：dyeing/washing/fixing/dehydrating/drying → on_hold（设备故障暂停）；on_hold → dyeing/washing/...（恢复）；任意非终态 → failed（彻底失败）；failed → cancelled/terminated（终态）。
- on_hold 状态增加超时告警机制（如暂停超过 4 小时触发告警）。

#### ❌ 缺陷项 2：色卡状态机缺少"发放→已收到→已使用→已过期"闭环

**风险等级：P2**（色卡生命周期管理不完整）

**证据**：
- `/workspace/backend/src/models/status.rs:1118-1126`：color_card 状态仅有 `ARCHIVED(已归档)` 和 `LOST(已丢失)` 两个常量。
- 审计计划要求"色卡状态机：发放→已收到→已使用→已过期（终态）"。
- 缺少 issued(已发放)/received(已收到)/used(已使用)/expired(已过期) 状态常量定义。

**业务影响**：
- 色卡作为面料行业客户对色的重要载体，其生命周期管理（发放给客户→客户确认收到→客户使用对色→过期回收）缺失，导致色卡库存和流转状态无法精确追踪。

**修复建议**：
- 在 color_card 状态枚举中补充 `ISSUED(已发放)/RECEIVED(已收到)/USED(已使用)/EXPIRED(已过期)` 状态。
- 实现色卡状态机流转校验：draft → issued → received → used → expired（终态）/ lost（终态）/ archived（终态）。

#### ❌ 缺陷项 3：MatchStatus 缺少 Disputed 终态

**风险等级：P3**（对色争议处理路径缺失）

**证据**：
- `/workspace/backend/src/models/status.rs:750-765`：lab_dip_sample.matching_result 仅有 `PENDING(待对色)/MATCHED(对色OK)/NOT_MATCHED(不匹配)/SELECTED(客户选中OK样)` 4 种状态。
- 审计计划要求"MatchStatus：增加 Disputed 终态"。
- 缺少 `DISPUTED(对色争议)` 状态，无法标识客户与厂方对色结果存在分歧的场景。

**业务影响**：
- 面料行业对色争议是常见场景（客户认为色差超标，厂方认为可接受），缺少 Disputed 状态导致争议对色样无法独立追踪和复盘。

**修复建议**：
- 在 lab_dip_sample 状态枚举中补充 `DISPUTED(对色争议)` 状态。
- 增加 PENDING/NOT_MATCHED → DISPUTED 流转，DISPUTED 后由研发组长仲裁后流转到 MATCHED（厂方让步）或 NOT_MATCHED（重打）。

---

## 维度 4：资源生命周期闭环（5.4）

### 检查方法
1. Grep `CancellationToken|cancellation_token` 在 backend/src 中
2. Grep `JoinHandle|spawn|abort|shutdown` 在 backend/src/services/event_bus.rs 中
3. Grep `recv_task|send_task|ConnectionEntry` 在 backend/src 中
4. Grep `mpsc|tx.send|rx.recv` 在 backend/src 中
5. Read `/workspace/backend/src/websocket/notifications.rs:340-475`（WebSocket 连接管理）
6. Read `/workspace/backend/src/main.rs:450-575`（后台任务 spawn 句柄保存）
7. Read `/workspace/backend/src/services/audit_log_service.rs:37-103`（mpsc channel 模式）
8. Grep `染缸设备|equipment.*occupy|equipment.*release|PDA|工控终端` 在 backend/src 中

### 发现

#### ✅ 已落实的项

1. **5 个后台定时任务句柄保存 + shutdown abort**：
   - `/workspace/backend/src/main.rs:77-95`：MAIN_BACKGROUND_TASKS 静态 Mutex<Vec<JoinHandle>>，shutdown_main_background_tasks() 遍历 abort。
   - `/workspace/backend/src/main.rs:528-534`：慢查询采集句柄保存。
   - `/workspace/backend/src/main.rs:546-556`：admin 角色缓存清理句柄保存。
   - `/workspace/backend/src/main.rs:563-573`：JTI 黑名单清理句柄保存。
   - `/workspace/backend/src/utils/app_state.rs:10-13`：APP_STATE_BACKGROUND_TASKS 静态 Mutex，保存审计清理 + 用户吊销清理句柄。
   - `/workspace/backend/src/utils/app_state.rs:388-401`：shutdown_app_state_background_tasks() 遍历 abort。
   - `/workspace/backend/src/main.rs:889-890`：graceful shutdown 时调用 shutdown_main_background_tasks() + shutdown_app_state_background_tasks()。

2. **Kafka 消费/事件监听/库存桥接/OmniAudit spawn 句柄保存到 EventBusState，shutdown 时 abort**：
   - `/workspace/backend/src/services/event_bus.rs:210-212`：EventBusState.consumer_handle 保存 Kafka 消费桥接句柄。
   - `/workspace/backend/src/services/event_bus.rs:409-410`：MAIN_LISTENER_HANDLE 保存主事件监听器句柄。
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:190-191`：BRIDGE_LISTENER_HANDLE 保存库存财务桥接监听器句柄。
   - `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs:24-25`：DYE_BATCH_COST_LISTENER_HANDLE 保存染色成本桥接监听器句柄。
   - `/workspace/backend/src/services/event_bus.rs:999-1006`：shutdown_event_bus() 统一 abort 所有事件总线 spawn task。
   - `/workspace/backend/src/main.rs:875-890`：graceful shutdown 时调用 shutdown_event_bus() + omni_audit.shutdown() + audit_log.shutdown()。

3. **WebSocket 长连接 recv_task/send_task 句柄保存与 abort**：
   - `/workspace/backend/src/websocket/notifications.rs:360`：recv_task = tokio::spawn(...)。
   - `/workspace/backend/src/websocket/notifications.rs:424`：send_task = tokio::spawn(...)。
   - `/workspace/backend/src/websocket/notifications.rs:459-466`：select! 用 &mut 借用 JoinHandle 而非消费（L-31 修复）。
   - `/workspace/backend/src/websocket/notifications.rs:468-472`：select! 后显式 abort recv_task 和 send_task，避免 detached task 泄漏。

4. **审计日志异步落库改 mpsc channel+单消费者模式**：
   - `/workspace/backend/src/services/audit_log_service.rs:25`：use tokio::sync::mpsc。
   - `/workspace/backend/src/services/audit_log_service.rs:34`：sender: mpsc::UnboundedSender。
   - `/workspace/backend/src/services/audit_log_service.rs:41`：mpsc::unbounded_channel 创建 channel。
   - `/workspace/backend/src/services/audit_log_service.rs:45-77`：后台消费者 task 循环 receiver.recv() 处理事件，panic 隔离（catch_unwind）。
   - `/workspace/backend/src/services/audit_log_service.rs:96-102`：shutdown() abort 后台 task，幂等安全。
   - `/workspace/backend/src/services/omni_audit_service.rs:39,85`：OmniAuditEngine 同样使用 mpsc::channel(10000) 模式。

5. **数据库事务 commit/rollback 路径完备**（同维度 2 已验证）。

6. **Arc<Mutex<T>> 锁中毒处理统一降级**（同维度 2 已验证）。

#### ❌ 缺陷项 1：5 个后台定时任务缺少 CancellationToken

**风险等级：P2**（功能等价但实现方式偏离审计计划要求）

**证据**：
- Grep `CancellationToken` 在 backend/src 中无任何匹配。
- 实际实现使用 `JoinHandle.abort()` 而非 `CancellationToken`。
- `/workspace/backend/src/main.rs:83-93`：shutdown_main_background_tasks() 通过 handle.abort() 终止任务。
- 审计计划明确要求"5 个后台定时任务有 CancellationToken"。

**业务影响**：
- abort() 是强制终止任务，可能在任务执行中间状态时中断（如数据库事务进行中）；CancellationToken 允许任务优雅退出（完成当前迭代后退出）。
- 对于涉及数据库操作的后台任务（如慢查询采集、审计清理），abort() 中断可能导致数据库连接泄漏或事务未提交。

**修复建议**：
- 引入 tokio_util::sync::CancellationToken，在每个后台任务的循环中检查 `token.is_cancelled()`，实现优雅退出。
- 保留 abort() 作为兜底（超时后强制终止）。

#### ❌ 缺陷项 2：染缸设备资源缺少显式占用/释放路径（V15 新增要求）

**风险等级：P2**（染缸设备资源管理缺失）

**证据**：
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:165-318`：缸号状态机流转涉及 dyeing 状态（进缸染色），但未发现染缸设备（equipment_id）的显式占用/释放资源管理逻辑。
- Grep `染缸设备|equipment.*occupy|equipment.*release|vat_occupy|vat_release` 在 backend/src 中无匹配。
- CreateTransitionRequest 含 equipment_id 字段（`dye_batch_state_machine_service.rs:411`），但仅作为记录字段，无占用/释放逻辑。

**业务影响**：
- 染缸是面料行业核心设备资源，多个缸号可能抢占同一染缸导致排缸冲突。缺少显式占用/释放管理可能导致：
  1. 同一染缸被多个缸号同时占用（排缸冲突）
  2. 缸号流转到 dyeing 状态时未校验染缸可用性
  3. 缸号流转出 dyeing 状态时未释放染缸资源

**修复建议**：
- 新增染缸设备占用表（dye_vat_occupation），记录 vat_id/batch_id/occupied_at/released_at。
- 在 dyeing 状态流转时（schedule/prepare → dyeing）校验染缸可用性并占用资源。
- 在 dyeing → washing 流转时释放染缸资源。
- 增加染缸超时占用告警（如占用超过 24 小时未释放）。

#### ❌ 缺陷项 3：PDA/工控终端连接缺少资源管理（V15 新增要求）

**风险等级：P2**（设备连接管理缺失）

**证据**：
- Grep `PDA|工控终端|device_connection|terminal_connection` 在 backend/src 中无匹配。
- 审计计划要求"PDA/工控终端连接：设备连接/断开有资源管理（V15 新增）"。

**业务影响**：
- 面料行业车间场景大量使用 PDA 扫码设备和工控终端，缺少连接资源管理可能导致：
  1. 设备断开后连接状态未清理，占用系统资源
  2. 设备重连时无法识别重复连接
  3. 设备离线时扫码数据无法缓存和重传

**修复建议**：
- 新增设备连接管理模块，记录 device_id/device_type/connected_at/disconnected_at/last_heartbeat。
- 实现 PDA/工控终端连接注册/注销/心跳检测/超时清理。
- 设备离线时缓存扫码数据，重连后自动同步。

---

## 维度 5：配置依赖闭环（5.5）

### 检查方法
1. Read `/workspace/backend/.env.example`（环境变量示例）
2. Read `/workspace/backend/config.yaml.example`（配置文件示例）
3. Read `/workspace/deploy/deploy-latest.sh:250-310`（config.yaml 生成注入）
4. Read `/workspace/deploy/deploy-backend.sh`（部署脚本）
5. Read `/workspace/backend/src/main.rs:420-460`（fail-fast 校验）
6. Read `/workspace/backend/src/utils/app_state.rs:177-199`（密钥强度校验）
7. Grep `DYEHOUSE_VAT_COUNT|PROCESS_UNIT_PRICE_BASE|ENERGY_ALLOCATION_RULE|QUALITY_GRADE_THRESHOLD|DYEBATCH_STATUS_TIMEOUT` 在 backend 中
8. Grep `AUTH_CHECK_USER_ACTIVE|AUDIT_RETENTION_DAYS|BINGXI_SLOW_QUERY_MS|RATE_LIMIT_REDIS_URL|ELASTICSEARCH_URL` 在 backend/src 中
9. Grep `INIT_TOKEN|BINGXI_SYSTEMD_DIR|BINGXI_ENV_FILE` 在 workspace 中

### 发现

#### ✅ 已落实的项

1. **.env.example 声明所有环境变量**：
   - `/workspace/backend/.env.example:87`：INIT_TOKEN 显式声明（L-43 修复）。
   - `/workspace/backend/.env.example:131`：BINGXI_ENV_FILE 显式声明（L-44 修复）。
   - `/workspace/backend/.env.example:133`：BINGXI_SYSTEMD_DIR 显式声明（L-44 修复）。
   - 包含 DATABASE_PASSWORD/JWT_SECRET/COOKIE_SECRET/WEBHOOK_SECRET/AUDIT_SECRET_KEY/KAFKA_*/ELASTICSEARCH_URL/REDIS_URL/CACHE_*/OTEL_*/EMAIL_*/AUTH_CHECK_USER_ACTIVE 等完整清单。

2. **deploy-latest.sh 在 config.yaml 生成时注入密钥**：
   - `/workspace/deploy/deploy-latest.sh:270-307`：cat > config.yaml 注入 `${JWT_SECRET}/${COOKIE_SECRET}/${WEBHOOK_SECRET}/${DB_HOST}/${DB_PORT}/${DB_NAME}/${DB_USER}/${DB_PASS}`。
   - `/workspace/deploy/deploy-latest.sh:180-241`：自动生成 JWT_SECRET/COOKIE_SECRET/WEBHOOK_SECRET/AUDIT_SECRET_KEY 强随机密钥并持久化到 .env。

3. **backend/config.yaml.example 包含示例**：
   - `/workspace/backend/config.yaml.example:14-95`：完整示例包含 server/database/auth/log/cors/kafka/slow_query/env 段。

4. **main.rs/app_state.rs fail-fast 校验完备**：
   - `/workspace/backend/src/main.rs:427-431`：COOKIE_SECRET 缺失时 eprintln FATAL + exit(1)。
   - `/workspace/backend/src/main.rs:436-461`：COOKIE_SECRET/WEBHOOK_SECRET 长度不足 32 字节时 exit(1)。
   - `/workspace/backend/src/utils/app_state.rs:179-184`：cookie_secret 长度不足 32 字节返回 Err。
   - `/workspace/backend/src/utils/app_state.rs:188-199`：webhook_secret 长度不足 + 与 jwt_secret 相同校验。

5. **AUTH_CHECK_USER_ACTIVE 启动期显式读取并打印当前值**：
   - `/workspace/backend/src/middleware/auth.rs:125-140`：L-36 修复，LazyLock 首次调用时打印当前值。未设置时打印"使用默认值 true"，已设置时打印 value 和 enabled。

6. **BINGXI_SLOW_QUERY_MS 统一走 AppSettings**：
   - `/workspace/backend/config.yaml.example:85-89`：slow_query.threshold_ms 配置项。
   - `/workspace/backend/src/main.rs:525`：settings.slow_query.threshold_ms 读取配置。

7. **ELASTICSEARCH_URL 启动期 warn 提示**：
   - `/workspace/backend/src/main.rs:624-634`：L-39 修复，生产环境未设置时 warn，开发环境未设置时 info。

8. **RATE_LIMIT_REDIS_URL 生产环境未配置 Redis warn 提示**：
   - `/workspace/backend/src/middleware/rate_limit.rs:137-158`：生产环境未配置时 warn"分布式限流未启用（使用内存限流，多实例部署下限流不共享）"。

9. **COOKIE_SECRET/WEBHOOK_SECRET/AUDIT_SECRET_KEY fail-fast 已完备**：
   - `/workspace/backend/.env.example:22-27`：占位符 value-placeholder-change-me 命中 validate_secret 黑名单。
   - `/workspace/backend/src/main.rs:427-461`：缺失/弱密钥/与 JWT_SECRET 相同时 exit(1)。

#### ❌ 缺陷项 1：面料行业配置环境变量完全缺失（V15 新增要求）

**风险等级：P1**（面料行业核心配置缺失影响业务可配置性）

**证据**：
- Grep `DYEHOUSE_VAT_COUNT|PROCESS_UNIT_PRICE_BASE|ENERGY_ALLOCATION_RULE|QUALITY_GRADE_THRESHOLD_A|QUALITY_GRADE_THRESHOLD_B|QUALITY_GRADE_THRESHOLD_C|DYEBATCH_STATUS_TIMEOUT` 在 backend 中**无任何匹配**。
- `/workspace/backend/.env.example`：未声明上述 6 个面料行业配置环境变量。
- `/workspace/backend/config.yaml.example`：未包含上述配置段。
- 审计计划要求"新增面料行业配置形成闭环"。

**业务影响**：
- 6 个面料行业核心配置缺失导致：
  1. DYEHOUSE_VAT_COUNT（染缸设备数）：无法配置染厂染缸总数，影响排缸算法和产能统计。
  2. PROCESS_UNIT_PRICE_BASE（工序单价基准）：工序单价硬编码，无法按客户/订单调整。
  3. ENERGY_ALLOCATION_RULE（能耗分摊规则）：能耗分摊规则无法配置，只能使用默认工时分摊。
  4. QUALITY_GRADE_THRESHOLD_A/B/C（A/B/C 分级阈值）：质量分级阈值硬编码 95%/80%，无法按产品/客户调整。
  5. DYEBATCH_STATUS_TIMEOUT（缸号状态机超时）：缸号状态超时无法配置，影响异常告警。

**修复建议**：
- 在 AppSettings 中新增 FabricIndustryConfig 结构体，包含上述 6 个配置项。
- 在 .env.example 和 config.yaml.example 中补充示例。
- 在 main.rs 启动期 fail-fast 校验关键配置（如 DYEHOUSE_VAT_COUNT > 0）。
- 将 quality_inspection_service.rs:26-27 的 QUALITY_GRADE 阈值常量改为从配置读取。

#### ❌ 缺陷项 2：AUDIT_RETENTION_DAYS 未走 AppSettings 结构体

**风险等级：P3**（功能正常但实现偏离审计计划要求）

**证据**：
- `/workspace/backend/src/main.rs:489`：`std::env::var("AUDIT_RETENTION_DAYS")` 直接读取环境变量。
- 审计计划要求"AUDIT_RETENTION_DAYS 通过 AppSettings 读取"。
- 实际通过 std::env::var 直接读取，未走 AppSettings 结构体。

**业务影响**：
- 直接读取环境变量无法通过 config.yaml 配置覆盖，配置灵活性降低。

**修复建议**：
- 在 AppSettings 中新增 audit_retention_days 字段，通过 config crate 读取（环境变量优先 > config.yaml）。

#### ❌ 缺陷项 3：根目录 .env.example 缺少 INIT_TOKEN 等显式声明

**风险等级：P3**（两份 .env.example 不一致）

**证据**：
- `/workspace/.env.example:1-95`：根目录 .env.example 仅包含 DATABASE__*/JWT_SECRET/COOKIE_SECRET/WEBHOOK_SECRET/AUDIT_SECRET_KEY/APP__ENV/SERVER__HOST/CORS__ALLOWED_ORIGINS。
- 缺少 INIT_TOKEN/BINGXI_ENV_FILE/BINGXI_SYSTEMD_DIR/KAFKA_*/ELASTICSEARCH_URL/REDIS_URL/AUTH_CHECK_USER_ACTIVE 等声明。
- `/workspace/backend/.env.example`：backend 目录下的 .env.example 包含完整声明（权威源）。
- `/workspace/.env.example:93-95`：注释说明"完整环境变量清单请参考 backend/.env.example（权威源）"。

**业务影响**：
- 两份 .env.example 不一致可能导致运维人员参考根目录示例时遗漏关键配置。

**修复建议**：
- 在根目录 .env.example 顶部增加醒目提示"本文件为精简版，完整配置请参考 backend/.env.example"。
- 或直接将根目录 .env.example 改为指向 backend/.env.example 的软链接。

---

## 维度 6：事件闭环（5.6）

### 检查方法
1. Read `/workspace/backend/src/services/event_bus.rs:44-184`（BusinessEvent 枚举）
2. Grep `BusinessEvent|publish\(.*Event|event_bus::publish` 在 backend/src 中
3. Grep `BpmProcessFinished|DyeBatchCompleted|SalesOrderApproved|InventoryTransactionCreated|PurchaseOrderApproved` 在 backend/src/services/event_bus.rs 中
4. Read `/workspace/backend/src/services/event_retry_service.rs`（事件重试与死信）
5. Grep `event_idempotency|try_mark_processed` 在 backend/src 中

### 发现

#### ✅ 已落实的项

1. **销售订单状态变更发布事件（6 个事件）**：
   - `/workspace/backend/src/services/event_bus.rs:51-83`：SalesOrderShipped/SalesOrderSubmitted/SalesOrderApproved/SalesOrderCompleted/SalesOrderCancelled/SalesOrderRejected 6 个事件（B-P1-4 修复批次 361）。
   - 发布点：order_workflow.rs:78,214,266,390 + delivery.rs:507 + contract.rs:68。

2. **采购订单审批发布事件**：
   - `/workspace/backend/src/services/event_bus.rs:100-103`：PurchaseOrderApproved 事件。
   - `/workspace/backend/src/services/event_bus.rs:484`：主监听器处理 PurchaseOrderApproved。

3. **库存盘点完成发布事件**：
   - `/workspace/backend/src/services/event_bus.rs:104-107`：InventoryCountCompleted 事件。

4. **客户/供应商主数据变更发布事件**：
   - `/workspace/backend/src/services/event_bus.rs:150-162`：CustomerUpdated/SupplierUpdated 事件（B-P1-3 修复批次 384）。

5. **BpmProcessFinished 事件处理覆盖全（含生产订单）**：
   - `/workspace/backend/src/services/event_bus.rs:108-114`：BpmProcessFinished 事件含 business_type/business_id/approved/approver_id。
   - `/workspace/backend/src/services/event_bus.rs:522-625`：主监听器处理 BpmProcessFinished，含生产订单 approve_order_via_bpm（L619），避免 BPM → 事件 → approve_order → BPM 死循环。
   - `/workspace/backend/src/services/event_bus.rs:539-542`：幂等检查 try_mark_processed("event_bus_main", &event_key, "BpmProcessFinished")。

6. **事件处理失败有重试（指数退避）+死信队列+告警**（同维度 2 已验证）：
   - `/workspace/backend/src/services/event_retry_service.rs`：完整实现。

7. **事件 payload 唯一键保证幂等性**：
   - `/workspace/backend/src/services/event_idempotency_service.rs`：EventIdempotencyService。
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:225-240`：使用 transaction_id 作为幂等键，try_mark_processed 防重复消费。

8. **DyeBatchCompleted 事件有发布者+订阅者**：
   - `/workspace/backend/src/handlers/dye_batch_handler.rs:321`：发布 DyeBatchCompleted。
   - `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs:40`：订阅 DyeBatchCompleted 创建成本归集草稿。
   - `/workspace/backend/src/services/event_bus.rs:939-949`：主监听器也处理 DyeBatchCompleted（可触发质检单生成/成本结转）。

9. **QualityInspectionCompleted 事件已定义**：
   - `/workspace/backend/src/services/event_bus.rs:176-183`：QualityInspectionCompleted 事件含 inspection_id/batch_id/product_id/result/inspector_id。

#### ❌ 缺陷项 1：染整工序扫码上报事件缺失

**风险等级：P1**（面料行业核心业务事件缺失）

**证据**：
- `/workspace/backend/src/services/event_bus.rs:44-184`：BusinessEvent 枚举 18 个变体中无"染整工序扫码上报事件"。
- `/workspace/backend/src/services/flow_card_service.rs:746-839`：扫码开始/结束工序仅更新 process_step_record 表，未发布 BusinessEvent。
- 审计计划要求"新增面料行业事件全部闭环：染整工序扫码上报事件"。

**业务影响**：
- 工序扫码上报是车间生产进度追踪的核心数据源，缺少事件导致：
  1. 实时生产看板无法被动感知工序进度（需轮询数据库）
  2. 工资计算无法被动触发（需定时任务扫描）
  3. 工序耗时统计无法实时更新

**修复建议**：
- 新增 BusinessEvent::ProcessStepReported { step_record_id, flow_card_id, route_code, operator_id, started_at, completed_at, quantity } 事件。
- 在 flow_card_service.rs 扫码开始/结束工序后发布事件。
- 订阅者：工资计算服务、生产看板服务、工序耗时统计服务。

#### ❌ 缺陷项 2：缸号状态变更事件缺失

**风险等级：P1**（缸号状态变更无法被下游感知）

**证据**：
- `/workspace/backend/src/services/event_bus.rs:44-184`：BusinessEvent 枚举中无"缸号状态变更事件"。
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:440-483`：record_transition 记录状态流转日志，未发布 BusinessEvent。
- 审计计划要求"新增面料行业事件全部闭环：缸号状态变更事件"。

**业务影响**：
- 缸号状态变更是染整生产的核心事件，缺少事件导致：
  1. 染缸设备占用/释放无法被动触发（需轮询缸号状态）
  2. 生产进度看板无法实时更新缸号状态
  3. 缸号状态机异常无法通过事件告警

**修复建议**：
- 新增 BusinessEvent::DyeBatchStatusChanged { batch_id, batch_no, from_status, to_status, transition_code, operator_id, transition_at } 事件。
- 在 record_transition 成功后发布事件。
- 订阅者：染缸设备管理服务、生产看板服务、状态机异常告警服务。

#### ❌ 缺陷项 3：验布分级事件缺失

**风险等级：P1**（验布分级结果无法被下游感知）

**证据**：
- `/workspace/backend/src/services/event_bus.rs:44-184`：BusinessEvent 枚举中无"验布分级事件"。
- `/workspace/backend/src/services/fabric_inspection_service.rs`：验布分级仅更新 fabric_inspection_record 表，未发布 BusinessEvent。
- 审计计划要求"新增面料行业事件全部闭环：验布分级事件"。

**业务影响**：
- 验布分级（A/B/C 级）决定产品流向（A 级入库/B 级降级销售/C 级返工或报废），缺少事件导致：
  1. 库存入库无法被动触发分级后的入库流程
  2. 降级销售定价无法被动调整
  3. 返工/报废工单无法自动生成

**修复建议**：
- 新增 BusinessEvent::FabricInspectionGraded { inspection_id, batch_id, grade, handling_method, inspector_id } 事件。
- 在 fabric_inspection_service 分级完成后发布事件。
- 订阅者：库存入库服务、销售定价服务、返工工单生成服务。

#### ❌ 缺陷项 4：产量上报事件缺失

**风险等级：P1**（产量数据无法被下游感知）

**证据**：
- `/workspace/backend/src/services/event_bus.rs:44-184`：BusinessEvent 枚举中无"产量上报事件"。
- `/workspace/backend/src/services/flow_card_service.rs`：工序扫码上报产量仅更新 process_step_record，未发布 BusinessEvent。
- 审计计划要求"新增面料行业事件全部闭环：产量上报事件"。

**业务影响**：
- 产量上报是工资计算和成本归集的核心数据源，缺少事件导致：
  1. 工资计算无法被动触发（需定时任务扫描）
  2. 成本归集无法按产量分摊
  3. 生产报表无法实时更新

**修复建议**：
- 新增 BusinessEvent::ProductionQuantityReported { step_record_id, flow_card_id, operator_id, actual_quantity, qualified_quantity } 事件。
- 在 process_step_record 完成后发布事件。
- 订阅者：工资计算服务、成本归集服务、生产报表服务。

#### ❌ 缺陷项 5：能耗采集事件缺失

**风险等级：P1**（能耗数据无法被下游感知）

**证据**：
- `/workspace/backend/src/services/event_bus.rs:44-184`：BusinessEvent 枚举中无"能耗采集事件"。
- `/workspace/backend/src/services/energy_service.rs`：能耗采集仅更新 energy_consumption_record 表，未发布 BusinessEvent。
- 审计计划要求"新增面料行业事件全部闭环：能耗采集事件"。

**业务影响**：
- 能耗采集是月末能耗分摊的核心数据源，缺少事件导致：
  1. 能耗异常无法实时告警（如能耗突增）
  2. 月末分摊无法被动触发（需手动调用）
  3. 能耗成本归集无法实时更新

**修复建议**：
- 新增 BusinessEvent::EnergyConsumptionRecorded { record_id, workshop, meter_type, consumption, cost, recorded_at } 事件。
- 在 energy_service 采集完成后发布事件。
- 订阅者：能耗异常告警服务、月末分摊服务、能耗成本归集服务。

#### ❌ 缺陷项 6：色卡发放事件缺失

**风险等级：P1**（色卡发放无法被下游感知）

**证据**：
- `/workspace/backend/src/services/event_bus.rs:44-184`：BusinessEvent 枚举中无"色卡发放事件"。
- `/workspace/backend/src/services/color_card_borrow_service.rs`：色卡借用/发放仅更新 color_card_borrow 表，未发布 BusinessEvent。
- 审计计划要求"新增面料行业事件全部闭环：色卡发放事件"。

**业务影响**：
- 色卡发放给客户后需追踪客户收到/使用/过期状态，缺少事件导致：
  1. 色卡库存无法被动扣减（需手动同步）
  2. 色卡过期回收无法自动触发
  3. 客户对色反馈无法关联到具体色卡

**修复建议**：
- 新增 BusinessEvent::ColorCardIssued { borrow_id, color_card_id, customer_id, issued_by, issued_at } 事件。
- 在 color_card_borrow_service 借用/发放完成后发布事件。
- 订阅者：色卡库存管理服务、过期回收服务、客户对色反馈服务。

---

## 维度 7：业财一致性闭环（5.7）

### 检查方法
1. Read `/workspace/backend/src/services/inventory_finance_bridge_service.rs:200-400`（库存财务桥接凭证生成）
2. Read `/workspace/backend/src/services/so/delivery.rs:400-500`（销售出库收入凭证）
3. Read `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs`（染色成本归集）
4. Read `/workspace/backend/src/services/production_order_service.rs:560-630`（生产订单成本归集）
5. Read `/workspace/backend/src/services/ap_payment_service.rs:311-402`（付款凭证）
6. Read `/workspace/backend/src/services/ar_service.rs:399-465`（收款凭证）
7. Read `/workspace/backend/src/services/energy_service.rs:1511-1636`（月末能耗分摊）
8. Read `/workspace/backend/src/services/voucher_service.rs:159-175,578-640`（凭证过账与科目余额回写）
9. Grep `ap_verification|ar_verification|核销|reconciliation|对账` 在 backend/src/services 中
10. Grep `移动加权平均|moving_average|weighted_average|月末|暂估|摊销|预提` 在 backend/src/services 中

### 发现

#### ✅ 已落实的项

1. **销售出库→收入凭证+成本凭证**：
   - `/workspace/backend/src/services/so/delivery.rs:405-495`：收入凭证（借：应收账款 / 贷：主营业务收入 + 应交税费-销项税额），F-P0-3+F-P0-6 修复。
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:253-256`：成本凭证（借：主营业务成本 / 贷：库存商品），通过 SALES_DELIVERY 事件触发。

2. **采购入库→存货凭证+应付凭证**：
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:244-247`：采购入库凭证（借：库存商品 / 贷：应付账款）。
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:284-382`：create_purchase_receipt_voucher 实现。

3. **库存调整→差异凭证**：
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:262-264`：INVENTORY_ADJUSTMENT 凭证（create_inventory_adjustment_voucher）。

4. **生产入库→生产成本凭证**：
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:266-269`：PRODUCTION_RECEIPT/PRODUCTION_OUTPUT 凭证（借：库存商品 / 贷：生产成本）。
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:271-274`：PRODUCTION_ISSUE/PRODUCTION_CONSUMPTION 凭证（借：生产成本 / 贷：库存商品）。

5. **采购退货+销售退货凭证**：
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:248-251`：PURCHASE_RETURN 凭证（借：应付账款红字 / 贷：库存商品红字），批次 356 B-P0-5 修复。
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:257-260`：SALES_RETURN 凭证（借：库存商品 / 贷：主营业务成本红字反转），批次 356 B-P0-6 修复。

6. **收付款→核销凭证**：
   - `/workspace/backend/src/services/ap_payment_service.rs:322-397`：F-P0-5 修复，确认付款后生成付款凭证（借：应付账款 / 贷：银行存款）。
   - `/workspace/backend/src/services/ar_service.rs:399-465`：F-P0-4 修复，确认收款后生成收款凭证（借：银行存款 / 贷：应收账款）。

7. **凭证科目余额回写**：
   - `/workspace/backend/src/services/voucher_service.rs:162-163`：F-P0-1 修复，post 内部调用 update_account_balances 实现科目余额回写。
   - `/workspace/backend/src/services/voucher_service.rs:578-640`：post 方法实现凭证过账（draft → submitted → reviewed → posted 状态机）。

8. **库存桥接凭证 create+post**：
   - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:163-175`：F-P0-2 修复，create_and_post 自动过账（原仅 create 不 post）。

9. **AR 对账单确认后生成凭证**：
   - `/workspace/backend/src/services/ar/recon.rs:274-338`：F-P2-4 修复，AR 对账单关闭后生成对账确认凭证（借贷均为应收账款，金额=期末余额），作为对账确认的审计凭证。

10. **染色完成→成本归集草稿**：
    - `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs:132-175`：监听 DyeBatchCompleted 事件自动创建 cost_collection 草稿记录，关联 batch_no/color_no。

11. **生产订单完成→成本归集**：
    - `/workspace/backend/src/services/production_order_service.rs:588-627`：批次 356 B-P0-3 修复，complete_production_order 调用 CostCollectionService::create。

12. **报表穿透追溯**：
    - `/workspace/backend/src/services/finance_report_service.rs:808-877`：drill_down_by_subject_prefix / drill_down_by_period_and_subject 实现试算平衡表穿透。

13. **事件幂等性保证**：
    - `/workspace/backend/src/services/inventory_finance_bridge_service.rs:222-240`：B-P1-8 修复，使用 transaction_id 作为幂等键，try_mark_processed 防重复消费。

#### ❌ 缺陷项 1：生产订单成本归集未按缸号（dye_lot_no=None）

**风险等级：P1**（成本归集维度缺失影响成本精确性）

**证据**：
- `/workspace/backend/src/services/production_order_service.rs:610-611`：`dye_lot_no: None,` 注释"v14 批次 422 T-P1-6：按缸号核算成本（生产订单当前无缸号，后续批次补全）"。
- 生产订单成本归集时 dye_lot_no 字段为 None，无法按缸号归集成本。

**业务影响**：
- 面料行业成本核算的最小单元是缸号，生产订单未关联缸号导致：
  1. 成本无法精确归集到缸号（仅归集到生产订单维度）
  2. 缸号成本分析报表数据缺失
  3. 与 inventory_piece.dye_lot_no、quality_inspection_records.dye_lot_no 字段不一致

**修复建议**：
- 为 production_order 表增加 dye_lot_no 字段（或通过 production_order_item 关联 dye_batch）。
- 在 complete_production_order 时从生产订单读取 dye_lot_no 传入 CostCollectionService。

#### ❌ 缺陷项 2：染色成本归集草稿 dye_lot_no=None

**风险等级：P1**（与缺陷项 1 同源，dye_batch 表无 dye_lot_no 字段）

**证据**：
- `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs:152-153`：`dye_lot_no: None,` 注释"dye_lot_no 暂为 None，dye_batch 表当前无此字段，后续批次补全"。
- 与批次 04 审计报告"维度 1 缺陷项 1：dye_batch 表缺少 dye_lot_no 字段"（P0）同源。

**业务影响**：
- 染色成本归集草稿未关联 dye_lot_no，导致缸号维度成本分析无法精确到 dye_lot_no 级别。

**修复建议**：
- 与批次 04 P0 修复同步：为 dye_batch 表添加 dye_lot_no 字段。
- 修改 dye_batch_cost_bridge_service.rs:152 从 dye_batch 表读取 dye_lot_no。

#### ❌ 缺陷项 3：销售成本未按移动加权平均法计算

**风险等级：P1**（成本核算方法不符合行业惯例）

**证据**：
- `/workspace/backend/src/services/inventory_finance_bridge_service.rs:313-316`：create_purchase_receipt_voucher 使用 `product.cost_price`（标准成本）计算金额：`let (product_name, cost_price) = self.get_product_info(product_id).await...`。
- `/workspace/backend/src/services/so/delivery.rs`：销售出库成本凭证通过 SALES_DELIVERY 事件触发 inventory_finance_bridge，同样使用 product.cost_price。
- 审计计划要求"销售成本按移动加权平均法计算"。
- Grep `移动加权平均|moving_average|weighted_average` 在 backend/src/services 中仅在 ai/recipe_opt.rs 和 bi_unit_tests.rs 出现（与成本核算无关）。

**业务影响**：
- 标准成本法无法反映实际采购价格波动，导致：
  1. 销售毛利失真（标准成本与实际成本差异未分摊）
  2. 库存价值与实际不符
  3. 不符合面料行业"按批号/缸号移动加权平均"的行业惯例

**修复建议**：
- 实现 inventory_stock 的移动加权平均成本计算：每次采购入库后更新 product.cost_price = (原库存金额 + 本次入库金额) / (原库存数量 + 本次入库数量)。
- 销售出库时按当前移动加权平均成本计算主营业务成本。
- 按缸号/批号维度维护独立成本（inventory_piece.unit_cost）。

#### ❌ 缺陷项 4：产量工资未生成人工成本凭证

**风险等级：P2**（人工成本归集缺失）

**证据**：
- `/workspace/backend/src/services/wage_service.rs`：工资计算与发放（confirmed → paid）仅更新 wage_record 表，未生成人工成本凭证。
- Grep `wage.*voucher|人工成本|direct_labor.*voucher` 在 backend/src/services 中无匹配。
- 审计计划要求"产量工资→人工成本凭证"。

**业务影响**：
- 人工成本是生产成本的重要组成部分，缺少凭证导致：
  1. 生产成本归集不完整（仅材料成本+制造费用，缺人工成本）
  2. 工资发放后未自动归集到 cost_collection.direct_labor 字段
  3. 成本报表人工成本数据缺失

**修复建议**：
- 在 wage_service 发放工资（confirmed → paid）后生成人工成本凭证（借：生产成本-直接人工 / 贷：应付职工薪酬）。
- 同时更新 cost_collection.direct_labor 字段（按缸号/工序归集）。

#### ❌ 缺陷项 5：月末能耗分摊未生成成本凭证

**风险等级：P2**（能耗成本归集不完整）

**证据**：
- `/workspace/backend/src/services/energy_service.rs:1518-1636`：monthly_allocation_by_duration 按工时分摊能耗到缸号/工序，生成 energy_allocation_record。
- `/workspace/backend/src/services/energy_service.rs:1317,1632`：`cost_collection_id: Set(None)` 注释表明分摊记录未关联 cost_collection。
- 审计计划要求"月末能耗分摊→成本凭证"。

**业务影响**：
- 能耗成本未生成凭证导致：
  1. 制造费用归集不完整（缺能耗成本）
  2. 能耗分摊记录未回写到 cost_collection.manufacturing_overhead 字段
  3. 成本报表能耗成本数据缺失

**修复建议**：
- 在 monthly_allocation_by_duration 分摊完成后生成能耗成本凭证（借：生产成本-制造费用 / 贷：应付账款-水电费）。
- 同时更新 cost_collection.manufacturing_overhead 字段（按缸号归集）。
- 在 energy_allocation_record 表回写 cost_collection_id 关联。

#### ❌ 缺陷项 6：期末调整机制（暂估/摊销/预提）缺失

**风险等级：P2**（期末调整凭证缺失影响账务准确性）

**证据**：
- Grep `暂估|摊销|预提|期末调整|accrual|amortization|provision` 在 backend/src/services 中无匹配。
- `/workspace/backend/src/services/accounting_period_service.rs:73`：月末结账功能存在，但未发现暂估/摊销/预提凭证生成逻辑。
- 审计计划要求"期末调整机制（暂估/摊销/预提）"。

**业务影响**：
- 期末调整是会计核算的关键环节，缺失导致：
  1. 已收货未收到发票的采购暂估入库缺失（资产负债表存货失真）
  2. 长期待摊费用未按月摊销（利润表费用失真）
  3. 已发生未支付的费用未预提（负债低估）

**修复建议**：
- 新增期末调整服务（PeriodAdjustmentService），在月末结账时自动生成：
  1. 采购暂估凭证（已收货未收到发票的采购按合同价暂估）
  2. 摊销凭证（长期待摊费用按月摊销）
  3. 预提凭证（已发生未支付的费用预提）

#### ❌ 缺陷项 7：AP 对账单确认后未生成凭证

**风险等级：P2**（与 AR 对账单凭证生成不对称）

**证据**：
- `/workspace/backend/src/services/ar/recon.rs:274-338`：AR 对账单关闭后生成对账确认凭证（F-P2-4 修复）。
- `/workspace/backend/src/services/ap_reconciliation_service.rs:121-125`：confirm_reconciliation 仅更新对账单状态，未生成凭证。
- Grep `ap_reconciliation.*voucher|ap_reconciliation.*create_and_post` 在 backend/src/services 中无匹配。
- 审计计划要求"AR/AP 对账单确认后生成凭证"。

**业务影响**：
- AR 对账单已生成凭证但 AP 对账单未生成，导致：
  1. AR/AP 凭证生成不对称
  2. AP 对账确认结果无法在凭证体系中追溯
  3. 供应商对账差异无法通过凭证审计

**修复建议**：
- 在 ap_reconciliation_service confirm_reconciliation 成功后生成对账确认凭证（借贷均为应付账款，金额=期末余额），与 AR 对账单凭证逻辑对称。

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 5.1 业务流程闭环 | 0 | 0 | 1 | 1 | 7 | 9 |
| 5.2 异常路径闭环 | 0 | 0 | 2 | 1 | 6 | 9 |
| 5.3 状态机闭环 | 0 | 1 | 1 | 1 | 5 | 8 |
| 5.4 资源生命周期闭环 | 0 | 0 | 3 | 0 | 6 | 9 |
| 5.5 配置依赖闭环 | 0 | 1 | 0 | 2 | 9 | 12 |
| 5.6 事件闭环 | 0 | 6 | 0 | 0 | 9 | 15 |
| 5.7 业财一致性闭环 | 0 | 3 | 4 | 0 | 13 | 20 |
| **合计** | **0** | **11** | **11** | **6** | **55** | **82** |

## 修复优先级队列

### P0 级（阻塞）
无 P0 级缺陷。本批次审计未发现阻塞级缺陷，所有核心闭环均已建立基础实现。

### P1 级（高）
1. **缸号状态机缺少 Failed + OnHold 异常态**（维度 5.3）：在 dye_batch_lifecycle_status 枚举增加 on_hold/failed 状态 + 流转规则 + 超时告警。
2. **面料行业配置环境变量完全缺失**（维度 5.5）：新增 DYEHOUSE_VAT_COUNT/PROCESS_UNIT_PRICE_BASE/ENERGY_ALLOCATION_RULE/QUALITY_GRADE_THRESHOLD_A/B/C/DYEBATCH_STATUS_TIMEOUT 6 个配置项。
3. **染整工序扫码上报事件缺失**（维度 5.6）：新增 BusinessEvent::ProcessStepReported 事件。
4. **缸号状态变更事件缺失**（维度 5.6）：新增 BusinessEvent::DyeBatchStatusChanged 事件。
5. **验布分级事件缺失**（维度 5.6）：新增 BusinessEvent::FabricInspectionGraded 事件。
6. **产量上报事件缺失**（维度 5.6）：新增 BusinessEvent::ProductionQuantityReported 事件。
7. **能耗采集事件缺失**（维度 5.6）：新增 BusinessEvent::EnergyConsumptionRecorded 事件。
8. **色卡发放事件缺失**（维度 5.6）：新增 BusinessEvent::ColorCardIssued 事件。
9. **生产订单成本归集未按缸号**（维度 5.7）：为 production_order 增加 dye_lot_no 字段。
10. **染色成本归集草稿 dye_lot_no=None**（维度 5.7）：与批次 04 P0 修复同步，为 dye_batch 表添加 dye_lot_no 字段。
11. **销售成本未按移动加权平均法计算**（维度 5.7）：实现 inventory_stock 移动加权平均成本计算。

### P2 级（中）
1. **缺少"反馈工艺优化"与"反馈配方优化"的显式闭环**（维度 5.1）：lab_dip PASSED 后反馈到 dye_recipe，DyeBatchCompleted 后反馈工艺优化。
2. **染整工序失败缺少显式"重染/补染"恢复路径**（维度 5.2）：在 rework_type 枚举增加 re_dye/replenish_dye 类型。
3. **缸号状态机异常缺少告警+死信队列机制**（维度 5.2）：validate_transition_with_rule 失败时增加告警 + 死信队列。
4. **色卡状态机缺少"发放→已收到→已使用→已过期"闭环**（维度 5.3）：补充 color_card 状态枚举。
5. **5 个后台定时任务缺少 CancellationToken**（维度 5.4）：引入 tokio_util::sync::CancellationToken 实现优雅退出。
6. **染缸设备资源缺少显式占用/释放路径**（维度 5.4）：新增 dye_vat_occupation 表 + 占用/释放逻辑。
7. **PDA/工控终端连接缺少资源管理**（维度 5.4）：新增设备连接管理模块。
8. **产量工资未生成人工成本凭证**（维度 5.7）：wage_service 发放工资后生成人工成本凭证。
9. **月末能耗分摊未生成成本凭证**（维度 5.7）：monthly_allocation 后生成能耗成本凭证。
10. **期末调整机制（暂估/摊销/预提）缺失**（维度 5.7）：新增 PeriodAdjustmentService。
11. **AP 对账单确认后未生成凭证**（维度 5.7）：ap_reconciliation_service confirm 后生成凭证，与 AR 对称。

### P3 级（低）
1. **染整 10 道工序闭环与行业工序类型映射不完整**（维度 5.1）：补充 process_type 枚举或文档说明自定义设计意图。
2. **生产订单成本归集失败仅 warn 不重试**（维度 5.2）：接入 event_retry_service 重试机制。
3. **MatchStatus 缺少 Disputed 终态**（维度 5.3）：补充 lab_dip_sample DISPUTED 状态。
4. **AUDIT_RETENTION_DAYS 未走 AppSettings 结构体**（维度 5.5）：迁移到 AppSettings 字段。
5. **根目录 .env.example 缺少 INIT_TOKEN 等显式声明**（维度 5.5）：增加醒目提示或改为软链接。
6. **染整工序失败恢复路径文档化不足**（维度 5.2）：补充 rework_type 与"重染/补染/降级"的映射文档。

---

## 审计结论

本批次（类五 运行逻辑闭环深化审计）共审计 7 个维度 82 个检查项，发现 11 个 P1 级、11 个 P2 级、6 个 P3 级缺陷，无 P0 级阻塞缺陷。

**核心亮点**：
1. 事件重试与死信队列机制完整实现（event_retry_service.rs），指数退避+死信队列+告警三段式闭环。
2. 锁中毒统一降级处理覆盖 7 处关键 Mutex（di_container/audit_log/omni_audit/main/event_bus/app_state/dye_batch_cost_bridge）。
3. WebSocket recv_task/send_task 句柄保存与显式 abort（L-31 修复），避免 detached task 泄漏。
4. 审计日志 mpsc channel+单消费者模式（L-32 修复），替代每次 record_async 创建 detached spawn task。
5. 销售订单审批→库存预留→MRP 物料需求计算三段闭环（批次 356 B-P0-1 + 批次 386 B-P2-4）。
6. 销售出库收入凭证+成本凭证双凭证闭环（F-P0-3+F-P0-6 修复）。
7. 缸号状态机 14 种状态 + 33 条流转规则 + rework 恢复路径，覆盖染整全生命周期。

**主要风险**：
1. **面料行业事件闭环缺口**（6 个 P1）：染整工序扫码/缸号状态变更/验布分级/产量上报/能耗采集/色卡发放 6 个核心业务事件缺失，导致下游订阅方无法被动感知，依赖轮询数据库。
2. **业财一致性按缸号归集断链**（3 个 P1）：生产订单和染色成本归集 dye_lot_no=None，销售成本未按移动加权平均法，导致缸号维度成本分析无法精确到 dye_lot_no 级别。
3. **状态机异常态缺失**（1 个 P1）：缸号状态机缺少 Failed/OnHold 异常态，无法区分"临时暂停"与"彻底终止"。
4. **面料行业配置缺失**（1 个 P1）：6 个核心配置环境变量完全缺失，影响业务可配置性。

建议优先修复 P1 级缺陷，特别是 6 个面料行业事件缺失和 3 个业财一致性按缸号归集断链问题，这两类问题直接影响面料行业核心业务闭环的完整性。
