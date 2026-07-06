# 前端 v11 全项目复审报告

**复审日期**：2026-07-06
**复审范围**：`/workspace/frontend/src/` 下 views / components / api / router / stores / utils / composables / directives / locales
**扫描维度**：占位符/stub、any 类型、未接入功能、mock 数据、错误处理、i18n、v-permission
**项目技术栈**：Vue 3 + TypeScript + Element Plus + Pinia + vue-i18n + ECharts
**总文件数**：约 280 个 .vue / .ts 文件

---

## 一、复审总结

| 优先级 | 问题数 | 说明 |
|--------|--------|------|
| **P0（阻塞）** | 3 类共 8 处 | 假成功（不调用 API 即提示成功）、安全功能占位、死代码假审批 |
| **P1（重要）** | 6 类共 30+ 处 | 占位 stub、死代码文件、响应结构不一致、绕过 API 层、硬编码兜底数据 |
| **P2（次要）** | 7 类 | any 类型泛滥（379 处）、i18n 接入不全、菜单硬编码、tech-debt TODO |

**关键结论**：
- 已无 `console.log` 调试残留、无原生 `alert()` 调用、无 mockData/dummyData 命名的假数据
- v-permission 在编辑/删除按钮上覆盖较完整（70+ 处使用）
- 但 CRM 模块存在 6 处**假成功**（最严重）、inventory/bpm/trading 等模块存在大量 ElMessage.info 占位
- `any` 类型共 379 处分布在 100 个文件，是技术债大头
- i18n 仅 Login.vue 接入，其余全部硬编码中文

---

## 二、P0 问题（阻塞 - 必须立即修复）

### P0-1：CRM 线索/商机操作假成功（未调用后端 API 即提示成功）

**文件 1**：`/workspace/frontend/src/views/crm/leads/index.vue`

| 行号 | 函数 | 问题描述 |
|------|------|----------|
| 320-332 | `handleContact` | `await ElMessageBox.confirm(...)` 后直接 `ElMessage.success('操作成功')` + `getList()`，**未调用任何后端 API** |
| 334-346 | `handleConvert` | 同上模式，显示"转化成功"但未调用线索转化 API |
| 348-360 | `handleLost` | 同上模式，显示"操作成功"但未调用标记流失 API |

**文件 2**：`/workspace/frontend/src/views/crm/opportunities/index.vue`

| 行号 | 函数 | 问题描述 |
|------|------|----------|
| 370-382 | `handleWin` | 显示"操作成功"但未调用商机成交 API |
| 384-396 | `handleLost` | 显示"操作成功"但未调用商机流失 API |
| 398-400 | `handleExport` | `ElMessage.success('导出成功')` 但**未调用任何导出 API**，也未生成文件 |

**修复建议**：
- `handleContact` / `handleConvert` / `handleLost`：调用 `@/api/crm` 中对应的线索状态变更接口（如 `updateLeadStatus`/`convertLeadToCustomer`）
- `handleWin` / `handleLost`：调用商机状态变更 API
- `handleExport`：参照 `crm/leads/index.vue:367-383` 的 `handleExport` 实现（调用 `exportOpportunities` API 并触发浏览器下载）

**优先级判定理由**：用户执行业务操作后看到"成功"提示，但数据未真正写入后端。这会导致：(1) 用户误以为操作已完成而离开；(2) 列表刷新后状态未变，造成困惑；(3) 业务数据丢失/不一致。属于功能性数据完整性问题，必须立即修复。

---

### P0-2：库存调拨审批假成功（死代码中的假审批）

**文件**：`/workspace/frontend/src/views/inventory/tabs/TransferTab.vue`
**行号**：113-115

```typescript
const handleApproveTransfer = (row: TransferRow) => {
  ElMessage.success(`审批通过调拨单 ${row.transfer_no}`)
}
```

**问题描述**：审批按钮点击后仅显示 `ElMessage.success`，**未调用任何调拨审批 API**，未真正变更调拨单状态。用户看到"审批通过"提示，但后端调拨单状态仍为 `pending`。

**修复建议**：调用 `inventoryApi.approveTransfer(row.id)` 后再提示成功并刷新列表。或者直接删除此死代码文件（见 P1-3）。

