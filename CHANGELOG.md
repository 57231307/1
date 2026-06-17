# 更新日志

本项目的所有显著变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [Unreleased] - 2026-06-17

### .monkeycode/ 文档重构（2026-06-17）

#### 背景
main 分支的 `.monkeycode/MEMORY.md` 处于**未解决的合并冲突状态**：
- 第 342 行有 `<<<<<<< HEAD` 标记
- 第 343 行有 `=======` 标记
- **缺少 `>>>>>>>` 结束标记**（文件被截断）
- 文件 500 行包含 main 自身内容（1-341 行）+ test 抽离内容（344-500 行）

#### 用户决策
- ✅ 用 test 分支的精简版覆盖 main
- ✅ 任务相关条目抽离到独立 doto.md

#### 实施
- `.monkeycode/MEMORY.md`：用 test 精简版覆盖（500 行 → 330 行）
- `.monkeycode/doto.md`：从 test 新增（406 行），保存任务相关条目

#### 关键内容
- MEMORY.md 保留：**用户指令/偏好** + **工作流规范**类条目
- doto.md 包含：**功能实现进度** / **路由架构变动** / **任务规划** / **波次总结** / **GitHub 分支策略**等任务相关条目

#### 影响文件
- `.monkeycode/MEMORY.md` — 500 行 → 330 行（覆盖）
- `.monkeycode/doto.md` — 新增 406 行
- `CHANGELOG.md` — 本段记录

#### 与 P12 批 1 关系
- 独立 PR，不与 PR #178（roadmap v0.3）混合
- 为 P12 批 1 启动前的文档基线准备

---
### 文档重新规划（2026-06-17，项目收尾）

#### 新建
- **综合路线图**：[docs/superpowers/plans/2026-06-17-roadmap.md](docs/superpowers/plans/2026-06-17-roadmap.md)
  - 整合所有未完成任务：Wave 2 遗留（9 子任务）+ Wave 4 候选（6 任务）+ B 复查（3 任务）+ .monkeycode/ 归档任务
  - 按优先级分类：🟠 P0（1）/ 🟠 P1（3）/ 🟡 P2（7）/ 🟡 P3（5）
  - 推荐 P12 批 1 范围：P2-1（5 PR 串行）+ B-type-check（CI 5 job）+ P2-2 性能优化
  - 派发策略：3 个独立子代理串行（参照 P11 批 1 验证通过的模式）
  - 7 节结构：目标背景 / 任务清单 / 优先级 / 批次规划 / 执行策略 / 关联文档 / 待办确认

#### 删除（旧 plans/specs）
- **plans/ 删除 10 个**：
  - 2026-06-03-backend-refactor-security.md
  - 2026-06-03-comprehensive-bug-fix.md
  - 2026-06-03-fix-api-route-mismatches.md
  - 2026-06-13-quality-improvement.md
  - 2026-06-15-b7-completion-report.md
  - 2026-06-15-wave1-3-evaluation.md
  - 2026-06-15-wave2-revised-plan.md
  - 2026-06-15-wave3-evaluation-plan.md
  - 2026-06-15-wave3-evaluation-v2.md
  - 2026-06-15-wave3-wrap-up-completion-report.md
- **specs/ 删除 3 个**：
  - 2026-06-03-backend-refactor-security-design.md
  - 2026-06-15-b7-console-cleanup-design.md
  - 2026-06-15-wave3-wrap-up-design.md
- **保留 3 个**（被新路线图引用）：
  - plans/2026-06-16-wave4-p2-1-plan.md（Wave 4 P2-1 详细子任务计划）
  - plans/2026-06-17-p11-h3-deadcode-cleanup-report.md（最近 P11 H3 产出）
  - specs/2026-06-16-wave4-p2-1-design.md（Wave 4 P2-1 设计稿）

#### MEMORY.md 同步
- "P12 待启动" → "P12 批 1：详见综合路线图"
- "Wave 3 收尾报告" / "Wave 1-3 综合评估" 引用改为指向 roadmap
- "关键文档位置" 表精简为 7 项（突出 roadmap + 3 个保留文档 + 3 个根目录文档）

#### 用户决策点（2026-06-17 AskUserQuestion）
- ✅ 新规划文件命名：`2026-06-17-roadmap.md`
- ✅ 旧文件处理：直接删除（保留最近 3 个）
- ✅ 新规划范围：全覆盖（Wave 2 + Wave 4 + B6 + B 复查）

### 文档状态核实（2026-06-17，项目收尾续）

