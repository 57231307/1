# 更新日志

本项目的所有显著变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [Unreleased] - 2026-06-17

### Added - P0-4 色卡仓储管理（合并 PR #129）
- 新增 3 张表：`color_cards`（色卡主表）/ `color_card_items`（色卡明细）/ `color_card_borrow_records`（借出记录）
- 新增 3 个 entity + 7 个 DTO（含分页响应、扫码响应、配方摘要、价格摘要）
- 新增 4 个 service：ColorCardCrudService / ColorCardItemService / ColorCardBorrowService / ColorCardScanService
- 新增 13 个 handler + 16 个 API 端点（CRUD + 色号 + 借出/归还/遗失/损坏/扫码/批量导入/CSV 导出）
- 新增 CIELab 色彩空间转换工具（RGB/CMYK/Lab/HEX/ΔE 互转 + 5 单元测试）
- 新增 4 个前端页面（list / create / detail / borrow）+ 3 个组件（ColorCardGrid / ColorItemEditor / BorrowRecordTimeline）
- 新增 16 端点 TypeScript API 客户端 + Playwright E2E 测试
- 新增 5 个集成测试（共 29 用例）覆盖 CRUD / 色号 / 借出 / 扫码 / E2E
- 新增 TEST 测试版本（Docker + docker-compose + 19 个测试场景）
- 新增 3 个文档：用户手册 / API 文档 / 部署指南

### Changed
- 复用 P0-1 `product_color_prices`（色号价格关联）
- 复用现有 `customers`（借出客户）和 `dye_recipes`（染色配方关联）
- 复用现有 `users`（经办员工关联）
- models / services / handlers / routes / utils mod.rs 添加新模块声明
- frontend router 添加 4 个色卡路由

### Technical
- 状态机：borrowed → returned / lost / damaged（终态不可转换）
- 行业规则：GB/T 26377-2022 / PANTONE / CNCS（中国颜色体系）/ ΔE ≤ 3
- 多租户隔离：强制 `extract_tenant_id`
- 8 个 commit（fb302b7 → 503c184），合入 test 分支后删除 P0-4 分支

---

## [Released] - 2026-06-16 - P0-3 定制订单全流程跟踪（PR #128）

### P0-2 主备隔离模块（数据库 + 缓存）