**优先级判定理由**：审批是关键业务流程，假审批会导致库存调拨流程断裂。虽然此文件本身是死代码（未被 inventory/index.vue 引用），但代码存在即隐患（可能被误用）。

---

### P0-3：2FA 恢复码占位（安全功能不完整）

**文件**：`/workspace/frontend/src/views/security/two-factor/composables/useTfa.ts`
**行号**：27-28

```typescript
// Step 4 数据：恢复码（后端目前未提供，使用占位）
const recoveryCodes = ref<string[]>([])
```

**问题描述**：双因素认证的恢复码始终为空数组。用户启用 2FA 后无法获取恢复码，一旦丢失 TOTP 设备将无法登录。`useTfaProc.ts:82` 调用 `generateRecoveryCodes()` 但 `useTfa.ts` 的 `recoveryCodes` 未与该调用关联。

**修复建议**：
1. 后端补全恢复码生成/返回接口
2. 前端在 `enableTotp` 成功后调用 `generateRecoveryCodes()` 并将结果写入 `recoveryCodes.value`
3. 在 Step 4 UI 中展示恢复码并要求用户确认保存

**优先级判定理由**：2FA 是安全功能，恢复码缺失会导致用户被锁死在账户外无法恢复，属于安全功能不完整。

---

## 三、P1 问题（重要 - 应在下一迭代修复）

### P1-1：大量功能为占位 stub（ElMessage.info 提示，未实现真实逻辑）

以下函数仅 `ElMessage.info(...)` 提示，未实现真实业务逻辑：

| 文件 | 行号 | 函数 | 占位提示 |
|------|------|------|----------|
| `inventory/index.vue` | 305-307 | `handleViewTransfer` | `查看调拨单：${row.transfer_no}` |
| `inventory/index.vue` | 308-310 | `handleApproveTransfer` | `审批调拨单：${row.transfer_no}` |
| `inventory/index.vue` | 311-313 | `handleView` | `查看库存：${row.product_name}` |
| `inventory/index.vue` | 314-316 | `handlePurchase` | `采购：${row.product_name}` |
| `bpm/index.vue` | 326-328 | `handleDetail` | `查看详情：${row.business_key}` |
| `bpm/index.vue` | 355-357 | `handleTrace` | `追溯流程：${row.instance_id}` |
| `bpm/index.vue` | 358-360 | `handleCancel` | `撤回流程：${row.instance_id}`（按钮有 `v-permission="'bpm_process:cancel'"` 但函数空实现） |
| `bpm/index.vue` | 361-363 | `handleViewProcess` | `查看流程：${row.instance_id}` |
| `bpm/index.vue` | 364-366 | `handleProcessImage` | `查看流程图：${row.instance_id}` |
| `trading/tabs/PurchasePriceTab.vue` | 91 | `handleEdit` | `编辑价格: ...` |
| `trading/tabs/SalesPriceTab.vue` | 91 | `handleEdit` | `编辑价格: ...` |
| `trading/tabs/SalesReturnTab.vue` | 134 | `handleView` | `查看销售退货: ${row.return_no}` |
| `trading/tabs/PurchaseContractTab.vue` | 145 | `handleView` | `查看采购合同: ${row.contract_no}` |
| `trading/tabs/SalesContractTab.vue` | 145 | `handleView` | `查看销售合同: ${row.contract_no}` |
| `ar/tabs/InvoiceTab.vue` | 317 | `handleView` | `查看发票: ${row.invoice_no}` |
| `ap/tabs/InvoiceTab.vue` | 318 | `handleView` | `查看发票: ${row.invoice_no}` |
| `ap/tabs/InvoiceTab.vue` | 350 | `handlePrint` | `打印功能请参考原实现` |
| `ap/tabs/InvoiceTab.vue` | 354 | `handleExport` | `导出功能请参考原实现` |
| `currency/tabs/CurrencyListTab.vue` | 253-255 | `setBase` | `请通过后端接口将指定币种设为基准币种` |
| `quality/tabs/ApproveDialogTab.vue` | 105 | `handleReject` | `驳回功能待后端实现` |
| `quality/index.vue` | 394 | `handleReject` | `驳回功能待后端实现` |
| `quality/tabs/RecordTab.vue` | 134 | `handleView` | `查看检验记录` |
| `financial-analysis/tabs/AnalysisListTab.vue` | 247 | `handleView` | `查看报表: ...` |
| `fund/tabs/TransferTab.vue` | 280 | `handleView` | `查看转账详情: ...` |
| `fixed-assets/tabs/AssetListTab.vue` | 527 | `handleBatchDepreciate` | `请逐个对资产计提折旧` |
| `color-prices/detail.vue` | 144 | `handleAddTier` | `请通过批量调价页或 API 创建阶梯价` |
| `crm/leads/index.vue` | 316-318 | `handleView` | 空函数体，仅注释 `// 查看详情（占位）` |
| `crm/leads/index.vue` | 362-364 | `handleImport` | `导入功能开发中` |
| `crm/leads/index.vue` | 385-388 | `handleSelectionChange` | 占位，仅 `logger.debug` |
| `inventory/tabs/StockTab.vue` | 330 | `handleView` | `查看 ${row.product_name} 详情` |
| `inventory/tabs/AlertTab.vue` | 73 | `handlePurchase` | `为 ${row.product_name} 创建采购单` |
| `inventory/tabs/TransferTab.vue` | 110 | `handleViewTransfer` | `查看调拨单 ${row.transfer_no}` |
| `inventoryBatch/tabs/BatchListTab.vue` | 159 | `handleCreateTransfer` | `为批次 ${row.batchNo} 创建调拨单` |

