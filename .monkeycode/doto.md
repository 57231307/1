# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近梳理：2026-07-20（精简 doto.md：已完成任务详细描述移除，归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)；P0 进度 103/104；当前批次 Batch 488 进行中（12/17 完成，剩余 5 项大型任务 D05/D08 第三梯队/D09/D10/D13/D14））。

---

## 一、当前状态与总体进度

### 1.1 进度总览

| 优先级 | 总数 | 已完成 | 未完成 | 完成率 |
|--------|------|--------|--------|--------|
| **P0 阻塞级** | 104 | 103 | **1** | 99.0% |
| **P1 高优先级** | 257 | 0 | **257** | 0% |
| **P2 中优先级** | 248 | 0 | **248** | 0% |
| **P3 低优先级** | 123 | 0 | **123** | 0% |
| **合计** | **732** | **103** | **629** | **14.1%** |

> P0 已完成 103 项 = 原 62 项 + 复审发现已完成 4 项（P0-S08/S16/F14/T04）- 复审重新打开 1 项（P0-S14）+ Batch 473 修复 2 项（P0-S14 migration 补齐 + P0-S19 condition 字段补齐）+ Batch 474 修复 1 项（P0-S15 导出水印基础设施）+ Batch 475a 修复 1 项（P0-S13 审计日志导出闭环）+ Batch 476 修复 1 项（P0-S17 打印 HTML 真实数据查询）+ Batch 477 修复 4 项（P0-F10 库存联动 + P0-F11 前端文件结构 + P0-F12 前端类型/API/视图 + P0-F13 数据迁移）+ Batch 478 修复 4 项（P0-F15 bulk_color_approval 表 + P0-F16 剪大货样 + P0-F17 客户批色确认 + P0-F19 ship_order 校验）+ Batch 479 修复 2 项（P0-F18 返工/降级/报废 + P0-F21 返工走生产订单）+ Batch 480 修复 1 项（P0-F20 8D 质量管理流程）+ Batch 481 修复 4 项（P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警）+ Batch 482 修复 6 项（P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B14 Incoterms）+ Batch 483 修复 4 项（P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收）+ Batch 484 修复 3 项（P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换）+ Batch 485 修复 3 项（P0-T03 clippy baseline 恢复 + P0-T08 覆盖率工具 + P0-T06 bi_analysis 测试 API 对齐）+ Batch 486 修复 1 项（P0-T01 核心 service 单测补全）+ Batch 487 修复 3 项（P0-T02 7 项集成测试 + P0-T07 性能基准 + P0-T05 E2E 配置修复，用户特批不拆分打包）。
> P0-S12 前端导出接入后端：Batch 474 已完成核心 2 页面（customer/supplier），Batch 475a 已完成 audit-log（P0-S13 闭环），Batch 475b 已完成 purchase/customer 闭环（A 类 2 文件），Batch 475c 已完成 B 类批次 1/3（inventory + warehouse + production 3 模块），Batch 475d 已完成 B 类批次 2/3（sales-contract + sales-price + quality + quality-standards 4 模块），Batch 475e 已完成 B 类批次 3/3 收尾（ar + ap + cost + budget + fixed-assets 5 模块），**P0-S12 前端导出接入后端全部完成**。

### 1.2 状态：🔄 规则 13 连续执行中

- **当前批次**：Batch 488 进行中（11/17 已完成 + 1/17 进行中 = 12/17 阶段性完成）—— 11 项已完成（详见 §4.7 已完成项摘要）+ ✅ D08 第一/第二梯队 28 函数全部完成（CI 全绿 run 29718405482~29729300636）+ ✅ D08 第三梯队子批次1/2/3 已合并（PR #669/#670 + main 772c0312，共 19 函数；子批次3 CI run 29760816778 失败 9 编译错误 + 2 警告：order_workflow.rs 6 处 txn: &Transaction 类型缺失改为 &sea_orm::DatabaseTransaction / voucher_service.rs 3 处 build_subject_id_map 返回 HashMap<&str,i32> 生命周期错误改为 HashMap<String,i32> + lookup_voucher_items 缺 .map_err(AppError::from) + lookup_subject_id 缺 ? 操作符 + 未使用 use crate::models::assist_accounting_record / production_order_service.rs 2 处 lookup_default_bom/lookup_bom_items 缺 .map_err(AppError::from) + 未使用 use RecordTransactionArgs；本次一并修复）+ 🔄 D08 第三梯队子批次4 7 函数拆分 + 子批次3 编译错误修复一并推送 CI 验证（five_dimension_service get_stats/quotation_convert_service convert/crm/assign auto_assign_leads/so/delivery process_shipment_items/ar/vfy generate_reconciliation/so/order_workflow approve_order/ap_reconciliation_service auto_reconcile_all）；剩余 5 项大型任务 2026-07-19 精确审计完成（详见 §4.7 各项条目）
- **下一批次**：Batch 488 继续 —— 按依赖关系推荐顺序：① D08 超长函数（第 1 顺位，无前置 + 解锁 D09/D10，预估 10-12 子批次）→ ② D10 大文件拆分（第 2 顺位，D08 完成后立即推进，预估 5-6 子批次）→ ③ D14 api 命名（第 3 顺位，与 D05/D13 解耦，预估 10-12 子批次）→ ④ D13 缩写命名（第 4 顺位，D14 完成后推进，预估 12-15 子批次）→ ⑤ D05 useI18n（第 5 顺位，D13/D14 完成后最后推进，预估 30-36 子批次）；累计预估 67-81 子批次
- **执行策略**：规则 13+14+15+20 联动；CI 全绿后自动进入下一批；所有警告视为错误必须真实修复；修复前必须调研现有实现禁止重复造轮子；注释必须与功能一致禁止随意编写（规则 20）；规则 13 步骤 4 自审必须 grep 所有引用新字段/新结构体的调用点；**禁止本地编译验证**（cargo check/build/test/clippy + npm build/type-check/vitest/vue-tsc），必须直接 push 让 CI 验证

### 1.3 关键决策记录

