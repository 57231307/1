# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近梳理：2026-07-18（规则 13 四次迭代新增步骤 0"确定审计结果内容是否存在"修复前置门 + 步骤 4"修复后推送前自审"与规则 20 联动；按业务模块重组未完成任务 + 新增依赖关系图 + 新增批次规划表 + 新增工作量预估；Batch 473 已合并 PR #656（P0-S14 migration + P0-S19 审计字段补齐）；Batch 474 已合并 PR #657（P0-S15 导出水印基础设施完成 + P0-S12 前端导出接入后端核心 2 页面完成）；Batch 475a 已合并 PR #658（P0-S13 审计日志导出闭环完成）；Batch 475b 已合并 PR #659（P0-S12 前端导出 purchase/customer 闭环，A 类 2 文件完成）；Batch 475c 已合并 PR #660（P0-S12 前端导出 B 类批次 1/3 完成，inventory + warehouse + production 3 模块闭环）；Batch 475d 已合并 PR #661（P0-S12 前端导出 B 类批次 2/3 完成，sales-contract + sales-price + quality + quality-standards 4 模块闭环）；Batch 475e 已合并 PR #662（P0-S12 前端导出 B 类批次 3/3 收尾完成，ar + ap + cost + budget + fixed-assets 5 模块闭环，P0-S12 前端导出接入后端全部完成）；Batch 476 已合并 main 直接提交 eb57484（P0-S17 打印 HTML 真实数据查询完成，print_service + print_handler 2 文件，6 个 get_*_print_data 方法从硬编码占位改为真实查询 DB）；Batch 477 已合并 main 直接提交 a3798f4 + daeab0f（P0-F10/F11/F12/F13 色卡发放库存联动 + 前端文件结构 + legacy 数据迁移完成，15 文件，PR #665 因 main 抢先直接提交被关闭冲突）；P0 进度 75/104；剩余 29 P0 + 257 P1 + 248 P2 + 123 P3 = 657 项；规则 13 连续执行；禁止本地编译验证；批次节奏调整为每批 9-12 文件）。

---

## 一、当前状态与总体进度

### 1.1 进度总览

| 优先级 | 总数 | 已完成 | 未完成 | 完成率 |
|--------|------|--------|--------|--------|
| **P0 阻塞级** | 104 | 75 | **29** | 72.1% |
| **P1 高优先级** | 257 | 0 | **257** | 0% |
| **P2 中优先级** | 248 | 0 | **248** | 0% |
| **P3 低优先级** | 123 | 0 | **123** | 0% |
| **合计** | **732** | **75** | **657** | **10.2%** |

> P0 已完成 75 项 = 原 62 项 + 复审发现已完成 4 项（P0-S08/S16/F14/T04）- 复审重新打开 1 项（P0-S14）+ Batch 473 修复 2 项（P0-S14 migration 补齐 + P0-S19 condition 字段补齐）+ Batch 474 修复 1 项（P0-S15 导出水印基础设施）+ Batch 475a 修复 1 项（P0-S13 审计日志导出闭环）+ Batch 476 修复 1 项（P0-S17 打印 HTML 真实数据查询）+ Batch 477 修复 4 项（P0-F10 库存联动 + P0-F11 前端文件结构 + P0-F12 前端类型/API/视图 + P0-F13 数据迁移）。
> P0-S12 前端导出接入后端：Batch 474 已完成核心 2 页面（customer/supplier），Batch 475a 已完成 audit-log（P0-S13 闭环），Batch 475b 已完成 purchase/customer 闭环（A 类 2 文件），Batch 475c 已完成 B 类批次 1/3（inventory + warehouse + production 3 模块），Batch 475d 已完成 B 类批次 2/3（sales-contract + sales-price + quality + quality-standards 4 模块），Batch 475e 已完成 B 类批次 3/3 收尾（ar + ap + cost + budget + fixed-assets 5 模块），**P0-S12 前端导出接入后端全部完成**。

### 1.2 状态：🔄 规则 13 连续执行中

- **当前批次**：Batch 477 已合并（main 直接提交 a3798f4 + daeab0f）—— P0-F10/F11/F12/F13 色卡发放库存联动 + 前端文件结构 + legacy 数据迁移完成（m0057 建表+stock_quantity 合并迁移 + m0058 旧表数据迁移 + color_card_migrate_legacy.sql + 5 方法接入库存联动 + 5 前端新文件 + 重构 issues.vue）
- **下一批次**：Batch 478（P0-F15 bulk_color_approval 表 + P0-F16 剪大货样 + P0-F17 客户批色确认 + P0-F19 ship_order 校验，~10 文件）
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

