# 2026-06-19 P14 批 2 I-3 第 1 批 - VoucherListTab.vue 拆分计划

> **创建日期**：2026-06-19
> **基线版本**：main @ 2f508cb（P14 批 1 I-2 收尾后）
> **关联路线图**：[2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3
> **父计划**：[2026-06-19-p14-batch2-i3-split-vue-plan.md](2026-06-19-p14-batch2-i3-split-vue-plan.md)
> **任务编号**：P14 批 2 I-3 第 1 批

---

## 一、目标

拆分 `frontend/src/views/voucher/tabs/VoucherListTab.vue`（870 行）为：

| 类型 | 文件 | 职责 |
|------|------|------|
| 父文件 | `voucher/tabs/VoucherListTab.vue` | 组件组合 + 状态管理（870 → ~150 行） |
| composable | `voucher/tabs/composables/useVchrLst.ts` | 主业务（列表查询/表单/科目加载） |
| composable | `voucher/tabs/composables/useVchrLstProc.ts` | 业务流程（审核/记账/反记账/打印/导出） |
| 工具 | `voucher/tabs/composables/vchrLstFmts.ts` | 状态标签/类型映射/格式化 |
| 子组件 | `voucher/tabs/components/VchrLstFilter.vue` | 过滤栏 |
| 子组件 | `voucher/tabs/components/VchrLstTbl.vue` | 列表表格 |
| 子组件 | `voucher/tabs/components/VchrLstForm.vue` | 新建/编辑对话框 |
| 子组件 | `voucher/tabs/components/VchrLstDetail.vue` | 详情对话框 |

**预计 commit 数**：1（主拆分 commit + 可能 CI 修复 commit）

---

## 二、命名规范

- composable：`useVchrLst` / `useVchrLstProc`（≤ 9 字符 + 描述性）
- 工具：`vchrLstFmts`
- 子组件：`VchrLstFilter` / `VchrLstTbl` / `VchrLstForm` / `VchrLstDetail`（≤ 9 字符）

**重要**：与 I-2 已拆分的 `VchrFilter` / `VchrTbl` / `VchrForm` / `VchrDetail` **不同**（那是 `finance/tabs/` 下的，使用 `useVchr`），本次是 `voucher/tabs/` 下的凭证列表 Tab，使用 `VchrLst` 前缀。

---

## 三、行为保持

**完全保持**：
- 表格列（凭证号/日期/类型/借方/贷方/状态/制单人/审核人/记账人/操作）
- 操作按钮（查看/编辑/审核/记账/反记账/删除，按状态显示）
- 过滤栏（凭证号/开始日期/结束日期/状态）
- 借方/贷方合计计算
- 借贷平衡校验
- 打印（printJS）
- 导出（CSV）
- 凭证号自动生成（generateVoucherNo）
- 科目树加载（getAccountSubjectTree）

---

## 四、CI 经验教训（必须遵守）

1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**：
   - prop 类型从 `{ value: string }` 改为 `string`（基础类型）
   - 父组件绑定 `v-model:foo="ref.value"` 走 Vue 模板 ref 自动解包
3. **vue/no-mutating-props ESLint 错误**：
   - 必须在每个子组件的 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`
   - **ESLint 在 `<template>` 中不识别 `<script>` 顶部的 disable 注释**
4. **v-model 不能用于 prop**：必须用 `:model-value` + `@update:model-value` + emit 模式
5. **真实修复**：CI 报错后必须实际修改代码

---

## 五、验证清单

- [ ] 父文件 870 → ~150 行
- [ ] 4 个子组件文件已创建（VchrLstFilter / VchrLstTbl / VchrLstForm / VchrLstDetail）
- [ ] 3 个 composable/工具已创建
- [ ] 命名 ≤ 9 字符
- [ ] 行为完全保持一致
- [ ] 中文注释
- [ ] CI 5/5 全绿

---

## 六、拆分模板（与 I-2 保持一致）

```typescript
// composables/useVchrLst.ts
export function useVchrLst() {
  // 状态：tableData / total / loading / searchForm / pagination / form / dialogVisible / viewDialogVisible
  // 列表数据
  // 表单数据
  // 详情数据
  // 方法：loadData / loadVoucherTypes / loadAccountSubjects / addEntry / removeEntry / calculateTotals
  //        openAddDialog / openEditDialog / openViewDialog / handleSubmit / handleSearch / handleReset
  //        handlePageChange / handlePageSizeChange / getStatusLabel / getStatusClass / getTypeLabel
  return { ... }
}
```

```typescript
// composables/useVchrLstProc.ts
export function useVchrLstProc(...)
// 方法：handleApprove / handlePost / handleUnpost / handleDelete / handlePrint / handleExport
```

```typescript
// composables/vchrLstFmts.ts
// 状态标签/类型映射常量
// getStatusLabel / getStatusClass / getTypeLabel
```

```
VchrLstFilter.vue:   过滤栏 UI（4 个字段：凭证号/开始/结束/状态 + 查询/重置/新增/打印/导出）
VchrLstTbl.vue:      表格 + 分页 + 行内操作
VchrLstForm.vue:     新建/编辑对话框（凭证基本信息 + 分录明细 + 借贷合计）
VchrLstDetail.vue:   详情对话框（凭证头 + 分录 + 制单人/审核人/记账人）
```

---

## 七、关联文档

- [2026-06-19-p14-batch2-i3-split-vue-plan.md](2026-06-19-p14-batch2-i3-split-vue-plan.md) - 父计划
- [2026-06-18-p14-batch1-i2-split-vue-plan.md](2026-06-18-p14-batch1-i2-split-vue-plan.md) - I-2 拆分模式参考
- [2026-06-18-p13-batch1-comprehensive-plan.md](2026-06-18-p13-batch1-comprehensive-plan.md) - I-1 拆分模式参考
