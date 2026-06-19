# 2026-06-19 P14 批 2 - I-3 拆分大 .vue 综合推进计划

> **创建日期**：2026-06-19
> **基线版本**：main @ 2f508cb（P14 批 1 I-2 PR #194 收尾后）
> **关联路线图**：[2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3
> **派发策略**：单子代理派发 I-3 PR 第 1 批（拆分 3 个大 .vue）
> **总目标**：完成 roadmap 剩余 P3 候选任务（I-3 第 1 批是最高 P3 优先级）

---

## 一、背景

P12 批 1+2+3（12 PR）+ P13 批 1（3 PR）+ P14 批 1 I-2（PR #194）共 **16/16 PR 全部完成**。

roadmap v0.3 §二 节剩余候选任务（按 roadmap 列出顺序）：

| 任务 ID | 任务名 | 优先级 | 当前状态 | 派发顺序 |
|---------|--------|--------|---------|----------|
| **I-3 PR 第 1 批** | 拆分 3 个大 .vue | 🟡 **P3** | ❌ 未启动 | **本批派发** |
| I-3 PR 第 2 批 | 拆分剩余 3-4 个大 .vue | 🟡 P3 | ❌ 未启动 | 后续 |
| I-3 PR 第 3 批 | 拆分剩余 3-4 个大 .vue | 🟡 P3 | ❌ 未启动 | 后续 |
| B4 完成 10 Tab 业务骨架 | 🟡 P3 | ❌ 未启动 | 后续 |
| 集成 E2E 测试覆盖 | 🟡 P3 | ❌ 未启动 | 后续 |
| OpenAPI 3.1 规范生成 | 🟡 P3 | ❌ 未启动 | 后续 |
| product_color_price 反向 port | 🟡 P3 | ❌ 未启动 | 后续 |

**P14 批 2 范围**：1 个子代理 / 1 个 PR（I-3 第 1 批）/ 预计 5-6 commit

---

## 二、子任务详细计划

### 2.1 子代理 I-3 第 1 批：拆分 3 个大 .vue（>700 行）

**目标**：延续 I-1（PR #193）和 I-2（PR #194）模式，拆分 3 个 700+ 行的 .vue 文件为多个子组件 + composables

**预计文件数**：3 父文件 → 3 父文件 + ~15-20 子文件（子组件 + composables）
**预计行数**：净增 ~1500-2000 行（拆分需要新模板/样式）
**预计 commit 数**：5-6（每个大文件 1 commit + 2 修复 commit）

**拆分目标（3 个最大 .vue 文件，按行数倒序）**：

| 文件 | 当前行数 | 路径 | 业务领域 |
|------|---------|------|---------|
| `frontend/src/views/voucher/tabs/VoucherListTab.vue` | 870 | 凭证列表 Tab | 财务（voucher） |
| `frontend/src/views/system-update/index.vue` | 725 | 系统更新管理 | 系统（system-update） |
| `frontend/src/views/sales-contract/index.vue` | 717 | 销售合同管理 | 销售（sales-contract） |

**重要提示**：
- 凭证列表 Tab 在 voucher 域下，但路径是 `voucher/tabs/VoucherListTab.vue`（与 I-2 拆分的 `finance/tabs/VoucherTab.vue` 是不同文件）
- I-2 已拆分的 `VoucherTab.vue`（117 行）是子组件，本次再拆 `VoucherListTab.vue`（870 行）是父级
- 命名 ≤ 9 字符 + 中文注释 + 行为完全保持一致

### 2.2 拆分模板（沿用 I-1/I-2 经验）

每个父文件按以下模式拆分：

```
父文件 (870 → ~150 行)
├── composables/
│   ├── useXxxXxx.ts       # 主业务 composable
│   ├── useXxxProc.ts      # 业务流程 composable
│   └── xxxFmts.ts         # 格式化工具
└── components/ 或 tabs/
    ├── XxxFilter.vue      # 过滤/搜索栏
    ├── XxxTbl.vue         # 列表表格
    ├── XxxForm.vue        # 新建/编辑表单
    ├── XxxDetail.vue      # 详情对话框
    ├── XxxConfirm.vue     # 确认对话框（可选）
    └── XxxChart.vue       # 图表（可选）
```

### 2.3 CI 经验教训（必读，避免重蹈 I-2 覆辙）

**已知问题与预防**：
1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**（prop 不能直接赋值）：
   - 改 v-model + emit 模式（prop 类型从 `{ value: string }` 改为 `string`）
   - 父组件绑定 `v-model:foo="ref.value"` 走 Vue 模板 ref 自动解包
3. **vue/no-mutating-props ESLint 错误**（在 `<template>` 中）：
   - **ESLint 在 `<template>` 中不识别 `<script>` 顶部的 disable 注释**
   - 必须在每个子组件的 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`
4. **v-model 不能用于 prop**：必须用 `:model-value` + `@update:model-value` + emit 模式

### 2.4 验证清单

子代理完成提交前必须自检：
- [ ] vue-tsc 无错误（本地或推送到 CI 验证）
- [ ] ESLint 无 vue/no-mutating-props 错误
- [ ] 所有新组件命名 ≤ 9 字符
- [ ] 行为完全保持一致（无 UI/交互变化）
- [ ] 主代理审核 diff 后方可推送触发 CI

---

## 三、PR 模板

**PR 标题**：`refactor(frontend): B3 拆分 3 个大 .vue 文件（voucher 870 + system-update 725 + sales-contract 717）(P14 批 2 I-3 第 1 批)`

**PR 描述**：
```
拆分 3 个大 .vue（行为完全保持一致，纯结构重构）：
- VoucherListTab.vue: 870 → ~150 行 + 4 子组件 + 3 composable + 1 工具
- system-update/index.vue: 725 → ~130 行 + 4 子组件 + 2 composable + 1 工具
- sales-contract/index.vue: 717 → ~130 行 + 4 子组件 + 2 composable + 1 工具

CI: 5/5 全绿（构建后端/构建前端/前端测试/运行测试/前端类型检查）
```

---

## 四、后续规划

- **P15 批 1**（I-3 第 2 批）：拆分剩余 3-4 个大 .vue（purchase-return 695 / scheduling/gantt 691 / scheduling/index.vue 689 / sales-price 677）
- **P15 批 2**（I-3 第 3 批）：拆分剩余 3-4 个大 .vue
- **P16 批 1**：B4 完成 10 Tab 业务骨架
- **P16 批 2**：集成 E2E 测试覆盖
- **P17 批 1**：OpenAPI 3.1 规范生成
- **P17 批 2**：product_color_price 反向 port
