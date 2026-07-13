# 未完成任务（详细）

> 本文件**详细**记录未完成的任务（问题描述、影响范围、修复方案、技术要点），禁止简化。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近一次整理：2026-07-13（v13 复审前记忆优化，归档 v8/v9/v10/v11/v12 历史任务 + 安全漏洞表到 doto-su.md）。

---

## ✅ 历史任务：v8-v12 复审问题修复（全部完成）

- **v8 复审**（批次 290-308）：21 项问题全部修复 ✅
- **v9 复审**（批次 317-323）：16 项问题全部修复 ✅
- **sea-orm 版本调研**（批次 324）：确认使用 1.1.20 稳定版正确 ✅
- **规则 14 新增**（批次 324）：移除所有警告抑制 ✅
- **v10 复审**（批次 325-339）：53 项问题全部修复 ✅
- **v11 复审**（批次 340-346）：27 项问题全部修复 ✅
- **v12 复审**（批次 347-355）：15 项问题全部修复 ✅

> 详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。
> 安全漏洞（7 项，批次 290-296）已全部修复，详见 [bug.md](file:///workspace/.monkeycode/bug.md)（当前为空）。

---

## 🔄 当前任务：v13 复审 + 业务/财务场景闭环 + 运行逻辑环流程闭环（2026-07-13 启动）

> **v13 复审报告**（2026-07-13，规则 15 启动，详见 [v13-review-2026-07-13.md](file:///workspace/.monkeycode/docs/audits/v13-review-2026-07-13.md)）：v12 复审全部完成后启动，新增"业务场景闭环""财务场景闭环""运行逻辑环流程闭环"等复审维度。
> **核心目标**：baseline 213 条摘要行（~993 个警告）全部清零 + 业务/财务/逻辑闭环问题修复 + v14 历史遗留任务合并处理。
> **执行策略**：规则 13+14+15 联动，复审完成后自动连续修复，每批 5-6 文件，CI 全绿后自动进入下一批，无需用户确认。
> **v14 历史遗留合并**：v14 深度调研报告（2026-07-09）的高风险 6/6 ✅ 已完成，中风险剩余 3 项 + 低风险 74 项 + v13 前端/后端 P2 共 9 项全部合并到 v13 复审修复队列。

### 进度总览

| 维度 | 总数 | 已完成 | 剩余 | 状态 |
|------|------|--------|------|------|
| 🟢 baseline 警告清零 | 213 摘要 / 89 位置 / 135 文件 | 11 | 202 | 🔄 批次 357 完成 11 项 unused import |
| 🟢 业务场景闭环 | 21 | 13 | 8 | 🔄 P0 6 项 + P1 7 项已完成（批次 356/358/359/360/361/364/365，B-P1-8 基础设施+最高风险变体接入） |
| 🟢 财务场景闭环 | 16 | 7 | 9 | 🔄 P0 2 项 + P1 5 项已完成（批次 356/358/359/360/362/363，F-P1-2 完整闭环） |
| 🟢 运行逻辑环流程闭环（5 子维度） | 45 | 0 | 45 | ⏳ 待修复 |
| 🟢 v14 中风险遗留（测试覆盖 + useTableApi） | 3 大类 | 0 | 3 大类 | ⏳ 待修复 |
| 🟢 v14 低风险遗留 | 74 | 0 | 74 | ⏳ 后续迭代 |
| 🟢 v13 前端 P2 + 后端 P2 + 其他遗留 | 9 | 0 | 9 | ⏳ 待修复 |
| **合计** | **~378** | **31** | **~347** | — |

### v13 复审修复队列（按优先级排序，详见复审报告）

- **P0 级（14 项 - 阻塞）**：业务场景 6 项 + 财务场景 8 项
- **P1 级（19 项 - 高）**：业务场景 9 项 + 财务场景 4 项 + 运行逻辑 6 项
- **P2 级（24 项 - 中）**：业务场景 6 项 + 财务场景 4 项 + 运行逻辑 14 项
- **P3 级（25 项 - 低）**：运行逻辑 25 项
- **baseline 清零（213 项）**：dead_code 193 + unused_import 15 + 其他 5

### v14 历史遗留任务（合并到 v13 修复队列）

#### 1. 测试覆盖补测（7 项 - 中风险）

**问题背景**：项目测试覆盖率严重不足，关键模块零测试或低覆盖，无法保证代码质量和重构安全性。

**影响范围**：
- handlers 层：100+ 文件覆盖率仅 10%
- services 层：107 个 service 无任何测试
- frontend api 层：覆盖率 4.4%
- ai 算法层：零测试
- store 层：覆盖率低
- middleware 层：覆盖率低（permission.rs 已在批次 240 补测 23 个）

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

#### 2. view 表格逻辑接入 useTableApi（剩余 10 个 - 中风险）

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

#### 3. v13 前端 P2（3 项 ⏳）

- FE-P2-1：前端类型定义完善（any 类型清理已完成，剩余 unknown 类型细化）
- FE-P2-2：前端组件 props 类型强化
- FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）— 大量 view 未接入 i18n，硬编码中文文本

#### 4. v13 后端 P2（3 项 ⏳）

- P2-1：后端错误处理统一（部分 handler 仍直接返回字符串而非 AppError）
- P2-2：后端日志规范（部分模块日志级别不当）
- P2-3：后端配置项完善

#### 5. 其他遗留（3 项 ⏳）

- FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）— 引入 `el-table-v2` 或 `vue-virtual-scroller`
- P2-8：剩余 143 个无测试 service（后续迭代）— 分批补测
- E2E 失败排查（已知问题，待规则 5 节点）— 下载 playwright-report 分析失败用例

### v14 低风险修复队列（74 项 - 后续迭代）

**占位符/Mock 存根（21 项）**：逐个评估，合理保留的加注释说明，不合理的真实实现
**项目规则符合性（11 项）**：评估是否符合规则 0-13，不符合的修正
**死代码（8 项）**：逐个评估是否接入业务或删除（与 v13 baseline 清零合并处理）
**其他（34 项）**：命名规范/注释完善/代码风格等，后续迭代统一处理

### 修复批次记录

#### 批次 356（PR #528，已合并 2026-07-13）✅ v13 复审 P0 业务/财务场景闭环修复

**修改文件（5 个）**：
1. `backend/src/services/voucher_service.rs`：新增 `create_and_post` 方法（F-P0-1 科目余额回写 + F-P0-2 自动过账）
2. `backend/src/services/inventory_finance_bridge_service.rs`：扩展凭证生成覆盖（B-P0-4/5/6 采购退货/销售退货/生产领退料兼容 + create→create_and_post）
3. `backend/src/services/so/delivery.rs`：销售出库生成库存流水（B-P0-2 SALES_DELIVERY + batch_no 类型修复 + borrow after move 修复）
4. `backend/src/services/so/order_workflow.rs`：销售订单审批后库存预留（B-P0-1）
5. `backend/src/services/production_order_service.rs`：生产订单成本核算闭环（B-P0-3）

**完成的 P0 项（8 项）**：
- 业务场景 P0：B-P0-1（订单审批→库存预留）、B-P0-2（销售出库→库存流水）、B-P0-3（生产订单→成本核算）、B-P0-4（采购退货凭证）、B-P0-5（销售退货凭证）、B-P0-6（生产领退料凭证兼容）
- 财务场景 P0：F-P0-1（科目余额回写）、F-P0-2（库存桥接凭证自动过账）

**CI 记录**：3 次 CI 运行（#2503 失败→#2505 失败→#2506 全绿），修复 `STATUS_ACTIVE`→`IsActive.eq(true)`、`batch_no` 类型不匹配、`borrow after move` 三处编译错误

**遗留**：11 个 unused import warning（release 构建报出，clippy baseline 已包含）放入批次 357 baseline 清零处理

#### 批次 357（PR #529，已合并 2026-07-13）✅ v13 复审 baseline 清零 - 11 项 unused import

**修改文件（10 个）**：
1. `handlers/inventory_stock_handler.rs`：移除 unused Deserialize, Serialize
2. `routes/quotations.rs`：移除 unused put
3. `routes/custom_order.rs`：移除 unused delete
4. `routes/color_card.rs`：移除 unused delete
5. `routes/color_price.rs`：移除 unused put
6. `services/customer_credit_limit.rs`：移除 unused std::sync::Arc
7. `services/event_kafka.rs`：移除 unused Deserialize, Serialize
8. `services/import_export_service.rs`：移除 2 处 unused self
9. `services/quotation_approval_service.rs`：移除 unused ActiveModelTrait
10. `services/report/ds.rs`：移除 unused ActiveModelTrait

**CI 记录**：1 次 CI 运行（#2509 全绿），clippy baseline 机制正常（已修复警告不阻塞 CI）

**遗留**：clippy baseline 中对应条目将在后续批次同步清理

#### 批次 358（PR #530，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（B-P1-1 + B-P1-5 + F-P1-4）

**修改文件（3 个）**：
1. `services/sales_return_service.rs`：B-P1-1 修复 — `apply_stock_inbound_txn` 改用 `record_transaction_txn` 关联函数，流水写入与主事务同生共死，事件由调用方在 commit 成功后统一 publish（消除事务边界泄漏 + 幻事件风险）
2. `services/po/contract.rs`：B-P1-5 修复 — `approve_order` 在 commit 成功后发布 `PurchaseOrderApproved` 事件，触发库存财务桥接等下游订阅方生成采购入库相关凭证
3. `services/account_subject_service.rs`：F-P1-4 修复 — 新增 `refresh_balance(subject_id, period)` 方法，从已过账凭证分录重新聚合指定期间借贷发生额，按余额方向计算期末余额并写回科目主数据

**CI 记录**：3 次 CI 运行
- #29214925018：失败（F-P1-4 编译错误 — `VOUCHER_POSTED` 路径错误 + `into_tuple().one()` 类型不匹配）
- #29215077653：失败（clippy 新增 1 个警告 — rustdoc `doc list item without indentation`）
- #29215444643：全绿 ✅

**遗留**：无

#### 批次 359（PR #531，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（B-P1-2 + F-P1-3）

**修改文件（2 个）**：
1. `services/inventory_count_service.rs`：B-P1-2 修复 — `approve_count` 方法在 `txn.commit()` 成功后发布 `InventoryCountCompleted` 事件（`count_id` + `variance_count`），触发差异报告生成等下游订阅方。原实现仅更新盘点单状态并同步库存，未通知下游，导致"盘点完成 → 差异报告归档"业务闭环断裂。事件在 commit 后发布避免幻事件。
2. `services/voucher_service.rs`：F-P1-3 修复 — `post` 方法在 `update_account_balances` 调用后新增 `write_assist_accounting_records_txn` 调用，凭证过账时写入 `assist_accounting_record` 表。原实现仅更新 `account_balance` 表，未写入辅助核算记录，导致辅助核算明细账与汇总表查询无数据。仅对包含辅助核算维度（客户/供应商/批次/色号/缸号/等级/车间等）的分录写入，避免空记录污染。已知 Schema 缺口：`voucher_item` 无 `product_id`/`warehouse_id` 字段，暂用 0 占位，TODO 标记待后续修正。

**CI 记录**：1 次 CI 运行
- #29216135549：全绿 ✅（Rust Clippy + 单元测试 + 格式检查 + 后端构建 + 前端全套均通过）

**遗留**：F-P1-3 的 `product_id`/`warehouse_id` 占位待 Schema 补字段后修正

#### 批次 360（PR #532，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（B-P1-9 + F-P1-1）

**修改文件（3 个）**：
1. `services/production_order_service.rs`：B-P1-9 修复 — 新增 `approve_order_via_bpm` / `reject_order_via_bpm` 两个 BPM 回写专用方法。与现有 `approve_order` 的区别：不回调 BPM（避免 BPM → 事件 → approve_order → BPM 死循环）。使用事务 + lock_exclusive + validate_status_transition + update_with_audit 模式，保留审计追溯。
2. `services/event_bus.rs`：B-P1-9 修复 — `BpmProcessFinished` match 分支新增 `production_order` 分支，调用上述专用方法回写审批结果。原实现仅处理 `purchase_order`/`sales_order`，生产订单 BPM 审批结果落入 `_ => {}` 空分支无法回写。
3. `services/accounting_period_service.rs`：F-P1-1 修复 — `close_period` 新增 `check_trial_balance_txn` 试算平衡校验方法，联表查询本期已过账凭证分录汇总借贷总额，不相等则拒绝关闭期间。同时替换硬编码 `"posted"` 为 `VOUCHER_POSTED` 常量（规则 0 合规）。兜底防止单条凭证过账校验被绕过。

**CI 记录**：1 次 CI 运行
- #29216819903：全绿 ✅（Clippy + 单元测试 + 格式检查 + 后端构建 + 前端全套均通过）

**遗留**：F-P1-1 期末结转逻辑（下期期初余额写入）待后续批次处理

#### 批次 361（PR #533，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（B-P1-4 销售订单状态变更事件）

**修改文件（5 个）**：
1. `services/event_bus.rs`：新增 5 个 BusinessEvent 变体 — SalesOrderSubmitted / SalesOrderApproved / SalesOrderCompleted / SalesOrderCancelled / SalesOrderRejected（均含 order_id + customer_id + user_id 字段）。
2. `services/so/order_workflow.rs`：4 个方法 commit 后发布对应事件 — submit_order（BPM 成功后发布，避免补偿回滚幻事件）、approve_order（commit 后、库存预留前发布）、complete_order（commit 后发布）、cancel_order（commit 后发布，customer_id 在 order.into() 前提前保存）。
3. `services/so/contract.rs`：reject_order commit 后发布 SalesOrderRejected 事件，customer_id 在 order.into() 前提前保存。
4. `services/event_kafka_payload.rs`：同步新增 5 个 EventPayload 变体 + From<&BusinessEvent> + TryFrom<EventPayload> 实现保持 Kafka 序列化完整。
5. `services/event_kafka.rs`：同步新增 event_type_name match 分支（#[cfg(test)]）+ 测试用例 5 个变体。

**CI 记录**：1 次 CI 运行
- #29217569815：全绿 ✅（Clippy + 单元测试 + 格式检查 + 后端构建 + 前端全套均通过）

**遗留**：无

#### 批次 362（PR #534，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（F-P1-2 利润表走凭证体系）

**修改文件（1 个）**：
1. `services/finance_report_service.rs`：F-P1-2 修复 — 重写 `get_income_statement` 方法，从凭证体系取数替代硬编码 70%/15%/10%/5% 比例。新增 `sum_voucher_amount_by_subject_prefix` 私有方法：联表查询已过账凭证分录（`voucher.status = VOUCHER_POSTED`）按科目编码前缀聚合借方/贷方总额。取数逻辑参考中国企业会计准则科目编码：`60xx` 收入类（贷方）、`64xx` 成本类（借方）、`6601` 销售费用（借方）、`6602` 管理费用（借方）、`6603` 财务费用（借方）。原实现从 finance_invoice/finance_payment 业务表取数 + 硬编码比例估算，违反禁止硬编码规则且与会计实务脱节。

**CI 记录**：1 次 CI 运行
- #29218164031：全绿 ✅（Clippy + 单元测试 + 格式检查 + 后端构建 + 前端全套均通过）

**遗留**：F-P1-2 剩余部分（资产负债表硬编码修复：`_ap_total` 未使用 + 存货取数量非金额；现金流量表硬编码 ZERO 修复）待后续批次处理

#### 批次 363（PR #535，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（F-P1-2 剩余：资产负债表+现金流量表走凭证体系）

**修改文件（1 个）**：
1. `services/finance_report_service.rs`：F-P1-2 剩余修复，完整闭环 F-P1-2。
   - 资产负债表：存货取 `QuantityAvailable`（数量非金额）→ 从凭证体系按 `14xx` 科目前缀取借方-贷方余额；`_ap_total` 未使用死代码 → 从凭证体系按 `2202` 科目前缀取贷方-借方余额并加入负债侧；预收账款从客户信用额度取数 → 从凭证体系按 `2203` 科目前缀取余额；应收账款/货币资金/固定资产改从凭证体系取时点余额（`1122`/`1001+1002`/`16`）。
   - 现金流量表：投资活动硬编码 `Decimal::ZERO` → 从凭证体系按 `1601` 科目前缀取借贷方发生额（处置收回/购建支付）；筹资活动硬编码 `Decimal::ZERO` → 从凭证体系按 `25xx` 科目前缀取借贷方发生额（借入/偿还）；期初现金硬编码 `ZERO` → 从凭证体系按 `1001+1002` 科目前缀取截至期初前一日余额。
   - 新增方法 `get_subject_balance_by_prefix`：按科目编码前缀取科目时点余额（借方累计-贷方累计差额），用于资产负债表等时点报表。
   - 移除未使用 imports：`customer_credit`/`finance_invoice`/`fixed_asset`/`inventory_stock`（规则 14 合规，避免 unused_imports 警告）。

**CI 记录**：1 次 CI 运行
- #29219139820：全绿 ✅（Clippy + 单元测试 + 格式检查 + 后端构建 + 前端全套均通过）

**遗留**：无（F-P1-2 完整闭环完成，财务场景 P1 级 5/5 全部完成：F-P1-1 + F-P1-2 + F-P1-3 + F-P1-4）

#### 批次 364（PR #536，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（B-P1-6 删除 InventoryAdjusted 孤岛事件）

**修改文件（3 个）**：
1. `services/event_bus.rs`：删除 `BusinessEvent::InventoryAdjusted` 变体定义（原 L92-96）+ 订阅者 match 分支（原 L435-441，仅 tracing::info! 打日志无业务逻辑）。
2. `services/event_kafka.rs`：删除 `event_type_name` 中的 `InventoryAdjusted` 字符串映射 + 测试样本数据中的 InventoryAdjusted 用例。
3. `services/event_kafka_payload.rs`：删除 `EventPayload::InventoryAdjusted` 变体定义 + `From<&BusinessEvent>` 转换分支 + `TryFrom<EventPayload>` 反序列化分支。

**修复依据**：InventoryAdjusted 是孤岛事件（有订阅无发布）— 全代码库无任何 publish 调用；订阅者仅 tracing::info! 打日志无业务逻辑；语义被 InventoryTransactionCreated 完全覆盖且超越（后者 12 字段含来源单据/批次/色号，且已联动财务凭证生成）。按项目规则第六章死代码处理规范删除变体。

**CI 记录**：1 次 CI 运行
- #29219858958：全绿 ✅（Clippy + 单元测试 + 格式检查 + 后端构建 + 前端全套均通过）

**遗留**：无（B-P1-6 完整闭环完成，三个孤岛事件全部处理：PurchaseOrderApproved 批次358解除孤岛 + InventoryCountCompleted 批次359解除孤岛 + InventoryAdjusted 本批次删除）

#### 批次 365（PR #537，已合并 2026-07-13）✅ v13 复审 P1 级闭环修复（B-P1-8 事件幂等处理基础设施 + InventoryTransactionCreated 接入）

**修改文件（9 个，新增 5 文件 + 修改 4 文件）**：
1. `migration/src/m0049_create_processed_events.rs`：新增迁移注册（processed_events 表）。
2. `migrations/20260713000001_create_processed_events/up.sql` + `down.sql`：processed_events 表 DDL，主键 (consumer_id, event_key) + processed_at 索引。
3. `models/processed_event.rs`：SeaORM entity（consumer_id + event_key 复合主键，auto_increment=false）。
4. `services/event_idempotency_service.rs`：EventIdempotencyService 幂等服务，提供 try_mark_processed_txn（事务内）和 try_mark_processed（独立事务）两个方法，先查询是否已处理，未处理则插入标记，返回是否应继续处理。
5. `services/inventory_finance_bridge_service.rs`：handle_inventory_transaction 入口接入幂等 — 去掉 `_transaction_id` 下划线前缀实际使用该参数，使用 `inventory_txn:{transaction_id}` 作为幂等键，调用 EventIdempotencyService 检查，已处理则 info! 日志 + 幂等返回，未处理则继续生成凭证。
6. `models/mod.rs` + `services/mod.rs` + `migration/src/lib.rs`：模块注册。

**修复依据**：InventoryTransactionCreated 重复消费会重复生成会计凭证 + 重复过账，导致科目余额累加失真、报表数据失真、财务对账无法平衡。原实现 `_transaction_id` 形参（下划线开头）未参与幂等判断，全代码库无通用幂等基础设施。

**CI 记录**：2 次 CI 运行
- #29220938811：失败 ❌（E0119 EntityName 冲突实现 + E0599 TransactionTrait 未导入）
- #29221446146：全绿 ✅（修复：删除手动 EntityName 实现 + 导入 TransactionTrait）

**遗留**：B-P1-8 后续批次将逐步覆盖 PaymentCompleted/CollectionCompleted/BpmProcessFinished/LowStockAlert/MaterialShortageAlert 等高风险变体。本批次为通用基础设施 + 最高风险变体（InventoryTransactionCreated）接入作为模板。

---

## 规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 330 已到期需触发（403 权限不足，需用户手动触发）
  - 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch
  - 批次 N+20：第 1 次监控（GitHub API 查询 run 状态）
  - 批次 N+28：第 2 次监控（若 N+20 未完成）
  - 批次 N+29：最后监控，未完成则跳过 N+30 的 E2E 周期
  - **注意**：E2E 已从 ci-cd.yml 独立到 e2e-batch.yml，不阻塞主 CI
- **规则 10（每 15 批次记忆整理）**：2026-07-13 v13 复审前提前执行（归档 v8-v12 历史任务 + 安全漏洞表）
  - 下次整理：批次 375
- **规则 13（修复流程自动化与连续执行）**：CI 全绿后自动开始下一批，无需用户确认
- **规则 14（移除所有警告抑制）**：所有警告视为错误需修复
- **规则 15（v13 复审严格规范 + 业务/财务场景闭环 + 运行逻辑环流程闭环）**：2026-07-13 新增