**修复建议**：
- 优先实现有 `v-permission` 控制的按钮对应函数（如 `bpm/index.vue` 的 `handleCancel`）
- 对于"查看详情"类占位，统一改为打开详情对话框或跳转详情页
- 对于"导入/打印/导出"占位，参照已实现的模块（如 `crm/leads/index.vue:367-383` 的导出实现）补全
- 短期无法实现的，将按钮 `disabled` 并提示"功能开发中"，避免用户点击后产生困惑

**优先级判定理由**：这些占位虽然不会造成数据错误，但严重影响用户体验和功能完整性。特别是 `bpm/index.vue` 的 `handleCancel` 按钮带有 `v-permission="'bpm_process:cancel'"`，权限控制存在但功能空实现，属于"有权限但无功能"的尴尬状态。

---

### P1-2：死代码文件（inventory/tabs 旧版本未删除）

以下 4 个文件**未被任何文件 import**，是 `inventory/index.vue` 重构后遗留的旧版本：

| 文件路径 | 替代文件 | 状态 |
|----------|----------|------|
| `/workspace/frontend/src/views/inventory/tabs/StockTab.vue` | `InventoryStockTab.vue` | 死代码（仅 grep 到自身注释引用） |
| `/workspace/frontend/src/views/inventory/tabs/AlertTab.vue` | `InventoryAlertTab.vue` | 死代码 |
| `/workspace/frontend/src/views/inventory/tabs/TransferTab.vue` | `InventoryTransferTab.vue` | 死代码（含 P0-2 假审批） |
| `/workspace/frontend/src/views/inventory/tabs/TransferDialogTab.vue` | `components/TransferDialog.vue` | 死代码 |

**验证方法**：grep 搜索 `from '...tabs/StockTab`、`tabs/AlertTab`、`tabs/TransferTab`、`tabs/TransferDialogTab`，仅 `crm/pool.vue` 和 `fund/index.vue` 的 `TransferDialogTab`/`TransferTab` 是不同目录的同名文件，与 inventory 无关。

**修复建议**：直接删除这 4 个文件。

**优先级判定理由**：项目规范要求"CI 在 clippy 检查中失败"针对死代码（虽然前端无 clippy，但 eslint/vue-tsc 应同等约束）。维护两份代码易产生不一致（如 P0-2 的假审批就藏在死代码中）。

---

### P1-3：omniAudit/barcodeScanner 响应结构 `res.data!.data` 双层嵌套

**文件 1**：`/workspace/frontend/src/views/omniAudit/index.vue`
**行号**：65, 87, 88

```typescript
const res: any = await getDashboardStats()
stats.value = res.data!.data           // 双层 .data
logs.value = res.data!.data.items      // 三层 .data
total.value = res.data!.data.total
```

**文件 2**：`/workspace/frontend/src/views/barcodeScanner/index.vue`
**行号**：67, 96

