# 2026-06-18 P14 批 1 综合推进计划

> **创建日期**：2026-06-18
> **基线版本**：main @ 06bd925（P13 批 1 收尾后）
> **关联路线图**：[2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3
> **派发策略**：单子代理派发 I-2 PR（voucher 870 + api-gateway 835 + arReconciliation 789）
> **总目标**：完成 roadmap 剩余 P2 候选任务（I-2 PR 是最高 P2 优先级）

---

## 一、背景

P13 批 1（3 PR）已全部 squash merge 到 main：
- ✅ PR #191 P3-2 审计日志增强（940dca1）
- ✅ PR #192 B-慢查询审计（04b12cd）
- ✅ PR #193 B3 拆分大 .vue - I-1（c6ca72f）

累计 P12 批 1+2+3 + P13 批 1 共 **15/15 PR 全部完成**。

roadmap v0.3 §二 节剩余候选任务：

| 任务 ID | 任务名 | 优先级 | 当前状态 | 派发顺序 |
|---------|--------|--------|---------|----------|
| **I-2 PR** | 拆分 3 个次大 .vue | 🟡 **P2** | ❌ 未启动 | **本批派发** |
| I-3 PR | 拆分剩余 24 个大 .vue | 🟡 P3 | ❌ 未启动 | 后续 P15 批 1 |
| B4 完成 10 Tab 业务骨架 | 🟡 P3 | ❌ 未启动 | 后续 |
| 集成 E2E 测试覆盖 | 🟡 P3 | ❌ 未启动 | 后续 |
| OpenAPI 3.1 规范生成 | 🟡 P3 | ❌ 未启动 | 后续 |
| product_color_price 反向 port | 🟡 P3 | ❌ 未启动 | 后续 |

**P14 批 1 范围**：1 个子代理 / 1 个 PR（I-2） / 预计 5-6 commit

---

## 二、子任务详细计划

### 2.1 子代理 I-2：拆分 3 个次大 .vue

**目标**：延续 I-1 模式，拆分 3 个 700-900 行的 .vue 文件为多个子组件 + composables

**预计文件数**：3 父文件 → 3 父文件 + ~15-20 子文件（子组件 + composables）
**预计行数**：净增 ~1500-2000 行（拆分需要新模板/样式）
**预计 commit 数**：5-6（每个大文件 1 commit + 2 修复 commit）

**拆分目标（3 个次大 .vue 文件，按行数倒序）**：

| 文件 | 当前行数 | 路径 | 业务领域 |
|------|---------|------|---------|
| `frontend/src/views/finance/voucher/tabs/VoucherListTab.vue` | 870 | 凭证列表 Tab | 财务 |
| `frontend/src/views/system/api-gateway/index.vue` | 835 | API 网关管理 | 系统 |
| `frontend/src/views/finance/arReconciliation/enhanced.vue` | 789 | 增强版 AR 对账 | 财务 |

**实际路径需子代理核验**（参考 I-1 经验：原 plan 路径与实际有偏差，子代理应先 ls 确认）

**拆分策略**（每文件，参考 I-1 模式）：
1. **识别边界**：表单区域 / 表格区域 / 详情区域 / 操作按钮 / 状态显示
2. **抽组件**：每个区域抽为独立 .vue（**子组件 < 300 行**）
3. **抽 composable**：业务逻辑（API 调用 / 状态管理）抽为 composables/xxx.ts（**composable < 250 行**）
4. **保留 props/emit 契约**：保证父组件 API 稳定
5. **类型化**：所有 props/emit 用 TypeScript interface

**CI 风险**（参考 I-1 4 轮迭代经验）：
- v-model on prop（必须用 `:model-value` + `@update:model-value` + emit 模式）
- TypeScript 导入路径错误（拆分时容易从原文件未更新 import）
- vue/no-mutating-props ESLint 错误（v-model 绑定对象 prop）
- 子组件 props/emit 与父组件类型不匹配

**关键约束**：
- **行为完全一致**（不改业务逻辑/UI/交互）
- 命名 ≤ 9 字符
- 中文 props/emit 注释
- 禁止 `unwrap_or(0)`（虽然前端用不到，但保持规范一致性）
- 不用 file-level `#![allow(dead_code)]`（前端不涉及）

**文件命名规范**：
- 子组件：`frontend/src/views/<父模块>/components/<父名>-<子区域>.vue`
- composable：`frontend/src/views/<父模块>/composables/use-<业务>.ts`
- 命名 ≤ 9 字符（如 `useVchr` / `useApiGw` / `useArRec` 等）

---

## 三、派发策略

### 3.1 单子代理派发

P11/P12/P13 经验：1 个子代理同时做 1 个 PR（3 个大文件）效率最高。

**P14 批 1 顺序**：
1. **子代理 I-2** → 1 个 PR
2. 等 I-2 合并后，决策是否继续派发 P15 批 1（I-3 或其它）

### 3.2 子代理 Prompt 模板

子代理需接收：
- 任务详细描述（来自本文件 §2.1）
- 项目规范摘要（MEMORY.md 关键约束）
- I-1 PR 4 轮 CI 修复经验（避免常见错误）
- CI 反馈循环指引
- PR 创建指引
- 文档同步要求

---

## 四、风险与回退

| 风险 | 等级 | 缓解 |
|------|------|------|
| I-2 子代理 1 PR 5+ commit 超时 | 中 | 拆分 I-2 为 I-2a（voucher 870） / I-2b（api-gateway 835 + arReconciliation 789）2 个 PR 串行 |
| 路径错误 | 低 | 子代理先 ls 核验实际路径（参考 I-1 经验）|
| 行为变更 | 中 | 强制行为不变 + 完整测试通过 + 关键路径 E2E 验证 |
| CI 多轮迭代 | 中 | 提前告知 4 轮经验（v-model on prop / 导入错误 / ESLint / 真实修复）|

---

## 五、文档基线

- ✅ `MEMORY.md` 进展表 15/15 PR
- ✅ `CHANGELOG.md` P13 批 1 I-1 段（869-908 行）
- ⏳ 本文档（创建中）
- ⏳ I-2 完成后追加 I-2 行到 MEMORY.md
- ⏳ I-2 完成后追加 I-2 段到 CHANGELOG.md

---

## 六、关联文档

- [2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3 — 综合路线图
- [2026-06-18-p13-batch1-comprehensive-plan.md](2026-06-18-p13-batch1-comprehensive-plan.md) — P13 批 1 详细子任务计划（参考 I-1 实施细节）
