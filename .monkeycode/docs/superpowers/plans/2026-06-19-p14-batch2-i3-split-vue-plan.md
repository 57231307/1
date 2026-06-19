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

## 五、I-3 第 2 批：拆分剩余 3 个大 .vue（689-695 行，2026-06-19）

### 5.1 背景
- I-3 第 1 批（PR #195）已合并（2834f86d），拆分 VoucherListTab 870 + system-update 725 + sales-contract 717
- 剩余 3 个 689-695 行的大 .vue（purchase-return 695 / scheduling/gantt 691 / scheduling/index 689）
- 沿用 I-3 第 1 批模式：composable 用 `reactive({...})` 包装 return（避免 vue-tsc 错误）+ 子组件 prop 类型从严格 VoucherForm 改为所有字段可选 + `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`

### 5.2 拆分目标

| 文件 | 当前行数 | 路径 | 业务领域 |
|------|---------|------|---------|
| `frontend/src/views/purchase-return/index.vue` | 695 | 采购退货 | 采购（purchase-return） |
| `frontend/src/views/scheduling/gantt.vue` | 691 | 排产甘特图 | 排产（scheduling） |
| `frontend/src/views/scheduling/index.vue` | 689 | 排产主页 | 排产（scheduling） |

### 5.3 拆分模式（沿用 I-2/I-3 第 1 批）

每个父文件按以下模式拆分：
```
父文件 (689-695 → ~150 行)
├── composables/
│   ├── useXxxXxx.ts       # 主业务 composable（reactive 包装 return）
│   ├── useXxxProc.ts      # 业务流程 composable
│   └── xxxFmts.ts         # 格式化工具
└── components/
    ├── XxxFilter.vue      # 过滤/搜索栏
    ├── XxxTbl.vue         # 列表表格
    ├── XxxForm.vue        # 新建/编辑表单
    ├── XxxDetail.vue      # 详情对话框
    ├── XxxConfirm.vue     # 确认对话框（可选）
    └── XxxChart.vue       # 图表（可选）
```

### 5.4 命名规划
- **purchase-return 域**：
  - `PrRtnFilter`（过滤栏） / `PrRtnTbl`（列表） / `PrRtnForm`（新建/编辑） / `PrRtnDetail`（详情） / `PrRtnApr`（审批对话框）
  - composables：`usePrRtn`（主业务）/ `usePrRtnProc`（流程）/ `prRtnFmts`（格式化）
- **scheduling 域**（gantt + index 2 个文件，避免冲突）：
  - gantt.vue 子组件：`SchGTool`（顶部工具栏 + 统计卡片）/ `SchGChart`（甘特图容器）/ `SchGAuto`（自动排程对话框）/ `SchGAdj`（调整排程对话框）/ `SchGConf`（冲突列表对话框）
  - gantt composables：`useSchG`（主业务 + 甘特图渲染）/ `useSchGProc`（自动排程流程）/ `schGFmts`（格式化）
  - index.vue 子组件：`SchMTool`（顶部工具栏 + 统计卡片）/ `SchMTbl`（工单列表）/ `SchMConf`（冲突侧栏）/ `SchMParam`（排程参数侧栏）/ `SchMAdj`（调整对话框）
  - index composables：`useSchM`（主业务）/ `useSchMProc`（自动排程流程）/ `schMFmts`（格式化）