```typescript
const res: any = await scanInventory(barcodeInput.value)
scanResult.value = res.data!.data      // 双层 .data
scanMessage.value = res.data!.data.message
```

**问题描述**：`request.ts` 拦截器（line 100-116）返回 `ApiResponse`（含 `code`/`message`/`data` 字段），所以 `res.data` 应为业务数据。此处 `res.data!.data` 暗示后端返回了 `ApiResponse<ApiResponse<T>>` 嵌套结构，与其他模块（如 `inventory/index.vue:131` 的 `res.data?.list`）不一致。`any` 类型掩盖了此不一致。

**修复建议**：
1. 核实 `/finance/audit/stats`、`/scanner/scan-inventory` 等后端接口的实际响应结构
2. 若后端确实返回双层 data，统一在 `omniAudit.ts`/`barcode-scanner.ts` API 层做拆包，view 层只用 `res.data`
3. 移除 `const res: any`，改为 `const res = await ...`（API 函数已有返回类型）

**优先级判定理由**：响应结构不一致是潜在运行时 bug 源；`any` 类型掩盖了类型检查本应发现的问题。

---

### P1-4：arReconciliation 直接调用 `request.get` 绕过 API 层

**文件 1**：`/workspace/frontend/src/views/arReconciliation/enhanced.vue`
**行号**：96

```typescript
const res: any = await request.get('/customers/select')
```

**文件 2**：`/workspace/frontend/src/views/arReconciliation/index.vue`
**行号**：97

```typescript
const res: any = await request.get('/customers/select')
```

**问题描述**：直接在 view 层调用 `request.get('/customers/select')`，绕过了 `@/api/customer.ts` 的统一封装。`@/api/customer.ts` 已有 `listCustomers` 等函数（`quotations/list.vue:192` 使用了 `listCustomers`）。此处的 `/customers/select` 端点可能是 customer API 未封装的"下拉选项"接口。

**修复建议**：在 `@/api/customer.ts` 新增 `getCustomerSelectOptions()` 函数封装 `/customers/select`，view 层改为调用该函数。

**优先级判定理由**：违反"API 调用通过 request.ts 统一封装"的项目规范；view 层直接拼 URL 不利于后端路径变更维护。

---

### P1-5：dataPermission 失败时使用硬编码 mock 数据兜底

**文件**：`/workspace/frontend/src/views/dataPermission/index.vue`
**行号**：200-219

```typescript
const fetchScopeTypes = async () => {
  try {
    const res: any = await listScopeTypes()
    if (res.data) {
      scopeTypeList.value = res.data! || []
    }
  } catch (e) {
    // 使用默认值
    scopeTypeList.value = [
      { value: 'ALL', label: '全部数据', description: '可以查看所有数据' },
      { value: 'DEPT', label: '本部门数据', description: '只能查看本部门的数据' },
      // ... 5 项硬编码
    ]
  }
}
```

**问题描述**：API 失败时使用硬编码的 scopeTypeList 兜底，且未通过 ElMessage 告知用户"加载范围类型失败，使用默认值"。用户无法区分是后端故障还是真实数据。

**修复建议**：
- catch 块中加 `ElMessage.warning('范围类型加载失败，使用默认值')` 提示用户
- 或将硬编码列表抽到 `@/api/data-permission.ts` 作为 `DEFAULT_SCOPE_TYPES` 常量，避免 view 层硬编码

**优先级判定理由**：静默兜底会让用户误以为数据正常；硬编码业务数据在 view 层违反"禁止硬编码"规范。

---

### P1-6：currency `setBase` 完全未实现

**文件**：`/workspace/frontend/src/views/currency/tabs/CurrencyListTab.vue`
**行号**：253-255

```typescript
const setBase = (_row: Currency) => {
  ElMessage.info('请通过后端接口将指定币种设为基准币种')
}
```

**问题描述**：设置基准币种功能完全未实现，仅提示用户"请通过后端接口"操作。这意味着用户必须直接调用后端 API 才能完成此操作，前端按钮形同虚设。

**修复建议**：在 `@/api/currency.ts` 新增 `setBaseCurrency(id)` 函数，调用后端 `PUT /currencies/{id}/set-base`，view 层调用并刷新列表。

