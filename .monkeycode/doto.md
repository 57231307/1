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

> **v13 复审报告**（2026-07-13，规则 15 启动）：v12 复审全部完成后启动，新增"业务场景闭环""财务场景闭环""运行逻辑环流程闭环"等复审维度。
> **核心目标**：baseline 213 条摘要行（~993 个警告）全部清零 + 业务/财务/逻辑闭环问题修复。
> **执行策略**：规则 13+14+15 联动，复审完成后自动连续修复，每批 5-6 文件，CI 全绿后自动进入下一批，无需用户确认。

### 进度总览

| 维度 | 总数 | 已完成 | 剩余 | 状态 |
|------|------|--------|------|------|
| 🟢 baseline 警告清零 | 213 摘要 / 89 位置 / 135 文件 | 0 | 213 | ⏳ 待修复 |
| 🟢 业务场景闭环 | 待扫描 | 0 | 待扫描 | ⏳ 待复审 |
| 🟢 财务场景闭环 | 待扫描 | 0 | 待扫描 | ⏳ 待复审 |
| 🟢 运行逻辑环流程闭环 | 待扫描 | 0 | 待扫描 | ⏳ 待复审 |
| 🟢 异常路径闭环 | 待扫描 | 0 | 待扫描 | ⏳ 待复审 |
| 🟢 状态机闭环 | 待扫描 | 0 | 待扫描 | ⏳ 待复审 |
| 🟢 资源生命周期闭环 | 待扫描 | 0 | 待扫描 | ⏳ 待复审 |
| 🟢 配置依赖闭环 | 待扫描 | 0 | 待扫描 | ⏳ 待复审 |

### baseline 警告分类（v13 复审扫描结果）

| 类型 | 数量 | 说明 |
|------|------|------|
| dead_code | ~193 | associated function/struct/enum/variant/field never used/read/constructed |
| unused_import | 15 | 未使用的导入 |
| needless_borrow/asref | 1 | 不必要的借用/引用 |
| too_many_arguments (8/7) | 1 | 函数参数过多 |
| unreachable_code | 1 | 不可达代码 |
| unused_variable | 1 | 未使用的变量 |
| unused_mut | 1 | 不需要的 mut |

### 警告最多的文件（按 `-->` 位置行统计，89 条位置行）

| 文件 | 警告数 |
|------|--------|
| tests/bi_analysis_test.rs | 18 |
| src/services/bi_analysis_service.rs | 11 |
| src/services/auth/password_policy_service.rs | 3 |
| src/search/elastic.rs | 3 |
| src/websocket/notifications.rs | 2 |
| src/utils/failover/cache.rs | 2 |
| src/services/p9_5_sales_extra_tests.rs | 2 |
| src/services/p9_5_inventory_extra_tests.rs | 2 |
| src/services/failover_service.rs | 2 |
| src/services/ap_payment_service.rs | 2 |
| src/middleware/auth_context.rs | 2 |
| src/handlers/ap_payment_handler.rs | 2 |
| 其余 48 个文件 | 各 1 |

### 修复批次记录

（待填充，按规则 13 连续执行）

---

## 🔄 历史任务：v14 深度调研报告修复（高风险 6/6 ✅，中风险 22/25 🔄）