| 批次 | 模块 | 任务 | 工作量 | 依赖 | 说明 |
|------|------|------|--------|------|------|
| ✅ 473 | A | P0-S14 migration 047 + P0-S19 审计字段补齐 | S+M | 无 | 已合并 PR #656（squash e19c1aa）；复审重新打开项优先；解锁后续安全链路 |
| ✅ 474 | A | P0-S15 导出水印 + P0-S12 前端导出接入后端（核心页面） | M+XL | 473 | 已合并 PR #657（squash 33c2e7c）；P0-S15 完成；P0-S12 完成 customer/supplier 2 页面，剩 23+ 页面在 475 批次；CI 修复 v3：merge_range 补 format + export.ts 改用 axios 绕过 request.ts 拦截器 |
| 🟡 475a | A | P0-S13 审计日志导出闭环 | M | 474 | 已合并 PR #658（squash 7c7cfc7）；P0-S13 完成；后端 audit_log_handler 注入水印 + 前端 audit-log/index.vue 切换 exportFromBackend + 测试 mock 更新；CI 2 轮修复 |
| 🟡 475b | A | P0-S12 前端导出 purchase/customer 闭环（A 类）| S | 474 | 已合并 PR #659（squash cde7e9a）；A 类 2 文件完成（usePurchAct.ts + CustomerListTab.vue）；后端 export_orders 注入水印；CI 一次过 13/13 全绿 |
| 🟡 475c | A | P0-S12 前端导出接入后端（B 类批次 1/3）| XL | 474 | 已合并 PR #660（squash 38e8e43）；inventory + warehouse + production 3 模块：后端新增 3 端点（export_stock/export_warehouses/export_production_orders）+ 前端切换 4 文件（inventory/index.vue + warehouse/index.vue + usePrdProc.ts + production/index.vue）+ 3 路由注册 + 自审门 = 11 文件；CI 2 轮（第 1 轮 E0599 WarehouseListQuery 未派生 Clone，第 2 轮修复后 13/13 全绿） |
| 🟡 475d | A | P0-S12 前端导出接入后端（B 类批次 2/3）| XL | 475c | 已合并 PR #661（squash 4bb7005）；sales-contract/sales-price + quality/quality-standards 4 模块：后端新增 4 端点（export_contracts/export_prices/export_records/export_standards）+ 前端切换 7 文件（useScProc.ts + useSpProc.ts + sales-contract/index.vue + sales-price/index.vue + RecordTab.vue + StandardTab.vue + quality-standards/index.vue）+ 4 路由注册 + 自审门 = 14 文件；CI 2 轮（第 1 轮前端类型检查失败 useTableApi queryParams 类型为 Ref<Record<string, unknown>> 与 getQueryParams 强类型返回值不兼容，第 2 轮修复添加类型断言后 13/13 全绿） |
| ✅ 475e | A | P0-S12 前端导出接入后端（B 类批次 3/3）| XL | 475d | 已合并 PR #662（squash ff07549）；ar/ap/cost/budget/fixed-assets 5 模块：后端新增 5 端点（export_ar_invoices/export_ap_invoices/export_cost_items/export_budgets/export_fixed_assets）+ 前端切换 5 文件（Tab.vue × 5）+ 5 路由注册 = 12 文件；CI 一次过 13/13 全绿；**P0-S12 前端导出接入后端全部完成** |
| ✅ 476 | A | P0-S17 打印 HTML 真实数据查询 | L | 无 | 已合并 main 直接提交 eb57484（PR #664 因 main 抢先直接提交被关闭冲突）；print_service.rs 6 个 get_*_print_data 方法从硬编码占位改为真实查询 DB（sales_order/sales_contract/purchase_order/purchase_receipt/inventory_transfer/voucher）+ print_handler.rs 注入 AppState；2 文件；CI 13/13 全绿 + 2 skipped |
| ✅ 477 | B | P0-F10 色卡发放库存联动 + P0-F11/F12 前端文件结构 + P0-F13 数据迁移 | M+L+S | 无 | 已合并 main 直接提交 a3798f4 + daeab0f（PR #665 因 main 抢先直接提交被关闭冲突）；m0057_create_color_card_issues_and_stock_fields（建表+stock_quantity 合并迁移）+ m0058_migrate_color_card_borrow_records（旧表数据迁移）+ color_card_migrate_legacy.sql + color_card.rs Model 新增 stock_quantity + color_card_crud_service.rs create() 初始化 + color_card_issue_service.rs 5 方法接入库存联动（issue 扣减/return_card+cancel_issue 还原/mark_lost+mark_damaged 不还原）+ 事务+lock_exclusive + 前端 5 新文件（types/colorCardIssue.ts + store/colorCardIssue.ts + composables/useColorCardIssue.ts + components/ColorCardIssueForm.vue + components/ColorCardIssueDetail.vue）+ 重构 issues.vue + color-card-issue.ts 独立 API 模块 = 15 文件；CI 2 轮（第 1 轮前端类型检查失败 ColorCardIssueForm.vue 未使用 props 变量 TS6133，第 2 轮修复后 13/13 全绿） |
| 478 | C | P0-F15 bulk_color_approval 表 + P0-F16 剪大货样 + P0-F17 客户批色确认 + P0-F19 ship_order 校验 | M+M+M+S | 无 | 数据库基础 + 业务规则 + 末端校验 = ~10 文件 |
| 479 | C | P0-F18 返工/降级/报废 + P0-F21 返工走生产订单 | L+M | 478 | 批色闭环：返工走生产订单 + 库存转报废仓 + 等级降级 = ~9 文件 |
| 480 | D | P0-F20 8D 质量管理流程 | XL | 无 | D0~D8 八步流程：quality_issue_service.rs + quality_issue_handler.rs + schema migrations + 11 态状态机 = ~12 文件 |
| 481 | E | P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警 | L+M+M+L | 无 | 坏账链路 + 催收 + 预警 4 项打包 = ~12 文件 |
| 482 | E | P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B14 Incoterms | S+M+S+S+S+S | 无 | 财务小项 6 项打包 = ~10 文件 |
| 483 | E | P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收 | M+L+M+M | 480 | BI + 定制订单 + 售后 + 物流 4 项打包（B12 依赖 480 的 8D 流程）= ~11 文件 |
| 484 | E | P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换 | M+M+L | 无 | 运维相关 3 项打包 = ~9 文件 |
| 485 | F | P0-T03 baseline 移除 + P0-T08 覆盖率工具 + P0-T01 核心 service 单测 + P0-T06 bi_analysis 测试 | M+S+L+M | 无 | CI 信号解锁 + 单测先行 4 项打包 = ~12 文件 |
| 486 | F | P0-T02 7 项集成测试 + P0-T07 性能基准 + P0-T05 E2E 通过率 | XL+M+XL | 485 | 集成测试 + 性能 + E2E 3 项打包 = ~12 文件 |
| 487 | G | P0-D01 Docker + P0-D02 install.sh + P0-D07 img alt + P0-D05 useI18n | S+S+S+XL | 无 | 部署小项 + 可访问性 4 项打包 = ~10 文件 |
| 488 | G | P0-D06 aria-label + P0-D08 超长函数 + P0-D09 100 行函数 + P0-D10 1000 行文件 | XL+XL+L+L | 无 | 前端可访问性 + 代码质量链路 4 项打包 = ~12 文件 |
| 489 | G | P0-D03 5 service 缓存 + P0-D04 moka→Redis + P0-D11 setup_test_db + P0-D12 圈复杂度 | L+L+M+M | 无 | 缓存迁移 + 测试工具 + 复杂度 4 项打包 = ~10 文件 |
| 490 | G | P0-D13 前端命名 + P0-D14 api 命名 + P0-D15 升级零停机 + P0-D16 报表订阅 + P0-D17 OA 公告 | XL+XL+M+M+M | 无 | 命名统一 + 部署运维收尾 5 项打包 = ~12 文件 |