**优先级判定理由**：多币种管理中"基准币种"是核心功能，按钮存在但功能空实现会让用户感到困惑。

---

## 四、P2 问题（次要 - 技术债，按迭代清理）

### P2-1：any 类型泛滥（379 处，100 文件）

**统计**：`grep -E ":\s*any\b|as\s+any\b|<any>|reactive<any>|ref<any>"` 共 379 处，分布在 100 个文件。

**最严重的文件**（按 any 出现次数）：

| 文件 | any 数 | 主要形式 |
|------|--------|----------|
| `views/bpm/index.vue` | 20 | `row: any`, `as any`, `catch (error: any)` 注释 |
| `views/quotations/create.vue` | 13 | `as any`, `: any`, `validator: (_rule: any, value: any[], cb: any)` |
| `views/sales-returns/composables/useSr.ts` | 12 | `ref<any>`, `reactive<any>`, `items: [] as any[]`, `catch (error: any)` |
| `views/advanced/composables/useAi.ts` | 12 | `ref<any>(null)`, `res: any`, `catch (e: any)` |
| `views/bom/index.vue` | 10 | `as any`, `data: any` |
| `views/color-prices/detail.vue` | 7 | `as any`（el-tag :type 断言） |
| `views/dataPermission/index.vue` | 6 | `res: any`, `as unknown as CustomCondition` |
| `views/arReconciliation/index.vue` | 8 | `res: any`, `as any` |
| `views/notification/index.vue` | 7 | `res: any`, `} as any)` |
| `views/user-profile/index.vue` | 6 | `validator: (_rule: any, value: string, callback: any)` |

**典型模式**：
1. `const res: any = await someApi()` — 应改为 `const res = await someApi()`（API 已有返回类型）
2. `row: any` — 应定义 `interface Row {...}` 并替换
3. `as any` 用于 el-tag `:type` — 应使用 `as ComponentType` 或定义类型映射
4. `catch (error: any)` — 项目已统一改为 `catch (error: unknown)` + 类型守卫，但仍有 139 处注释标记的旧代码（见 P2-1 备注）
5. `ref<any>(null)` / `reactive<any>({})` — 应定义具体接口

**修复建议**：按模块分批清理，优先处理 `views/bpm/`、`views/quotations/`、`views/sales-returns/`、`views/advanced/` 这 4 个 any 重灾区。

**优先级判定理由**：any 不会导致运行时错误，但削弱了 TypeScript 类型保护的价值，且与项目规范"使用有意义的、描述性的名称"不符。

---

### P2-2：i18n 接入不完整（仅 Login.vue）

**文件**：`/workspace/frontend/src/i18n/index.ts`
**行号**：6-7

```typescript
// TODO(tech-debt): 批次 23 v5 P0-1 仅完成 Login.vue 示范接入，
// 其余 .vue 文件的硬编码文本待后续迭代逐步替换为 $t() 调用。
```

**问题描述**：
- `@/locales/zh-CN.ts` 和 `en-US.ts` 已就绪（4506 行资源文件）
- 但仅 `Login.vue` 使用 `$t()` / `useI18n()`
- 其余 ~150 个 .vue 文件全部硬编码中文文本（如 `ElMessage.success('操作成功')`、`placeholder="请输入..."`、`label="产品编码"` 等）
- 部分模块（ai-extend、budget、cost、inventoryTransfer、system/tabs/UserTab 等）已在注释中标注"批次 34 v9 P1：接入 i18n"但实际仅替换了 ElMessage 部分，模板文本仍硬编码

**修复建议**：按模块分批接入，每批 5-10 个文件，优先处理用户可见的高频页面（Dashboard、system、inventory、sales）。

**优先级判定理由**：项目规范要求"所有文本需要使用中文"，但同时提供了 i18n 框架。当前状态是 i18n 框架就绪但未启用，属于技术债而非功能缺失。

---

### P2-3：MainLayout 菜单硬编码 path 列表

**文件**：`/workspace/frontend/src/components/Layout/MainLayout.vue`
**行号**：302-319

```typescript
const subMenus: Record<string, string[]> = {
  fabric: ['/fabric', '/greige-fabrics', '/product', ...],
  inventory: ['/inventory', '/warehouse', ...],
  // ... 10 个分组
}
```

