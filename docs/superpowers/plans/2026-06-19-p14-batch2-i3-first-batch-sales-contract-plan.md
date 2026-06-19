# 2026-06-19 P14 批 2 I-3 第 1 批 - sales-contract/index.vue 拆分计划

> **创建日期**：2026-06-19
> **基线版本**：main @ 2f508cb（P14 批 1 I-2 收尾后）
> **关联路线图**：[2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3
> **父计划**：[2026-06-19-p14-batch2-i3-split-vue-plan.md](2026-06-19-p14-batch2-i3-split-vue-plan.md)
> **任务编号**：P14 批 2 I-3 第 1 批

---

## 一、目标

拆分 `frontend/src/views/sales-contract/index.vue`（717 行）为：

| 类型 | 文件 | 职责 |
|------|------|------|
| 父文件 | `sales-contract/index.vue` | 组件组合 + 状态管理（717 → ~150 行） |
| composable | `sales-contract/composables/useSc.ts` | 主业务（列表查询/表单/客户加载/CRUD） |
| composable | `sales-contract/composables/useScProc.ts` | 业务流程（提交审批/审批/执行/打印/导出） |
| 工具 | `sales-contract/composables/scFmts.ts` | 状态标签/类型映射/货币格式化 |
| 子组件 | `sales-contract/components/ScFilter.vue` | 过滤栏 |
| 子组件 | `sales-contract/components/ScTbl.vue` | 合同列表表格 |
| 子组件 | `sales-contract/components/ScForm.vue` | 新建/编辑对话框 |
| 子组件 | `sales-contract/components/ScDetail.vue` | 查看详情对话框 |

**预计 commit 数**：1

---

## 二、命名规范

- composable：`useSc` / `useScProc`（Sales Contract 缩写，≤ 9 字符）
- 工具：`scFmts`
- 子组件：`ScFilter` / `ScTbl` / `ScForm` / `ScDetail`（≤ 9 字符）

---

## 三、行为保持

**完全保持**：
- 顶部页头（面包屑 + 新建/打印/导出）
- 过滤栏（关键词/客户/状态/签订日期范围）
- 表格列（合同编号/名称/客户/金额/签订日期/生效日期/到期日期/状态/操作）
- 操作按钮（查看/编辑/提交/审批/执行/删除，按状态显示）
- 新建/编辑对话框（14 个字段）
- 查看详情（ElMessageBox.alert 弹出）
- 打印（HTML 窗口）
- 导出（CSV）
- 货币格式化
- 状态标签/类型映射
- 懒加载客户列表（loadIfNot + createLazyLoader）

---

## 四、CI 经验教训（必须遵守）

1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**：prop 类型用基础类型，复杂对象用 `.value` 显式解包
3. **vue/no-mutating-props ESLint 错误**：必须在每个子组件的 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`
4. **v-model 不能用于 prop**：必须用 `:model-value` + `@update:model-value` + emit 模式
5. **真实修复**：CI 报错后必须实际修改代码

---

## 五、验证清单

- [ ] 父文件 717 → ~150 行
- [ ] 4 个子组件文件已创建
- [ ] 3 个 composable/工具已创建
- [ ] 命名 ≤ 9 字符
- [ ] 行为完全保持一致
- [ ] 中文注释
- [ ] CI 5/5 全绿

---

## 六、拆分模板

```
useSc.ts: 查询参数/列表数据/客户列表/对话框状态/表单数据/表单验证规则
         方法：getList / getCustomers / handleQuery / handleReset / handleCreate / handleEdit
                handleView / handleSubmitForm / handleSizeChange / handleCurrentChange
                handleDateChange
useScProc.ts: handleSubmitForApproval / handleApprove / handleExecute / handleDelete
              handlePrint / handleExport
scFmts.ts: formatCurrency / getStatusType / getStatusLabel
ScFilter.vue: 过滤栏
ScTbl.vue: 合同列表表格 + 分页
ScForm.vue: 新建/编辑对话框（14 个字段 + 表单验证）
ScDetail.vue: 查看详情（ElMessageBox.alert）
```

---

## 七、关联文档

- [2026-06-19-p14-batch2-i3-split-vue-plan.md](2026-06-19-p14-batch2-i3-split-vue-plan.md) - 父计划
- [2026-06-18-p14-batch1-i2-split-vue-plan.md](2026-06-18-p14-batch1-i2-split-vue-plan.md) - I-2 拆分模式参考
