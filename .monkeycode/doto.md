# 未完成任务（详细）

> 本文件**详细**记录未完成的任务（问题描述、影响范围、修复方案、技术要点），禁止简化。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近一次整理：2026-07-12（批次 345 规则 10 整理，已完成批次 330-344 迁移到 doto-su.md）。

---

## ✅ 历史任务：v8/v9 复审问题修复（全部完成）

- **v8 复审**（批次 290-308）：21 项问题（4 高 + 8 中 + 9 低）全部修复，详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。
- **v9 复审**（批次 317-323）：16 项问题（2 P0 + 2 高 + 5 中 + 7 低）全部修复，详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。
- **sea-orm 版本调研**（批次 324）：确认使用 1.1.20 稳定版正确，2.0 仍 RC 不升级。
- **规则 14 新增**（批次 324）：移除所有警告抑制，clippy baseline 渐进清理。

---

## ✅ 历史任务：v10 复审问题修复（P0 1/1 ✅，P1 5/5 ✅，P2 4/4 ✅，P3 43/43 ✅ 全部完成）

> **v10 复审报告**（2026-07-12，Task 工具扫描）：v9 + sea-orm 调研 + 规则 14 新增后复审，扫描所有 `#[allow(...)]` 警告抑制。
> 发现 180 个抑制标注（108 例外 models/ + 72 非例外），非例外分类：1 P0 + 5 P1 + 4 P2 + ~43 P3。
> **v10 复审全部完成**（批次 325-339）：所有 `#[allow(clippy::too_many_arguments)]` 抑制已全部移除，规则 14 合规。
> 批次 325-344 详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。

### 进度总览

| 优先级 | 总数 | 已完成 | 状态 |
|--------|------|--------|------|
| 🔴 P0 死代码 | 1 | 1 | ✅ 全部完成（批次 325） |
| 🟠 P1 文件级抑制过宽+未使用重导出 | 5 | 5 | ✅ 全部完成（批次 325） |
| 🟡 P2 clippy 代码味道 | 4 | 4 | ✅ 全部完成（批次 326） |
| 🟢 P3 too_many_arguments | 43 | 43 | ✅ 全部完成（批次 327-339，含 9 项误报删除） |

### 修复方案与技术要点

- 引入参数对象（Parameter Object）重构模式，将相关参数分组为 struct
- 含借用参数（&mut Vec / &mut HashMap / &str）的函数：标量参数聚合为参数对象，借用参数保留在签名中
- 每批次处理 5-6 个文件，优先处理 service 层（业务逻辑核心），再处理 handler 层

---

## ✅ 历史任务：v11 复审问题修复（P0 1/1 ✅，P1 8/8 ✅，P2 10/10 ✅，P3 8/8 ✅ 全部完成）

> **v11 复审报告**（2026-07-12，批次 339 合并后 Task 工具扫描）：v10 复审全部完成后复审，扫描所有剩余 `#[allow(...)]` 警告抑制（非 models/ SeaORM 例外）。
> 发现 27 个抑制标注：1 P0 + 8 P1 + 10 P2（带 TODO 保留）+ 8 P3（合理保留）。
> **v11 复审全部完成**（批次 340-346）：所有可修复的 `#[allow]` 抑制已全部移除，规则 14 合规。
> 批次 340-344 详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。

### 进度总览

| 优先级 | 总数 | 已完成 | 状态 |
|--------|------|--------|------|
| 🔴 P0 文件级抑制超出例外 | 1 | 1 | ✅ 全部完成（批次 340） |
| 🟠 P1 clippy 警告抑制 | 8 | 8 | ✅ 全部完成（批次 340+344+346） |
| 🟡 P2 带 TODO 的 dead_code | 10 | 10 | ✅ 全部完成（批次 340-342+345） |
| 🟢 P3 测试代码/防御性抑制 | 8 | 8 | ✅ 全部完成（批次 342+343） |
| **合计** | **27** | **27** | ✅ v11 复审全部完成 |

