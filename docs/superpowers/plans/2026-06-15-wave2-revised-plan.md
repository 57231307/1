# Wave 2 重新评估计划 - 2026-06-15

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 B0 实测数据，重新评估并修订 Wave 2 任务清单，提升执行效率

**Architecture:** 4 个并行 B 类前端子代理（B3 + B4 + B5 + B6）+ 1 个复查子代理
**Tech Stack:** Vue 3 / TypeScript / Element Plus 2.4+ / Vite 5

---

## 0. 颠覆性发现：B0 实测数据

### 0.1 原计划假设 vs 实测数据

| 项 | 原计划 | B0 实测 | 差异 |
|------|--------|---------|------|
| 真正孤儿 API 文件 | 假设 118 | **0 个** | ✅ 所有 78 个 API 文件均被引用 |
| 仅 API 实现无前端页面 | 假设 118 个 | **0 个** | ✅ 全部有对应页面 |
| 需补充前端页面的 API | 假设 118 | **0 个** | ✅ 不存在此需求 |
| 待处理未用函数 | 未评估 | **8 个**（budget 2 + cost 6） | 🆕 新发现 |

### 0.2 关键修正

**P1-6 任务（补齐 118 个仅 API 实现的前端页面）：完全取消**

理由：
- B0 准确扫描（含绝对路径 `@/api/` 和相对路径 `../../api/` 双重验证）显示 **0 个孤儿 API**
- 5 个 .vue 文件使用相对路径（mrp / budget / cost / fund / 1 个未列出），第一次扫描漏判
- 80 个 API 文件均被前端页面正常引用
- 449 个 API 函数（跨 80 文件）中绝大部分被调用

**P1-6 替代任务：B6 清理未用 API 函数（8 处）**

- `frontend/src/api/budget.ts`：`getBudget`、`approveBudget` 2 个未用函数
- `frontend/src/api/cost.ts`：`listCollections`、`getCollection`、`createCollection`、`updateCollection`、`deleteCollection`、`auditCollection` 6 个未用函数

### 0.3 B0 扫描方法（双路径验证）

```bash
# 绝对路径扫描
grep -rl "from.*@/api/${name}" /workspace/frontend/src

# 相对路径扫描
grep -rl "from.*['\"]\\.\\..*/api/${name}['\"]" /workspace/frontend/src

# 对象导出名引用扫描
grep -rl "\b${constname}\b" /workspace/frontend/src/views /workspace/frontend/src/components
```

---

## 1. 修订后 Wave 2 任务清单

| 任务 ID | 任务名称 | 子代理数 | 周期 | 优先级 | 状态 |
|---------|---------|---------|------|--------|------|
| **B3-1** | 拆分 6 个 > 1000 行巨型 .vue | 1 | 1 周 | 🟠 P1 | 🟡 |
| **B3-2** | 拆分财务/会计域 12 个 .vue | 1 | 1 周 | 🟠 P1 | 🟡 |
| **B3-3** | 拆分 CRM/客户域 8 个 .vue | 1 | 0.5 周 | 🟡 P2 | 🟡 |
| **B3-4** | 拆分库存/产品域 8 个 .vue | 1 | 0.5 周 | 🟡 P2 | 🟡 |
| **B4** | 完成 system/index.vue 10 Tab 骨架 | 1 | 1 周 | 🟠 P1 | 🟡 |
| **B5** | P2-1 el-table-v2 虚拟列表 POC | 1 | 1 周 | 🟠 P1 | 🟡 |
| **B6** | 清理 budget.ts / cost.ts 未用函数 | 1 | 0.5 天 | 🟢 P2 | 🟡 🆕 |
| **复查** | 代码质量审查 | 1 | 0.5 天 | 🟠 P1 | 🟡 |
| **收尾** | CI 全绿 + CHANGELOG | 1 | 0.5 天 | 🟠 P1 | 🟡 |

**总资源**：6 个执行子代理 + 1 个复查 = **7 个**，并行峰值 6
**总周期**：2 周（10 工作日）

---

## 2. 详细任务定义

### Task 1: B3-1 - 拆分 6 个 > 1000 行巨型 .vue 文件