**总计**：22 个批次（含 475c/475d/475e 微批次拆分），覆盖 39 P0 任务。

### 3.2 批次工作量分布

| 工作量 | 批次数 | 占比 |
|--------|--------|------|
| S（≤3 文件） | 0 | 0% |
| M（4-6 文件） | 0 | 0% |
| L（7-10 文件） | 5 | 23% |
| XL（>10 文件） | 17 | 77% |
| **合计** | **22** | 100% |

> 注：批次节奏从 6-8 提升至 9-12 文件后，单批文件数普遍进入 L/XL 区间，每批覆盖任务更多，批次总数从 27 压缩为 22。

### 3.3 批次执行原则

1. **严格按顺序**：批次编号即执行顺序，前批未完成不进入下批
2. **依赖优先**：有依赖的任务排在被依赖任务之后
3. **每批 9-12 文件**：批次节奏统一为 9-12 文件，提升单批吞吐量
4. **模块内连续**：同模块任务连续执行，减少上下文切换
5. **CI 全绿推进**：每批 CI 全绿后自动进入下一批（规则 13）
6. **直接 push 验证**：禁止本地编译验证，所有验证直接 push 让 CI 执行

---

## 四、未完成任务清单（按业务模块分组）

### 4.1 模块 A：安全与权限（0 项未完成，原 7 项 - Batch 473 完成 P0-S14 + P0-S19，Batch 474 完成 P0-S15，Batch 475a 完成 P0-S13，Batch 475b-e 完成 P0-S12，Batch 476 完成 P0-S17）

> 模块 A 全部 P0 任务已完成。下方保留已完成任务摘要供参考。

#### P0-S12 前端本地导出完全无审计（类十三，✅ 已完成）

- **来源**：batch-11 P0-11-10/11
- **进度**：✅ 全部完成（Batch 474 核心页面 + 475a-e 5 个微批次），共改造 25+ 页面，覆盖所有模块
- **已完成内容**：
  - Batch 474 PR #657：新增 `exportFromBackend` 函数（独立 axios 实例，绕过 request.ts 拦截器避免 Blob 类型丢失 + router 导入链副作用）；customer/index.vue + supplier/index.vue 改用后端 API；后端 `/crm/customers/export` + `/purchase/suppliers/export` 端点
  - Batch 475a PR #658：audit-log/index.vue 改用后端 API（`/audit-logs/export`）；后端 audit_log_handler 注入水印
  - Batch 475b PR #659：usePurchAct.ts + CustomerListTab.vue 改用后端 API；后端 export_orders 注入水印
  - Batch 475c PR #660：inventory/index.vue + warehouse/index.vue + usePrdProc.ts + production/index.vue 改用后端 API；后端新增 export_stock / export_warehouses / export_production_orders 3 端点 + 3 路由注册；CI 2 轮修复（E0599 WarehouseListQuery 未派生 Clone）
  - Batch 475d PR #661：useScProc.ts + useSpProc.ts + sales-contract/index.vue + sales-price/index.vue + RecordTab.vue + StandardTab.vue + quality-standards/index.vue 共 7 文件改用后端 API；后端新增 export_contracts / export_prices / export_records / export_standards 4 端点 + 4 路由注册；4 个 Query struct 派生 Clone（防 E0599 重现）；CI 2 轮修复（第 1 轮前端类型检查失败 useTableApi queryParams 类型为 Ref<Record<string, unknown>> 与 getQueryParams 强类型返回值不兼容，第 2 轮添加类型断言后全绿）；自审门发现 StandardTab.vue 同类问题并同步改造
  - Batch 475e PR #662：ar/ap/cost/budget/fixed-assets 5 模块 5 个 Tab.vue 改用后端 API；后端新增 export_ar_invoices / export_ap_invoices / export_cost_items / export_budgets / export_fixed_assets 5 端点 + 5 路由注册；CI 一次过 13/13 全绿