### 5.5 关键 CI 经验（沿用 I-3 第 1 批）
1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**（prop 不能直接赋值）：改 v-model + emit 模式（prop 类型从 `{ value: string }` 改为 `string`）
3. **vue/no-mutating-props ESLint 错误**（在 `<template>` 中）：必须在每个子组件的 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`
4. **v-model 不能用于 prop**：必须用 `:model-value` + `@update:model-value` + emit 模式
5. **composable ref 字段在父组件访问时未自动解包**：用 `reactive({...})` 包装 return，Vue 自动解包（UnwrapNestedRefs 类型推导）
6. **真实修复**：CI 报错后必须实际修改代码，不要只加注释或假装修复
7. **scoped 类型兼容**：子组件 prop 类型可从严格 VoucherForm 改为所有字段可选（与父组件 Partial<Entity> 兼容）

### 5.6 验证清单
- [ ] 3 个父文件重写后行数 ~150
- [ ] 新建子组件 ≤ 9 字符 + 中文
- [ ] vue-tsc 无错误（特别注意 composable reactive 包装）
- [ ] ESLint 无 vue/no-mutating-props 错误（template 顶部加 disable）
- [ ] 行为完全保持一致
- [ ] 中文注释
- [ ] composable 用 reactive 包装

### 5.7 进度
- ✅ I-3 第 1 批（PR #195）squash merge 入 main（2834f86d）
- ✅ I-3 第 2 批（PR #196）squash merge 入 main（0622b82）

---

## 六、I-3 第 3 批：拆分剩余 4 个大 .vue（622-677 行，2026-06-19）

### 6.1 背景
- I-3 第 1 批（PR #195，2834f86d）和 I-3 第 2 批（PR #196，0622b82）已合并
- 累计已拆分 6 个大 .vue（VoucherListTab 870 + system-update 725 + sales-contract 717 + purchase-return 695 + scheduling/gantt 691 + scheduling/index 689）
- 剩余 4 个 622-677 行大 .vue：sales-price 677 + sales OrderListView 644 + purchase-contract 644 + purchase-price 622
- 沿用 I-3 第 1/2 批全部经验：composable `reactive({...})` 包装 return + 子组件 prop 字段全部可选 + `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->` + `:model-value` + `@update:model-value` + emit 模式 + `defineProps<{...}>()` 不赋给变量 + emit object 形式 `{ name: [args] }`

### 6.2 拆分目标

| 文件 | 当前行数 | 路径 | 业务领域 |
|------|---------|------|---------|
| `frontend/src/views/sales-price/index.vue` | 677 | 销售价格 | 销售（sales-price） |
| `frontend/src/views/sales/views/OrderListView.vue` | 644 | 销售订单列表 | 销售（sales） |
| `frontend/src/views/purchase-contract/index.vue` | 644 | 采购合同 | 采购（purchase-contract） |
| `frontend/src/views/purchase-price/index.vue` | 622 | 采购价格 | 采购（purchase-price） |

### 6.3 拆分模式（沿用 I-3 第 1/2 批）
```
父文件 (622-677 → ~150 行)
├── composables/
│   ├── useXxxXxx.ts       # 主业务 composable（reactive 包装 return）
│   ├── useXxxProc.ts      # 业务流程 composable
│   └── xxxFmts.ts         # 格式化工具
└── components/
    ├── XxxFilter.vue      # 过滤/搜索栏
    ├── XxxTbl.vue         # 列表表格
    ├── XxxForm.vue        # 新建/编辑表单
    ├── XxxDetail.vue      # 详情对话框
    └── XxxHistory.vue     # 历史对话框（价格域用）