#### 核实发现（与 v0.1 描述严重不符项）
- **P2-1 PR-1 已完成**：V2Table 组件 + useTableApi composable + 5 单元测试已落地
  - [useTableApi.ts](file:///workspace/frontend/src/composables/useTableApi.ts) 145 行
  - [V2Table/index.vue](file:///workspace/frontend/src/components/V2Table/index.vue) 127 行
  - [V2Table/types.ts](file:///workspace/frontend/src/components/V2Table/types.ts)
  - [V2Table.spec.ts](file:///workspace/frontend/tests/unit/V2Table.spec.ts)
  - **B5 POC 文件、DraggableTable.vue、inventory-poc 路由已提前清理**
- **P2-1 实际为 4 PR 串行**（PR-1 完成 + PR-2~5 未启动），非 5 PR
- **B-PR 模板已完成**：[PULL_REQUEST_TEMPLATE.md](file:///workspace/.github/PULL_REQUEST_TEMPLATE.md) 66 行
- **B5 / B6 / 部署 容器化 / .monkeycode 归档 4 阶段**：均已基本完成

#### v0.2 更新内容
- **状态标注**：
  - B3 拆分 / B4 完成 10 Tab：⚠️ 部分完成（仍有 30 个 > 488 行 .vue，system/ 下 11 Tab 仍为骨架）
  - 复查 代码质量：⚠️ 持续进行（P11-H3 已完成 116→30，30 项剩余带 TODO 注释）
  - 收尾 / 文档 OpenAPI：✅ 持续 / ⚠️ 部分完成
  - B5 / B6 / 部署 / B-PR 模板：✅ 完成
- **P12 批 1 范围调整**：
  - P2-1 从 5 PR 缩为 4 PR（PR-1 已完成）
  - 总 PR 数 7 → 6
  - .monkeycode/ 归档 4 阶段从 P12+ 移除
- **MEMORY.md 同步**："P12 待启动"段扩展为 v0.2 范围说明；"最后更新"段新增 v0.2 时间戳

#### 用户决策点（2026-06-17 17:xx AskUserQuestion）
- ✅ P2-1 状态与 roadmap 严重不符处理：按核实结果更新范围
- ✅ roadmap 中已完成/部分完成任务文档：立即更新 roadmap 反映实际

#### 影响文件
- [docs/superpowers/plans/2026-06-17-roadmap.md](docs/superpowers/plans/2026-06-17-roadmap.md) — 全面更新（v0.1 → v0.2）
- [MEMORY.md](MEMORY.md) — "P12 待启动"段扩展 + "最后更新"时间戳

### test 分支 1:1 核对（2026-06-17）

#### 核实方法
- `git fetch origin test:test` 拉取远端 test 分支
- `git log main..test` / `git log test..main` / `git merge-base test main`
- `git diff test..main --name-status` 文件级差异

#### 关键发现
- **test 分支与 main 完全分叉**（`merge-base` 输出空），不是简单的"main 早期状态"
- **test 独有 1154 commit**，main 独有 5 commit（PR #175/#176/#177 + release + memory）
- **文件差异**：629 文件，+8228 / **-106290**（test 删除的文件远多于新增）
- **test 独有数据库迁移**：29 个文件（销售报价单 / 产品色价 / 故障转移 / 生产/质量/售后 / 色卡 / AI / 数据仓库）
- **test 独有后端 handler**：7 个（ai_extend / bi / color_card / color_price / custom_order / failover / quotation）
- **main 已有但被 `#[allow(dead_code)]` 标注**：csrf.rs / event_kafka.rs

#### 业务价值评估
- **P0 高价值**：销售报价单系统 + 产品色价系统（⭐⭐⭐⭐⭐）
- **P1 中价值**：生产管理 / 质量+售后 / 色卡管理
- **P2 低价值**：BI / AI / 故障转移 / microservices/notifications

#### 用户决策点
- ✅ test 资产处理：先 port P0 高价值资产
- ✅ P0 port 范围：完整 port 销售报价单（3-4 PR 串行），暂缓产品色价
- ✅ v0.3 合并策略：在 PR-178 追加 v0.3 改动一次性合并

---

### Roadmap v0.3：加入 P0 销售报价单 port 计划（2026-06-17）

#### 变更内容
- **新增 2.5 节**：test 独有资产 P0 port 概要
- **新增 3.1 节**：P0 关键路径加入 P0 port 销售报价单
- **调整 4.1 节**：P12 批 1 总 PR 从 6 升至 10，4 子代理并行
- **调整 5.1 节**：执行方案加入子代理 A（P0 port 4 PR 串行）
- **调整 5.3 节**：风险预警加入 4 项 P0 port 特有风险

#### 新建 plan 文档
- [docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md](docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md) — 销售报价单 4 PR 串行详细计划

#### 4 子代理并行策略
- 子代理 A：P0 port 销售报价单（4 PR 串行）
- 子代理 B：P2-1 el-table-v2（4 PR 串行）
- 子代理 C：B-type-check（1 PR）
- 子代理 D：P2-2 性能优化（1 PR）

A/B 内部强依赖（各自串行），A/B/C/D 之间可并行（CI 资源允许）

#### P0 port 关键约束
- test 与 main 无共同祖先，所有代码**重新实现**（不能直接 copy）
- main 已有 `sales_order_handler`，需注意命名区分
- `quotation_pricing_service` 依赖 `product_color_price`（test 独有），port 时 **stub pricing**（标 `#[allow(dead_code)]` + TODO(tech-debt)）

#### 影响文件
- [docs/superpowers/plans/2026-06-17-roadmap.md](docs/superpowers/plans/2026-06-17-roadmap.md) — 升至 v0.3
- [docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md](docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md) — 新建
- [MEMORY.md](MEMORY.md) — "P12 待启动"段扩展为 v0.3 范围

---

### P11 批 1 合并汇总（2026-06-17，3 个高风险任务全部完成）

| PR | 任务 | 子代理 | 提交 | 状态 |
|------|------|--------|------|------|
| [#173](https://github.com/57231307/1/pull/173) | P11-H1 CSRF 防护（后端中间件 + 前端 X-CSRF-Token 注入） | 主代理 | [475e79b](https://github.com/57231307/1/commit/475e79b) | ✅ 已合并 |
| [#174](https://github.com/57231307/1/pull/174) | P11-H2 Kafka 真实集成（双后端 + 自动降级） | 主代理 | [3e87b81](https://github.com/57231307/1/commit/3e87b81) | ✅ 已合并 |
| [#175](https://github.com/57231307/1/pull/175) | P11-H3 dead_code 全面清理（services/handlers/middleware/routes） | 主代理 | [0b1c9ac](https://github.com/57231307/1/commit/0b1c9ac) | ✅ 已合并 |

#### P11-H1 CSRF 防护
- **后端中间件**：`backend/src/middleware/csrf.rs`（216 行，6 单元测试）
- **中间件集成**：`backend/src/middleware/mod.rs` 注册 + `main.rs` 挂载在 auth → permission 之间
- **缓存扩展**：`backend/src/utils/cache.rs` 新增 `consume_csrf_token`（一次性消费，rotation 模式）
- **集成测试**：`backend/tests/test_csrf_middleware.rs`（277 行，7 测试覆盖：GET 放行 / POST 缺失 / POST 无效 / 有效 + rotation / 公开路径 / HEAD/OPTIONS / cache 单元测试）
- **前端拦截**：`frontend/src/api/request.ts` axios 自动注入 `X-CSRF-Token` + 403 拦截清理并跳转登录 + 公开路径白名单
- **前端 storage**：`frontend/src/api/auth.ts` 登录/刷新后保存 csrf_token 到 localStorage
- **关键安全特性**：
  - 跳过方法：GET / HEAD / OPTIONS（无副作用）
  - 跳过路径：所有 `public_routes.rs` PUBLIC_PATHS
  - Token rotation：验证通过后从缓存移除（一次性使用）
  - 错误响应：JSON 格式 `CSRF_TOKEN_MISSING` / `CSRF_TOKEN_INVALID`

#### P11-H2 Kafka 真实集成
- **依赖**：`backend/Cargo.toml` 新增 `rskafka = { version = "0.5", default-features = false }`（纯 Rust，无 C 依赖）
- **后端实现**：`backend/src/services/event_kafka.rs`（Kafka 真实后端：连接/生产/消费/重连/降级）
- **后端重构**：`backend/src/services/event_bus.rs` 抽象 `EventBackend` trait + 双后端
- **配置**：`backend/config.yaml` / `config.yaml.example` / `.env.example` 新增 `kafka:` 配置节
- **集成测试**：`backend/tests/test_event_bus.rs`（5 测试：Broadcast 收/发、降级、serde round-trip、Kafka 配置、不可达）
- **关键设计**：
  - 默认 `enabled=false`，CI 环境无 Kafka 不阻塞
  - Kafka 不可达时 5s 超时 + 自动降级到 BroadcastBackend + tracing::error 记录
  - 启动超时 5s，避免启动卡死
  - 兼容原有 13 种 BusinessEvent 变体
  - `ShippedItem` 补 `Serialize/Deserialize` 派生以支持 Kafka 序列化

#### P11-H3 dead_code 全面清理
- **总览**：`#[allow(dead_code)]` 标记从 **116 → 30**（减少 74%）
- **删除死函数/结构**：24 项
- **删除死文件**：1 个（`backend/src/services/scheduler_service.rs` 整文件 336 行）
- **删除 #[allow(unused_imports)]**：4 处 + 死 pub use 重导出
- **修复未使用 import**：15+ 项
- **删除 _unused/DbArc 抑制函数/类型别名**：13 项
- **保留项**：30 项 `#[allow(dead_code)]` 全部按规范补齐 `TODO(tech-debt)` 注释
- **完成报告**：[docs/superpowers/plans/2026-06-17-p11-h3-deadcode-cleanup-report.md](docs/superpowers/plans/2026-06-17-p11-h3-deadcode-cleanup-report.md)
- **关键修复**：子代理初版清理时误删 15 处实际被使用的 import + 1 个 ExportRequest struct，主代理接手后通过 CI 反馈精确定位并全部恢复
- **格式问题**：`cargo fmt --check` 失败后用 `cargo fmt` 单文件修复后通过

#### 远端工作分支清理
- 3 个临时 P11 特性分支已由 GitHub squash merge 自动删除
- 主分支 main 始终保持可发布
- P11 批 1 收尾后 main 已更新至 `0b1c9ac`

---

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
