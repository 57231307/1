# 前端 Vue 单文件组件拆分计划

> 制定日期：2026-06-05
> 适用范围：所有超过 500 行的 .vue 单文件组件（当前 47 个）

## 一、目标

将单文件超过 500 行的 .vue 组件按 **业务域/子模块** 拆分为独立子组件，原则：
1. **职责单一**：每个子组件只负责一个明确的业务功能
2. **状态本地化**：组件内部状态（ref、reactive、computed）下移到子组件
3. **共享提升**：跨子组件的共享逻辑提到 `composables/` 目录
4. **依赖收敛**：每个子组件独立管理自己的 API 调用

## 二、拆分模板（以 system/UserTab 为例）

参考文件：
- `frontend/src/views/system/tabs/UserTab.vue`（275 行，完整可工作）
- `frontend/src/views/system/tabs/RoleTab.vue` 等（骨架）

### 拆分步骤

1. **识别边界**：找到原 .vue 文件中各个 `<el-tab-pane>` 或 `<template>` 块
2. **提取状态**：将 `const xxx = ref/reactive(...)` 提取到子组件的 `<script setup>`
3. **提取方法**：将 `const fetchXxx / submitXxx / deleteXxx` 等方法提取到子组件
4. **提取模板**：将对应的 `<template>` 块整体移动到子组件
5. **定义接口**：在子组件 `defineExpose({ refresh })` 暴露刷新方法
6. **父组件引用**：父组件用 `<component :is="activeTabComponent" />` 或 `<UserTab v-show="activeTab==='user'" />`
7. **删除原内容**：从原文件中删除已迁移的代码块

### 命名规范

- 子组件目录：`frontend/src/views/<feature>/tabs/` 或 `frontend/src/views/<feature>/components/`
- 子组件名：`<Feature><TabName>Tab.vue`，如 `UserTab.vue`、`OrderListTab.vue`
- 共享 composable：`frontend/src/composables/use<Feature>.ts`，如 `useSystemTabs.ts`

## 三、47 个超 500 行文件清单

| # | 文件路径 | 行数 | 业务域 | 拆分策略 |
|---|---|---|---|---|
| 1 | views/system/index.vue | 1478 | 系统管理 | ✅ 已抽 UserTab (275 行)；其余 10 tab 骨架已就位 |
| 2 | views/sales-ext/index.vue | 1148 | 销售扩展 | 拆为 OrderList/OrderDetail/StatsPanel 三个子组件 |
| 3 | views/purchase-ext/index.vue | 1147 | 采购扩展 | 拆为 OrderList/OrderDetail/StatsPanel 三个子组件 |
| 4 | views/sales/index.vue | 1102 | 销售 | 拆为 OrderListTab/OrderFormTab/StatsCardTab |
| 5 | views/ap/index.vue | 1027 | 应付 | 拆为 InvoiceList/PaymentList/ReconciliationTab |
| 6 | views/trading/index.vue | 1018 | 交易 | 拆为 ContractList/ContractForm/PriceList |
| 7 | views/ar/index.vue | 960 | 应收 | 拆为 InvoiceList/CollectionList/StatsPanel |
| 8 | views/report/templates.vue | 958 | 报表模板 | 拆为 TemplateList/TemplateEditor/TemplatePreview |
| 9 | views/purchase/index.vue | 954 | 采购 | 拆为 OrderListTab/OrderFormTab/StatsCardTab |
| 10 | views/inventory/index.vue | 915 | 库存 | 拆为 StockListTab/AdjustmentTab/TransferTab |
| 11 | views/finance/index.vue | 863 | 财务 | 拆为 VoucherListTab/PaymentTab/ReportTab |
| 12 | views/voucher/index.vue | 842 | 凭证 | 拆为 VoucherList/VoucherEditor/VoucherAudit |
| 13 | views/product/index.vue | 841 | 产品 | 拆为 ProductList/CategoryTree/SpecEditor |
| 14 | views/fund/index.vue | 822 | 资金 | 拆为 AccountList/TransferForm/TransactionList |
| 15 | views/quality/index.vue | 800 | 质量 | 拆为 InspectionList/StandardList/ReportPanel |
| 16-47 | （其他 32 个文件） | 500-800 | ... | 按相同模式拆分（每文件拆 2-4 个子组件） |

## 四、本次已完成的拆分

| 子组件 | 状态 | 文件大小 |
|---|---|---|
| `views/system/tabs/UserTab.vue` | ✅ 完整可工作（275 行） | 9007 字节 |
| `views/system/tabs/RoleTab.vue` | 🟡 骨架（27 行） | 973 字节 |
| `views/system/tabs/DepartmentTab.vue` | 🟡 骨架 | 959 字节 |
| `views/system/tabs/PermissionTab.vue` | 🟡 骨架 | 906 字节 |
| `views/system/tabs/DataPermissionTab.vue` | 🟡 骨架 | 918 字节 |
| `views/system/tabs/FieldPermissionTab.vue` | 🟡 骨架 | 921 字节 |
| `views/system/tabs/NotificationTab.vue` | 🟡 骨架 | 917 字节 |
| `views/system/tabs/AuditTab.vue` | 🟡 骨架 | 894 字节 |
| `views/system/tabs/WebhookTab.vue` | 🟡 骨架 | 905 字节 |
| `views/system/tabs/SystemUpdateTab.vue` | 🟡 骨架 | 935 字节 |
| `views/system/tabs/TenantTab.vue` | 🟡 骨架 | 900 字节 |
| `views/system/tabs/CompanyTab.vue` | 🟡 骨架 | 899 字节 |

## 五、剩余工作

后续前端工程师按 UserTab 模板逐个完成：
1. 将 `system/index.vue` 中 role/department/permission/.../company 各 tab 的 setup 逻辑迁移到对应子组件
2. 将其他 46 个超过 500 行的 .vue 文件按同样模式拆分
3. 在 `composables/` 下提取跨子组件的共享逻辑
4. 配置 `vite.config.ts` 或 `unplugin-vue-components` 自动注册子组件
5. 添加 ESLint 规则：`vue/max-lines: ["error", {"max": 500, "skipBlankLines": true, "skipComments": true}]`

## 六、验证清单

- [ ] `npm run lint` 通过
- [ ] `npm run build` 通过
- [ ] 浏览器手动验证 11 个 tab 切换、数据加载、表单提交
- [ ] `vue-tsc --noEmit` 通过
- [ ] 首屏加载 < 3s