> 批次 340-346 详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

---

## ✅ 历史任务：v12 复审问题修复（P2 死代码 8/8 ✅，P1 4/4 ✅，P3 3/3 ✅ 全部完成）

> **v12 复审报告**（2026-07-12，批次 346 合并后 Task 工具扫描）：v11 复审全部完成后复审，扫描死代码、unwrap/expect/panic 使用、baseline 渐进清理。
> 发现 15 个问题：0 P0 + 4 P1 + 8 P2 + 3 P3。
> 关键结论：`#[allow]` 抑制彻底清除 ✅；生产代码无 panic 风险 ✅；baseline 部分过时（5 项已接入业务但 baseline 未更新）。
> 修复策略：按规则 13+14 连续执行，P2 → P1 → P3，每批 5-6 个文件，CI 全绿后合并 main。
> **v12 复审全部完成**（批次 347-355）。

### 进度总览

| 优先级 | 总数 | 已完成 | 剩余 | 状态 |
|--------|------|--------|------|------|
| 🟡 P2 死代码 | 8 | 8 | 0 | ✅ 全部完成（批次 347-350） |
| 🟠 P1 clippy baseline 风格警告 | 4 | 4 | 0 | ✅ 全部完成（批次 351-355） |
| 🟢 P3 测试代码/业务术语 | 3 | 3 | 0 | ✅ 全部完成（批次 355） |
| **合计** | **15** | **15** | **0** | ✅ 全部完成 |

### ✅ 已完成（批次 347-350）

**批次 347（PR #519）**：v12 复审 P2 死代码清理 4 项（4 文件，utils/ 函数级删除）
- `unwrap_safe.rs`：删除 `must_some` + `must_ok` 函数及 4 个对应测试（pub 函数仅自身测试调用）
- `hash.rs`：删除 `sha256_hex_multi` 函数（整个项目无调用）
- `color_space_converter.rs`：删除 `rgb_to_hex` + `delta_e_76` 函数及对应测试，新增 `test_delta_e_is_acceptable` 替代
- `process_state_machine.rs`：删除 `node_type_to_status` 函数及对应测试
- 规则 0+14 合规：删除死代码而非抑制，CI 13 success + 2 skipped 全绿

**批次 348（PR #520，commit 8aeeb231）**：v12 复审 P2-1+P2-2 死代码删除（5 文件，3 删 + 2 改）
- `services/ar_collection_service.rs`：删除（零外部引用，功能被 ArService 完全覆盖）
- `services/five_dimension_query_service.rs`：删除（零外部引用，功能被 FiveDimensionService 内联覆盖）
- `utils/fabric_five_dimension.rs`：连带删除（仅被已删除的 FiveDimensionQueryService 引用）
- `services/mod.rs` + `utils/mod.rs`：删除 3 个 pub mod 声明
- CI 13 success + 2 skipped 全绿

**批次 349（PR #521，commit d3ada6b8）**：v12 复审 P2-3 cleanup_expired_jti 接入定时任务（4 文件）
- `main.rs`：参照 cleanup_expired_admin_cache 模式接入 tokio::spawn 定时任务（间隔 3600 秒）
- `services/ar/mod.rs`：修正过时注释（ar_collection_service → ar_service）
- `services/ar_invoice_service.rs`：修正错误注释（ar_collection_service.confirm → ar_service.confirm_payment）
- `services/accounting_period_service.rs`：移除对已删除 ar_collection_service.rs:42 的引用
- CI 13 success + 2 skipped 全绿

**批次 350（PR #522，commit bfb2321d）**：v12 复审 P2-4 baseline 过时条目清理（1 文件，删 19 行）
- `backend/.clippy-baseline.txt`：删除 5 条摘要行 + 14 条非摘要行（对应 P2-1/P2-2/P2-3 已修复项）
- baseline 行数：1508 → 1489
- CI 13 success + 2 skipped 全绿，v12 复审 P2 全部完成（8/8）

### 🟠 P1 修复进度（4 项，4/4 ✅ 全部完成）