> **v14 深度调研报告**（2026-07-09，[bug.md](file:///workspace/.monkeycode/bug.md)）：12 维度全量扫描，15 高/25 中/74 低风险，共 114 个问题。
> v13 后端 P0/P1 全部完成（批次 229-236），v13 剩余 P2 任务合并到 v14 队列。
> 修复策略：按优先级（高→中→低）+ 影响范围（核心路径→边缘功能）排序，每批 1 commit，CI 全绿后合并 main。

### 进度总览

| 风险等级 | 总数 | 已完成 | 剩余 | 状态 |
|----------|------|--------|------|------|
| 🔴 高风险 | 6 | 6 | 0 | ✅ 全部完成（批次 237-242） |
| 🟡 中风险 | 25 | 22 | 3 | 🔄 进行中 |
| 🟢 低风险 | 74 | 0 | 74 | ⏳ 后续迭代 |
| **合计** | **114** | **28** | **86** | — |

### 🟡 中风险待修复项（3 项 ⏳）

#### 1. 测试覆盖（7 项 ⏳ 待修复）

**问题背景**：bug.md 中风险测试覆盖问题 — 项目测试覆盖率严重不足，关键模块零测试或低覆盖，无法保证代码质量和重构安全性。

**影响范围**：
- **handlers 层**：100+ 文件覆盖率仅 10%，大部分 handler 无单元测试
- **services 层**：107 个 service 无任何测试，业务逻辑错误只能在运行时发现
- **frontend api 层**：覆盖率 4.4%，前端 API 调用逻辑无测试保障
- **ai 算法层**：零测试，AI 相关算法（RFM 评分、预测、推荐等）的正确性无验证
- **store 层**：覆盖率低，状态管理逻辑无测试
- **middleware 层**：覆盖率低（permission.rs 已在批次 240 补测 23 个，其余 middleware 仍无测试）

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

#### 2. view 表格逻辑接入 useTableApi（46/56 完成 🔄，剩余 10 个 ⏳）

**问题背景**：56 个前端 view 文件各自实现表格加载/分页/排序/查询逻辑，与已封装的 `useTableApi` composable 重复。

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

### 🟢 低风险修复队列（74 项 ⏳ 后续迭代）

**占位符/Mock 存根（21 项）**：
- 问题描述：21 处占位符或 Mock 存根，多数为测试夹具或合理设计
- 修复方案：逐个评估，合理保留的加注释说明，不合理的真实实现
- 优先级：低（多数无需修复）

**项目规则符合性（11 项）**：
- 问题描述：11 处配置层默认值或 best-effort 合理模式
- 修复方案：评估是否符合规则 0-13，不符合的修正
- 优先级：低

**死代码（8 项）**：
- 问题描述：8 处合规标注的死代码（`#[allow(dead_code)]` + TODO）
- 修复方案：逐个评估是否接入业务或删除
- 优先级：低

**其他（34 项）**：
- 问题描述：34 处其他低风险问题（命名规范/注释完善/代码风格等）
- 修复方案：后续迭代统一处理
- 优先级：低

### 📋 合并到 v14 的历史遗留任务（⏳ 待修复）

**v13 前端 P2（3 项 ⏳）**：
- FE-P2-1：前端类型定义完善（any 类型清理已完成，剩余 unknown 类型细化）
- FE-P2-2：前端组件 props 类型强化
- FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）— 大量 view 未接入 i18n，硬编码中文文本

**v13 后端 P2（3 项 ⏳）**：
- P2-1：后端错误处理统一（部分 handler 仍直接返回字符串而非 AppError）
- P2-2：后端日志规范（部分模块日志级别不当）
- P2-3：后端配置项完善

**其他遗留（3 项 ⏳）**：
- FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）— 引入 `el-table-v2` 或 `vue-virtual-scroller`
- P2-8：剩余 143 个无测试 service（后续迭代）— 分批补测
- E2E 失败排查（已知问题，待规则 5 节点）— 下载 playwright-report 分析失败用例

---

## 规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 330 已到期需触发（403 权限不足，需用户手动触发）
  - 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch
  - 批次 N+20：第 1 次监控（GitHub API 查询 run 状态）
  - 批次 N+28：第 2 次监控（若 N+20 未完成）
  - 批次 N+29：最后监控，未完成则跳过 N+30 的 E2E 周期
  - **注意**：E2E 已从 ci-cd.yml 独立到 e2e-batch.yml，不阻塞主 CI
- **规则 10（每 15 批次记忆整理）**：2026-07-13 v13 复审前提前执行（归档 v8-v12 历史任务 + 安全漏洞表）
  - 下次整理：批次 360
- **规则 13（修复流程自动化与连续执行）**：CI 全绿后自动开始下一批，无需用户确认
- **规则 14（移除所有警告抑制）**：所有警告视为错误需修复
- **规则 15（v13 复审严格规范 + 业务/财务场景闭环 + 运行逻辑环流程闭环）**：2026-07-13 新增