- **设计文档**：
  - Spec：[`docs/superpowers/specs/2026-06-16-failover-isolation-design.md`](file:///workspace/docs/superpowers/specs/2026-06-16-failover-isolation-design.md)
  - Plan：[`docs/superpowers/plans/2026-06-16-failover-isolation-plan.md`](file:///workspace/docs/superpowers/plans/2026-06-16-failover-isolation-plan.md)
  - 设计报告：[`docs/superpowers/reports/2026-06-16-failover-design.md`](file:///workspace/docs/superpowers/reports/2026-06-16-failover-design.md)
- **范围**：P0 阶段（数据库 + 缓存 + 进程内 LRU 备 + 监控告警 + 故障注入测试）
- **核心特性**：
  - `FailoverCall` trait（统一主备调用接口，P1/P2 阶段可复用）
  - 熔断器（Closed/Open/HalfOpen 状态机，阈值 5，时长 30s）
  - 数据库主备隔离（PostgreSQL 主库 + 备库自动切换）
  - 缓存主备隔离（Redis 主 + moka 进程内 LRU 备）
  - 5 个 Prometheus 指标（primary/backup/switch/circuit_state）
  - 4 条告警规则（P0/P1/P2 级别）
  - 自动回切（主调用恢复后 < 30s 自动回切）
  - 4 个 HTTP API 端点（status / metrics / test/switch / health）
  - admin 监控页面（状态卡片 + 切换历史 + 健康检查）
  - 9 个故障注入测试场景
  - TEST 测试版本交付（Docker + docker-compose + 启动脚本）
- **数据模型**：
  - `failover_status`（主备实时状态）
  - `failover_event`（切换事件流水）
  - `failover_config`（配置持久化）
- **关键文件**：
  - `backend/src/utils/failover/{mod,database,cache,circuit_breaker}.rs`
  - `backend/src/config/failover.rs`
  - `backend/src/services/failover_service.rs`
  - `backend/src/handlers/failover_handler.rs`
  - `backend/src/routes/failover.rs`
  - `backend/src/models/failover_{status,event,config}.rs`
  - `backend/migrations/20260616000005_create_failover_tables/`
  - `backend/tests/failover_{trait,circuit,config,metrics}_test.rs`
  - `frontend/src/views/admin/failover.vue` + 3 组件
  - `frontend/src/api/failover.ts`
  - `dist/test-version-P0-2/`（Docker + compose + start.sh）
  - `docs/failover-deployment-guide.md`
  - `docs/chaos-test-scenarios.md`
  - `monitoring/grafana/failover-dashboard.json`
  - `monitoring/prometheus/failover-alert-rules.yml`

### P0-3 定制订单全流程跟踪模块

- **设计文档**：
  - Spec：[`docs/superpowers/specs/2026-06-16-custom-order-design.md`](file:///workspace/docs/superpowers/specs/2026-06-16-custom-order-design.md)
  - Plan：[`docs/superpowers/plans/2026-06-16-custom-order-plan.md`](file:///workspace/docs/superpowers/plans/2026-06-16-custom-order-plan.md)
- **范围**：5 阶段工艺流程跟踪（纱线采购 → 染整 → 后整理 → 交付 → 售后）
- **核心特性**：
  - 5 张表数据模型（含 5 阶段状态机 + 8 状态枚举）
  - 16 个 REST API 端点（CRUD + 流程推进 + 质检 + 售后）
  - 5 阶段工艺状态机（draft → yarn_purchasing → dyeing → finishing → delivery → after_sales → completed）
  - 工艺节点状态机（pending / in_progress / completed / blocked）
  - 售后工单状态机（opened → processing → resolved / closed / rejected）
  - 质检规则：GB/T 26377-2022 色差 ΔE 校验 + ISO 105 色牢度等级 1-5 校验
  - 4 种售后类型：客诉 / 维修 / 换货 / 退款（退款类型必填金额）
  - 5 阶段工艺流程甘特图（tracking 大屏）
  - 自动生成 5 阶段工艺节点（创建订单时）
  - 完整时间线（节点 + 操作日志）
  - 4 前端页面 + 3 组件
  - 5 集成测试 + E2E 测试
  - TEST 测试版本交付
- **数据模型**：
  - `custom_orders`（定制订单主表，8 状态枚举）
  - `process_nodes`（工艺节点，5 节点类型 + 4 状态）
  - `process_logs`（操作日志，含 JSONB 附件）
  - `quality_issues`（质量异常，4 严重度）
  - `after_sales`（售后工单，4 类型 + 5 状态）
- **关键文件**：
  - `backend/migrations/2026061700000{1..5}_create_{custom_orders,process_nodes,process_logs,quality_issues,after_sales}/`
  - `backend/src/models/{custom_order,process_node,process_log,quality_issue,after_sales}.rs`（5 entity）
  - `backend/src/models/{custom_order_create_dto,custom_order_update_dto,custom_order_response_dto,process_node_dto,quality_issue_dto}.rs`（5 DTO）
  - `backend/src/services/custom_order_{crud,state,process,quality,aftersales}_service.rs`（5 service）
  - `backend/src/handlers/custom_order_handler.rs`（13 handler）
  - `backend/src/routes/custom_order.rs`（16 路由）
  - `backend/src/utils/process_state_machine.rs`（5 阶段状态机 + 9 单元测试）
  - `backend/tests/custom_order_{e2e,state,process,quality,aftersales}_test.rs`（5 集成测试）
  - `frontend/src/views/custom-orders/{list,create,detail,tracking}.vue`（4 页面）
  - `frontend/src/components/{ProcessFlow,QualityCheck,AfterSalesPanel}.vue`（3 组件）
  - `frontend/src/api/custom-order.ts`（16 端点 API 客户端）
  - `frontend/e2e/custom-order.spec.ts`（E2E 测试）
  - `dist/test-version-P0-3/`（Docker + compose + start.sh + config + test-scenarios）
  - `docs/custom-order-{user-manual,api,deployment-guide}.md`
- **关键参数**：
  - 色差 ΔE 警告阈值：5.0
  - 色牢度等级范围：1-5
  - 售后工单超时：72 小时
  - 5 阶段工艺顺序：强制顺序，不可跳跃
  - 多租户隔离：extract_tenant_id 强制（无 unwrap_or(0)）

- **关键参数**（P0-2 主备隔离）：
  - 主调用超时：3s
  - 备用调用超时：5s
  - 熔断阈值：5 次失败
  - 熔断时长：30s
  - 半开探测：1 次
- **验收标准**：
  - 主调用失败 → 备用切换延迟 < 100ms
  - 主调用恢复 → 自动回切延迟 < 30s
  - 9 个故障注入场景全部通过
  - 4 条告警规则按级别触发
  - TEST 测试版本可在 Docker 中启动

### Wave 4 P2-1 综合评估

- **评估报告**：[`docs/superpowers/plans/2026-06-16-wave4-p2-1-evaluation.md`](file:///workspace/docs/superpowers/plans/2026-06-16-wave4-p2-1-evaluation.md)（310 行，PR #117 squash merge → commit dbd472d）
- **关键指标**：
  - 5 PR 100% 完成（#108-#112，1h45min 串行调度）
  - 代码变更：+1090 / -1379（净减 289 行）
  - CI 验证：5 × 4 job = 20 job 全部全绿
  - 自动发版：5 个 tag（v2026.616.1235 至 v2026.616.1420）
  - 拒收率：0%
- **关键决策**：
  - PR-1 抽象前置：useTableApi composable + V2Table 组件，4 页面复用
  - 串行调度模式再次验证（与 Wave 3 B7 经验一致）
  - 死代码随 PR-5 一次性清理
- **关键经验**：
  - 抽通用组件前置（PR-1 模式）：下游 PR 成本 -60%
  - 串行 + 串行调度：避免云端卡死
  - 死代码随主任务清理：避免技术债务积累
- **下一波推荐**：P2-2 性能优化（V2Table 性能验证 + 后端 N+1 修复）

### 已整理（记忆文件分类）

- **新增** [`.monkeycode/doto.md`](file:///workspace/.monkeycode/doto.md)：从 `.monkeycode/MEMORY.md` 抽离所有**任务相关条目**，包括：
  - 功能实现进度（751 子功能清单）
  - 路由架构变动记录（2026-06-06 修复）
  - 16 任务总规划 / 13 任务重新规划
  - P0-2 / Wave 1 / Wave 3 B7 波次执行总结
  - 当前待办 + Wave 4 P2-1 完成回顾
- **精简** `.monkeycode/MEMORY.md`：从 498 行精简为 184 行，仅保留**用户指令/偏好/工作流规范**类条目
- **文件分类**：
  - `MEMORY.md` → 用户指令/偏好/工作流规范（必读）
  - `doto.md` → 任务进度、规划、波次总结（按需查）
- **敏感信息**：已移除服务器密码、数据库密码等敏感信息
- **影响范围**：本地 `.monkeycode/` 目录（在 `.gitignore` 中），仅影响本地工作记录

## [Unreleased] - 2026-06-15

### Wave 1 合并汇总（2026-06-15）

| PR | 任务 | 子代理 | 提交 | 状态 |
|------|------|--------|------|------|
| [#89](https://github.com/57231307/1/pull/89) | .clippy.toml 宏路径警告 | C | [a779078](https://github.com/57231307/1/commit/a779078) | ✅ 已合并 |
| [#90](https://github.com/57231307/1/pull/90) | P1-5 入库单明细类型强化 | B2 | [2974c6d](https://github.com/57231307/1/commit/2974c6d) | ✅ 已合并 |
| [#87](https://github.com/57231307/1/pull/87) | P0-2 销售→AR 应收账款 | A1 | [042d123](https://github.com/57231307/1/commit/042d123) | ✅ 已合并 |
| [#88](https://github.com/57231307/1/pull/88) | P1-1 generate-no 4 端点 | B1 | [5f28212](https://github.com/57231307/1/commit/5f28212) | ✅ 已合并 |

- 4 个 PR 全部以 Squash 策略合并入 main
- 远端源分支（feature/p0-2-sales-ar* / feature/p1-1-generate-no / fix/clippy-toml-warnings / feature/P1-5-completed-2-todos）已由 GitHub 自动删除
- 定时轮询任务 `NLIZU5YY.FK660` 已停止
- Wave 1 全部子代理成果已合入 main，可以启动 Wave 2

### Wave 2 合并汇总（2026-06-15）

| 任务 | 提交 | 状态 |
|------|------|------|
| B6 清理 budget.ts / cost.ts 中 8 个未用 API 函数 | [9f832a8](https://github.com/57231307/1/commit/9f832a8) | ✅ 已合并 |
| B5 P2-1 el-table-v2 虚拟列表 POC 通过 | [7a1d27f](https://github.com/57231307/1/commit/7a1d27f) | ✅ 已合并 |
| B3-1 拆分 6 个 > 1000 行巨型 .vue 文件 | [9864b38](https://github.com/57231307/1/commit/9864b38) | ✅ 已合并 |
| B3-2 拆分财务/会计域 12 个 .vue 文件 | [5749d65](https://github.com/57231307/1/commit/5749d65) | ✅ 已合并 |
| B3-3 拆分 CRM/客户域 8 个 .vue 文件 | [aa7b8f9](https://github.com/57231307/1/commit/aa7b8f9) | ✅ 已合并 |
| B3-4 拆分库存/产品域 8 个 .vue 文件 | [bdcc67b](https://github.com/57231307/1/commit/bdcc67b) | ✅ 已合并 |

#### 拆分成果
- **> 1000 行 .vue 文件**：6 → **0**（100% 消除）
- **> 500 行 .vue 文件**：60 → **32**（-47%）
- **新建子组件**：80+ 个（system/tabs/ + 各业务域 tabs/）
- **B4 任务意外完成**：10 Tab 骨架升级为完整实现（顺手在 B3-1 中完成）

#### B5 POC 通过标准
- 1 万行数据生成：13.2ms
- Type-check / Vite build / 单测：全部通过
- 真实性能数据（FPS/内存/渲染）：需本地复现 `frontend/scripts/poc-perf-test.cjs`

#### 远端工作分支清理
- 6 个临时 feature 分支（feature/B3-1~4 / B5 / B6）已从远端删除
- 定时轮询任务保持停用状态
- Wave 3 启动条件已达成（el-table-v2 POC 通过）

### Wave 2 状态汇总
- Wave 2 进度：6/6 完成 ✅
- B3-1 ~ B3-4 + B5 + B6 全部以 Squash 策略合并入 main
- 主入口 < 100 行（除 inventory 292 行因含统计卡片）
- 调度策略：单子代理串行执行，避免云端卡死
- 启动条件：Wave 3（el-table-v2 POC 通过）✅ 可启动
- 启动条件：Wave 4（≥ 1 个 P3 任务完成 PoC）🔵 待启动

### Wave 3 合并汇总（2026-06-15）

| 任务 | 子代理 | 提交 | 状态 |
|------|--------|------|------|
| B7 spec 编写 | 主代理 | [fee7507](https://github.com/57231307/1/commit/fee7507) | ✅ 已合并 |
| B7-1 替换 purchase+inventory 域 console.* 为 logger (8 文件 37 处) | 1 B | [313084e](https://github.com/57231307/1/commit/313084e) | ✅ 已合并 ([#91](https://github.com/57231307/1/pull/91)) |
| B7-2 替换 crm+sales 域 console.* 为 logger (4 文件 11 处) | 1 B | [c641239](https://github.com/57231307/1/commit/c641239) | ✅ 已合并 ([#92](https://github.com/57231307/1/pull/92)) |
| B7-3 替换 bpm+report+arReconciliation 域 console.* 为 logger (7 文件 22 处) | 1 B | [374a3af](https://github.com/57231307/1/commit/374a3af) | ✅ 已合并 ([#93](https://github.com/57231307/1/pull/93)) |
| B7-4 替换 dye/logistics/security/email/tenant 等域 console.* 为 logger (12 文件 42 处) | 1 B | [979feca](https://github.com/57231307/1/commit/979feca) | ✅ 已合并 ([#94](https://github.com/57231307/1/pull/94)) |

#### B7 替换成果
- **console.* 总数**：112 → **0**（-100%，除 logger.ts 自身 4 处）
- **涉及文件数**：31 个 .vue / .ts 文件
- **PR 数**：4 个（#91-#94）
- **替换映射**：log/info/debug → logger.info/debug、warn → logger.warn、error → logger.error
- **特殊处理**：catch 块中 `e:unknown` 用 `String(e)` 转换（消除 TS2345 错误）

#### 已知遗留
- 基线存在 32 个预存 type-check 错误（来自 Wave 2 合并），分布在 fiveDimension/print-templates/quality-standards/data-import/dataPermission/dye-batch/dye-recipe/warehouse/system-update/user-profile 等模块
- B7 4 批均**无新增错误**（基线 = 当前 = 32）
- 清理预存错误属于 Wave 4 启动前置 P 任务，不在 B7 范围

#### 远端工作分支清理
- 4 个临时 B7 特性分支已由 GitHub squash merge 自动删除
- 主分支 main 始终保持可发布

### Wave 3 收尾汇总（2026-06-15）

| 任务 | 子代理 | 提交 | 状态 |
|------|--------|------|------|
| B 任务 5 批 4 PR：清理 32 个预存 type-check 错误 → 0 | 主代理串行 | [7de8b0d](https://github.com/57231307/1/commit/7de8b0d) | ✅ 已合并 |
| A2-1 工艺优化（recipe_opt）后端+前端+4 单测 | AI 实施子代理 | [f157f56](https://github.com/57231307/1/commit/f157f56) | ✅ 已合并 ([#99](https://github.com/57231307/1/pull/99)) |
| A2-2 质量预测（quality_pred）后端+前端+4 单测 | AI 实施子代理 | [dd9faa4](https://github.com/57231307/1/commit/dd9faa4) | ✅ 已合并 ([#100](https://github.com/57231307/1/pull/100)) |

#### B 任务（type-check 清理 32 → 0）
- **B-批 1** ([#95](https://github.com/57231307/1/pull/95))：修复 `cost.ts` B6 重命名引用 + `index.ts` ReportData 重复导出（4 错误）
- **B-批 2** ([#96](https://github.com/57231307/1/pull/96))：`ApiResponse<T>` 扩展可选 `total` / `timestamp` 字段（13 错误）
- **B-批 3** ([#97](https://github.com/57231307/1/pull/97))：`five-dimension.ts` 扩展 `StatsQueryParams` / `SearchQueryParams` / `FiveDimensionStats` 字段（9 错误）
- **B-批 4** ([#98](https://github.com/57231307/1/pull/98))：`dataPermission` 类型守卫 + `user-profile` 删 rule + `warehouse` `String()` 转换（6 错误）
- 4 批均按文件细粒度划分，主代理串行调度避免云端卡死

#### A2-1 工艺优化（recipe_opt）
- **后端 service**：`backend/src/services/ai/recipe_opt.rs`（680 行，含 4 单测）
- **后端 handler**：`backend/src/handlers/advanced/recipe_opt.rs`（100 行）
- **路由**：`POST /api/v1/erp/advanced/ai/recipe-optimization`
- **前端 API**：`optimizeRecipe(params)` + `RecipeOptParams` 类型
- **前端 Tab**："工艺优化"（表单 + 4 字段描述 + candidates 表格）
- **算法核心**：k-NN 相似度（color_no 1.0 / 前缀 0.7 / fabric 0.2 / dye 0.1，最大 1.3）+ 退化兜底（80°C/45min/pH6.0/浴比1:8）
- **冷启动**：命中 ≥ 3 条走 k-NN，否则退化；k=0 强制退化
- **4 单测**：`test_typical_params_fallback` / `test_color_match_knn` / `test_temperature_recommendation` / `test_fallback_path`
- **CI 验证**：run 27555546133，4 job 全绿，143 单测全过

#### A2-2 质量预测（quality_pred）
- **后端 service**：`backend/src/services/ai/quality_pred.rs`（681 行，含 4 单测）
- **后端 handler**：`backend/src/handlers/advanced/quality_pred.rs`（89 行）
- **路由**：`POST /api/v1/erp/advanced/ai/quality-prediction`
- **前端 API**：`predictQuality(params)` + `QualityPredParams` 类型
- **前端 Tab**："质量预测"（表单 + 4 统计卡片 + 问题表格 + 建议列表 + 周期明细）
- **算法核心**：基于 `quality_inspection_records` 历史合格率 + 时间窗口趋势 + 风险评分（0-100）
- **风险评分**：`risk = (100 - avg_rate) * 0.6 + 下降趋势惩罚 * 0.4`
- **趋势判定**：(recent - previous) / previous，超过 ±5% 视为上升/下降
- **退化兜底**：数据 < 5 条 → 默认 95% + confidence 0.3
- **4 单测**：`test_risk_score_low` / `test_risk_score_high` / `test_trend_calculation` / `test_fallback_low_data`
- **CI 验证**：PR #100 squash merge 后 4 job 全绿，CI 自动发布 tag v2026.615.2350

#### Wave 3 收尾总成果
- 实施总文件数：约 23 个（5 新增 + 18 修改）
- 新增 8 个单元测试（4 recipe_opt + 4 quality_pred），全部覆盖核心算法
- type-check 错误：32 → 0（-100%）
- AI 智能分析服务：新增 recipe_opt + quality_pred 两个子模块
- 前端 Advanced 页面：Tab 数 3 → 5（新增工艺优化 + 质量预测）
- CI 流水线：所有任务均以 Squash 策略合并，4 job 全绿，自动发布
- 远端 3 个临时特性分支（A2-1 / A2-2 / B-批 1-4）已全部清理

#### 关键经验（Wave 3 收尾新沉淀）
- **CI/CD 验证优先**：项目全程仅在 CI/CD 构建验证，本地禁止任何 cargo / npm / vue-tsc / tsc / vite 命令
- **代码质量护栏**：PR 触发 CI → 4 job 全绿 → squash merge → 远端分支自动删除 → 本地手动清理
- **多语言化推进**：所有 UI 文本 / 注释 / 日志强制中文；API 路径仍保持英文 snake_case
- **基线修复边界**：A2-1 子代理顺手修复了 ar/inv.rs、accounting-period.ts 等 main 预存错误（必要以让 CI 通过），A2-2 子代理严格限制边界，未做超出范围的修复

#### 待启动
- **Wave 4**：el-table-v2 已通过 POC（B5），Wave 3 收尾已完成 AI 深化，Wave 4 启动条件已就绪
- **Wave 5+**：高级 P2/P3 任务（移动端 / 性能优化 / 安全加固）待规划

### 已新增（P1-1 generate-no 4 端点补齐）

#### 后端 Handler
- 在 `backend/src/handlers/inventory_transfer_handler.rs` 新增 `generate_no` 端点（前缀 `IT`）
- `inventory_count_handler.generate_no`（前缀 `IC`）、`purchase_receipt_handler.generate_no`（前缀 `RK`）、`inventory_adjustment_handler.generate_no`（前缀 `IA`）已在 P1-1 任务中确认存在
- 全部 4 个 Handler 调用 `DocumentNumberGenerator::generate_no_with_width`，流水位宽 4 位
- 单据号格式：`{前缀}{yyyyMMdd}{4 位流水}`，例如 `IC202605140001`

#### 路由注册
- 在 `backend/src/routes/inventory.rs` 注册 3 个新路由：
  - `GET /api/v1/erp/inventory/counts/generate-no`
  - `GET /api/v1/erp/inventory/adjustments/generate-no`
  - `GET /api/v1/erp/inventory/transfers/generate-no`
- 在 `backend/src/routes/purchase.rs` 注册 1 个新路由：
  - `GET /api/v1/erp/purchase/receipts/generate-no`
- 路径与 `backend/src/routes/finance.rs` 现有 `/vouchers/generate-no` 保持一致风格

#### 前端 API 函数
- `frontend/src/api/inventoryCount.ts` 新增 `generateInventoryCountNo`（返回 `{ count_no }`）
- `frontend/src/api/purchaseReceipt.ts` 新增 `generatePurchaseReceiptNo`（返回 `{ receipt_no }`）
- `frontend/src/api/inventoryAdjustment.ts` 新增 `generateInventoryAdjustmentNo`（返回 `{ adjustment_no }`）
- `frontend/src/api/inventoryTransfer.ts` 新增 `generateInventoryTransferNo`（返回 `{ transfer_no }`）
- 全部函数返回 `Promise<ApiResponse<...>>`，TypeScript 类型完整

#### 测试
- 新增 `backend/tests/test_generate_no_endpoints.rs`，包含 4 个单据号格式单元测试
- 覆盖 4 个前缀（`IC` / `RK` / `IA` / `IT`）与 4 位流水宽度的契约
- 防止后续误将流水宽度回退为 3 位

#### 并发安全说明
- 沿用 `DocumentNumberGenerator` 的"读当日数量 + 1"策略，业务侧依赖数据库 `UNIQUE` 约束最终去重
- 文档已说明后续可接入 PostgreSQL `SEQUENCE` 升级为真正无锁实现

### 已修复（P0-2 销售→AR 业务流）
#### 业务流补齐（P0）
- 在 `backend/src/services/ar/inv.rs` 的 `impl ArReconciliationService` 块中新增 `create_receivable` 方法，作为销售发货→AR 应收的业务流入口
- 方法接收调用方事务引用（`&DatabaseTransaction`），与库存扣减、订单状态更新共用同一事务，保证三阶段原子提交
- 幂等保证：按 `source_type=SALES_ORDER` + `source_bill_id=order_id` 联合判定，重复调用返回 `BusinessError`
- 客户账期处理：调用方传入 `payment_terms_days`，<= 0 时回退为 30 天默认值
- 应收单号连续：复用 `DocumentNumberGenerator`（`AR + YYYYMMDD + 3 位流水号`），与销售订单/采购订单/对账单共用流水生成器
- 配套单元测试 6 个：正常发货、取消回滚、部分发货、账期默认值、幂等性、应收单号格式连续
- 业务事件 `ReceivableCreated` 在事务 commit 成功后再发布，避免订阅方在事务回滚时误处理
- 影响范围：`backend/src/services/so/delivery.rs::ship_order` 第 192-224 行的 AR 集成代码原本调用了不存在的 `ar_service.create_receivable`，本次实现补全该方法，使现有调用可编译

---

## [Unreleased] - 2026-06-14

### 已规划（16 任务总规划 - 阶段一）

#### 项目管理（P0）
- 完成项目深度评估报告（17 万行代码，751 子功能，评分 8.0/10）
- 完成 16 任务总规划（5 P0 + 6 P1 + 4 P2 + 4 P3 = 19 项）
- 建立多子代理并行 + 复查子代理 + 总代理审批工作流
- 归档规划文档：[规划-16tasks-2026-06-14.md](file:///workspace/.monkeycode/docs/规划-16tasks-2026-06-14.md)
- 更新用户记忆（MEMORY.md）：[16 任务总规划] 条目

#### 工作流设计
- **4 类执行子代理**：A 业务实现 / B 前端实现 / C 基础设施 / D 架构演进
- **6 波推荐批次**：每波 4-6 任务，约 5 周完成
- **资源限制**：同时运行子代理数 ≤ 6
- **Git 分支策略**：`feature/{task-id}` 独立分支
- **强制报告模板**：子代理必须输出工作报告（改动/决策/测试/风险/自评）
- **复查清单**：10 项（代码规范/dead_code/clippy/eslint/tsc/租户隔离/敏感信息/错误处理/文档/CHANGELOG）

#### 待启动 Wave 1（5 任务）
- P0-5 修复 MaterialShortageAlert 事件定义（C）
- P1-1 补齐 generate-no 端点（4 页面）（A）
- P1-2 注册未挂载页面路由（sales-analysis/security）（B）
- P2-3 修复 CI 测试编译错误（C）
- 创建 logger 工具（C）

#### 项目管理（阶段二）
- 完成项目进度评估（实时代码扫描）
- **重大发现**：原 19 任务中 5 个已完成（P0-1/3/4/5、P1-2）
- 业务流已通过事件驱动架构实现（event_bus.rs:121-123 InventoryFinanceBridgeService.start_listener）
- 实际未完成任务修正为 **13 个**
- 重新规划文档：[规划-重新规划-13tasks-2026-06-14.md](file:///workspace/.monkeycode/docs/规划-重新规划-13tasks-2026-06-14.md)
- 5 波调度：Wave 1（4 子代理，1 周）→ Wave 2（6 子代理，2 周）→ Wave 3（2 子代理，1 周）→ Wave 4（4 子代理，4 周）→ Wave 5 复查
- 总资源：13 执行子代理 + 1 复查；同时运行峰值 6；总周期约 8 周
- 更新用户记忆（MEMORY.md）：[13 任务重新规划] 条目

#### 修订后 13 任务清单
- 业务流：P0-2 销售发货→AR（60%→100%）
- 基础设施：P2-3 rustc 升级（CI 编译失败修复）
- 前端重构：P1-3 拆分 52 大 .vue、P1-4 完成 10 Tab、P1-5 完成 2 TODO、P2-1 虚拟列表、P2-2 console 替换
- 端点：P1-1 generate-no 4 端点
- AI：P2-4 工艺优化 + 质量预测
- 长期：P3-1 微服务、P3-2 WebSocket、P3-3 React Native、P3-4 BI

### Wave 1 执行结果（2026-06-15）

派发 4 子代理并行执行 Wave 1 任务，全部通过总代理审阅。

#### A1 P0-2 销售发货→AR 应收账款（已完成 100%）
- Commit：`b191398 feat(sales): P0-2 销售发货自动生成 AR 应收账款`
- 文件：[backend/src/services/ar/inv.rs](file:///workspace/backend/src/services/ar/inv.rs)
- 新增 `create_receivable` 方法 92 行 + 6 单元测试 130 行
- 关键发现：[backend/src/services/so/delivery.rs:192-224](file:///workspace/backend/src/services/so/delivery.rs#L192-L224) `ship_order` 已实现 AR 集成调用，本次为"补全缺失方法"
- 剩余风险：R3 voucher 凭证未实现；R2 与 ar_invoice_service 双入口隐患

#### C1 P2-3 编译验证（颠覆性发现）
- CICD Run：[https://github.com/57231307/1/actions/runs/27522504353](https://github.com/57231307/1/actions/runs/27522504353)
- **✅ 已验证通过，无代码修改**：当前 main 分支在 Rust 1.94.1 编译完全通过，P2-3 实际已完成
- 6 个 CICD Job 全绿（test / 前端 test / build-backend 12:29 / vite build / release / notify）
- ~~仅 2 个 .clippy.toml 配置提示警告（`std::println` / `std::eprintln` 宏路径）~~ **已修复**：移除 `std::` 前缀（宏不是方法），2026-06-15
- GitHub Release [v2026.615.1138](https://github.com/57231307/1/releases/tag/v2026.615.1138) 已自动发布

#### B1 P1-1 generate-no 4 端点（已完成 100%）
- Commit：`fe91dc9 feat(generate-no): P1-1 补齐 4 端点 generate-no`
- 4 端点 + 4 前端 API + 4 单测，共 9 文件 +255 行
- 前缀：IC（inventoryCount）/ RK（purchaseReceipt）/ IA（inventoryAdjustment）/ IT（inventoryTransfer）
- 路径风格沿用 RESTful 嵌套（`/api/v1/erp/{domain}/{resource}/generate-no`）

#### B2 P1-5 完成 2 TODO（已完成 100%）
- Commit：`a3b18ca fix(frontend): P1-5 入库单明细 API 类型强化`
- 已推送 `feature/P1-5-completed-2-todos` 等 CICD
- 重大发现：`ca0ca48` 提交已完整实现 2 处 TODO，本次仅做类型补强（消除 `as` 强转）

#### 状态汇总
- Wave 1 进度：4/4 完成 ✅
- 4 PR 全部合并入 main（#87 / #88 / #89 / #90，2026-06-15）
- 远端源分支 + 本地工作分支 + 定时轮询任务已全部清理
- 更新用户记忆（MEMORY.md）：[Wave 1 执行结果]、[沙箱与CICD验证策略] 条目

---

## [2026.614.1353] - 2026-06-14

### 已修复（项目全方位校验、整理与清理 - 第二轮）

#### 代码质量（P1）
- 后端 `backend/src/services/inventory_count_service.rs` 已拆分为子模块（`inventory_count/`）并完成对外公开 API 兼容
- 在 `backend/src/services/mod.rs` 新增 `pub mod inventory_count` 声明

#### 前端重构（P1）
- 完善 `views/system/tabs/RoleTab.vue`：从骨架升级为完整可工作组件（包含 CRUD、权限配置对话框，共 267 行）
- 修复角色编辑时"角色名称"和"角色编码"在编辑模式下禁用的问题

### 已修复（项目全方位校验、整理与清理）

#### 安全（P0）
- 删除未使用 CI 备份文件 `.github/workflows/ci-cd.yml.backup`
- 统一 SQL 迁移目录：删除两个无引用的重复迁移目录（`backend/database/migration/` 26 文件、`backend/src/database/migration/` 9 文件），归档至 `docs/database/legacy-migration-snapshots/`
- 修复 `backend/src/cli/migrate.rs` 中错误的迁移目录注释（指向不存在的 `src/database/migration/`）

#### 重复资源（P1）
- 合并三套密码哈希工具：删除 `backend/hasher_tool/` 和 `backend/Cargo.toml.hash`，保留主项目 `backend/src/bin/hash_password.rs`
- 清理 `backend/src/services/mod.rs` 中 7 个旧路径兼容层（purchase_order_service、sales_service、crm_service、inventory_transfer_service、ar_reconciliation_service、ai_analysis_service、report_engine_service）
- 批量迁移 21 个文件中的 31 处 `crate::services::<alias>::` 引用到新路径（`po::order`、`so::order`、`crm::cust`、`inv`、`ar`、`ai`、`report`）

#### 前端重构（P1）
- 拆分 1478 行的 `views/system/index.vue`：
  - 抽出 `views/system/tabs/UserTab.vue`（完整可工作，275 行）
  - 创建 11 个 Tab 骨架（RoleTab/DepartmentTab/PermissionTab/DataPermissionTab/FieldPermissionTab/NotificationTab/AuditTab/WebhookTab/SystemUpdateTab/TenantTab/CompanyTab）
  - 在 `system/index.vue` 顶部添加拆分指引注释
  - 详细拆分计划见 `docs/refactoring/frontend-vue-splitting-plan.md`

#### 依赖升级（P1）
- 前端依赖升级：
  - `vite`: `^6.4.2` → `^6.4.3`（修复 dev server SSRF 相关依赖）
  - `vitest`: `^1.2.0` → `^2.1.0`（缓解 esbuild 嵌套漏洞）
  - `esbuild`: `^0.25.0` → `^0.25.12`（由 `npm audit fix` 自动升级）
- 完整 npm audit 报告：`.audit-reports/npm-audit.json`（含 2 critical + 3 moderate 漏洞记录与升级路径）

#### 文档与规范（P2）
- 创建 `CHANGELOG.md`（本文件）
- 创建 `docs/database/legacy-migration-snapshots/README.md`（归档说明）
- 创建 `docs/refactoring/frontend-vue-splitting-plan.md`（47 个 Vue 组件拆分计划）
- 创建 `.audit-reports/` 目录（保存审计报告）
- 补充 `frontend/.env.production.example`（生产环境模板）
- 迁移根目录散落运维文档至 `docs/reports/historical/`
- 迁移前端调试脚本至 `frontend/scripts/`
- 补充 LICENSE 第三方组件许可声明

### 已知遗留问题
- `views/system/index.vue` 还有 10 个 Tab 仍为骨架，需前端工程师按 UserTab 模板完成数据加载与表单逻辑（详见 `docs/refactoring/frontend-vue-splitting-plan.md`）
- 其他 46 个超过 500 行的 .vue 文件（sales-ext、purchase-ext、sales、ap、trading 等）仍待拆分
- `inventory_count_service.rs`（949 行）建议拆为 query/writer/reporter 子模块
- 前端虚拟列表化（vue-virtual-scroller 或 Element Plus `el-table-v2`）尚未引入

---

## [2026.522.2] - 2026-05-22

### 新增
- 资金管理模块
- 销售/采购合同模块
- 多币种与汇率模块
- 工作流引擎 BPM

### 修复
- 库存调整审批流
- 销售订单状态机

---

## [2026.1.0] - 2026-01-15

### 新增
- 核心业务模块：采购、销售、库存、生产、财务、CRM
- AI 智能分析（销售预测、库存优化、异常检测）
- 报表引擎（Excel/PDF 导出）
- 多租户 SaaS 架构
- 消息通知（站内信、邮件、短信）
- 移动端响应式支持

### 技术栈
- **后端**：Rust 1.75+ / Axum 0.7 / SeaORM 1.0 / PostgreSQL 15
- **前端**：Vue 3.4 / Vite 5.0 / Element Plus 2.4 / Pinia 2.1
- **基础设施**：Redis 7 / gRPC（Tonic）/ GitHub Actions / Prometheus / Grafana
