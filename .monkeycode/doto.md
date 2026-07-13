# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-13（精简归档批次详细记录，重组结构）。

---

## 一、当前任务：v13 复审 + 业务/财务/运行逻辑闭环修复

> **v13 复审报告**：[v13-review-2026-07-13.md](file:///workspace/.monkeycode/docs/audits/v13-review-2026-07-13.md)
> **执行策略**：规则 13+14+15 联动，CI 全绿后自动进入下一批。
> **已完成批次**：356-383（详见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)）

### 进度总览

| 维度 | 总数 | 已完成 | 剩余 | 状态 |
|------|------|--------|------|------|
| baseline 警告清零 | 213 | 11 | 202 | 🔄 批次 357 完成 11 项 |
| 业务场景闭环 | 21 | 13 | 8 | 🔄 P0 6/6 ✅ + P1 7/9 |
| 财务场景闭环 | 16 | 8 | 8 | 🔄 **P0 8/8 ✅** + P1 5/6（仅剩 F-P1-1 期末结转） |
| 运行逻辑环闭环 | 45 | 45 | 0 | ✅ 全部完成（P1 6 + P2 13 + P3 26） |
| v14 中风险遗留 | 3 大类 | 0 | 3 大类 | ⏳ 待修复 |
| v14 低风险遗留 | 74 | 0 | 74 | ⏳ 后续迭代 |
| v13 前端/后端 P2 | 9 | 0 | 9 | ⏳ 待修复 |
| **合计** | **~378** | **72** | **~306** | — |

---

## 二、未完成任务清单

### 2.1 业务场景 P1 剩余（2 项）

- **B-P1-3**：客户/供应商主数据变更未同步关联单据 — 发布 `CustomerUpdated`/`SupplierUpdated` 事件，监听器异步刷新关联单据（改动面大，需评估冗余字段范围）
- **B-P1-7**：事件处理失败无重试 + 死信队列 + 告警 — 引入重试机制（指数退避）+ 死信队列 + 告警

### 2.2 业务场景 P2 剩余（6 项）

- B-P2-1：ar_service create_payment 与 mark_as_paid 状态更新重复
- B-P2-2：customer_credit_evaluate 孤岛 service（评估后删除或接入业务）
- B-P2-3：CostCollectionService 仅 HTTP 调用，无业务联动
- B-P2-4：MrpEngineService 仅 HTTP 调用，无业务联动
- B-P2-5：CapacityService 仅 HTTP 调用，无业务联动
- B-P2-6：InventoryReservationService 仅 HTTP 调用，销售流程未集成

### 2.3 财务场景 P1 剩余（1 项）

- **F-P1-1 剩余**：期末结转逻辑（下期期初余额写入）— close_period 新增期末结转，将本期期末余额写入下期期初余额

### 2.4 财务场景 P2 剩余（4 项）

- F-P2-1：无期末调整（暂估/摊销/预提）机制
- F-P2-2：报表无穿透追溯功能
- F-P2-3：销售成本按 product.cost_price 计算未与采购实际单价联动
- F-P2-4：AR/AP 对账单生成不触发凭证

### 2.5 v14 中风险遗留（3 大类）

#### 测试覆盖补测（7 项）
- handlers 层覆盖率仅 10%，services 层 107 个 service 无测试，前端 api 层 4.4%
- 修复方案：按模块优先级分批补测（auth/user/order/inventory 核心 service 优先）
- 技术要点：service 测试 mock DatabaseConnection，handler 用 TestServer，前端用 vitest

#### view 表格接入 useTableApi（剩余 10 个）
- `frontend/src/views/finance/voucher/*`
- `frontend/src/views/data-import/*`
- `inventory/tabs/InventoryStockTab`（1-based 分页）
- `inventoryAdjustment/AdjustmentListTab`
- `inventoryTransfer/TransferListTab`
- `barcodeScanner`（0-based 分页需特殊处理）
- `assistAccounting`（0-based 分页需特殊处理）
- 其他待扫描发现的遗漏文件

### 2.6 v13 前端/后端 P2（6 项）

- FE-P2-1：前端类型定义完善（unknown 类型细化）
- FE-P2-2：前端组件 props 类型强化
- FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）
- P2-1：后端错误处理统一（部分 handler 直接返回字符串）
- P2-2：后端日志规范（部分模块日志级别不当）
- P2-3：后端配置项完善

### 2.7 其他遗留（3 项）

- FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）
- P2-8：剩余 143 个无测试 service（后续迭代）
- E2E 失败排查（已知问题，待规则 5 节点）

### 2.8 v14 低风险修复队列（74 项 - 后续迭代）

- 占位符/Mock 存根（21 项）：逐个评估，合理保留加注释，不合理的真实实现
- 项目规则符合性（11 项）：评估是否符合规则 0-13
- 死代码（8 项）：与 v13 baseline 清零合并处理
- 其他（34 项）：命名规范/注释完善/代码风格等

---

## 三、规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 330 已到期需触发（403 权限不足，需用户手动触发）
- **规则 10（每 15 批次记忆整理）**：批次 375 已完成，下次整理批次 390
- **规则 13（修复流程自动化）**：CI 全绿后自动开始下一批，无需用户确认
- **规则 14（移除所有警告抑制）**：所有警告视为错误需修复
- **规则 15（v13 复审严格规范）**：业务/财务场景闭环 + 运行逻辑环流程闭环

---

## 四、历史任务（全部完成）

- v8 复审（批次 290-308）：21 项问题全部修复 ✅
- v9 复审（批次 317-323）：16 项问题全部修复 ✅
- sea-orm 版本调研（批次 324）：确认使用 1.1.20 稳定版 ✅
- 规则 14 新增（批次 324）：移除所有警告抑制 ✅
- v10 复审（批次 325-339）：53 项问题全部修复 ✅
- v11 复审（批次 340-346）：27 项问题全部修复 ✅
- v12 复审（批次 347-355）：15 项问题全部修复 ✅
- v13 复审批次 356-383：详见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)

> 详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。