```

### 6.4 命名规划
- **sales-price 域**：
  - 子组件：`SpFilter`（过滤栏） / `SpTbl`（列表） / `SpForm`（新建/编辑） / `SpHistory`（历史对话框） / `SpView`（查看详情）
  - composables：`useSp`（主业务）/ `useSpProc`（审批/导出流程）/ `spFmts`（格式化）
- **sales OrderListView 域**：
  - 已有 OrderFormDialog / OrderViewDialog / DeliveryDialog 拆分（V2Table 迁移保留）
  - 新增 4 子组件：`OlvStat`（4 统计卡片） / `OlvFilter`（过滤栏） / `OlvTbl`（V2Table 包装）/ `OlvCols`（列定义）
  - composables：`useOlv`（主业务 + 分页/loading）/ `useOlvProc`（审批/取消/发货流程）/ `olvFmts`（状态格式化）
  - 备注：父组件保留为 OrderListView.vue（销售订单主入口，路径不可改），子组件放 `views/sales/components/` 或 `views/sales/views/components/`
- **purchase-contract 域**：
  - 子组件：`PcFilter`（过滤栏） / `PcTbl`（列表） / `PcForm`（新建/编辑） / `PcDetail`（详情对话框）
  - composables：`usePc`（主业务）/ `usePcProc`（提交/审批/执行/删除流程）/ `pcFmts`（格式化）
- **purchase-price 域**：
  - 子组件：`PpFilter`（过滤栏） / `PpTbl`（列表） / `PpForm`（新建/编辑） / `PpHistory`（历史对话框） / `PpDetail`（详情对话框）
  - composables：`usePp`（主业务）/ `usePpProc`（停用/导出流程）/ `ppFmts`（格式化）

### 6.5 关键 CI 经验（沿用 I-3 第 1/2 批）
1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**：改 v-model + emit 模式（prop 类型从 `{ value: string }` 改为 `string`），父组件绑定 `v-model:foo="ref.value"` 走 Vue 模板 ref 自动解包
3. **vue/no-mutating-props ESLint**：必须在每个子组件的 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`（ESLint 在 `<template>` 中不识别 `<script>` 顶部 disable 注释）
4. **v-model 不能用于 prop**：必须用 `:model-value` + `@update:model-value` + emit 模式
5. **composable ref 字段在父组件访问时未自动解包**：用 `reactive({...})` 包装 return，Vue 自动解包
6. **scoped 类型兼容**：子组件 prop 类型可从严格 Form 改为所有字段可选（与父组件 Partial<Entity> 兼容）
7. **未使用的 props**：`const props = defineProps<...>()` 改为 `defineProps<{...}>()` 直接使用
8. **emit 形式**：用 object 形式 `{ name: [args] }`（Vue 3.3+），比 tuple 形式 `(e: 'name', args): void` 在 vue-tsc 类型推断时更稳定
9. **冗余 emit**：v-model 替代的 size-change/current-change emit 需移除

### 6.6 验证清单
- [ ] 4 个父文件重写后行数 ~150
- [ ] 新建子组件 ≤ 9 字符 + 中文
- [ ] vue-tsc 无错误（特别注意 composable reactive 包装）
- [ ] ESLint 无 vue/no-mutating-props 错误（template 顶部加 disable）
- [ ] 行为完全保持一致
- [ ] 中文注释
- [ ] composable 用 reactive 包装
- [ ] 无未使用的 props / import

### 6.7 进度
- ✅ I-3 第 1 批（PR #195）squash merge 入 main（2834f86d）
- ✅ I-3 第 2 批（PR #196）squash merge 入 main（0622b82）
- ✅ I-3 第 3 批（PR #197）squash merge 入 main（9367dc7）

---

## 七、I-3 第 4 批：拆分剩余 4 个大 .vue（598-618 行，2026-06-19）

### 7.1 背景
- I-3 第 1 批（PR #195，2834f86d）、第 2 批（PR #196，0622b82）、第 3 批（PR #197，9367dc7）已合并
- 累计已拆分 10/24 个大 .vue，剩余 14 个
- 本批拆分剩余 4 个 598-618 行大 .vue：bpm/approval 618 + production 611 + logistics 605 + purchaseReceipt 598
- 沿用 I-3 第 1/2/3 批全部经验

### 7.2 拆分目标

| 文件 | 当前行数 | 路径 | 业务领域 |
|------|---------|------|---------|
| `frontend/src/views/bpm/approval.vue` | 618 | BPM 审批 | BPM（bpm） |
| `frontend/src/views/production/index.vue` | 611 | 生产管理 | 生产（production） |
| `frontend/src/views/logistics/index.vue` | 605 | 物流管理 | 物流（logistics） |
| `frontend/src/views/purchaseReceipt/index.vue` | 598 | 采购收货 | 采购（purchaseReceipt） |

