# 历史任务归档（v13-v15 修复阶段）

> 本文件归档 doto.md §三批次规划表 + §四已完成模块 A-F + §七历史任务阶段记录。
> 归档日期：2026-07-22
> 归档原因：违反规则 10「doto.md 只存未完成任务」原则，已完成模块的批次规划表与历史任务详情应归档到独立文件。

---

## 一、P0 批次规划表（历史，39 项已规划为 22 个批次）

> Batch 473~487 共 18 个批次全部已合并（PR #656~#668 + main 直接提交），覆盖 P0 任务 39 项（模块 A/B/C/D/E/F 全部完成）。
> 详细批次内容（合并方式、commit hash、CI 验证轮次、教训）已归档到 [doto-su.md §V15 Batch 473-487](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。
>
> **当前进行中**：Batch 488（D 系列 17 项打包，已完成 12/17，剩余 5 项大型任务 D05/D08 第三梯队/D09/D10/D13/D14）。

### 1.1 批次工作量分布

> 批次节奏从 6-8 提升至 9-12 文件后，单批文件数普遍进入 L/XL 区间，每批覆盖任务更多，批次总数从 27 压缩为 22。

### 1.2 批次执行原则（参考）

1. **严格按顺序**：批次编号即执行顺序，前批未完成不进入下批
2. **依赖优先**：有依赖的任务排在被依赖任务之后
3. **每批 9-12 文件**：批次节奏统一为 9-12 文件，提升单批吞吐量
4. **模块内连续**：同模块任务连续执行，减少上下文切换
5. **CI 全绿推进**：每批 CI 全绿后自动进入下一批（规则 13）
6. **直接 push 验证**：禁止本地编译验证，所有验证直接 push 让 CI 执行

---

## 二、已完成模块清单（模块 A-F，39 项 P0 任务全部完成）

### 2.1 模块 A：安全与权限（7 项全部完成）

- **完成项**：P0-S12 前端导出 / P0-S13 审计日志导出 / P0-S14 migration / P0-S15 导出水印 / P0-S17 打印 HTML / P0-S19 审计字段，共 7 项
- **完成批次**：Batch 473（PR #656）+ Batch 474（PR #657）+ Batch 475a-e（PR #658~#662）+ Batch 476（main eb57484）
- **详细记录**：[doto-su.md §V15 Batch 473/474/475a-475e/476](file:///workspace/.monkeycode/doto-su.md)

### 2.2 模块 B：面料行业-色卡发放（4 项全部完成）

- **完成项**：P0-F10 库存联动 / P0-F11 前端文件结构 / P0-F12 前端类型/API/视图 / P0-F13 数据迁移
- **完成批次**：Batch 477（main a3798f4 + daeab0f，15 文件，m0057+m0058 迁移 + color_card.rs Model + color_card_issue_service.rs 5 方法库存联动 + 前端 5 新文件）
- **详细记录**：[doto-su.md §V15 Batch 477](file:///workspace/.monkeycode/doto-su.md)

### 2.3 模块 C：面料行业-大货批色（5 项全部完成）

- **完成项**：P0-F15 bulk_color_approval 表 / P0-F16 剪大货样 / P0-F17 客户批色确认 / P0-F18 返工/降级/报废 / P0-F19 ship_order 校验
- **完成批次**：Batch 478（main 9d01a42 + 6aca804，11 文件，8 态状态机 + 9 端点）+ Batch 479（main 642d2c09 + cc1ee381 + c06109fd + bbf38a30，7 文件，customer_rework 联动返工生产订单 + downgrade 联动库存等级降级 + scrap 联动库存报废标记）
- **详细记录**：[doto-su.md §V15 Batch 478/479](file:///workspace/.monkeycode/doto-su.md)

### 2.4 模块 D：面料行业-质量管理（2 项全部完成）

- **完成项**：P0-F20 8D 质量管理流程 / P0-F21 返工走生产订单
- **完成批次**：Batch 479（main 642d2c09 + cc1ee381 + c06109fd + bbf38a30，与 P0-F18 合并，m0059_add_rework_order_fields + production_order_service.rs create_rework_order + RW-YYYYMMDD-NNN 订单号 + 不触发 MRP）+ Batch 480（main 5334bf13 + 8d7ea998 + ae87219f，13 文件，11 态状态机 + 10 条合法边 + From<AppError> 透传）
- **详细记录**：[doto-su.md §V15 Batch 479/480](file:///workspace/.monkeycode/doto-su.md)

### 2.5 模块 E：财务与业务流程（17 项全部完成）

- **完成项**：P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警 + P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收 + P0-B14 Incoterms + P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换（B18~B30 为归并项不独立计）
- **完成批次**：Batch 481（PR #666 squash 00261365，25 文件）+ Batch 482（PR #667，13 文件）+ Batch 483（PR #668 squash e094846e，15 文件）+ Batch 484（main df5286ee + c012a3b9，11 文件）
- **详细记录**：[doto-su.md §V15 Batch 481/482/483/484](file:///workspace/.monkeycode/doto-su.md)

### 2.6 模块 F：测试体系（8 项全部完成）

- **完成项**：P0-T01 核心 service 单测 + P0-T02 7 项集成测试 + P0-T03 clippy baseline + P0-T05 E2E 通过率 + P0-T06 bi_analysis 测试 + P0-T07 性能基准 + P0-T08 覆盖率工具（T04 在复审中归档为已完成项）
- **完成批次**：Batch 485（main af0f16b + 5e4e78f + 7cc82cc，4 文件）+ Batch 486（main 01faa60，2 文件，38 测试）+ Batch 487（main 3919255 + d7e3b73 + a456a53，28 文件 +1836 -29，T02 73 测试 + T07 11 bench + T05 配置修复）
- **详细记录**：[doto-su.md §V15 Batch 485/486/487](file:///workspace/.monkeycode/doto-su.md)

---

## 三、历史阶段任务（全部完成，详细记录已归档）

| 阶段 | 批次范围 | 内容 | 归档位置 |
|------|----------|------|----------|
| v13 复审修复 | 270-394 | 213 baseline + 业务/财务/运行逻辑闭环 | [doto-su.md §v13](file:///workspace/.monkeycode/doto-su.md) |
| v14 复审修复 | 395-432 | 12 P0 + 31 P1 + 12 P2 + 6 P3 + 213 baseline | [doto-su.md §v14](file:///workspace/.monkeycode/doto-su.md) |
| V15 审计 | 2026-07-16 | 25 大类 195 维度 21 批并行子代理审计 | [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/) |
| V15 修复阶段一（P0 部分） | 433-459 | 16 P0 任务完成 | [doto-su.md §V15](file:///workspace/.monkeycode/doto-su.md) |
| V15 修复阶段一续（P0 续） | 460-472 | P0-F01~F09 + P0-F07 前端重写 | [doto-su.md §V15](file:///workspace/.monkeycode/doto-su.md) |
| V15 复审归档 | 2026-07-17 | 4 项标记未完成实际已完成项归档 | [doto-su.md §V15 复审核实发现的已完成项](file:///workspace/.monkeycode/doto-su.md) |
| V15 复审报告 | PR #649 | 30 P0 任务实际状态复查报告 | [docs/audits/v15-fix-reaudit-2026-07-17.md](file:///workspace/.monkeycode/docs/audits/v15-fix-reaudit-2026-07-17.md) |