**Files:**
- Modify: `frontend/src/views/system/index.vue` (1521 行)
- Modify: `frontend/src/views/purchase-ext/index.vue` (1151 行)
- Modify: `frontend/src/views/sales-ext/index.vue` (1148 行)
- Modify: `frontend/src/views/sales/index.vue` (1125 行)
- Modify: `frontend/src/views/ap/index.vue` (1035 行)
- Modify: `frontend/src/views/trading/index.vue` (1034 行)

**拆分原则**（沿用 P3/P4 阶段已建立的模板）：

- [ ] **Step 1**：参照 [docs/refactoring/frontend-vue-splitting-plan.md](file:///workspace/docs/refactoring/frontend-vue-splitting-plan.md) 模板
- [ ] **Step 2**：每个大文件拆分为「主入口 + 子组件」
- [ ] **Step 3**：主入口只保留 Tab 切换逻辑 + 公共数据
- [ ] **Step 4**：业务逻辑迁入子组件，props/emit 通信
- [ ] **Step 5**：保持原有 `name` 不变，路由零影响
- [ ] **Step 6**：每个子组件 < 300 行
- [ ] **Step 7**：跑 `npm run type-check` + `npm run lint` 验证 0 错误
- [ ] **Step 8**：本地启动 dev server，手动验证 Tab 切换
- [ ] **Step 9**：commit + push 到 `feature/B3-1-split-6-large-vue`

**拆分模板**（参考 `system/index.vue` 已完成的拆分）：

```vue
<!-- 主入口 system/index.vue -->
<template>
  <el-tabs v-model="activeTab">
    <el-tab-pane label="用户" name="user">
      <UserTab />
    </el-tab-pane>
    <!-- ... 其他 Tab -->
  </el-tabs>
</template>
<script setup lang="ts">
import { ref } from 'vue'
import UserTab from './tabs/UserTab.vue'
// ... 其他 import
const activeTab = ref('user')
</script>
```

### Task 2: B3-2 - 拆分财务/会计域 12 个 .vue 文件

**Files:** 12 个 500-1000 行的财务/会计域文件
- `frontend/src/views/ar/index.vue` (967)
- `frontend/src/views/finance/index.vue` (867)
- `frontend/src/views/voucher/index.vue` (842)
- `frontend/src/views/fund/index.vue` (826)
- `frontend/src/views/financeReport/index.vue`
- `frontend/src/views/financial-analysis/index.vue`
- `frontend/src/views/currency/index.vue`
- `frontend/src/views/fixed-assets/index.vue`
- `frontend/src/views/budget/index.vue`
- `frontend/src/views/cost/index.vue`
- `frontend/src/views/accountSubject/index.vue` (536)
- `frontend/src/views/accountingPeriod/index.vue` (521)

**执行步骤同 B3-1**

### Task 3: B3-3 - 拆分 CRM/客户域 8 个 .vue 文件

**Files:** CRM/客户域
- `frontend/src/views/crm/index.vue` (668)
- `frontend/src/views/crm/detail.vue` (663)
- `frontend/src/views/crm/opportunities/index.vue` (602)
- `frontend/src/views/crm/leads/index.vue` (595)
- `frontend/src/views/customer/index.vue` (551)
- `frontend/src/views/customerCredit/index.vue`
- `frontend/src/views/crm/pool.vue` (485)
- `frontend/src/views/crm/assignment.vue`

### Task 4: B3-4 - 拆分库存/产品域 8 个 .vue 文件

**Files:** 库存/产品域
- `frontend/src/views/inventory/index.vue` (915)
- `frontend/src/views/product/index.vue` (847)
- `frontend/src/views/quality/index.vue` (800)
- `frontend/src/views/fabric/index.vue` (729)
- `frontend/src/views/inventoryTransfer/index.vue` (600)
- `frontend/src/views/inventoryBatch/index.vue` (517)
- `frontend/src/views/inventoryAdjustment/index.vue` (557)
- `frontend/src/views/inventoryCount/index.vue` (457)

### Task 5: B4 - 完成 system/index.vue 剩余 10 Tab 骨架

**Files:** 10 个骨架 Tab 文件，每个 27 行
- `frontend/src/views/system/tabs/DepartmentTab.vue`
- `frontend/src/views/system/tabs/DataPermissionTab.vue`
- `frontend/src/views/system/tabs/FieldPermissionTab.vue`
- `frontend/src/views/system/tabs/NotificationTab.vue`
- `frontend/src/views/system/tabs/PermissionTab.vue`
- `frontend/src/views/system/tabs/AuditTab.vue`
- `frontend/src/views/system/tabs/WebhookTab.vue`
- `frontend/src/views/system/tabs/SystemUpdateTab.vue`
- `frontend/src/views/system/tabs/TenantTab.vue`
- `frontend/src/views/system/tabs/CompanyTab.vue`

**已完成模板**（参考）：
- [UserTab.vue](file:///workspace/frontend/src/views/system/tabs/UserTab.vue)（279 行）
- [RoleTab.vue](file:///workspace/frontend/src/views/system/tabs/RoleTab.vue)（270 行）

- [ ] **Step 1**：每个 Tab 仿照 UserTab.vue/RoleTab.vue 实现 CRUD
- [ ] **Step 2**：从原 [system/index.vue](file:///workspace/frontend/src/views/system/index.vue) 提取相关 setup 逻辑
- [ ] **Step 3**：移除文件顶部 `// TODO: 从原 system/index.vue 抽取 XXX 相关 setup 逻辑` 注释
- [ ] **Step 4**：每个 Tab 完整可工作（CRUD + 列表 + 表单 + 权限）
- [ ] **Step 5**：跑 `npm run type-check` + `npm run lint` 0 错误
- [ ] **Step 6**：commit + push 到 `feature/B4-complete-10-tabs`

**模板示例**（参考 RoleTab.vue）：

```vue
<template>
  <div class="role-tab">
    <el-card>
      <el-button type="primary" @click="handleCreate">新建角色</el-button>
      <el-table :data="roles" v-loading="loading">
        <el-table-column prop="name" label="角色名称" />
        <el-table-column prop="code" label="角色编码" />
        <el-table-column label="操作">
          <template #default="{ row }">
            <el-button @click="handleEdit(row)">编辑</el-button>
            <el-button type="danger" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
    <!-- 编辑对话框 -->
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { listRoles, createRole, updateRole, deleteRole, type Role } from '@/api/role'
import { ElMessage, ElMessageBox } from 'element-plus'

const roles = ref<Role[]>([])
const loading = ref(false)
// ... CRUD 逻辑
onMounted(() => loadData())
</script>
```

### Task 6: B5 - P2-1 el-table-v2 虚拟列表 POC

**Files:**
- Modify: `frontend/src/views/inventory/index.vue` (POC 目标)
- Create: `frontend/src/composables/useVirtualTable.ts`（可选 composable）

**POC 范围（3 个高优先级页面）**：

| 页面 | 文件 | 当前实现 | POC 后 |
|------|------|---------|--------|
| 库存台账 | `inventory/index.vue` | `el-table` | `el-table-v2` |
| 凭证管理 | `voucher/index.vue` | `el-table` | `el-table-v2` |
| 财务管理 | `finance/index.vue` | `el-table` | `el-table-v2` |

- [ ] **Step 1**：在 `inventory/index.vue` 引入 `el-table-v2` 替换 `el-table`
- [ ] **Step 2**：保持原有列定义、筛选、分页逻辑
- [ ] **Step 3**：造 1 万行测试数据验证渲染性能
- [ ] **Step 4**：使用 Chrome DevTools Performance 录制对比：
  - 初次渲染时间
  - 滚动 FPS
  - 内存占用
- [ ] **Step 5**：输出 POC 报告（HTML 或 Markdown）
- [ ] **Step 6**：通过 → 推广到 voucher / finance 2 个页面
- [ ] **Step 7**：未通过 → 回退 + 调研 vue-virtual-scroller
- [ ] **Step 8**：commit + push 到 `feature/B5-p2-1-virtual-table`

**POC 通过标准**：
- 1 万行数据初次渲染 < 500ms
- 滚动 FPS ≥ 50
- 内存占用 < 100MB

### Task 7: B6 - 清理 budget.ts / cost.ts 未用 API 函数

**Files:**
- Modify: `frontend/src/api/budget.ts`（移除 2 个未用函数）
- Modify: `frontend/src/api/cost.ts`（移除 6 个未用函数）

**B0 实测数据**：

| 文件 | 未用函数 | 状态 |
|------|---------|------|
| budget.ts | `getBudget` | 0 引用 |
| budget.ts | `approveBudget` | 0 引用（但后端有端点）|
| cost.ts | `listCollections` | 0 引用（别名 `listCostCollections` 被引用 1 次）|
| cost.ts | `getCollection` | 0 引用 |
| cost.ts | `createCollection` | 0 引用（别名 `createCostCollection` 被引用 1 次）|
| cost.ts | `updateCollection` | 0 引用（别名 `updateCostCollection` 被引用 1 次）|
| cost.ts | `deleteCollection` | 0 引用 |
| cost.ts | `auditCollection` | 0 引用 |

**评估**：
- `getBudget` / `deleteCollection` / `auditCollection` 等：后端有端点但前端未用 → **保守删除**（不轻易移除可能后续需要）
- `listCollections` / `getCollection` / `createCollection` / `updateCollection`：被 `listCostCollections` 等别名替代 → **清理重复**

- [ ] **Step 1**：与用户确认"是否删除"（因后端有端点）
- [ ] **Step 2**：删除 `budget.ts` 中 `getBudget` 和 `approveBudget`（如果用户同意）
- [ ] **Step 3**：删除 `cost.ts` 中 `listCollections` / `getCollection` / `createCollection` / `updateCollection`（保留别名）
- [ ] **Step 4**：评估 `deleteCollection` / `auditCollection`（保守策略：加 `#[allow(dead_code)]` 注释）
- [ ] **Step 5**：跑 `npm run type-check` + `npm run lint`
- [ ] **Step 6**：commit + push 到 `feature/B6-cleanup-unused-api`

**保守策略模板**（参考 P3.4 阶段 utils/ 模板）：

```typescript
// TODO(tech-debt): 前端接入后移除
/** @allow-unused */
export const deleteCollection = (id: number) =>
  request.delete(`/production/cost-collections/${id}`)
```

### Task 8: 复查子代理

- [ ] **Step 1**：审查所有 B3-1 ~ B6 提交的代码质量
- [ ] **Step 2**：运行 `npm run type-check` 确认 0 错误
- [ ] **Step 3**：运行 `npm run lint` 确认 0 错误
- [ ] **Step 4**：本地启动 dev server，验证关键页面（system / inventory / voucher / finance / crm）
- [ ] **Step 5**：审查清单（10 项）：
  1. 代码规范（[.eslintrc.cjs](file:///workspace/frontend/.eslintrc.cjs)）
  2. `any` 类型禁用
  3. console.* 替换为 logger（[utils/logger.ts](file:///workspace/frontend/src/utils/logger.ts)）
  4. TypeScript strict 模式
  5. 租户隔离（无 `unwrap_or(0)` 等价物）
  6. 错误处理（[utils/error.ts](file:///workspace/backend/src/utils/error.rs) 统一）
  7. 文档（CHANGELOG 更新）
  8. 死代码清理
  9. PR 模板填写
  10. CI 绿色

### Task 9: 收尾

- [ ] **Step 1**：所有 PR Squash merge 到 main
- [ ] **Step 2**：CI #N 全绿验证（cargo check / npm test / cargo clippy / eslint）
- [ ] **Step 3**：CHANGELOG.md 更新（[Unreleased] - 2026-06-15 增加 "Wave 2 合并汇总"）
- [ ] **Step 4**：本地 + 远端工作分支清理
- [ ] **Step 5**：GitHub Release 自动发布（CICD 触发）
- [ ] **Step 6**：更新 MEMORY.md [Wave 2 执行结果] 条目

---

## 3. 调度时序

```
Day 1-2 (并行):
  ├─ B3-1 (1 子代理): 6 个 > 1000 行文件
  ├─ B3-2 (1 子代理): 财务 12 个文件
  ├─ B3-3 (1 子代理): CRM 8 个文件
  ├─ B3-4 (1 子代理): 库存 8 个文件
  ├─ B4   (1 子代理): 10 Tab 骨架
  └─ B5   (1 子代理): el-table-v2 POC
                       ↑ 6 子代理并行峰值

Day 3:
  └─ B6   (1 子代理): 清理 8 个未用 API 函数（半天）

Day 4:
  └─ 复查子代理 (1 子代理): 全量代码审查

Day 5:
  └─ 收尾：CI + CHANGELOG + PR 合并
```

---

## 4. 风险与缓解

| 风险 | 等级 | 缓解 |
|------|------|------|
| 34 个 .vue 并行拆分 → Git 冲突 | 🟠 中 | 按业务域隔离，B3-1/2/3/4 文件无重叠 |
| 拆分后类型不兼容 | 🟡 低 | 强制 `npm run type-check` 通过 |
| 拆分后路由失效 | 🟡 低 | 保留原 `name`，子组件 lazy load |
| el-table-v2 与 el-table 功能差异 | 🟠 中 | POC 阶段先验证 1 个页面，失败回退 |
| budget/cost 函数被误删 | 🟡 低 | 保守策略 + TODO 注释 + 用户确认 |

---

## 5. 验收标准

### 5.1 拆分任务验收
- [ ] 51 个 > 500 行 .vue 文件全部 < 500 行
- [ ] 6 个 > 1000 行巨型文件全部 < 500 行
- [ ] 主入口 < 200 行（仅 Tab 切换）
- [ ] 行为 100% 兼容（手动回归测试）

### 5.2 Tab 骨架完成
- [ ] 10 个 Tab 全部含 CRUD/列表/表单
- [ ] 与 UserTab/RoleTab 模式一致
- [ ] 移除全部 TODO 注释

### 5.3 虚拟列表 POC
- [ ] 1 万行数据初次渲染 < 500ms
- [ ] 滚动 FPS ≥ 50
- [ ] 内存占用 < 100MB
- [ ] POC 报告输出

### 5.4 API 函数清理
- [ ] budget.ts 未用函数：0 个（或加 TODO 注释）
- [ ] cost.ts 重复别名：清理
- [ ] type-check 0 错误

### 5.5 CI/CD
- [ ] `npm run type-check` 0 错误
- [ ] `npm run lint` 0 错误
- [ ] `npm run build` 成功
- [ ] 6 个 PR 全部合并入 main
- [ ] GitHub Actions 全绿

---

## 6. Wave 3 启动条件

**P2-1 el-table-v2 POC 通过**：

| 条件 | 阈值 |
|------|------|
| 初次渲染（1 万行） | < 500ms |
| 滚动 FPS | ≥ 50 |
| 内存占用 | < 100MB |
| 兼容性 | el-table 95% 功能覆盖 |

**通过**：Wave 3 启动 P2-2（150 处 console.* → logger）+ P2-4（AI 深化）
**未通过**：回退 vue-virtual-scroller 方案 + 重新评估

## 7. Wave 4 启动条件

**至少 1 个 P3 任务完成 PoC 验证**：

- P3-1（微服务）：至少完成 1 个业务域（如 crm-service）的拆分 PoC
- P3-2（WebSocket）：至少完成 1 个场景（如通知推送）的端到端 PoC
- P3-3（React Native）：至少完成 1 个核心页面（如库存查询）的 RN 移植 PoC
- P3-4（数据仓库）：至少完成 1 张核心表（如 sales_orders）的 CDC 同步 + OLAP 查询 PoC

任一 PoC 通过 → Wave 4 启动

---

## 8. 与原计划的关键差异

| 维度 | 原 13 任务规划 | B0 后修订 |
|------|--------------|----------|
| P1-6 范围 | 118 个仅 API 页面 | **0 个**（取消） |
| 孤儿 API 文件 | 假设多个 | **0 个**（B0 实测） |
| 新增 B6 任务 | 无 | **8 个未用 API 函数清理** |
| 总子代理数 | 13 + 1 复查 | 7 + 1 复查（-46%） |
| Wave 2 周期 | 2 周 | 2 周（保持） |
| Wave 3 启动条件 | 未明确定义 | **el-table-v2 POC 通过** |
| Wave 4 启动条件 | 未明确定义 | **≥1 个 P3 PoC 通过** |

---

## 9. 总结

基于 B0 实测数据，Wave 2 计划已从 13 个任务大幅精简到 6 个执行子代理 + 1 个复查，剔除了基于错误假设的 P1-6（118 个仅 API 页面），新增了基于实测的 B6（8 个未用 API 函数清理）。同时明确了 Wave 3 / Wave 4 的启动条件。

**立即可执行**：6 个 B 类前端子代理并行启动，按 B3-1 → B3-4 → B4 → B5 → B6 顺序派发。