| ID | 警告类型 | 数量 | 状态 |
|----|----------|------|------|
| P1-1 | `clippy::too_many_arguments` | 2 | ✅ 全部完成（批次 352） |
| P1-2 | `clippy::useless_asref` | 1 | ✅ 全部完成（批次 351） |
| P1-3 | `unused_imports` | 23 | ✅ 全部完成（批次 351+353+354） |
| P1-4 | baseline 过时条目清理 | — | ✅ 全部完成（批次 355 删除 25 行 1482→1457） |

### 🟢 P3 修复进度（3 项，全部完成 ✅）

| ID | 警告类型 | 数量 | 状态 |
|----|----------|------|------|
| P3-1 | `upper_case_acronyms` | 7 | ✅ 全部完成（批次 355，Incoterms2020 + CustomerLevel 枚举变体重命名） |
| P3-2 | 测试代码合理保留 | 2 | ✅ 全部完成（批次 353-354 已清理测试模块 unused_imports） |
| P3-3 | 测试代码合理保留 | — | ✅ 全部完成（批次 354 inventory_stock_handler_query 测试模块 super::* 移除） |

**批次 355（PR #527，commit 33b4c24b，✅ 已合并）**：v12 复审 P1-4 baseline 清理 + P3 upper_case_acronyms 修复
- baseline 删除 25 行（18 条 P1 已修复摘要行 + 7 条 P3 摘要行）
- utils/incoterms.rs: Incoterms2020 枚举变体 FOB→Fob, CIF→Cif, EXW→Exw, DDP→Ddp, DAP→Dap
- services/quotation_pricing_service.rs: CustomerLevel 枚举变体 VIP→Vip, NORMAL→Normal
- tests/quotation_pricing_test.rs + tests/quotation_dto_serde_test.rs: 同步更新测试引用
- #[serde(rename_all = "UPPERCASE")] 保持序列化为大写，API 契约不变
- CI 初次失败（4 个新警告）→ 恢复 baseline 误删的 6 条历史摘要行（too_many_arguments(8/7), ActiveModelTrait, crate::middleware AuthContext, rust_decimal::prelude::*, tracing::info, self, super::*）→ CI 全绿
- CI 13 success + 2 skipped 全绿
- **v12 复审 P3 全部完成（3/3）✅**

### 🔴 P0 待修复项（0 项 ✅ 全部完成）

v11 复审 P0 已在批次 340 全部修复完成。

### 🟠 P1 剩余项（0 项 ✅ 全部完成）

P1 全部 8 项已完成：批次 340（P1-1~P1-5）+ 批次 344（P1-8 FromStr trait 迁移）+ 批次 346（P1-6+P1-7 宏 metavariable 修复）。

### 🟡 P2 剩余项（0 项 ✅ 全部完成）

P2 全部 10 项已完成：批次 340（P2-2~P2-5 dto/mod.rs + P2-10 crm/mod.rs）、批次 341（P2-6+P2-7 status.rs）、批次 342（P2-1 bpm_dto.rs + P2-9 user_notification_setting.rs）、批次 345（P2-8 app_state.rs Default 重构）。

### 🟢 P3 剩余项（0 项 ✅ 全部完成）

P3 全部 8 项已在批次 342（event_bus.rs unreachable_patterns）+ 批次 343（7 个测试模块 unused_imports）修复完成。

---

## 🔄 当前任务：v13 复审 + 运行逻辑环流程闭环（2026-07-13 启动）

> **v13 复审报告**（2026-07-13，规则 15 启动）：v12 复审全部完成后启动，新增"运行逻辑环流程闭环"复审维度。
> **核心目标**：baseline 213 条摘要行（~993 个警告）全部清零 + 运行逻辑闭环问题修复。
> **执行策略**：规则 13+14+15 联动，复审完成后自动连续修复，每批 5-6 文件，CI 全绿后自动进入下一批，无需用户确认。

### 进度总览

