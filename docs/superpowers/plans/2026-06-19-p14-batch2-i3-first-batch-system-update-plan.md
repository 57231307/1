# 2026-06-19 P14 批 2 I-3 第 1 批 - system-update/index.vue 拆分计划

> **创建日期**：2026-06-19
> **基线版本**：main @ 2f508cb（P14 批 1 I-2 收尾后）
> **关联路线图**：[2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3
> **父计划**：[2026-06-19-p14-batch2-i3-split-vue-plan.md](2026-06-19-p14-batch2-i3-split-vue-plan.md)
> **任务编号**：P14 批 2 I-3 第 1 批

---

## 一、目标

拆分 `frontend/src/views/system-update/index.vue`（725 行）为：

| 类型 | 文件 | 职责 |
|------|------|------|
| 父文件 | `system-update/index.vue` | 组件组合 + 状态管理（725 → ~150 行） |
| composable | `system-update/composables/useSysUpd.ts` | 主业务（3 个 tab 的数据加载 + 操作） |
| composable | `system-update/composables/useSysUpdProc.ts` | 业务流程（下载/安装/回滚/取消/恢复/删除） |
| 工具 | `system-update/composables/sysUpdFmts.ts` | 状态标签/类型映射/格式化文件大小 |
| 子组件 | `system-update/components/SuInfoCards.vue` | 顶部信息卡（当前版本/最新版本/更新状态） |
| 子组件 | `system-update/components/SuVerTbl.vue` | 版本列表 Tab + 表格 + 分页 |
| 子组件 | `system-update/components/SuTaskTbl.vue` | 更新任务 Tab + 表格 + 分页 |
| 子组件 | `system-update/components/SuBkpTbl.vue` | 系统备份 Tab + 表格 + 分页 |
| 子组件 | `system-update/components/SuVerDetail.vue` | 版本详情对话框 |
| 子组件 | `system-update/components/SuBkpForm.vue` | 创建备份对话框 |

**预计 commit 数**：1

---

## 二、命名规范

- composable：`useSysUpd` / `useSysUpdProc`（≤ 9 字符）
- 工具：`sysUpdFmts`
- 子组件：`SuInfoCards` / `SuVerTbl` / `SuTaskTbl` / `SuBkpTbl` / `SuVerDetail` / `SuBkpForm`（Su = SystemUpdate，≤ 9 字符）

---

## 三、行为保持

**完全保持**：
- 顶部 3 个信息卡（当前版本/最新版本/更新状态）
- 3 个 Tab 切换（版本列表/更新任务/系统备份）
- 表格列、操作按钮、分页
- 版本详情对话框
- 创建备份对话框
- 文件大小格式化（B/KB/MB/GB）

---

## 四、CI 经验教训（必须遵守）

1. **JSDoc 中文注释在 TS 泛型内解析失败**：不要在 `defineProps<{...}>` 上方写 `/** 中文 JSDoc */`，改用 `// 中文行注释`
2. **TS2540 readonly 错误**：prop 类型用基础类型，复杂对象用 `.value` 显式解包
3. **vue/no-mutating-props ESLint 错误**：必须在每个子组件的 `<template>` 顶部加 `<!-- eslint-disable vue/no-mutating-props -->`
4. **v-model 不能用于 prop**：必须用 `:model-value` + `@update:model-value` + emit 模式
5. **真实修复**：CI 报错后必须实际修改代码

---

## 五、验证清单

- [ ] 父文件 725 → ~150 行
- [ ] 6 个子组件文件已创建
- [ ] 3 个 composable/工具已创建
- [ ] 命名 ≤ 9 字符
- [ ] 行为完全保持一致
- [ ] 中文注释
- [ ] CI 5/5 全绿

---

## 六、拆分模板

```
useSysUpd.ts: 包含 3 个 tab 的数据状态 + 加载方法（fetchCurrentVersion / handleCheckUpdate / fetchVersions / fetchTasks / fetchBackups / openBackupDialog / viewVersionDetail / handleBackupSubmit / 各种 delete/restore/download/handleDownloadBackup/handleInstall 等）
useSysUpdProc.ts: 业务流程（涉及 ElMessageBox.confirm 的确认操作：handleDownload / handleInstall / handleCancelTask / handleRollback / handleDeleteBackup / handleRestore / handleDownloadBackup）
sysUpdFmts.ts: versionStatusMap / versionStatusTypeMap / taskStatusMap / taskStatusTypeMap / backupTypeMap / backupStatusMap / backupStatusTypeMap / formatFileSize
SuInfoCards.vue: 顶部 3 张信息卡
SuVerTbl.vue: 版本列表表格
SuTaskTbl.vue: 更新任务表格
SuBkpTbl.vue: 系统备份表格
SuVerDetail.vue: 版本详情对话框
SuBkpForm.vue: 创建备份对话框
```

---

## 七、关联文档

- [2026-06-19-p14-batch2-i3-split-vue-plan.md](2026-06-19-p14-batch2-i3-split-vue-plan.md) - 父计划
- [2026-06-18-p14-batch1-i2-split-vue-plan.md](2026-06-18-p14-batch1-i2-split-vue-plan.md) - I-2 拆分模式参考