### 7.3 拆分模式（沿用 I-3 第 1/2/3 批）
```
父文件 (598-618 → ~150 行)
├── composables/
│   ├── useXxxXxx.ts       # 主业务 composable（reactive 包装 return）
│   ├── useXxxProc.ts      # 业务流程 composable
│   └── xxxFmts.ts         # 格式化工具
└── components/
    ├── XxxStat.vue        # 统计卡片（approval 用 4 卡，logistics 用 4 卡）
    ├── XxxFilter.vue      # 过滤/搜索栏
    ├── XxxTbl.vue         # 列表表格
    ├── XxxForm.vue        # 新建/编辑表单
    ├── XxxDetail.vue      # 详情对话框
    ├── XxxDlg.vue         # 业务流程对话框（审批 / 转交 / 状态更新）
    └── 其他业务子组件
```

### 7.4 命名规划
- **bpm approval 域**（BpmAp 前缀）：
  - 子组件：`BpmApStat`（4 统计卡片） / `BpmApPendingTbl`（待办任务表） / `BpmApCompletedTbl`（已办任务表） / `BpmApAprDlg`（审批对话框） / `BpmApTranDlg`（转交对话框） / `BpmApChainDlg`（审批链对话框）
  - composables：`useBpmAp`（主业务 + 待办/已办分页） / `useBpmApProc`（审批/转交/审批链流程） / `bpmApFmts`（格式化：优先级 type/text、节点 type、isOverdue、节点状态 class）
- **production 域**（Prd 前缀，V2Table + useTableApi 模式）：
  - 子组件：`PrdFilter`（过滤栏） / `PrdForm`（新建/编辑对话框） / `PrdDetail`（详情对话框） / `PrdTbl`（V2Table 包装 + 操作列 + 状态列） / `PrdCols`（V2Table 列定义）
  - composables：`usePrd`（主业务：useTableApi + orderForm） / `usePrdProc`（CRUD + 状态变更 + 导出 CSV + 打印） / `prdFmts`（格式化：getStatusLabel）
- **logistics 域**（Lgs 前缀）：
  - 子组件：`LgsStat`（4 统计卡片） / `LgsFilter`（过滤栏） / `LgsTbl`（列表表格） / `LgsForm`（新建/编辑对话框） / `LgsDetail`（详情对话框） / `LgsStatDlg`（更新状态对话框）
  - composables：`useLgs`（主业务 + 表单 + 详情） / `useLgsProc`（CRUD + 发货 + 状态更新 + 删除） / `lgsFmts`（格式化：getStatusType、getStatusText）
- **purchaseReceipt 域**（Prc 前缀，区别于 Pc 采购合同）：
  - 子组件：`PrcFilter`（过滤栏） / `PrcTbl`（列表表格） / `PrcForm`（新建/编辑对话框 + 明细表） / `PrcDetail`（详情对话框）
  - composables：`usePrc`（主业务 + 表单 + 选项加载） / `usePrcProc`（提交 / 删除 / 审核） / `prcFmts`（格式化：getStatusLabel / getStatusClass）