**问题描述**：菜单分组与 `router/index.ts` 的路由定义存在重复维护风险。新增路由需同时修改两处。

**修复建议**：改为基于路由表 `children` 的 `meta.group` 字段自动派生（与 P3 4-7 侧边栏动态化一同处理）。

**优先级判定理由**：TODO 已标注为 P3 4-7，当前功能正常，仅维护成本问题。

---

### P2-4：budget.ts / cost.ts 标注 tech-debt 的未接入 API

**文件 1**：`/workspace/frontend/src/api/budget.ts:43-46`

```typescript
// TODO(tech-debt): 前端接入后移除（后端端点保留）
export function approveBudget(id: number): Promise<ApiResponse<void>> {
  return request.post(`/budgets/${id}/approve`)
}
```

**文件 2**：`/workspace/frontend/src/api/cost.ts:35-40`

```typescript
// TODO(tech-debt): 前端接入后移除（后端端点保留）
export const deleteCollection = (id: number) => request.delete(`/production/cost-collections/${id}`)
// TODO(tech-debt): 前端接入后移除（后端端点保留）
export const auditCollection = (id: number, approved: boolean, comment?: string) => ...
```

**问题描述**：budget.ts 的 `approveBudget` 实际上**已被使用**（`budget/tabs/BudgetListTab.vue:269` 调用 `approveBudgetApi`），TODO 注释过时。cost.ts 的 `deleteCollection`/`auditCollection` 通过 `deleteCostCollection`/`auditCostCollection` 别名导出，需核实是否被 view 层使用。

**修复建议**：核实使用情况，移除过时的 TODO 注释；真正未使用的应删除函数定义。

**优先级判定理由**：注释与代码不一致，但不影响功能。

---

### P2-5：quality 分页未接入

**文件**：`/workspace/frontend/src/views/quality/index.vue`
**行号**：217

```typescript
// TODO(tech-debt): 后端 listQualityRecords 暂未支持分页字段；待后端分页参数就绪后接入 page/size。
```

**问题描述**：质量检验记录列表未分页，数据量大时性能堪忧。

**修复建议**：待后端补全分页参数后，前端接入 `page`/`page_size` 并添加分页组件。

**优先级判定理由**：依赖后端改造，前端无法独立修复。

---

### P2-6：custom-order target_status 必填但调用方未传

**文件**：`/workspace/frontend/src/api/custom-order.ts`
**行号**：113

```typescript
* TODO(tech-debt): 后端 target_status 为必填，但现有调用方未传，
```

**问题描述**：定制订单状态推进 API 的 `target_status` 是后端必填字段，但前端调用方未传值，可能导致 422 错误。

**修复建议**：核实 `custom-orders/list.vue:203` 和 `detail.vue:135` 的 `advanceOrder` 调用，补全 `target_status` 参数。

**优先级判定理由**：潜在运行时错误，但需核实后端实际行为。

---

### P2-7：inventory 主页全 `any[]` 状态

**文件**：`/workspace/frontend/src/views/inventory/index.vue`
**行号**：105-108

```typescript
const stocks = ref<any[]>([])
const alerts = ref<any[]>([])
const transfers = ref<any[]>([])
const warehouses = ref<any[]>([])
```

**问题描述**：4 个核心状态全为 `any[]`，丢失了类型保护。`InventoryStockTab.vue` 的 props 也接受 `any[]`（line 90, 94）。

**修复建议**：在 `@/api/inventory.ts` 定义 `Stock`/`Alert`/`Transfer`/`Warehouse` 接口，替换所有 `any[]`。

**优先级判定理由**：与 P2-1 同类，但 inventory 是核心模块，单独列出以示重要。

---

## 五、其他扫描结果（无问题项）

### 5.1 console.log 调试残留：✅ 无
全项目仅 `utils/websocket.ts:80` 在注释中有 `console.log` 示例代码，无实际调试残留。
`utils/logger.ts` 的 `console.debug/info/warn/error` 是封装后的日志器，符合规范。

### 5.2 原生 alert() 调试：✅ 无
所有 `alert(`/`confirm(`/`prompt(` 匹配均为 Element Plus 的 `ElMessageBox.confirm`/`prompt`，是规范的 UI 交互。