| 维度 | 总数 | 已完成 | 剩余 | 状态 |
|------|------|--------|------|------|
| 🟢 baseline 警告清零 | 213 摘要 / ~993 警告 | 0 | 213 | ⏳ 待修复 |
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

### 警告最多的文件（按 `-->` 位置行统计，253 条位置行，135 个文件）

| 文件 | 警告数 |
|------|--------|
| tests/bi_analysis_test.rs | 18 |
| src/services/bi_analysis_service.rs | 11 |
| src/services/auth/password_policy_service.rs | 5 |
| src/utils/incoterms.rs | 5 |
| src/observability/config.rs | 4 |
| src/services/ar/mod.rs | 4 |
| src/services/p9_5_inventory_extra_tests.rs | 4 |
| src/handlers/sales_fabric_order_handler.rs | 4 |
| src/handlers/warehouse_handler.rs | 4 |
| src/middleware/slow_query.rs | 4 |

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

---

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

**待修复文件清单**（部分示例，完整清单见 bug.md）：
- `backend/src/services/auth_service.rs`（核心，高优先级）
- `backend/src/services/user_service.rs`（核心，高优先级）
- `backend/src/services/order_service.rs`（核心业务）
- `backend/src/services/inventory_service.rs`（核心业务）
- `backend/src/services/ai/*.rs`（AI 算法，零测试）
- `backend/src/handlers/*.rs`（100+ 文件）
- `frontend/src/api/*.ts`（前端 API 层）
- `frontend/src/stores/*.ts`（状态管理）

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

#### 3. 重复实现 service 分页（35/35 全部清零 ✅）

**状态**：✅ 已全部完成（批次 255-266）。35 个 service 文件已全部接入 `paginate_with_total` 工具函数。详细记录见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

---

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

---

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

## 🐛 安全漏洞待修复（来自 bug.md）

> 详见 [bug.md](file:///workspace/.monkeycode/bug.md)。7 项安全漏洞已全部修复（批次 290-296）。

| 优先级 | 漏洞 | 位置 | 状态 |
|--------|------|------|------|
| P0 | 1.1 SQL 注入 (LIMIT) | tracking_service.rs:258-259 | ✅ 已修复（批次 290，PR #470） |
| P0 | 1.2 命令注入 (backup) | backup.rs:149 | ✅ 已修复（批次 291，PR #471） |
| P0 | 1.3 SSRF (currency) | currency_service.rs:301-305 | ✅ 已修复（批次 292，PR #472） |
| P1 | 2.1 日志泄露 | webhook_service.rs:235 | ✅ 已修复（批次 293，PR #473） |
| P1 | 2.2 速率限制 | webhook_handler.rs:114-135 | ✅ 已修复（批次 294，PR #474） |
| P1 | 2.3 文件权限 | system_update_service.rs:438 | ✅ 已修复（批次 295，PR #475） |
| P2 | 3.2 备份权限 | backup.rs:54-62 | ✅ 已修复（批次 296，PR #476） |

---

## ✅ v8 全项目复审（2026-07-11，批次 290-308 全部修复）

> 详见 [v8-review-2026-07-11.md](file:///workspace/.monkeycode/docs/audits/v8-review-2026-07-11.md)。复审发现 21 个问题（4 高 + 8 中 + 9 低），全部在批次 290-308 修复完成，详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

---

## 规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 270 触发（403 权限不足，需用户手动触发）
  - 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch
  - 批次 N+20：第 1 次监控（GitHub API 查询 run 状态）
  - 批次 N+28：第 2 次监控（若 N+20 未完成）
  - 批次 N+29：最后监控，未完成则跳过 N+30 的 E2E 周期
  - **注意**：E2E 已从 ci-cd.yml 独立到 e2e-batch.yml，不阻塞主 CI
  - **下次触发**：批次 330（已到期，需触发）
- **规则 10（每 15 批次记忆整理）**：批次 345 已执行（迁移批次 330-344 到 doto-su.md）
  - 下次整理：批次 360
