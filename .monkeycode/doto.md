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

## 二、任务重新规划（每批 5-8 文件，完成后新一轮复审）

> **规划原则**：每个批次修复 5-8 个文件，按优先级分阶段推进，所有阶段完成后进行 v14 新一轮复审。
> **执行策略**：规则 13+14+15 联动，CI 全绿后自动进入下一批，无需用户确认。

### 阶段 1：P1 级闭环修复（批次 384，1 批，约 7 文件）

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| B-P1-3 | event_bus.rs / customer_service.rs / supplier_service.rs | 客户/供应商主数据变更事件发布+监听器异步刷新关联单据 |
| B-P1-7 | event_bus.rs / 新建 dead_letter_service.rs / 新建 alert_service.rs | 事件重试（指数退避）+ 死信队列 + 告警 |
| F-P1-1 | accounting_period_service.rs / account_subject_service.rs | close_period 新增期末结转，本期期末余额写入下期期初 |

### 阶段 2：业务场景 P2 闭环修复（批次 385-386，2 批，约 12 文件）

**批次 385（业务场景 P2 前 3 项，约 6 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| B-P2-1 | ar_service.rs | create_payment 与 mark_as_paid 状态更新重复，合并为单一入口 |
| B-P2-2 | customer_credit_evaluate_service.rs + mod.rs | 孤岛 service 评估后删除或接入业务 |
| B-P2-3 | cost_collection_service.rs + handler + routes | 仅 HTTP 调用，接入业务联动 |

**批次 386（业务场景 P2 后 3 项，约 6 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| B-P2-4 | mrp_engine_service.rs + handler + routes | 仅 HTTP 调用，接入业务联动 |
| B-P2-5 | capacity_service.rs + handler + routes | 仅 HTTP 调用，接入业务联动 |
| B-P2-6 | inventory_reservation_service.rs + handler + routes | 仅 HTTP 调用，销售流程集成 |

### 阶段 3：财务场景 P2 闭环修复（批次 387，1 批，约 7 文件）

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| F-P2-1 | accounting_period_service.rs + 新建 period_adjustment_service.rs | 期末调整机制（暂估/摊销/预提） |
| F-P2-2 | finance_report_service.rs + handler | 报表穿透追溯功能 |
| F-P2-3 | inventory_finance_bridge_service.rs | 销售成本与采购实际单价联动 |
| F-P2-4 | ar_service.rs / ap_invoice_service.rs + voucher_service.rs | AR/AP 对账单生成触发凭证 |

### 阶段 4：v13 前后端 P2（批次 388-389，2 批，约 14 文件）