### 5.3 mockData/dummyData 命名的假数据：✅ 无
无 `mockData`、`dummyData`、`fakeData`、`sampleData`、`fixture` 等命名的硬编码假数据。

### 5.4 v-permission 权限控制：✅ 覆盖较好
- 编辑/删除按钮在 70+ 处使用 `v-permission`
- 系统管理、BPM、CRM、采购、销售、库存等模块的关键操作均有权限控制
- `views/production/components/PrdTbl.vue` 因 `h()` 渲染函数无法用指令，改为 `can()` 条件 push（行为等价）
- 少数占位函数（如 `bpm/index.vue` 的 `handleCancel`）虽有 `v-permission` 但功能空实现，见 P1-1

### 5.5 路由注册：✅ 完整
所有 `views/` 下的页面（除 `components-demo/index.vue` 等示例页）均在 `router/index.ts` 注册，无孤儿页面。

### 5.6 组件使用：✅ 完整
`components/` 下的组件（AIPredictionChart、AdvancedFilter、BatchActions、BorrowRecordTimeline、ColorCardGrid、ColorItemEditor、PasswordStrengthMeter、PriceHistoryChart、ProcessFlow、QualityCheck、Charts/*、V2Table）均被至少一个 view 引用。

### 5.7 错误处理：✅ 大部分规范
- `request.ts` 拦截器统一处理 401/403/5xx，自动跳转登录或重试
- 大部分 `catch` 块使用 `unknown` + 类型守卫（批次 98 P2-D 修复后）
- 仍有部分旧代码使用 `catch (error: any)` 但已加注释标记待迁移（139 处注释）

---

## 六、修复优先级建议

### 第一优先（本迭代必须完成）
1. **P0-1**：CRM 线索/商机 6 处假成功 → 接入真实 API
2. **P0-3**：2FA 恢复码占位 → 后端补全 + 前端关联
3. **P1-2**：删除 inventory/tabs 4 个死代码文件（含 P0-2）

### 第二优先（下一迭代）
4. **P1-1**：实现 bpm/inventory/trading 等 30+ 处占位 stub
5. **P1-3**：修复 omniAudit/barcodeScanner 响应结构不一致
6. **P1-4**：arReconciliation 绕过 API 层 → 封装到 customer API
7. **P1-5**：dataPermission 硬编码兜底 → 抽常量 + 失败提示
8. **P1-6**：currency setBase 实现

### 第三优先（技术债清理，按模块分批）
9. **P2-1**：any 类型清理（优先 bpm/quotations/sales-returns/advanced）
10. **P2-2**：i18n 接入（优先 Dashboard/system/inventory/sales）
11. **P2-3 ~ P2-7**：其余 tech-debt 项

---

## 七、附录：扫描方法说明

**扫描工具**：`Grep`（ripgrep）+ `Read` 文件审查

**扫描模式**：
- 占位符：`TODO|FIXME|XXX|HACK`、`console\.(log|debug|info|warn|error)`、`alert\(|confirm\(|prompt\(`、`return\s+\[\s*\]`
- any 类型：`:\s*any\b|as\s+any\b|<any>|reactive<any>|ref<any>`
- mock 数据：`mockData|dummyData|mock-data|fakeData|dummy_data|sampleData|test_data|fixture`
- 错误处理：`\.catch\(\s*\(\s*\)\s*=>\s*\{\s*\}\s*\)|catch\s*\([^)]*\)\s*\{\s*\}|catch\s*\{\s*\}`
- 占位 stub：`ElMessage\.info\(['"\`]`、`开发中|功能未实现|待开发|敬请期待|暂未|暂不支持`
- v-permission：`v-permission`
- 死代码：`from\s+['"][^'"]*tabs/StockTab` 等导入路径搜索

**未覆盖项**（需人工补充）：
- E2E 功能验证（需启动 dev server）
- 性能问题（大数据量列表虚拟滚动、内存泄漏）
- 可访问性（WCAG/ARIA 完整性）
- 跨浏览器兼容性

---

**报告生成时间**：2026-07-06
**复审执行**：v11 全项目扫描
**下一步**：按"第六节 修复优先级建议"创建批次任务逐项修复
