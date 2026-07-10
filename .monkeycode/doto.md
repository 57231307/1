# 未完成任务（详细）

> 本文件**详细**记录未完成的任务（问题描述、影响范围、修复方案、技术要点），禁止简化。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 🔄 当前任务：v14 深度调研报告修复（高风险 6/6 完成，中风险 20/25 完成）

> **v14 深度调研报告**（2026-07-09，[bug.md](file:///workspace/.monkeycode/bug.md)）：12 维度全量扫描，15 高/25 中/74 低风险，共 114 个问题。
> v13 后端 P0/P1 全部完成（批次 229-236），v13 剩余 P2 任务合并到 v14 队列。
> 修复策略：按优先级（高→中→低）+ 影响范围（核心路径→边缘功能）排序，每批 1 commit，CI 全绿后合并 main。

### v14 修复任务队列

#### 🔴 高风险修复队列（6 项，全部完成 ✅）

| 批次 | 编号 | 问题 | 状态 |
|------|------|------|------|
| 237 | P0-1 | 并发-async 阻塞（spawn_blocking 包装 Argon2id） | ✅ PR #414 |
| 238 | P0-2 | 性能-全表扫描（ar_service SQL 聚合） | ✅ PR #415 |
| 239 | P0-3 | 空实现-业务失效（handleView 只读模式） | ✅ PR #416 |
| 240 | P0-4 | 测试覆盖-安全核心（permission.rs 23 测试） | ✅ PR #417 |
| 241 | P0-5 | API 文档缺失（恢复 docs.rs + 删 openapi.rs） | ✅ PR #418 |
| 242 | P0-6 | 简化阉割-RFM 分布真实计算 | ✅ PR #419 |

#### 🟡 中风险修复队列（25 项，已完成 19/25 🔄）

**待修复项（8 项 ⏳）**：

##### 1. 测试覆盖（7 项 ⏳ 待修复）

**问题背景**：bug.md 中风险测试覆盖问题 — 项目测试覆盖率严重不足，关键模块零测试或低覆盖，无法保证代码质量和重构安全性。

**影响范围**：
- **handlers 层**：100+ 文件覆盖率仅 10%，大部分 handler 无单元测试，API 行为变更无法被测试捕获
- **services 层**：107 个 service 无任何测试，业务逻辑错误只能在运行时发现
- **frontend api 层**：覆盖率 4.4%，前端 API 调用逻辑无测试保障
- **ai 算法层**：零测试，AI 相关算法（RFM 评分、预测、推荐等）的正确性无验证
- **store 层**：覆盖率低，状态管理逻辑无测试
- **middleware 层**：覆盖率低（permission.rs 已在批次 240 补测 23 个，其余 middleware 仍无测试）
- **其他模块**：utils/handlers/services 子模块等需补测

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

##### 2. 重复实现（2 项 🔄 进行中）

**问题背景**：bug.md 中风险重复实现问题 — 项目中存在大量重复代码，违反 DRY 原则，维护成本高，修改时容易遗漏同步更新。

**子任务 2.1：service 分页逻辑接入 paginate_with_total（累计 30/35 完成，剩余 5/35 ⏳）**

**问题描述**：35 个 service 文件手写 `num_items + fetch_page` 分页逻辑，与已封装的 `paginate_with_total` 工具函数（`backend/src/utils/pagination.rs`）重复。手写逻辑存在不一致实现（部分未做 `saturating_sub(1)` 偏移、部分未做 `clamp` 防 DoS），且修改分页逻辑需逐个文件修改。

**已修复文件（30 个 ✅）**：
- 批次 255：`sales_price_service.rs` / `ap_invoice_service.rs` / `role_service.rs`（修复偏移 bug）/ `supplier_service.rs`
- 批次 256：`email_log_service.rs` / `email_template_service.rs` / `report_subscription_service.rs` / `report_template_service.rs`
- 批次 257：`currency_service.rs`（2 处）/ `mrp_engine_service.rs` / `production_order_service.rs` / `scheduling_query.rs`
- 批次 258：`purchase_receipt_service.rs` / `purchase_inspection_service.rs` / `purchase_return_service.rs` / `supplier_evaluation_service.rs`
- 批次 259：`ap_payment_request_service.rs` / `ap_payment_service.rs` / `ap_reconciliation_service.rs` / `ap_verification_service.rs`
- 批次 260：`po/order.rs` / `inventory_count_service.rs` / `inventory_adjustment_service.rs` / `finance_payment_service.rs`
- 批次 263：`inventory_stock_query.rs`（list_transactions + get_stock_by_product）/ `inventory_stock_service.rs`（list_stock）/ `custom_order_aftersales_service.rs` / `custom_order_crud_service.rs` / `custom_order_quality_service.rs`

**待修复文件（5 个 ⏳）**：
- `quotation_service.rs`（需解决 ServiceError 转换）
- `inventory_reservation_service.rs`（total 转 i64 + page 无偏移）
- `inventory_stock_query.rs` get_inventory_summary（聚合查询 + into_model + 结果映射，需特殊处理）
- `color_price_crud_service.rs` / `color_price_history_service.rs` / `color_price_seasonal_service.rs`（需错误转换 From<AppError>）
- `fixed_asset_service.rs` / `fund_management_service.rs`（流式拉取/无 total 返回，需评估是否适合接入）

**修复模式**（统一标准）：
```rust
// 修复前（手写分页）
let total = select.clone().count(&*self.db).await?;
let items = select
    .order_by_desc(Entity::Column::CreatedAt)
    .paginate(&*self.db, page_size)
    .fetch_page(page.saturating_sub(1))  // 部分文件遗漏此偏移
    .await?;
Ok((items, total))

// 修复后（接入 paginate_with_total）
use crate::utils::pagination::paginate_with_total;
let paginator = select
    .order_by_desc(Entity::Column::CreatedAt)
    .paginate(&*self.db, page_size);
let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
Ok((items, total))
```

**技术要点**：
- `paginate_with_total` 内部已做 `page.saturating_sub(1)` 偏移，调用方不可再减 1
- 删除独立 `select.clone().count()` 查询，复用 paginator 的 `num_items()`（减少一次 DB 查询）
- 统一补充 `page.clamp(1, 1000)` 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）
- `PaginatorTrait` 导入保留（`.paginate()` 方法需要）
- `quotation_service.rs` 特殊处理：返回类型是 `ServiceError` 而非 `AppError`，需添加 `From<AppError> for ServiceError` 转换或改用 `AppError`

**子任务 2.2：30+ view 表格逻辑接入 useTableApi（⏳ 待修复）**

**问题描述**：30+ 前端 view 文件各自实现表格加载/分页/排序/查询逻辑，与已封装的 `useTableApi` composable 重复。每个 view 重复编写 `loadData` / `handlePageChange` / `handleSortChange` / `handleSearch` 等函数，代码冗余严重。

**影响范围**：30+ view 文件，涉及所有业务模块（销售/采购/库存/财务/CRM 等）

**修复方案**：
- 扫描所有使用 `el-table` + 分页的 view 文件
- 评估每个 view 的特殊逻辑（如有自定义排序/筛选需保留）
- 接入 `useTableApi` composable，删除重复的表格逻辑代码
- 保持 view 的业务逻辑不变，只替换通用表格逻辑

**待修复文件清单**（需扫描识别，预计 30+ 文件）：
- `frontend/src/views/sales-*/index.vue`
- `frontend/src/views/purchase-*/index.vue`
- `frontend/src/views/inventory-*/index.vue`
- `frontend/src/views/finance-*/index.vue`
- `frontend/src/views/crm-*/index.vue`
- 其他使用 el-table + 分页的 view

**技术要点**：
- `useTableApi` 已封装：分页参数管理 / 数据加载 / loading 状态 / 错误处理
- 接入时需保留 view 特有的查询参数构建逻辑（如日期范围/多字段搜索）
- 部分 view 有自定义列配置/导出功能，需评估是否纳入 composable

**已完成项（16 项 ✅）**：
- 空实现（4 项 ✅）：批次 246 handleViewVersion + 批次 252 bi_analysis unreachable! + 批次 253 AdvancedFilter handleLogicChange
- 简化阉割（3 项 ✅）：批次 249 capacity + 批次 250 budget + 批次 251 webhook retry
- 死代码（1 项 ✅）：批次 254 composable eslint-disable any 清理
- 重复实现 service 分页（1 项 ✅）：批次 255-263（累计 30/35）
- 项目规则符合性（1 项 ✅）：批次 247 CLI 硬编码 URL
- 性能问题（5 项 ✅）：批次 244 ar 报表 + 批次 245 ap 报表 + 批次 248 缓存接入
- 安全漏洞（2 项 ✅）：批次 243 XSS + 输入验证
- E2E 失败排查（1 项 ✅）：批次 260 规则 5 检查发现 auth 配置缺失根因，批次 261 修复（AuthConfig serde(default) + PUBLIC_PATHS + CSRF 头），初始化步骤首次通过

#### 🟢 低风险修复队列（74 项 ⏳ 后续迭代）

**占位符/Mock 存根（21 项）**：
- 问题描述：21 处占位符或 Mock 存根，多数为测试夹具或合理设计
- 修复方案：逐个评估，合理保留的加注释说明，不合理的真实实现
- 影响范围：测试夹具 / 开发占位 / 配置默认值
- 优先级：低（多数无需修复）

**项目规则符合性（11 项）**：
- 问题描述：11 处配置层默认值或 best-effort 合理模式
- 修复方案：评估是否符合规则 0-12，不符合的修正
- 影响范围：配置文件 / 环境变量默认值
- 优先级：低

**死代码（8 项）**：
- 问题描述：8 处合规标注的死代码（`#[allow(dead_code)]` + TODO）
- 修复方案：逐个评估是否接入业务或删除
- 影响范围：utils / models / services 子模块
- 优先级：低

**其他（34 项）**：
- 问题描述：34 处其他低风险问题（命名规范/注释完善/代码风格等）
- 修复方案：后续迭代统一处理
- 优先级：低

#### 📋 合并到 v14 的历史遗留任务（⏳ 待修复）

**v13 前端 P2（3 项 ⏳）**：
- FE-P2-1：前端类型定义完善（any 类型清理已完成，剩余 unknown 类型细化）
- FE-P2-2：前端组件 props 类型强化
- FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）— 问题描述：大量 view 未接入 i18n，硬编码中文文本；修复方案：逐个 view 提取文本到 i18n locale 文件

**v13 后端 P2（3 项 ⏳）**：
- P2-1：后端错误处理统一（部分 handler 仍直接返回字符串而非 AppError）
- P2-2：后端日志规范（部分模块日志级别不当）
- P2-3：后端配置项完善

**其他遗留（3 项 ⏳）**：
- FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）— 问题描述：大量 el-table 未使用虚拟滚动，大数据量时性能差；修复方案：引入 `el-table-v2` 或 `vue-virtual-scroller`
- P2-8：剩余 143 个无测试 service（后续迭代）— 问题描述：143 个 service 无单元测试；修复方案：分批补测
- E2E 失败排查（已知问题，待规则 5 节点）— 问题描述：CI 中 E2E 测试持续失败；修复方案：下载 playwright-report 分析失败用例，按规则 0/2 真实修复

---

## 🔄 进行中批次

### 批次 264：待启动（service 分页剩余 5 个特殊处理文件 + 测试覆盖 ⏳）

---

## 规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 270 触发（上次批次 240 → 独立工作流 e2e-batch.yml）
  - 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch
  - 批次 N+20：第 1 次监控（GitHub API 查询 run 状态）
  - 批次 N+28：第 2 次监控（若 N+20 未完成）
  - 批次 N+29：最后监控，未完成则跳过 N+30 的 E2E 周期
  - **注意**：E2E 已从 ci-cd.yml 独立到 e2e-batch.yml，不阻塞主 CI
- **规则 10（每 15 批次记忆整理）**：批次 270 触发（上次批次 255）— 需整理、归档、排序所有记忆文件，确保高效简洁