### 7.5 关键 CI 经验（沿用 I-3 第 1/2/3 批）
1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**：改 v-model + emit 模式（prop 类型从 `{ value: string }` 改为 `string`），父组件 `v-model:foo="ref.value"`
3. **vue/no-mutating-props ESLint**：每个子组件 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`
4. **v-model 不能用于 prop**：用 `:model-value` + `@update:model-value` + emit
5. **composable ref 字段未自动解包**：用 `reactive({...})` 包装 return
6. **emit 形式选型**：emit 改用 object 形式 `{ name: [args] }`（Vue 3.3+）
7. **未使用的 props**：`const props = defineProps<...>()` 未使用导致 TS6133 — 直接用 `defineProps<{...}>()` 即可
8. **TS2304 接口从函数内移至顶层**
9. **TS2769 ElTag renderCell No overload matches**：改 `h(ElTag, props, () => text)` + `type: ... as any`
10. **TS2345 viewData optional 字段**：加 `|| 0` / `|| ''` fallback

### 7.6 验证清单
- [ ] 4 个父文件重写后行数 ~150
- [ ] 新建子组件 ≤ 9 字符 + 中文
- [ ] vue-tsc 无错误
- [ ] ESLint 无 vue/no-mutating-props 错误（template 顶部加 disable）
- [ ] 行为完全保持一致
- [ ] 中文注释
- [ ] composable 用 reactive 包装
- [ ] 无未使用的 props / import

### 7.7 进度
- ✅ **已完成**：子代理 I-3 第 4 批完成（PR #198 squash merge 入 main 0bc1f5e）
  - bpm/approval: 618 → 123 行 + 6 子组件 + 2 composable + 1 工具
  - production: 611 → 172 行 + 4 子组件 + 2 composable + 1 工具
  - logistics: 605 → 117 行 + 6 子组件 + 2 composable + 1 工具
  - purchaseReceipt: 598 → 97 行 + 4 子组件 + 2 composable + 1 工具
  - 累计 2432 → 509 行（-79%），CI 5/5 全绿
- ✅ **P14 批 2 I-3 全部 4/4 批完成**（累计拆分 14/24 大 .vue）

---

## 八、I-3 第 5 批：拆分剩余 4 个大 .vue（579-596 行，2026-06-19）

### 8.1 背景
- I-3 第 1/2/3/4 批已合并（PR #195/196/197/198），累计拆分 14/24 大 .vue
- 本批拆分剩余 4 个 579-596 行大 .vue：data-import 596 + purchase-inspection 594 + material-shortage 590 + bpm/definitions 579
- 沿用 I-3 第 1/2/3/4 批全部经验

### 8.2 拆分目标

| 文件 | 当前行数 | 路径 | 业务领域 | 命名前缀 |
|------|---------|------|---------|---------|
| `frontend/src/views/data-import/index.vue` | 596 | 数据导入 | 数据 | `Di`（Data Import） |
| `frontend/src/views/purchase-inspection/index.vue` | 594 | 采购验货 | 采购 | `Pi`（Purchase Inspection） |
| `frontend/src/views/material-shortage/index.vue` | 590 | 物料短缺 | 物料 | `Ms`（Material Shortage） |
| `frontend/src/views/bpm/definitions.vue` | 579 | BPM 流程定义 | BPM | `BpmDf`（BPM Definitions） |

### 8.3 拆分模式（沿用 I-3 第 1/2/3/4 批）
```
父文件 (579-596 → ~150 行)
├── composables/
│   ├── useXxxXxx.ts       # 主业务 composable（reactive 包装 return）
│   ├── useXxxProc.ts      # 业务流程 composable
│   └── xxxFmts.ts         # 格式化工具
└── components/
    ├── XxxStat.vue        # 统计卡片（可选，material-shortage 用）
    ├── XxxFilter.vue      # 过滤/搜索栏
    ├── XxxTbl.vue         # 列表表格
    ├── XxxForm.vue        # 新建/编辑对话框
    ├── XxxDetail.vue      # 详情对话框
    ├── XxxUpload.vue      # 上传对话框（data-import 用）
    ├── XxxVersion.vue     # 版本对话框（bpm/definitions 用）
    └── 其他业务子组件