- **关联文件**：[frontend/src/utils/export.ts](file:///workspace/frontend/src/utils/export.ts) + 25+ 视图文件 + 各资源 handler/service
- **依赖**：✅ P0-S14 已完成（Batch 473）/ ✅ P0-S15 水印已完成（Batch 474）
- **工作量**：XL（拆分 5 个微批次完成）
- **批次**：474 + 475a-e（已全部完成）

> P0-S12 详细归档见 [doto-su.md §V15 Batch 474/475a-475e](file:///workspace/.monkeycode/doto-su.md)。

#### P0-S17 打印 HTML 是占位假数据（类十三，✅ 已完成）

- **来源**：batch-11 P0-11-15
- **进度**：✅ 已完成（main 直接提交 eb57484，Batch 476）
- **已完成内容**：print_service.rs 6 个 get_*_print_data 方法从硬编码占位改为真实查询 DB（sales_order/sales_contract/purchase_order/purchase_receipt/inventory_transfer/voucher）；print_handler.rs 注入 AppState，调用 PrintService::new(state.db.clone())；使用 sea-orm 直接查询主表 + 关联客户/供应商/仓库 + 明细项 + 产品（LoaderTrait）
- **关联文件**：[print_service.rs](file:///workspace/backend/src/services/print_service.rs) / [print_handler.rs](file:///workspace/backend/src/handlers/print_handler.rs)
- **依赖**：无
- **工作量**：L（实际 2 文件）
- **批次**：476（已合并 main 直接提交 eb57484）

> P0-S14（migration 补齐）+ P0-S19（审计字段补齐）已 Batch 473 PR #656 完成，详细记录归档到 [doto-su.md §V15 Batch 473](file:///workspace/.monkeycode/doto-su.md)。
> P0-S15（导出水印基础设施）已 Batch 474 PR #657 完成，详细记录归档到 [doto-su.md §V15 Batch 474](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.2 模块 B：面料行业-色卡发放（0 项未完成，4 项已完成）

#### ✅ P0-F10 色卡发放——库存联动（已完成，Batch 477）

- **状态**：✅ 已完成（main 直接提交 a3798f4 + daeab0f）
- **修复方案**：方案 A 采用 color_cards.stock_quantity INT NOT NULL DEFAULT 0 字段直接管理
  - m0057_create_color_card_issues_and_stock_fields.rs：建表 + 新增 stock_quantity 字段
  - color_card.rs Model 新增 stock_quantity: i32
  - color_card_crud_service.rs create() 初始化 stock_quantity=0
  - color_card_issue_service.rs 5 方法接入库存联动：
    - issue(): 事务 + lock_exclusive + stock_quantity -= issue_qty
    - return_card(): 事务 + lock_exclusive + stock_quantity += issue_qty
    - cancel_issue(): 事务 + lock_exclusive + stock_quantity += issue_qty
    - mark_lost / mark_damaged: 不还原（色卡消耗）
  - validate_issue_gates gate 2 增强：card.stock_quantity >= issue_qty

#### ✅ P0-F11 色卡发放——前端文件结构（已完成，Batch 477）

- **状态**：✅ 已完成
- **修复方案**：补齐 5 个前端文件
  - types/colorCardIssue.ts（类型定义模块，re-export + 业务专用类型）
  - store/colorCardIssue.ts（Pinia store，availableCards + issueRecords + 5 actions）
  - composables/useColorCardIssue.ts（业务 composable，封装 store + API）
  - components/ColorCardIssueForm.vue（发放表单组件）
  - components/ColorCardIssueDetail.vue（4 合 1 操作对话框）
- **重构**：issues.vue 使用新组件 + composable + store

#### ✅ P0-F12 色卡发放——前端类型/API/视图（已完成，Batch 477）

- **状态**：✅ 已完成
- **修复方案**：api/color-card.ts 类型/API 完整（含 IssueRecordInfo + 6 API 函数）；新增 color-card-issue.ts 独立 API 模块

#### ✅ P0-F13 色卡发放——数据迁移策略（已完成，Batch 477）

- **状态**：✅ 已完成
- **修复方案**：m0057_create_color_card_issues_and_stock_fields.rs 创建 color_card_issues 表（补齐 Batch 471 遗漏的建表迁移）+ m0058_migrate_color_card_borrow_records.rs 迁移旧表数据到新表（borrowed→issued 状态映射，幂等保护，序列同步 setval）+ color_card_migrate_legacy.sql SQL 迁移脚本
- **关键背景**：Batch 471 创建 model 但遗漏建表迁移，导致 API 运行时报 "relation color_card_issues does not exist"

---

### 4.3 模块 C：面料行业-大货批色（5 项）

#### P0-F15 大货批色——bulk_color_approval 表完全不存在（类十一）

- **来源**：batch-10 P0-10-1
- **证据**：`backend/src/models/bulk_color_approval.rs` model 不存在；`bulk_color_approval_service.rs` 不存在
- **修复方案**：CREATE TABLE `bulk_color_approval`（id, sales_order_id, dye_batch_id, customer_id, sample_type=cut_sample/lab_sample, approval_status=pending/approved/rejected/rework, approver_id, approval_date, reject_reason, attachment_url, remark）
- **关联文件**：migrations + bulk_color_approval.rs model
- **依赖**：无
- **工作量**：M
- **批次**：478（合并 F15+F16+F17+F19 为单批 ~10 文件）

#### P0-F16 大货批色——剪大货样业务规则未实现（类十一）

- **来源**：batch-10 P0-10-2
- **修复方案**：实现剪大货样 handler，从 dye_batch 剪取样品，扣减库存
- **关联文件**：bulk_color_approval_service.rs / bulk_color_approval_handler.rs
- **依赖**：P0-F15
- **工作量**：M
- **批次**：478（与 F15/F17/F19 合并）

#### P0-F17 大货批色——客户批色确认流程未实现（类十一）

- **来源**：batch-10 P0-10-3
- **修复方案**：客户通过链接/小程序确认批色，更新 approval_status
- **关联文件**：bulk_color_approval_handler.rs / 前端 customer_portal
- **依赖**：P0-F15
- **工作量**：M
- **批次**：478（与 F15/F16/F19 合并）

#### P0-F18 大货批色——返工/降级/报废未实现（类十一）

- **来源**：batch-10 P0-10-4
- **修复方案**：批色不通过时触发：返工（走生产订单）/ 降级（等级降 A→B→C）/ 报废（库存转报废仓）
- **关联文件**：bulk_color_approval_service.rs / production_order_service.rs / inventory_stock_service.rs
- **依赖**：P0-F15
- **工作量**：L
- **批次**：479（合并 F18+F21 为单批 ~9 文件）

#### P0-F19 大货批色——ship_order 不校验批色状态（类十一）

- **来源**：batch-10 P0-10-5
- **证据**：so/delivery.rs 无 bulk_color/批色 校验
- **修复方案**：发货前校验所有 dye_batch 的 bulk_color_approval.status='approved'，否则拒绝发货
- **关联文件**：[ship_order_service.rs](file:///workspace/backend/src/services/ship_order_service.rs)
- **依赖**：P0-F16/F17/F18
- **工作量**：S
- **批次**：478（与 F15/F16/F17 合并）

---

### 4.4 模块 D：面料行业-质量管理（2 项）

#### P0-F20 8D 质量管理流程完全缺失（类二十一）

- **来源**：batch-18 P0-18-1
- **证据**：quality_issue_service.rs 不存在；quality_issue.rs model 只有 status 字段无 8D 字段；quality_inspection_service.rs 无 D0~D8 实现
- **修复方案**：实现 D0~D8 八步流程：
  - D0 准备阶段 / D1 组队 / D2 描述问题 / D3 临时措施 / D4 根因分析 / D5 永久措施 / D6 实施 / D7 预防 / D8 表彰
  - quality_issue 表新增 8D 字段，状态机扩展为 11 态
- **关联文件**：quality_issue_service.rs / quality_issue_handler.rs / schema migrations
- **依赖**：无
- **工作量**：XL
- **批次**：480

#### P0-F21 返工未走生产订单（类二十一）

- **来源**：batch-18 P0-18-2
- **证据**：rework_service.rs 不存在，仅 dye_batch_rework.rs model 存在
- **修复方案**：返工必须创建 production_order（type='rework'），关联原 dye_batch，扣减库存
- **关联文件**：[rework_service.rs](file:///workspace/backend/src/services/rework_service.rs) / production_order_service.rs
- **依赖**：P0-F18（模块 C 的返工逻辑）
- **工作量**：M
- **批次**：479（与 F18 合并）

---

### 4.5 模块 E：财务与业务流程（17 项独立项 + 13 项归并）

#### P0-B01 坏账准备计提功能缺失（类十七）

- **来源**：batch-15 P0-15-1
- **证据**：bad_debt_service.rs 不存在；全代码库无 bad_debt/坏账 关键字匹配
- **修复方案**：实现坏账准备计提（账龄法：1年内 5% / 1-2年 20% / 2-3年 50% / 3年以上 100%），月末 cron 自动计提
- **关联文件**：bad_debt_service.rs / schema migrations / cron
- **依赖**：无
- **工作量**：L
- **批次**：481（合并 B01+B02+B03+B04 为单批 ~12 文件）

#### P0-B02 坏账核销与审批流缺失（类十七）

- **来源**：batch-15 P0-15-2
- **修复方案**：实现坏账核销二级审批（申请人→财务经理→总经理），核销后更新 ar_balance
- **关联文件**：bad_debt_service.rs / approval_service.rs
- **依赖**：P0-B01
- **工作量**：M
- **批次**：481（与 B01/B03/B04 合并）

#### P0-B03 催收任务管理缺失（类十七）

- **来源**：batch-15 P0-15-3
- **修复方案**：新增 collection_task 表，按账龄自动生成催收任务，分配给销售员，记录催收结果
- **关联文件**：collection_task_service.rs / collection_task_handler.rs / schema migrations
- **依赖**：无（可与 B01 并行设计）
- **工作量**：M
- **批次**：481（与 B01/B02/B04 合并）

#### P0-B04 财务预警机制缺失（类十七）

- **来源**：batch-15 P0-15-4
- **修复方案**：实现财务预警（应收超额 / 库存积压 / 现金流不足 / 预算超支 4 类），触发通知
- **关联文件**：finance_alert_service.rs / notification_service.rs
- **依赖**：无
- **工作量**：L
- **批次**：481（与 B01/B02/B03 合并）

#### P0-B05 大额调拨无额外验证（类十七）

- **来源**：batch-15 P0-15-5
- **修复方案**：资金调拨金额 > 阈值（如 10 万）需二级审批 + 短信验证码
- **关联文件**：fund_transfer_service.rs / approval_service.rs
- **依赖**：无
- **工作量**：S
- **批次**：482（合并 B05+B06+B07+B08+B09+B14 为单批 ~10 文件）

#### P0-B06 预算超支无拦截（类十七）

- **来源**：batch-15 P0-15-6
- **修复方案**：费用报销 / 采购订单创建时校验预算余额，超支拦截
- **关联文件**：budget_service.rs / expense_service.rs / purchase_order_service.rs
- **依赖**：无
- **工作量**：M
- **批次**：482（与 B05/B07/B08/B09/B14 合并）

#### P0-B07 回收规则无自动执行（类十七）

- **来源**：batch-15 P0-15-7
- **修复方案**：CRM 客户回收规则（30 天未联系 / 90 天无商机）自动执行，客户转入公海
- **关联文件**：customer_pool_service.rs / cron
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B08/B09/B14 合并）

#### P0-B08 赢率手填无自动计算（类十七）

- **来源**：batch-15 P0-15-8
- **修复方案**：商机赢率按阶段自动计算（prospecting 10% / qualification 25% / proposal 50% / negotiation 75% / closed_won 100%）
- **关联文件**：crm_opportunity_service.rs
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B07/B09/B14 合并）

#### P0-B09 输单原因未记录（类十七）

- **来源**：batch-15 P0-15-9
- **修复方案**：商机 closed_lost 时必填输单原因（价格/质量/服务/竞争对手/其他）
- **关联文件**：crm_opportunity_service.rs / 前端 opportunity_form.vue
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B07/B08/B14 合并）

#### P0-B10 BI 无数据权限过滤（类十九）

- **来源**：batch-16 P0-16-2
- **证据**：bi_analysis_service.rs 全文无 apply_data_scope 调用，使用 raw SQL 直接查询无权限过滤
- **修复方案**：BI 查询注入 apply_data_scope，按 user_id / department_id 过滤
- **关联文件**：bi_analysis_service.rs / dashboard_service.rs
- **依赖**：无
- **工作量**：M
- **批次**：483（合并 B10+B11+B12+B13 为单批 ~11 文件）

#### P0-B11 定制订单流程缺失打样和报价环节（类二十三）

- **来源**：batch-19 P0-19-1
- **修复方案**：定制订单流程补齐：需求确认 → 打样 → 客户确认 → 报价 → 生产订单
- **关联文件**：custom_order_service.rs / sample_service.rs / quotation_service.rs
- **依赖**：无
- **工作量**：L
- **批次**：483（与 B10/B12/B13 合并）

#### P0-B12 售后与质量集成完全缺失（类二十三）

- **来源**：batch-19 P0-19-2
- **证据**：after_sales 表无 quality_issue_id 关联
- **修复方案**：after_sales 表新增 quality_issue_id 字段，售后工单触发 8D 流程
- **关联文件**：after_sales_service.rs / quality_issue_service.rs / schema migrations
- **依赖**：P0-F20（8D 流程先行）
- **工作量**：M
- **批次**：483（与 B10/B11/B13 合并）

#### P0-B13 物流签收无电子签收单（类二十三）

- **来源**：batch-19 P0-19-3
- **修复方案**：
  1. 新增 electronic_signature 表（签收人/签收时间/签收图片/GPS 定位）
  2. 签收触发应收确认（ar_balance 增加 + 凭证生成）
- **关联文件**：logistics_service.rs / ar_service.rs / schema migrations
- **依赖**：无
- **工作量**：M
- **批次**：483（与 B10/B11/B12 合并）

#### P0-B14 Incoterms 2020 仅支持 5 种（类二十三）

- **来源**：batch-19 P0-19-4
- **证据**：当前仅 EXW/FOB/CIF/DAT/DDP
- **修复方案**：补齐 6 种（FCA/CPT/CIP/DPU/FAS/CFR），共 11 种
- **关联文件**：[incoterms.rs](file:///workspace/backend/src/models/incoterms.rs) / incoterms_service.rs / 前端选项
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B07/B08/B09 合并）

#### P0-B15 缺料预警状态不持久化（类二十二）

- **来源**：batch-18 P0-18-3
- **证据**：缺料预警仅内存计算，无法形成处理闭环
- **修复方案**：新增 material_shortage_alert 表（持久化预警记录 + 处理状态 + 责任人 + 月报）
- **关联文件**：material_shortage_service.rs / schema migrations
- **依赖**：无
- **工作量**：M
- **批次**：484（合并 B15+B16+B17 为单批 ~9 文件）

#### P0-B16 自动故障检测机制缺失（类二十）

- **来源**：batch-17 P0-17-1
- **证据**：health_check_service.rs 不存在；只有 health_handler.rs 单次健康检查接口；无 5s 间隔 + 3 次失败告警的 scheduler/cron/loop/sleep 逻辑
- **修复方案**：实现自动故障检测（5s 间隔 / 连续 3 次失败触发告警）
- **关联文件**：[health_check_service.rs](file:///workspace/backend/src/observability/health_check_service.rs)
- **依赖**：无
- **工作量**：M
- **批次**：484（与 B15/B17 合并）

#### P0-B17 主备切换自动完成缺失（类二十，部分实现）

- **来源**：batch-17 P0-17-2
- **复审状态**：⚠️ failover_service.rs 基础框架存在，仅事件记录/手动切换；缺自动心跳检测/VIP 漂移/10s 内自动完成
- **修复方案**：主备切换 10s 内自动完成（心跳检测 + VIP 漂移 + 数据同步）
- **关联文件**：failover_service.rs / deploy/ha/
- **依赖**：无
- **工作量**：L
- **批次**：484（与 B15/B16 合并）

#### P0-B18~B30 归并项（13 项，不计入独立项）

- P0-B18 自动故障检测 → 归并到 P0-B16
- P0-B19 报表订阅后台调度 → 归并到 P0-D16
- P0-B20 BI 数据权限过滤 → 归并到 P0-B10
- P0-B21 缺料预警状态持久化 → 归并到 P0-B15
- P0-B22 自动故障检测 → 归并到 P0-B16
- P0-B23 主备切换 → 归并到 P0-B17
- P0-B24 大货批色 ship_order 校验 → 归并到 P0-F19
- P0-B25 售后与质量集成 → 归并到 P0-B12
- P0-B26 物流签收 → 归并到 P0-B13
- P0-B27 Incoterms 补齐 → 归并到 P0-B14
- P0-B28 定制订单打样报价 → 归并到 P0-B11
- P0-B29 报表订阅后台调度 → 归并到 P0-D16
- P0-B30 BI 数据权限过滤 → 归并到 P0-B10

---

### 4.6 模块 F：测试体系（6 项）

#### P0-T01 核心 service 零单元测试（类六）

- **来源**：batch-06 P0-06-1
- **证据**：backend/tests/quotation_service_test.rs 不存在；quotation_service.rs 内部无 cfg(test) 模块
- **修复方案**：为两个 service 编写完整单元测试（覆盖率 ≥80%），抽取 mock 数据到 fixtures
- **关联文件**：backend/tests/quotation_service_test.rs / purchase_receipt_service_test.rs / tests/fixtures/
- **依赖**：无
- **工作量**：L
- **批次**：485（合并 T01+T03+T06+T08 为单批 ~12 文件）

#### P0-T02 7 项关键业务路径无集成测试（类六）

- **来源**：batch-06 P0-06-2
- **修复方案**：编写 7 项集成测试：生产订单→染色→质检→入库 / 采购订单→收货→付款 / 销售订单→发货→收款 / 染整全流程 / 化验室打样 / 大货处方 / 库存调拨
- **关联文件**：backend/tests/integration/
- **依赖**：P0-T01（单测先行）
- **工作量**：XL
- **批次**：486（与 T05/T07 合并）

#### P0-T03 CI baseline 机制掩盖编译失败（类六）

- **来源**：batch-06 P0-06-3
- **证据**：.github/workflows/ci-cd.yml 仍保留 clippy baseline 机制 line 354-605 和 cargo test baseline 机制 line 897-1031
- **修复方案**：移除 baseline 机制，所有失败必须真实修复
- **关联文件**：[.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml) / backend/tests/bi_analysis_test.rs
- **依赖**：无
- **工作量**：M
- **批次**：485（与 T01/T06/T08 合并）

#### P0-T05 E2E 通过率 0%（类六）

- **来源**：batch-06 P0-06-5
- **证据**：95 个 E2E 测试 88 个失败
- **修复方案**：逐个修复 E2E 失败用例，目标通过率 ≥90%
- **关联文件**：frontend/e2e/specs/
- **依赖**：无
- **工作量**：XL
- **批次**：486（与 T02/T07 合并）

#### P0-T06 bi_analysis_test.rs 16 个测试 API 脱节（类六）

- **来源**：batch-06 P0-06-6
- **修复方案**：更新 16 个测试用例的 API 调用，与源码对齐
- **关联文件**：backend/tests/bi_analysis_test.rs
- **依赖**：无
- **工作量**：M
- **批次**：485（与 T01/T03/T08 合并）

#### P0-T07 4 项关键 service 性能基准测试缺失（类六）

- **来源**：batch-06 P0-06-7
- **修复方案**：为 inventory_stock_service / sales_order_service / dye_batch_service / report_service 编写性能基准测试（P95 ≤2s）
- **关联文件**：backend/tests/bench/
- **依赖**：无
- **工作量**：M
- **批次**：486（与 T02/T05 合并）

#### P0-T08 CI 不集成覆盖率工具（类六）

- **来源**：batch-06 P0-06-8
- **修复方案**：CI 新增 `cargo tarpaulin` 步骤，上传 codecov；前端新增 `vitest --coverage`
- **关联文件**：.github/workflows/ci-cd.yml / codecov.yml
- **依赖**：无
- **工作量**：S
- **批次**：485（与 T01/T03/T06 合并）

---

### 4.7 模块 G：部署与运维（17 项，D01 部分实现）

#### P0-D01 Docker 文件违规（类七，部分实现）

- **来源**：batch-07 P0-07-1
- **复审状态**：⚠️ 3/4 文件已删除，剩 Dockerfile / docker-compose.yml / .dockerignore 3 个
- **修复方案**：删除剩余 3 个 Docker 文件
- **关联文件**：项目根 / deploy/ 下的 Docker 文件
- **依赖**：无
- **工作量**：S
- **批次**：487（合并 D01+D02+D05+D07 为单批 ~10 文件）

#### P0-D02 快速部署脚本安装 PostgreSQL 客户端（类七）

- **来源**：batch-07 P0-07-2
- **证据**：[install.sh](file:///workspace/deploy/install.sh) 安装 postgresql-client
- **修复方案**：移除 postgresql-client 安装步骤（数据库连接走远程模式）
- **关联文件**：deploy/install.sh / deploy/deploy.sh
- **依赖**：无
- **工作量**：S
- **批次**：487（与 D01/D05/D07 合并）

#### P0-D03 5 个 service 全部未接入缓存层（类七）

- **来源**：batch-07 P0-07-3
- **证据**：user_service.rs:67 注释说"命中 Redis 时直接返回缓存"但 find_by_id 方法实际无 Redis 调用；product/customer/supplier/role_service.rs 均无 redis/Redis/cache/moka 关键字。cache_service.rs 使用 moka 进程内缓存非 Redis，且 5 个 service 未使用
- **修复方案**：5 个 service 接入 Redis 缓存（5min TTL + 主动失效）
- **关联文件**：user_service.rs / product_service.rs / customer_service.rs / supplier_service.rs / role_service.rs
- **依赖**：无
- **工作量**：L
- **批次**：489（合并 D03+D04+D11+D12 为单批 ~10 文件）

#### P0-D04 缓存是内存缓存(moka)非 Redis（类七）

- **来源**：batch-07 P0-07-4
- **修复方案**：将 moka 内存缓存迁移到 Redis（多实例共享 + 持久化）
- **关联文件**：[cache.rs](file:///workspace/backend/src/utils/cache.rs) + 所有使用 moka 的 service
- **依赖**：P0-D03
- **工作量**：L
- **批次**：489（与 D03/D11/D12 合并）

#### P0-D05 useI18n 接入率仅 3.2%（类七）

- **来源**：batch-07 P0-07-5
- **修复方案**：85+ 视图组件全部接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts
- **关联文件**：frontend/src/views/ + locales/
- **依赖**：无
- **工作量**：XL
- **批次**：487（与 D01/D02/D07 合并）

#### P0-D06 aria-label 严重不足（类七）

- **来源**：batch-07 P0-07-6
- **证据**：仅 2 个文件 8 处 aria-label
- **修复方案**：所有交互元素补 aria-label（WCAG 2.1 AA）
- **关联文件**：所有 .vue 文件
- **依赖**：无
- **工作量**：XL
- **批次**：488（合并 D06+D08+D09+D10 为单批 ~12 文件）

#### P0-D07 图片 alt 属性完全缺失（类七）

- **来源**：batch-07 P0-07-7
- **证据**：0 处 alt 属性
- **修复方案**：所有 `<img>` 补 alt 描述
- **关联文件**：所有 .vue 文件
- **依赖**：无
- **工作量**：S
- **批次**：487（与 D01/D02/D05 合并）

#### P0-D08 130+ 超长函数（类七）

- **来源**：batch-07 P0-07-8
- **证据**：event_bus.rs:412 start_event_listener 函数从 line 412 延续到 line 997，长度约 586 行，超长函数仍存在
- **修复方案**：拆分超长函数为单一职责小函数（每个 ≤50 行）
- **关联文件**：event_bus.rs / ar_service.rs（1972 行）/ business_mode_service.rs / 等 26 个 >1000 行的文件
- **依赖**：无
- **工作量**：XL
- **批次**：488（与 D06/D09/D10 合并）

#### P0-D09 30+ 函数超过 100 行（类二）

- **来源**：batch-02 P0-02-01
- **修复方案**：拆分为 ≤50 行小函数
- **关联文件**：同 P0-D08
- **依赖**：P0-D08
- **工作量**：L
- **批次**：488（与 D06/D08/D10 合并）

#### P0-D10 26 个后端文件超过 1000 行（类二）

- **来源**：batch-02 P0-02-02
- **修复方案**：按职责拆分为多个文件（如 ar_service.rs 拆分为 ar_service / ar_aging_service / ar_collection_service）
- **关联文件**：26 个 >1000 行的文件
- **依赖**：P0-D09
- **工作量**：L
- **批次**：488（与 D06/D08/D09 合并）

#### P0-D11 setup_test_db 在 14 个文件重复定义（类二）

- **来源**：batch-02 P0-02-03
- **修复方案**：抽取到 backend/tests/common/mod.rs，所有测试文件引用
- **关联文件**：backend/tests/common/mod.rs + 14 个测试文件
- **依赖**：无
- **工作量**：M
- **批次**：489（与 D03/D04/D12 合并）

#### P0-D12 8 个函数圈复杂度 >15（类二）

- **来源**：batch-02 P0-02-04
- **修复方案**：重构降低复杂度（如 business_mode_service.rs:179 check_module_consistency ~35 → 拆分为多个 match 分支函数）
- **关联文件**：business_mode_service.rs / 等 8 个文件
- **依赖**：无
- **工作量**：M
- **批次**：489（与 D03/D04/D11 合并）

#### P0-D13 前端 60+ 组件缩写命名（类二）

- **来源**：batch-02 P0-02-05
- **修复方案**：重命名为描述性名称（如 SOList → SalesOrderList）
- **关联文件**：60+ .vue 文件
- **依赖**：无
- **工作量**：XL
- **批次**：490（合并 D13+D14+D15+D16+D17 为单批 ~12 文件）

#### P0-D14 前端 api 命名不统一（类二）

- **来源**：batch-02 P0-02-06
- **修复方案**：统一为 `getXxxList / createXxx / updateXxx / deleteXxx` 命名
- **关联文件**：90+ api/*.ts 文件
- **依赖**：无
- **工作量**：XL
- **批次**：490（与 D13/D15/D16/D17 合并）

#### P0-D15 升级流程非零停机（类二十五）

- **来源**：batch-21 P0-21-1
- **证据**：[upgrade.sh](file:///workspace/deploy/upgrade.sh) `systemctl stop` 导致 2-5s 服务中断
- **修复方案**：改为蓝绿部署 / 滚动重启，使用 systemctl reload nginx + 双实例切换
- **关联文件**：deploy/upgrade.sh / deploy/deploy.sh
- **依赖**：无
- **工作量**：M
- **批次**：490（与 D13/D14/D16/D17 合并）

#### P0-D16 报表订阅无后台调度任务（类十九）

- **来源**：batch-16 P0-16-1
- **证据**：report_subscription 表有 next_run_at 字段但无 cron 任务触发
- **修复方案**：新增 report_subscription_scheduler_service，每分钟扫描 next_run_at 到期的订阅，生成报表并发送通知
- **关联文件**：report_subscription_scheduler_service.rs / main.rs（启动 cron）
- **依赖**：无
- **工作量**：M
- **批次**：490（与 D13/D14/D15/D17 合并）

#### P0-D17 OA 公告完全未实现（类十九）

- **来源**：batch-16 P0-16-3
- **证据**：oa_announcement 仅有 Model，无 service/handler/路由
- **修复方案**：实现 oa_announcement_service / handler / 路由（CRUD + 可见性 + 权限）
- **关联文件**：oa_announcement_service.rs / oa_announcement_handler.rs / routes/
- **依赖**：无
- **工作量**：M
- **批次**：490（与 D13/D14/D15/D16 合并）

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