**批次 388（前端类型+后端错误处理，约 7 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| FE-P2-1 | frontend/src/types/*.ts（3-4 文件） | unknown 类型细化，完善类型定义 |
| FE-P2-2 | frontend/src/components/*.vue（2 文件） | 组件 props 类型强化 |
| P2-1 | backend/src/handlers/*.rs（1-2 文件） | 后端错误处理统一，handler 返回 AppError |

**批次 389（i18n+后端日志+配置，约 7 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| FE-P2-3 | frontend/src/locales/*.ts + views（3 文件） | i18n 覆盖率提升（首批核心视图） |
| P2-2 | backend/src/services/*.rs（2 文件） | 后端日志规范，日志级别修正 |
| P2-3 | backend/config.yaml.example + .env.example（2 文件） | 后端配置项完善 |

### 阶段 5：useTableApi 接入（批次 390-391，2 批，约 10 文件）

**批次 390（前 5 个 view，5 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| useTableApi-1 | frontend/src/views/finance/voucher/VoucherListTab.vue | 财务凭证列表 |
| useTableApi-2 | frontend/src/views/finance/voucher/VoucherDetailTab.vue | 财务凭证明细 |
| useTableApi-3 | frontend/src/views/data-import/DataImportListTab.vue | 数据导入列表 |
| useTableApi-4 | frontend/src/views/data-import/DataImportTaskTab.vue | 数据导入任务 |
| useTableApi-5 | frontend/src/views/inventory/tabs/InventoryStockTab.vue | 库存明细（1-based 分页） |

**批次 391（后 5 个 view，5 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| useTableApi-6 | frontend/src/views/inventoryAdjustment/AdjustmentListTab.vue | 库存调整 |
| useTableApi-7 | frontend/src/views/inventoryTransfer/TransferListTab.vue | 库存调拨 |
| useTableApi-8 | frontend/src/views/barcodeScanner/index.vue | 条码扫描（0-based 分页特殊处理） |
| useTableApi-9 | frontend/src/views/assistAccounting/index.vue | 辅助核算（0-based 分页特殊处理） |
| useTableApi-10 | 待扫描发现的遗漏文件 | 其他遗漏 |

### 阶段 6：测试覆盖补测（批次 392-394，3 批，约 18 文件）

**批次 392（核心 service 测试 - 认证/用户/订单，约 6 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| 测试-1 | backend/src/services/auth_service.rs + tests | auth_service 单元测试 |
| 测试-2 | backend/src/services/user_service.rs + tests | user_service 单元测试 |
| 测试-3 | backend/src/services/so/order.rs + tests | 销售订单 service 测试 |
| 测试-4 | backend/src/services/po/order.rs + tests | 采购订单 service 测试 |

**批次 393（核心 service 测试 - 库存/财务，约 6 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| 测试-5 | backend/src/services/inventory_stock_service.rs + tests | 库存 service 测试 |
| 测试-6 | backend/src/services/voucher_service.rs + tests | 凭证 service 测试 |
| 测试-7 | backend/src/services/ar_service.rs + tests | AR service 测试 |
| 测试-8 | backend/src/services/ap_invoice_service.rs + tests | AP service 测试 |

**批次 394（handler 测试，约 6 文件）**：

| 任务 | 涉及文件 | 说明 |
|------|----------|------|
| 测试-9 | backend/tests/auth_handler_test.rs | auth handler 集成测试 |
| 测试-10 | backend/tests/order_handler_test.rs | 订单 handler 集成测试 |
| 测试-11 | backend/tests/inventory_handler_test.rs | 库存 handler 集成测试 |
| 测试-12 | backend/tests/finance_handler_test.rs | 财务 handler 集成测试 |

### 阶段 7：baseline 清零（批次 395-424，约 30 批，约 202 项）

> **目标**：213 条 baseline 警告全部清零，每批 5-8 文件清理 7-10 项警告。
> **分类**：dead_code 193 + unused_import 15 + 其他 5（批次 357 已清理 11 项 unused import）。
> **执行方式**：按文件分组扫描，每批选取 5-8 个文件集中清理，CI 全绿后自动进入下一批。
> **完成后**：移除 baseline 机制，改为 `cargo clippy -- -D warnings`。

| 批次范围 | 文件数 | 警告清理数 | 说明 |
|----------|--------|-----------|------|
| 395-404 | 50-80 | 70-100 | dead_code 前 100 项（按文件分组） |
| 405-414 | 50-80 | 70-100 | dead_code 中 100 项 |
| 415-424 | 30-50 | 30-50 | dead_code 后 93 项 + 其他 5 项 |

### 阶段 8：v14 低风险修复（批次 425-435，约 11 批，74 项）

> **目标**：74 项低风险问题全部修复，每批 5-8 文件。

| 批次范围 | 任务类别 | 项数 | 说明 |
|----------|----------|------|------|
| 425-427 | 占位符/Mock 存根 | 21 | 逐个评估，合理保留加注释，不合理的真实实现 |
| 428-429 | 项目规则符合性 | 11 | 评估是否符合规则 0-13 |
| 430-431 | 死代码补充清理 | 8 | 与 baseline 清零合并处理后的遗漏 |
| 432-435 | 其他 | 34 | 命名规范/注释完善/代码风格等 |

### 阶段 9：其他遗留（批次 436-438，3 批，约 15 文件）

| 批次 | 任务 | 涉及文件 | 说明 |
|------|------|----------|------|
| 436 | FE-P2-6 | frontend/src/components/Table*.vue（5-8 文件） | 大列表虚拟化（el-table-v2 引入） |
| 437 | P2-8 | backend/src/services/*.rs + tests（5-8 文件） | 剩余无测试 service 补测 |
| 438 | E2E 失败排查 | e2e/*.spec.ts + 修复代码（5-8 文件） | E2E 失败用例分析与修复 |

### 阶段 10：v14 新一轮复审（批次 439+）

> **触发条件**：阶段 1-9 全部完成后自动触发。
> **复审目标**：在 v13 复审全部维度基础上，新增 17 个审计维度（通用 3 + 面料行业特性 7 + 面料行业模块专项 7），全面覆盖面料行业 ERP 业务场景。
> **复审流程**：扫描 → 生成 v14 复审报告 → 按优先级排序修复队列 → 自动开始修复（每批 5-8 文件）。

#### 10.1 通用审计维度（3 项）

| 维度 | 检查要点 | 涉及范围 |
|------|----------|----------|
| 业务功能完整性 | 所有规划功能是否真实实现，无占位符/stub；功能闭环（输入→处理→输出→反馈）；验收标准 100% 达成 | 全项目 |
| 逻辑完整性 | 业务逻辑无断点；状态机闭环有终态；异常路径有恢复/降级/告警；分支覆盖完整 | 全项目 |
| 数据流转性 | 跨模块数据流转连贯无断点；业财数据一致性；主数据变更同步关联单据；报表数据可追溯到底层凭证 | 全项目 |

#### 10.2 面料行业特性审计维度（7 项）

| 维度 | 检查要点 | 涉及范围 |
|------|----------|----------|
| 面料行业转项特性优化 | 色号/批号/缸号管理；色差等级（A/B/C 级）；门幅/克重/纱支规格；面料属性（成分/工艺/后整理）；双单位（米/公斤）换算 | models/fabric + services/fabric + dual_unit_converter |
| 面料行业术语 | 代码/注释/UI 文本使用行业术语（如：缸号、色号、批号、门幅、克重、纱支、染整、印花、色差、纬斜、缩水率）；无外行表述 | 全项目文本 |
| 面料行业流程完善情况 | 打色→试样→大货生产→质检→入库→发货全流程；染整工艺流程；印花工艺流程；面料检验流程（十项指标） | services/dye + services/production + services/inspection |
| 面料行业交易流程 | 询价→报价→打样确认→大货订单→生产→发货→对账→收款；面料定价（按米/按公斤/按码）；色卡管理 | services/so + services/quotation + services/color_card |
| 面料行业调货流程 | 调拨单（厂际/仓际）；借条/还条流程；调货计价（成本价/协议价）；在途库存管理 | services/inventory_transfer + services/color_card_borrow |
| 面料行业专用词汇/术语配套使用 | 缸号↔染色批次；色号↔颜色编码；批号↔生产批次；门幅↔规格；克重↔单位重量；术语在代码/DB/前后端一致配套 | 全项目术语一致性 |
| 面料行业业务流转 | 面料业务全链路：接单→打色→采购→生产→染色→印花→质检→入库→销售→退货；跨模块事件贯通 | services/* 全链路 |

#### 10.3 面料行业模块专项审计维度（7 项）

| 维度 | 检查要点 | 涉及范围 |
|------|----------|----------|
| 面料行业人事管理 | 员工档案（染色工/印花工/质检员等岗位）；考勤排班（三班倒）；计件工资（按米/按公斤计件）；绩效与产量挂钩 | services/hr + handlers/hr |
| 面料行业仓库管理 | 面料分类入库（按色号/缸号分区）；批次先进先出；库存双单位（米/公斤）；库位管理（染缸区/印花区/成品区）；盘点差异处理 | services/inventory + models/inventory |
| 面料行业销售管理 | 面料报价（含色卡费/打样费）；客户色卡管理；大货订单与打样订单关联；销售退货（按缸号/批号追溯）；客户信用额度（面料行业账期长） | services/so + services/customer + services/color_card |
| 面料行业公司管理 | 多公司/多工厂架构；公司间调货/交易；组织架构（总厂/分厂/车间）；公司级配置（染整工艺/印花工艺参数） | services/company + services/organization |
| 面料行业权限细化 | 按岗位授权（染色工/印花工/质检员/仓管员/销售员/采购员）；按车间授权；按面料品类授权；敏感操作（调价/退货/盘点）独立权限 | middleware/permission + services/auth |
| 面料行业财务专项优化 | 面料成本核算（染料/助剂/人工/水电分摊）；染整加工费核算；色卡/打样费用归集；面料库存估值（按批次实际成本）；面料行业税票（染整加工费税率） | services/voucher + services/cost + services/ar + services/ap |
| 面料行业 CRM 专业优化 | 客户面料偏好（品类/色系/规格）；客户色卡历史；客户打样记录追踪；客户报价历史（面料价格波动）；客户投诉与色差纠纷管理；客户复购预测（按面料品类） | services/crm + services/customer |

#### 10.4 v14 复审执行流程

1. **复审阶段**（自动执行）：
   - 扫描 v13 复审全部维度（baseline 清零 + 业务/财务/运行逻辑闭环回归验证）
   - 扫描 17 个新增维度（通用 3 + 面料行业特性 7 + 面料行业模块专项 7）
   - 生成 v14 复审报告（保存到 `.monkeycode/docs/audits/v14-review-YYYY-MM-DD.md`）
   - 按优先级排序修复队列（P0 阻塞 → P1 高 → P2 中 → P3 低）

2. **修复阶段**（复审完成后自动开始）：
   - 严格按规则 13 流程执行：建分支 → 修改 → commit → push → PR → CI → merge → 下一批
   - 每批 5-8 文件，CI 全绿后自动进入下一批，无需用户确认
   - 所有警告视为错误，必须真实修复（规则 14）

3. **闭环验证**：
   - 每批修复后更新 doto.md 进度 + CHANGELOG.md 一句话总结
   - 所有批次完成后进行 v14 复审回归验证（确认无新增问题）
   - 通过后触发 v15 复审（如需）

---

## 三、批次执行计划总览

| 阶段 | 批次范围 | 批次数 | 文件数 | 任务类别 |
|------|----------|--------|--------|----------|
| 1 | 384 | 1 | 7 | P1 级闭环（B-P1-3/7 + F-P1-1） |
| 2 | 385-386 | 2 | 12 | 业务场景 P2（B-P2-1~6） |
| 3 | 387 | 1 | 7 | 财务场景 P2（F-P2-1~4） |
| 4 | 388-389 | 2 | 14 | v13 前后端 P2（FE-P2-1~3 + P2-1~3） |
| 5 | 390-391 | 2 | 10 | useTableApi 接入（10 个 view） |
| 6 | 392-394 | 3 | 18 | 测试覆盖补测（核心 service + handler） |
| 7 | 395-424 | 30 | 200+ | baseline 清零（202 项） |
| 8 | 425-435 | 11 | 74 | v14 低风险修复（74 项） |
| 9 | 436-438 | 3 | 15 | 其他遗留（虚拟化+补测+E2E） |
| 10 | 439+ | - | - | v14 新一轮复审（v13 全维度 + 17 个新增维度：通用 3 + 面料行业特性 7 + 面料行业模块专项 7） |
| **合计** | **384-438** | **55** | **~357** | **所有未完成任务** |

---

## 四、规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 330 已到期需触发（403 权限不足，需用户手动触发）；批次 390、420、450 到期需触发
- **规则 10（每 15 批次记忆整理）**：批次 375 已完成，下次整理批次 390，后续 405/420/435
- **规则 13（修复流程自动化）**：CI 全绿后自动开始下一批，无需用户确认
- **规则 14（移除所有警告抑制）**：所有警告视为错误需修复
- **规则 15（v13 复审严格规范 + v14 面料行业特性复审）**：v13 业务/财务/运行逻辑闭环 + 阶段 1-9 完成后触发 v14 复审，新增 17 个维度（通用 3 + 面料行业特性 7 + 面料行业模块专项 7）

---

## 五、历史任务（全部完成）

- v8 复审（批次 290-308）：21 项问题全部修复 ✅
- v9 复审（批次 317-323）：16 项问题全部修复 ✅
- sea-orm 版本调研（批次 324）：确认使用 1.1.20 稳定版 ✅
- 规则 14 新增（批次 324）：移除所有警告抑制 ✅
- v10 复审（批次 325-339）：53 项问题全部修复 ✅
- v11 复审（批次 340-346）：27 项问题全部修复 ✅
- v12 复审（批次 347-355）：15 项问题全部修复 ✅
- v13 复审批次 356-383：详见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)

> 详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。