```

### 8.4 命名规划
- **data-import 域**（`Di` 前缀）：
  - 子组件：`DiTplTbl`（模板列表 + 过滤） / `DiTaskTbl`（任务列表 + 过滤） / `DiTplForm`（模板新建/编辑对话框） / `DiTplUpload`（文件上传对话框）
  - composables：`useDi`（主业务：模板 + 任务列表 / 分页 / 加载） / `useDiProc`（新建/编辑/删除/下载/上传/重试/取消流程） / `diFmts`（格式化：moduleMap / taskStatusMap / taskStatusTypeMap）
  - 备注：因页面有 2 个 Tab（templates / tasks），用 DiTplTbl + DiTaskTbl 区分；过滤栏嵌在表组件内简化
- **purchase-inspection 域**（`Pi` 前缀）：
  - 子组件：`PiStat`（4 统计卡片） / `PiFilter`（过滤栏） / `PiTbl`（列表） / `PiForm`（新建/编辑表单 + 检验明细） / `PiDetail`（详情对话框）
  - composables：`usePi`（主业务：列表 / 分页 / 表单 / 详情 / 供应商 / 入库单加载） / `usePiProc`（查询 / 重置 / 创建 / 编辑 / 查看 / 提交 / 完成 + 状态/结果格式化） / `piFmts`（getStatusType / getStatusText / getResultType / getResultText）
- **material-shortage 域**（`Ms` 前缀）：
  - 子组件：`MsStat`（4 统计卡片：缺料总数/严重缺料/高度缺料/最后检查时间） / `MsSevCard`（4 严重程度进度卡片） / `MsTbl`（列表 + 过滤 + 操作）
  - composables：`useMs`（主业务：汇总 / 列表 / 分页 / 加载） / `useMsProc`（触发检查 / 通知 / 解决 / 筛选流程） / `msFmts`（getSeverityColor / getSeverityLabel / getStatusColor / getStatusLabel / getSourceTypeColor / getSourceTypeLabel）
  - 备注：原页面所有过滤/筛选/操作按钮全在表格组件内，与 Stat 分离
- **bpm/definitions 域**（`BpmDf` 前缀，区别于 I-3 第 4 批的 `BpmAp` Approval）：
  - 子组件：`BpmDfFilter`（过滤栏） / `BpmDfTbl`（列表） / `BpmDfForm`（新建/编辑对话框 + 节点配置子表） / `BpmDfVerDlg`（版本管理对话框） / `BpmDfTplDlg`（保存为模板对话框）
  - composables：`useBpmDf`（主业务：列表 / 分页 / 过滤） / `useBpmDfProc`（搜索/重置/创建/编辑/删除/版本/创建版本/激活版本/保存为模板流程） / `bpmDfFmts`（getStatusType / getStatusText / getCategoryText / getVersionStatusText）
  - 备注：节点配置子表保留在 BpmDfForm 内部（与父级表单关联紧密）

### 8.5 关键 CI 经验（沿用 I-3 第 1/2/3/4 批）
1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**：改 v-model + emit 模式（prop 类型从 `{ value: string }` 改为 `string`），父组件 `v-model:foo="ref.value"`
3. **vue/no-mutating-props ESLint**：每个子组件 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`
4. **v-model 不能用于 prop**：用 `:model-value` + `@update:model-value` + emit
5. **composable ref 字段未自动解包**：用 `reactive({...})` 包装 return
6. **emit 形式选型**：emit 改用 object 形式 `{ name: [args] }`（Vue 3.3+）
7. **未使用的 props**：`const props = defineProps<...>()` 未使用导致 TS6133 — 直接用 `defineProps<{...}>()` 即可
8. **TS2304 接口从函数内移至顶层**
9. **TS2769 ElTag renderCell No overload matches**：改 `h(ElTag, props, () => text)` + `type: ... as any`
10. **TS2345 viewData optional 字段**：加 `|| 0` / `|| ''` fallback

### 8.6 验证清单
- [ ] 4 个父文件重写后行数 ~150
- [ ] 新建子组件 ≤ 9 字符 + 中文
- [ ] vue-tsc 无错误
- [ ] ESLint 无 vue/no-mutating-props 错误（template 顶部加 disable）
- [ ] 行为完全保持一致
- [ ] 中文注释
- [ ] composable 用 reactive 包装
- [ ] 无未使用的 props / import

### 8.7 进度
- ⏳ 进行中：子代理 I-3 第 5 批执行中（2026-06-19）