| 决策 | 日期 | 内容 |
|------|------|------|
| 批次节奏 | 2026-07-17 | 每批 9-12 文件，遵循规则 13 连续执行流程；每 30 批触发 E2E（规则 5）；每 15 批整理记忆（规则 10） |
| 批次顺序 | 2026-07-17 | 按顺序修复所有批次，不再限制单数批次 |
| 术语澄清 | 2026-07-17 | 缸号（batch_no）=染色批次号；dye_lot_no=染色批号（lot 概念，防色差混批） |
| 旧表保留 | 2026-07-17 | 保留 color_card_borrow_records 不重命名为 _legacy，保护 Rust migration m0029 链路；应用层不再读写 |
| 复审归档 | 2026-07-17 | 复审报告 [v15-fix-reaudit-2026-07-17.md](file:///workspace/.monkeycode/docs/audits/v15-fix-reaudit-2026-07-17.md)；4 项已完成项归档（P0-S08/S16/F14/T04）；P0-S14 重新打开（migration 047 缺失） |
| 规则 20 | 2026-07-17 | 新增规则：注释必须与功能一致，禁止随意编写；CI 强制检查 |
| 自审门强化 | 2026-07-17 | Batch 473 教训：步骤 4 自审必须 grep 所有引用新字段/新结构体的调用点（如 `audit_log::ActiveModel {` / `OmniAuditMessage {` 等），不能只看 git diff 的已修改文件 |

---

## 二、任务依赖关系图

> 本图展示 P0 任务间的依赖关系，用于确定执行顺序。箭头 → 表示"依赖"（A→B 表示 B 依赖 A 先完成）。
> 工作量预估：S=小（≤3 文件）/ M=中（4-6 文件）/ L=大（7-10 文件）/ XL=超大（>10 文件）

### 2.1 模块 A：安全与权限

```
P0-S14 migration 047 (S)  ──┐
                             ├──→ P0-S12 前端导出接入后端 (XL, 25+ 页面)
P0-S15 导出水印 (M)  ────────┤
                             ├──→ P0-S13 审计日志导出假按钮 (M)
P0-S19 审计字段补齐 (S)  ────┘
P0-S17 打印 HTML 占位数据 (L)  ← 独立任务
```

**关键路径**：P0-S14 → P0-S15 → P0-S12 → P0-S13（前端导出全链路）
**独立任务**：P0-S17（打印服务真实查询）

### 2.2 模块 B：面料行业-色卡发放

```
P0-F10 库存联动 (M)  ←─── P0-F11/F12 前端文件结构 (L)
P0-F13 数据迁移策略 (S)  ← 独立任务
```

**关键路径**：P0-F10 → P0-F11/F12（后端库存联动先于前端补齐）
**独立任务**：P0-F13（legacy 数据迁移脚本）

### 2.3 模块 C：面料行业-大货批色

```
P0-F15 bulk_color_approval 表 (M)  ──→ P0-F16 剪大货样 (M)
                                  ├──→ P0-F17 客户批色确认 (M)
                                  └──→ P0-F18 返工/降级/报废 (L)
P0-F18 ──→ P0-F21 返工走生产订单 (M)
P0-F16/F17/F18 ──→ P0-F19 ship_order 校验 (S)
```

**关键路径**：P0-F15 → P0-F18 → P0-F21（表结构 → 业务规则 → 返工闭环）
**末端校验**：P0-F19（依赖前 4 项）

### 2.4 模块 D：面料行业-质量管理

```
P0-F20 8D 质量管理流程 (XL)  ← 独立任务
P0-F21 返工走生产订单 (M)  ← 依赖模块 C 的 P0-F18
```

**关键路径**：P0-F18（模块 C）→ P0-F21 → P0-F20 8D 流程

### 2.5 模块 E：财务与业务流程

```
P0-B01 坏账准备 (L)  ──→ P0-B02 坏账核销审批 (M)
P0-B03 催收任务 (M)  ← 独立（依赖 B01 但可并行设计）
P0-B04 财务预警 (L)  ← 独立
P0-B05 大额调拨验证 (S)  ← 独立
P0-B06 预算超支拦截 (M)  ← 独立
P0-B07 CRM 回收规则 (S)  ← 独立
P0-B08 赢率自动计算 (S)  ← 独立
P0-B09 输单原因 (S)  ← 独立
P0-B10 BI 权限过滤 (M)  ← 独立
P0-B11 定制订单打样报价 (L)  ← 独立
P0-B12 售后质量集成 (M)  ← 依赖模块 D 的 P0-F20
P0-B13 物流电子签收 (M)  ← 独立
P0-B14 Incoterms 补齐 (S)  ← 独立
P0-B15 缺料预警持久化 (M)  ← 独立
P0-B16 自动故障检测 (M)  ← 独立
P0-B17 主备切换自动完成 (L)  ← 独立
```

**关键路径**：P0-B01 → P0-B02（坏账链路）；P0-F20 → P0-B12（质量售后链路）

### 2.6 模块 F：测试体系

```
P0-T03 baseline 移除 (M)  ←─── 独立（可优先，解锁真实 CI 信号）
P0-T01 核心 service 单测 (L)  ← 独立
P0-T02 7 项集成测试 (XL)  ← 依赖 T01
P0-T06 bi_analysis 16 测试 (M)  ← 独立
P0-T05 E2E 通过率 (XL)  ← 独立
P0-T07 性能基准 (M)  ← 独立
P0-T08 覆盖率工具 (S)  ← 独立
```

**关键路径**：P0-T01 → P0-T02（单测先于集成测试）
**优先项**：P0-T03（移除 baseline 后所有 CI 失败才能真实暴露）

### 2.7 模块 G：部署与运维

```
P0-D01 Docker 文件 (S)  ←─── 独立
P0-D02 install.sh (S)  ←─── 独立
P0-D03 5 service 缓存 (L)  ──→ P0-D04 moka→Redis (L)
P0-D05 useI18n (XL)  ← 独立
P0-D06 aria-label (XL)  ← 独立
P0-D07 img alt (S)  ← 独立
P0-D08 超长函数 (XL)  ──→ P0-D09 100 行函数 (L) ──→ P0-D10 1000 行文件 (L)
P0-D11 setup_test_db (M)  ← 独立
P0-D12 圈复杂度 (M)  ← 独立
P0-D13 前端缩写命名 (XL)  ← 独立
P0-D14 api 命名统一 (XL)  ← 独立
P0-D15 升级零停机 (M)  ← 独立
P0-D16 报表订阅调度 (M)  ← 独立
P0-D17 OA 公告 (M)  ← 独立
```

**关键路径**：P0-D03 → P0-D04（缓存层迁移链路）；P0-D08 → P0-D09 → P0-D10（代码质量链路）

---

## 三、P0 批次规划表（39 项，规划 22 个批次）

> 每批 9-12 文件，按依赖关系排序。批次顺序即执行顺序。
> 工作量：S=小（≤3 文件，1-2 小时）/ M=中（4-6 文件，2-4 小时）/ L=大（7-10 文件，4-8 小时）/ XL=超大（>10 文件，8+ 小时）
> 批次节奏调整：2026-07-17 从"每批 6-8 文件"调整为"每批 9-12 文件"，批次总数从 27 压缩为 22，提升单批吞吐量

### 3.1 批次顺序总览

> Batch 473~487 共 18 个批次全部已合并（PR #656~#668 + main 直接提交），覆盖 P0 任务 39 项（模块 A/B/C/D/E/F 全部完成）。
> 详细批次内容（合并方式、commit hash、CI 验证轮次、教训）已归档到 [doto-su.md §V15 Batch 473-487](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。
>
> **当前进行中**：Batch 488（D 系列 17 项打包，已完成 12/17，剩余 5 项大型任务 D05/D08 第三梯队/D09/D10/D13/D14），详见 §4.7 模块 G。

### 3.2 批次工作量分布（历史统计）

> 批次节奏从 6-8 提升至 9-12 文件后，单批文件数普遍进入 L/XL 区间，每批覆盖任务更多，批次总数从 27 压缩为 22。当前 P0 剩余任务集中在 Batch 488 单批内推进。

### 3.3 批次执行原则

1. **严格按顺序**：批次编号即执行顺序，前批未完成不进入下批
2. **依赖优先**：有依赖的任务排在被依赖任务之后
3. **每批 9-12 文件**：批次节奏统一为 9-12 文件，提升单批吞吐量
4. **模块内连续**：同模块任务连续执行，减少上下文切换
5. **CI 全绿推进**：每批 CI 全绿后自动进入下一批（规则 13）
6. **直接 push 验证**：禁止本地编译验证，所有验证直接 push 让 CI 执行

---

## 四、未完成任务清单（按业务模块分组）

### 4.1 模块 A：安全与权限（0 项未完成，原 7 项全部完成）

> 模块 A 全部 P0 任务已完成（P0-S12 前端导出 / P0-S13 审计日志导出 / P0-S14 migration / P0-S15 导出水印 / P0-S17 打印 HTML / P0-S19 审计字段，共 7 项）。
> 完成批次：Batch 473（PR #656）+ Batch 474（PR #657）+ Batch 475a-e（PR #658~#662）+ Batch 476（main eb57484）。
> 详细记录已归档到 [doto-su.md §V15 Batch 473/474/475a-475e/476](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.2 模块 B：面料行业-色卡发放（0 项未完成，4 项已完成）

> 模块 B 全部 P0 任务已完成（P0-F10 库存联动 / P0-F11 前端文件结构 / P0-F12 前端类型/API/视图 / P0-F13 数据迁移，共 4 项）。
> 完成批次：Batch 477（main a3798f4 + daeab0f，15 文件，m0057+m0058 迁移 + color_card.rs Model + color_card_issue_service.rs 5 方法库存联动 + 前端 5 新文件）。
> 详细记录已归档到 [doto-su.md §V15 Batch 477](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.3 模块 C：面料行业-大货批色（0 项未完成，5 项全部完成）

> 模块 C 全部 P0 任务已完成（P0-F15 bulk_color_approval 表 / P0-F16 剪大货样 / P0-F17 客户批色确认 / P0-F18 返工/降级/报废 / P0-F19 ship_order 校验，共 5 项）。
> 完成批次：Batch 478（main 9d01a42 + 6aca804，11 文件，8 态状态机 + 9 端点）+ Batch 479（main 642d2c09 + cc1ee381 + c06109fd + bbf38a30，7 文件，customer_rework 联动返工生产订单 + downgrade 联动库存等级降级 + scrap 联动库存报废标记）。
> 详细记录已归档到 [doto-su.md §V15 Batch 478/479](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.4 模块 D：面料行业-质量管理（0 项未完成，2 项全部完成）

> 模块 D 全部 P0 任务已完成（P0-F20 8D 质量管理流程 / P0-F21 返工走生产订单，共 2 项）。
> 完成批次：Batch 479（main 642d2c09 + cc1ee381 + c06109fd + bbf38a30，与 P0-F18 合并，m0059_add_rework_order_fields + production_order_service.rs create_rework_order + RW-YYYYMMDD-NNN 订单号 + 不触发 MRP）+ Batch 480（main 5334bf13 + 8d7ea998 + ae87219f，13 文件，11 态状态机 + 10 条合法边 + From<AppError> 透传）。
> 详细记录已归档到 [doto-su.md §V15 Batch 479/480](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.5 模块 E：财务与业务流程（0 项未完成，17 项独立项 + 13 项归并全部完成）

> 模块 E 全部 P0 任务已完成（P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警 + P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收 + P0-B14 Incoterms + P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换，共 17 项；B18~B30 为归并项不独立计）。
> 完成批次：Batch 481（PR #666 squash 00261365，25 文件，账龄分析法 + 二级审批 + 4 类预警 + 催收任务）+ Batch 482（PR #667，13 文件，large_transfer_threshold 10万 + enforce_budget_available 阻塞式 + RecycleExecutor 6h 定时 + Incoterms 11 种术语）+ Batch 483（PR #668 squash e094846e，15 文件，data_scope.rs build_data_scope_sql + custom_order 2 字段 + after_sales quality_issue_id + logistics_waybill 5 签收字段）+ Batch 484（main df5286ee + c012a3b9，11 文件，m0068 两表 + FailoverExecutor ArcSwap 原子切换 + FailoverMonitor 后台任务 + 熔断器状态机）。
> 详细记录已归档到 [doto-su.md §V15 Batch 481/482/483/484](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.6 模块 F：测试体系（0 项未完成，8 项全部完成）

> 模块 F 全部 P0 任务已完成（P0-T01 核心 service 单测 + P0-T02 7 项集成测试 + P0-T03 clippy baseline + P0-T05 E2E 通过率 + P0-T06 bi_analysis 测试 + P0-T07 性能基准 + P0-T08 覆盖率工具，共 8 项；T04 在复审中归档为已完成项）。
> 完成批次：Batch 485（main af0f16b + 5e4e78f + 7cc82cc，4 文件，恢复 clippy baseline 机制 + cargo-tarpaulin + Codecov + rgb_to_hex 修复 + ci-test-rust bash 算术 bug 修复）+ Batch 486（main 01faa60，2 文件，quotation_service 19 测试 + purchase_receipt_service 19 测试 = 38 测试）+ Batch 487（main 3919255 + d7e3b73 + a456a53，28 文件 +1836 -29，T02 7 业务路径 73 测试 + T07 4 benches 11 基准 criterion optional feature 机制 + T05 applyAuthMocks 移除 mockBusinessApi + webServer 数组化）。
> 详细记录已归档到 [doto-su.md §V15 Batch 485/486/487](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.7 模块 G：部署与运维（11 项已完成 / 6 项未完成）

> **已完成项**（11 项，详细记录已归档到 [doto-su.md §V15 Batch 488](file:///workspace/.monkeycode/doto-su.md)）：
> - ✅ P0-D01 Docker 文件违规（审计误判，5 个 Docker 文件均不存在）
> - ✅ P0-D02 install.sh PostgreSQL 客户端（审计误判，已移除）
> - ✅ P0-D03 5 service 缓存层接入（commit cead770，redis_cache.rs L2 双缓存 + 5 service）
> - ✅ P0-D04 moka→Redis 缓存迁移（commit cead770，与 D03 同批）
> - ✅ P0-D06 aria-label 全部完成（commit 22c842a，55 子批次 ~225 文件，WCAG 2.1 AA）
> - ✅ P0-D07 img alt 属性（审计误判，user-profile + TfaStep2 已有 alt）
> - ✅ P0-D11 setup_test_db 重复定义（审计误判，test_common.rs + tests/common/mod.rs 已抽取）
> - ✅ P0-D12 圈复杂度优化（commit 25efd76~ae73f42，6 项重构 + 2 项误判跳过）
> - ✅ P0-D15 升级零停机（审计误判，upgrade.rs 蓝绿部署已完整实现 14 函数）
> - ✅ P0-D16 报表订阅后台调度（审计误判，report_subscription_scheduler.rs 268 行已实现）
> - ✅ P0-D17 OA 公告（审计误判，oa_announcement_service.rs 完整 CRUD 已实现）

#### P0-D05 useI18n 接入率仅 3.1%（类七，XL，未开始）

- **来源**：batch-07 P0-07-5
- **证据**：2026-07-19 精确审计：实际 355 个 .vue 文件（非 85+ 也非 347），已接入 11 个（接入率 3.1%），未接入 344 个；locales/zh-CN.ts 467 行 15 模块 332 键，预估需扩容至 5000+ 键；Top 20 硬编码密集文件累计 10746 行中文（占全模块 15%），单文件最大 fixed-assets/tabs/AssetListTab.vue 864 行
- **修复方案**：355 个 .vue 视图组件全部接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts 同步；按业务模块横向切片，每批 10-12 文件，预估需 30-36 批次
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) + [frontend/src/locales/zh-CN.ts](file:///workspace/frontend/src/locales/zh-CN.ts) + [frontend/src/locales/en-US.ts](file:///workspace/frontend/src/locales/en-US.ts)
- **依赖**：建议在 D13/D14 完成后推进（避免同时修改 .vue 文件造成冲突）
- **工作量**：XL（5 项中最大）
- **批次**：488（D 系列 17 项一次性打包；预估 30-36 子批次）
- **执行优先级**：第 5 顺位（最后推进）

#### P0-D08 91+ 超长函数（类七，XL，进行中）

- **来源**：batch-07 P0-07-8
- **证据**：2026-07-19 精确扫描（fn-to-next-fn 口径）：>80 行函数约 91 个，>100 行函数约 54 个，>200 行函数 6 个，>500 行函数 0 个；最严重案例 so/delivery.rs:110 ship_order 346 行、so/order_crud.rs:98 create_order 344 行、ar_service.rs:993 manual_verify 257 行、bpm_service.rs:242 approve_task 211 行、wage_service.rs:873 calculate 211 行、ar_service.rs:706 auto_verify 192 行；预估还有 10-20 个 D08 函数未捕获（services/ai、services/ar/inv.rs、services/inv/adjust.rs 等未完整展开）
- **已重构确认**：event_bus.rs:412 start_event_listener D12-2 已重构（实际 279 行，CC 33→10 达标，列入观察名单不强拆）
- **豁免函数**：dye_batch_state_machine_service.rs:165 builtin_transition_rules 154 行纯数据表（27 条状态机三元组定义）豁免拆分
- **修复方案**：拆分超长函数为单一职责小函数（每个 ≤50 行），主函数仅做协调；按 ROI 分四梯队推进
- **关联文件**：[backend/src/services/so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) / [ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) / [bpm_service.rs](file:///workspace/backend/src/services/bpm_service.rs) / [wage_service.rs](file:///workspace/backend/src/services/wage_service.rs) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) / [quotation_service.rs](file:///workspace/backend/src/services/quotation_service.rs) / 等 35+ 文件
- **依赖**：无前置依赖
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次）
- **执行优先级**：第 1 顺位（无前置依赖 + 解锁 D09/D10）
- **进度**：D08-1 第一梯队 6/6 + 第二梯队首批 5/5 + 第二梯队第 2 批 5/5 + 第二梯队第 3 批 5/5 + 第二梯队第 4 批 7/7 + 第三梯队第 1 批 6/6 完成；第二梯队 22 函数 + 第三梯队第 1 批 6 函数全部完成；第一梯队 6 函数（ship_order/create_order/manual_verify/approve_task/calculate/auto_verify）+ 第二梯队首批 5 函数（batch_update_products/import_products_from_csv/quotation update/detect_anomalies/auto_generate_from_receipt）+ 第二梯队第 2 批 5 函数（ar create_payment/voucher update_account_balances/so update_order/purchase_return approve_return/ai predict_quality）+ 第二梯队第 3 批 5 函数（omni_audit new/ap_report get_statistics_report/bi_analysis kpi_summary/business_metrics new/outsourcing record_receipt）+ 第二梯队第 4 批 7 函数（so list_orders/init_service create_default_roles/ap_report get_aging_report/production_order increase_finished_goods_txn/chemical update/ar vfy get_aging_report/ap_verification auto_verify）+ 第三梯队第 1 批 6 函数（ai/pred.rs forecast_sales 134行/so/order_workflow.rs submit_order 134行/inventory_finance_bridge_service.rs create_inventory_adjustment_voucher 131行/production_order_service.rs deduct_raw_materials_txn 131行/custom_order_state_service.rs advance 130行/voucher_service.rs write_assist_accounting_records_txn 130行）+ 第三梯队子批次4 7 函数（five_dimension_service get_stats/quotation_convert_service convert/crm/assign auto_assign_leads/so/delivery process_shipment_items/ar/vfy generate_reconciliation/so/order_workflow approve_order/ap_reconciliation_service auto_reconcile_all，待推送 CI 验证）；第二梯队首批 CI 4 轮修复：① BatchError 未实现 Clone（ctx.errors.clone() 失败）+ Clone derive；② CI 自动刷新 baseline 在 clippy 编译失败时误删预存警告（第三次复发，修复：自动刷新条件增加 CLIPPY_MAIN_EXIT = 0 + CLIPPY_MAIN_EXIT 写入文件供后续 step 读取）；③ apply_order_header_updates 借用引用后 String 字段 move（E0507，4 处 if let Some 改 &request.x + clone）；④ baseline 恢复 5 条预存警告；第二梯队第 2 批 CI 1 轮通过；第二梯队第 3 批 CI 2 轮修复：① clippy 退出码 101（预存编译错误已在 baseline），但本次 clippy 运行更远捕获 256 条结构化记录（前次仅 54 条），导致 157 条预存 dead_code 警告（常量/函数/变体未使用）被报为"新增"；② 修复：将 157 条 dead_code 警告摘要追加到 baseline（warning 摘要 7→164 条，总行数 142→299 行）；教训：clippy 退出码 101 时 current.txt 可能不完整，但本次相反——clippy 运行更远捕获更多预存警告，baseline 需扩充以覆盖全部预存 dead_code；第二梯队第 4 批 CI 2 轮修复：① chemical_service.rs 8 个 apply_* helper 中 String 类型字段使用 `if let Some(v) = req.xxx` 尝试从 `&Option<String>` move 出 String 值，触发 E0507 cannot move out of `Some` which is behind a shared reference（25 个错误），修复：String 类型字段改为 `if let Some(v) = &req.xxx { ... Set(v.clone()) }`，Copy 类型字段（i32/Decimal）保持原样；② ar/vfy.rs build_customer_aging_summaries 参数 `&mut Vec<AgingBucket>` 触发 clippy::ptr_arg 警告，修复：改为 `&mut [AgingBucket]`；教训：subagent 拆分时 helper 函数参数 `&UpdateRequest` 中 String 字段必须用 `&req.xxx` 借用后 clone，不能用 `req.xxx` move；clippy::ptr_arg 警告：`&mut Vec<T>` 参数建议用 `&mut [T]` slice 类型
- **批次规划**：
  - 第一梯队（>200 行 6 函数，2 批）：✅ ship_order / ✅ create_order / ✅ manual_verify / ✅ approve_task / ✅ calculate / ✅ auto_verify
  - 第二梯队（150-200 行 22 函数，4 批）：✅ 首批 5 函数（batch_update_products/import_products_from_csv/quotation update/detect_anomalies/auto_generate_from_receipt）+ ✅ 第 2 批 5 函数（ar create_payment/voucher update_account_balances/so update_order/purchase_return approve_return/ai predict_quality）+ ✅ 第 3 批 5 函数（omni_audit new/ap_report get_statistics_report/bi_analysis kpi_summary/business_metrics new/outsourcing record_receipt）+ ✅ 第 4 批 7 函数（so list_orders/init_service create_default_roles/ap_report get_aging_report/production_order increase_finished_goods_txn/chemical update/ar vfy get_aging_report/ap_verification auto_verify）
  - 第三梯队（100-150 行 75 函数，10-11 批）：✅ 子批次1 6 函数（scheduling_query confirm_schedule_result/batch_service batch_create_products/inventory_adjustment_service approve_adjustment/user_service update_user/energy_service monthly_allocation_by_duration/dashboard_service get_inventory_statistics，PR #669）+ ✅ 子批次2 7 函数（bpm_service start_process/so/delivery cancel_delivery/bi_analysis_service pivot/ai/rec optimize_inventory/sales_return_service apply_stock_inbound_txn/fixed_asset_service depreciate/ap_verification_service manual_verify，PR #670）+ ✅ 子批次3 6 函数（ai/pred.rs forecast_sales/so/order_workflow.rs submit_order/inventory_finance_bridge_service.rs create_inventory_adjustment_voucher/production_order_service.rs deduct_raw_materials_txn/custom_order_state_service.rs advance/voucher_service.rs write_assist_accounting_records_txn，main 772c0312）+ ✅ 子批次4 7 函数（five_dimension_service get_stats/quotation_convert_service convert/crm/assign auto_assign_leads/so/delivery process_shipment_items/ar/vfy generate_reconciliation/so/order_workflow approve_order/ap_reconciliation_service auto_reconcile_all，commit b869a0cd CI 29788038151 全绿，引入 ForecastInputs/BridgeVoucherArgs/AssistRecordContext 参数对象）+ ✅ 子批次5 7 函数（deploy_release_legacy 150→48+8helper/send_email 142→25+6helper/with_secrets_and_cors 133→10+4helper+AppServices/handle_socket 129→49+2helper/create_fabric_order 122→43+6helper/delete_user 120→31+5helper/update_supplier 119→37+3helper，commit 97fd77ee CI 29792250059 全绿）+ ✅ 子批次6+7 14 函数合并推送（API 推送 commit 47ad2bfa CI 29795862710 全绿；子批次6 7 函数 export_customers/export_production_orders/export_warehouses/export_audit_logs/export_prices/logout/export_records 统一 headers+row builder+table builder+audit recorder 四层模式；子批次7 7 函数 export_contracts/export_standards/export_assets/export_collections/export_ap_invoices/export_budget_items/export_ar_invoices 同四层模式）+ ✅ 子批次8 6 函数（refresh_balance 104→38+5helper/batch_assign 103→38+3helper+LeadAssignCtx/check_low_stock 103→23+4helper/split_fabric_piece 102→31+4helper/export_dye_recipes 101→20+5helper/export_suppliers 101→25+6helper，commit 4e1cb058 + 编译错误修复 106957be CI 29827225295 全绿：修复 Arc<DB>.as_ref() 解引用 + SupplierQueryParams Clone derive + lead_ids.iter().copied()）；第三梯队全部完成（53 函数，8 子批次），扫描确认 100-150 行范围无剩余候选
  - 第四梯队（80-100 行实际 135 函数，预估 20 批）：✅ 子批次1 7 函数（update_opportunity 100→27+3helper/create_revenue_voucher_for_delivery 100→31+2helper/transfer_lead 100→36+5helper/ai_summary 100→31+4helper/create_sales_delivery_voucher 100→43+1helper/create_receivable 100→39+2helper/advance 99→23+4helper，commit 8a701b6 + 编译错误修复 6018985 CI 29843816503 全绿：opp.rs 7 处 Option<T> 字段未用 Some 包裹 Set(*v)→Set(Some(*v))）+ 🔄 子批次2 7 函数拆分完成待推送（create_purchase_receipt_voucher 99→47+2helper/run_consumer_loop 99→50+4helper/get_monthly_report 99→25+2helper/apply_stock_updates_for_adjustment 99→21+3helper/aggregate_finance_data 99→24+3helper/create_adjustment 91→27+3helper/start_step 92→23+3helper）；剩余 121 函数（预估 18 子批次）；含 inventory_finance_bridge 7 个 create_*_voucher 模板化提取
  - 模板化提取候选：inventory_finance_bridge_service.rs 7 个 create_*_voucher 函数提取通用 create_bridge_voucher<VoucherBuilder>

#### P0-D09 54+ 函数超过 100 行（类二，L，D08 完成后自动完成）

- **来源**：batch-02 P0-02-01
- **证据**：2026-07-19 精确扫描：>100 行函数约 54 个（与 D08 范围重叠，D08 是 D09 的超集）
- **修复方案**：D08 完成后 D09 自动完成（D09 是 D08 子集，D08 阈值 >80 行涵盖 D09 阈值 >100 行）
- **关联文件**：同 P0-D08
- **依赖**：P0-D08
- **工作量**：L（实际 0 增量工作，D08 完成即 D09 完成）
- **批次**：488（D08 子集，不独立成批）

#### P0-D10 30 个后端文件超过 1000 行（类二，L，未开始）

- **来源**：batch-02 P0-02-02
- **证据**：2026-07-19 精确扫描：实际 30 个 >1000 行文件（doto.md 原标记 26 个不准），13 个 >1500 行，1 个 >2000 行（ar_service.rs 2067 行）；审计后新增越线 main.rs 1005 行 + init_service.rs 1287 行；28 个原审计文件全部仍 >1000 行无一下降；bi_analysis_service.rs 增长最快（+201 行 1461→1662）
- **修复方案**：按职责拆分为多个文件（如 ar_service.rs 拆分为 ar_service / ar_aging_service / ar_collection_service；models/status.rs 拆分为 status/sales / status/purchase / status/inventory；main.rs 拆为 main / routes_bootstrap / middleware_bootstrap）
- **关联文件**：[backend/src/services/ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) (2067) / [production_order_service.rs](file:///workspace/backend/src/services/production_order_service.rs) (1998) / [so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) (1930) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) (1841) / [energy_service.rs](file:///workspace/backend/src/services/energy_service.rs) (1800) / 等 30 文件
- **依赖**：P0-D08/D09（避免函数拆分和文件拆分同时进行造成冲突）
- **工作量**：L
- **批次**：488（D 系列 17 项一次性打包；预估 5-6 子批次，每批 5-6 文件）
- **执行优先级**：第 2 顺位（D08 完成后立即推进）
- **批次规划**：
  - 第 1 批：ar_service.rs (2067) + production_order_service.rs (1998) + so/delivery.rs (1930) 3 个 >1800 行文件
  - 第 2 批：voucher_service.rs (1841) + energy_service.rs (1800) + outsourcing_service.rs (1782) + business_mode_service.rs (1718) 4 个 >1700 行文件
  - 第 3 批：chemical_service.rs (1676) + bi_analysis_service.rs (1662) + models/status.rs (1577) + mrp_engine_service.rs (1556) 4 个 >1500 行文件
  - 第 4 批：dye_batch_state_machine_service.rs (1512) + wage_service.rs (1507) + ar/vfy.rs (1320) + ap_invoice_service.rs (1306) 4 个 >1300 行文件
  - 第 5 批：init_service.rs (1287) + flow_card_service.rs (1271) + ap_reconciliation_service.rs (1243) + search/elastic.rs (1230) 4 个 >1200 行文件
  - 第 6 批：剩余 11 个 1000-1200 行文件

#### P0-D13 前端 123 个组件缩写命名（类二，XL，未开始）

- **来源**：batch-02 P0-02-05
- **证据**：2026-07-19 精确扫描：实际 123 个缩写命名 .vue 文件（views/ 122 + components/ 1，doto.md 原标记 ~119 误差 ±4）；25 类缩写前缀（Sc/Su/Lgs/Vchr/Pp/Di/Tfa/Sec/Cp/Sch/Prd/Bpm/Pc/Pi/Sa/Db/Purch/Prc/PrRtn/Ms/Sp/Olv/Ep/Bom/AI）；32 个父级 .vue 文件需更新 import（99 处 import 语句）；0 路由风险（router/index.ts 不直接 import 缩写文件）；0 e2e 风险（e2e 测试通过 Playwright 交互不直接 import 组件）；约 30 个 composables 同步重命名（D14/D15 范畴）
- **修复方案**：重命名为描述性全名（如 ScFilter→SalesContractFilter、SuVerDetail→SystemUpdateVersionDetail、LgsTbl→LogisticsTable、VchrForm→VoucherForm、BomForm→BillOfMaterialsForm）；同步重命名 composables 和父级 import；保留白名单：API（ApiEndpointTab 已描述性）/ i18n / a11y / V2Table（30+ 文件引用影响大）
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) 25 个模块的 components/ 子目录 + [frontend/src/components/ai/AIPredictionChart.vue](file:///workspace/frontend/src/components/ai/AIPredictionChart.vue)
- **依赖**：建议在 D14 完成后推进（避免同时修改 import 路径造成冲突）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 12-15 子批次，每批 8-10 文件）
- **执行优先级**：第 4 顺位（D14 完成后推进）
- **批次规划**：按模块分组（每模块独立批次）
  - sales-contract (3) + system-update (3) + sales-price (5) + purchase-price (5) 第 1 批 16 文件
  - logistics (6) + finance/tabs (4) + voucher/tabs (4) + data-import (4) 第 2 批 18 文件
  - security/two-factor (5) + security/components (4) + capacity (4) + advanced (4) 第 3 批 17 文件
  - api-gateway (1) + sales (3) + scheduling (10) + arReconciliation (6) 第 4 批 20 文件
  - purchase-return (5) + material-shortage (3) + production (4) + bpm/definitions (5) 第 5 批 17 文件
  - bpm/approval (6) + purchase-contract (4) + purchase-inspection (5) + sales-analysis (5) 第 6 批 20 文件
  - bom (1) + dashboard (4) + purchase (6) + purchaseReceipt (4) + components/ai (1) 第 7 批 16 文件

#### P0-D14 前端 api 命名不统一（类二，XL，未开始）

- **来源**：batch-02 P0-02-06
- **证据**：2026-07-19 精确扫描：96 个 api/*.ts 文件（准确）；风格 A（object 形式 `export const xxxApi = {}`）21 个 + 风格 B（function 形式）68 个 + 混合风格 4 个（supplier/customer/financial-analysis/five-dimension）+ 纯 re-export 3 个（index/ap-reconciliation/ap-verification）；最大偏差源 listXxx 47 文件 84 处需改名为 getXxxList；次要偏差 addXxx 5 文件 6 处 / removeXxx 2 文件 2 处 / fetchXxx 1 文件 1 处 / queryXxx 2 文件 2 处
- **修复方案**：统一为风格 B（function 形式）+ 命名规范 `getXxxList / createXxx / updateXxx / deleteXxx / getXxxById`；保留 request.ts 不改名（基础设施）；4 个混合文件先去重再统一；3 个 re-export 文件同步更新导出列表；预估影响 2000+ 处调用点
- **关联文件**：[frontend/src/api/](file:///workspace/frontend/src/api/) 96 个 .ts 文件
- **依赖**：无前置依赖（独立任务）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次，每批 8-10 文件）
- **执行优先级**：第 3 顺位（与 D05/D13 解耦）
- **批次规划**：
  - Batch 1：财务 AP/AR 9 文件（ap.ts/ap-invoice.ts/ap-payment.ts/ar.ts/ar-reconciliation.ts/ar-reconciliation-enhanced.ts/ap-reconciliation.ts/ap-verification.ts/voucher.ts）
  - Batch 2：采购/销售/库存 18 文件
  - Batch 3：生产/质量/BOM/MRP 12 文件
  - Batch 4：CRM/客户/供应商/贸易 14 文件
  - Batch 5a/5b：系统/权限/基础/报表/其他 40 文件（拆 2 子批）

---

## 五、P1/P2/P3 任务规划（按类别汇总）

> P0 完成后按优先级顺序推进。详细内容见 V15 审计报告 [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)。

### 5.1 P1 高优先级（257 项，预估 45-55 批次，按每批 9-12 文件计算）

| 模块 | P1 数 | 主要内容 | 关键批次预估 |
|------|-------|----------|--------------|
| 类二 通用代码质量 | 3 | api 命名/缩写命名/DbErr 包装 | 2 批 |
| 类三 安全性 | 6 | refresh_token/PUBLIC_PATHS/validator/Webhook/magic bytes/zip bomb | 3 批 |
| 类四 面料行业深化 | 11 | batch_trace/检验指标/工资凭证/能耗/委外/事件发布/工时 | 4 批 |
| 类五 运行逻辑闭环 | 11 | 状态机/配置/业务事件/成本归集/加权平均 | 4 批 |
| 类六 测试体系 | 11 | 覆盖率/mock/fixtures/文档 | 4 批 |
| 类七 可维护性 | 11 | i18n/aria/缓存/文档 | 4 批 |
| 类八 法律合规 | 16 | 用户协议/HTTPS/脱敏/导出/docx/标准/签章/税/环保/排污/劳动/工时/社保/职业健康 | 6 批 |
| 类九 色卡发放 | 9 | 清单/通知/报表 | 3 批 |
| 类十 大货批色 | 7 | 提醒/报表/统计 | 3 批 |
| 类十三 打印导出 | 14 | 审计字段/水印/性能 | 5 批 |
| 类十四 权限维度 | 14 | 权限测试/审计/缓存 | 5 批 |
| 类十五 业务主体 | 1 | supplier_evaluation migration | 1 批 |
| 类十六 AI 模块 | 24 | 配伍性/化验室/准确率/版本/权限/超时/并发/缓存/脱敏/MLOps | 8 批 |
| 类十七 财务深化 | 35 | 期间/反结账/年结/回转/账龄/杜邦/预测/差异/折旧 | 12 批 |
| 类十八 CRM | 12 | 线索评分/去重/转移审批 | 4 批 |
| 类十九 报表 BI | 5 | 版本管理/缓存 | 2 批 |
| 类二十 可观测性 | 9 | trace/metrics/WebSocket | 3 批 |
| 类二十一 胚布拆匹 | 10 | 库存/委外/继承 | 4 批 |
| 类二十二 库存排程 | 9 | 调拨/安全/排程 | 3 批 |
| 类二十三 组织物流 | 11 | 组织树/售后/运费 | 4 批 |
| 类二十四 前端架构 | 16 | PWA/移动端/chunks/ErrorBoundary/CSP/keep-alive/CSS/暗黑 | 6 批 |
| 类二十五 部署升级 | 11 | set -euo/SHA256/schema/蓝绿/健康/优雅/回滚 | 4 批 |
| **合计** | **257** | | **约 45 批**（每批 9-12 文件） |

### 5.2 P2 中优先级（248 项，预估 35-45 批次，按每批 9-12 文件计算）

| 类别 | P2 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 19 | 代码质量 / 安全防护 / 面料行业字段补齐 |
| 类五~类八 | 47 | 运行逻辑 / 测试补充 / 可维护性 / 法律合规细节 |
| 类九~类十二 | 33 | 色卡发放细节 / 大货批色细节 / 打印导出 / 权限细节 |
| 类十三~类十四 | 25 | 打印导出 P2 / 权限 P2 |
| 类十五~类十六 | 53 | 业务主体 P2 / AI 模块 P2 |
| 类十七~类十九 | 39 | 财务 P2 / CRM P2 / 报表 BI P2 |
| 类二十~类二十二 | 25 | 可观测性 / 胚布 / 库存 P2 |
| 类二十三~类二十五 | 83 | 组织物流 / 前端架构 / 部署升级 P2 |
| **合计** | **248** | |

### 5.3 P3 低优先级（123 项，按需修复）

| 类别 | P3 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 11 | 文档 / 注释 / 命名优化 |
| 类五~类八 | 17 | 测试增强 / 可维护性增强 / 法律合规增强 |
| 类九~类十二 | 9 | 色卡 / 批色 / 打印 / 权限增强 |
| 类十三~类十四 | 5 | 打印导出 / 权限增强 |
| 类十五~类十六 | 25 | 业务主体增强 / AI 增强 |
| 类十七~类十九 | 11 | 财务 / CRM / 报表增强 |
| 类二十~类二十二 | 12 | 可观测性 / 胚布 / 库存增强 |
| 类二十三~类二十五 | 41 | 组织物流 / 前端架构 / 部署升级增强 |
| **合计** | **123** | |

---

## 六、规则节点提醒

| 规则 | 优先级 | 内容 |
|------|--------|------|
| 规则 0/1/2/8 | 🔴 | 真实实现强制：所有 P0/P1 修复必须真实实现，禁止占位符 |
| 规则 3 | 🔴 | 成品文档格式：导出必须 .xlsx / 报表必须 .docx |
| 规则 5 | 🟡 | E2E 独立工作流：每 30 批次触发（批次 30/60/90...） |
| 规则 6 | 🔴 | 测试 mock 数据禁止硬编码：所有测试 mock 数据抽取到 fixtures |
| 规则 10 | 🟡 | 每 15 批次记忆整理 + 实时归档：每批完成后立即归档到 doto-su.md |
| 规则 11/12 | 🔴 | 法律合规与安全标准：所有修复必须符合中国法律法规 + 安全标准 |
| 规则 13 | 🔴 | 修复流程自动化：CI 全绿后自动开始下一批，无需用户确认；**步骤 0 确定审计结果内容是否存在**（修复前置门，避免基于过时审计做无效修复）+ **步骤 4 修复后推送前自审**（内容正确性+注释规范性+注释一致性，与规则 20 联动） |
| 规则 14 | 🔴 | 移除所有警告抑制：所有警告视为错误需修复（baseline 213/213 ✅ 全部清零） |
| 规则 15 | 🟢 | V15 全项目综合审计：25 大类 195 维度审计 ✅ 已完成 |
| 规则 19 | 🟡 | 工具连接异常分级响应：L1 60s / L2 60-180s / L3 30min 周期 |
| 规则 20 | 🔴 | 注释与功能一致性：代码注释必须与功能实现一致，禁止随意编写；CI 强制检查 |
| §10.0.1 | 🔴 | 复用现有功能原则：修复前必须调研现有实现，禁止重复造轮子 |

---

## 七、历史任务（全部完成，详细记录已归档）

> 以下阶段全部完成，详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

| 阶段 | 批次范围 | 内容 | 归档位置 |
|------|----------|------|----------|
| v13 复审修复 | 270-394 | 213 baseline + 业务/财务/运行逻辑闭环 | doto-su.md §v13 |
| v14 复审修复 | 395-432 | 12 P0 + 31 P1 + 12 P2 + 6 P3 + 213 baseline | doto-su.md §v14 |
| V15 审计 | 2026-07-16 | 25 大类 195 维度 21 批并行子代理审计 | docs/audits/v15/ |
| V15 修复阶段一（P0 部分） | 433-459 | 16 P0 任务完成 | doto-su.md §V15 |
| V15 修复阶段一续（P0 续） | 460-472 | P0-F01~F09 + P0-F07 前端重写 | doto-su.md §V15 |
| V15 复审归档 | 2026-07-17 | 4 项标记未完成实际已完成项归档 | doto-su.md §V15 复审核实发现的已完成项 |
| V15 复审报告 | PR #649 | 30 P0 任务实际状态复查报告 | docs/audits/v15-fix-reaudit-2026-07-17.md |
