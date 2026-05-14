# 前端功能实现详细规划

## 一、功能对比分析

### 后端API模块（93个Handler）

| 模块分类 | Handler名称 | 前端状态 | 优先级 |
|---------|------------|---------|-------|
| **基础管理** | auth_handler | ✅ 已实现 | - |
| | user_handler | ✅ 已实现 | - |
| | role_handler | ✅ 已实现 | - |
| | department_handler | ❌ 未实现 | 高 |
| | tenant_handler | ❌ 未实现 | 中 |

| **财务核心** | account_subject_handler | ❌ 未实现 | 高 |
| | accounting_period_handler | ❌ 未实现 | 高 |
| | voucher_handler | ❌ 未实现 | 高 |
| | finance_report_handler | ❌ 未实现 | 高 |
| | finance_invoice_handler | ❌ 未实现 | 高 |
| | finance_payment_handler | ❌ 未实现 | 高 |
| | omni_audit_handler | ❌ 未实现 | 高 |
| | assist_accounting_handler | ❌ 未实现 | 高 |
| | five_dimension_handler | ❌ 未实现 | 中 |

| **采购管理** | purchase_order_handler | ✅ 已实现 | - |
| | purchase_receipt_handler | ❌ 未实现 | 高 |
| | purchase_return_handler | ❌ 未实现 | 中 |
| | purchase_inspection_handler | ❌ 未实现 | 中 |
| | purchase_contract_handler | ❌ 未实现 | 中 |
| | purchase_price_handler | ❌ 未实现 | 中 |

| **销售管理** | sales_order_handler | ✅ 已实现 | - |
| | sales_fabric_order_handler | ❌ 未实现 | 高 |
| | sales_return_handler | ✅ 已实现 | - |
| | sales_contract_handler | ❌ 未实现 | 中 |
| | sales_price_handler | ❌ 未实现 | 中 |
| | sales_analysis_handler | ❌ 未实现 | 中 |

| **库存管理** | inventory_stock_handler | ✅ 已实现 | - |
| | inventory_batch_handler | ✅ 已实现 | - |
| | inventory_adjustment_handler | ❌ 未实现 | 高 |
| | inventory_count_handler | ❌ 未实现 | 高 |
| | inventory_transfer_handler | ❌ 未实现 | 高 |
| | warehouse_handler | ✅ 已实现 | - |

| **应收应付** | ap_invoice_handler | ✅ 已实现 | - |
| | ap_payment_handler | ✅ 已实现 | - |
| | ap_payment_request_handler | ✅ 已实现 | - |
| | ap_reconciliation_handler | ✅ 已实现 | - |
| | ap_report_handler | ✅ 已实现 | - |
| | ap_verification_handler | ✅ 已实现 | - |
| | ar_invoice_handler | ✅ 已实现 | - |
| | ar_reconciliation_handler | ❌ 未实现 | 高 |

| **专项管理** | cost_collection_handler | ✅ 已实现 | - |
| | budget_management_handler | ✅ 已实现 | - |
| | fund_management_handler | ✅ 已实现 | - |
| | fixed_asset_handler | ❌ 未实现 | 高 |
| | customer_credit_handler | ✅ 已实现 | - |
| | supplier_evaluation_handler | ✅ 已实现 | - |

| **面料行业** | dye_batch_handler | ❌ 未实现 | 高 |
| | dye_recipe_handler | ❌ 未实现 | 高 |
| | greige_fabric_handler | ❌ 未实现 | 高 |
| | bulk_product_handler | ❌ 未实现 | 中 |
| | dual_unit_converter_handler | ❌ 未实现 | 中 |
| | piece_split_handler | ❌ 未实现 | 中 |

| **生产管理** | production_order_handler | ✅ 已实现 | - |
| | logistics_handler | ❌ 未实现 | 中 |

| **质量管理** | quality_standard_handler | ✅ 已实现 | - |
| | quality_inspection_handler | ✅ 已实现 | - |

| **系统功能** | notification_handler | ✅ 已实现 | - |
| | user_notification_setting_handler | ❌ 未实现 | 中 |
| | data_permission_handler | ✅ 已实现 | - |
| | barcode_scanner_handler | ❌ 未实现 | 中 |
| | system_update_handler | ❌ 未实现 | 低 |
| | webhook_handler | ❌ 未实现 | 低 |
| | api_key_handler | ❌ 未实现 | 低 |

| **高级分析** | financial_analysis_handler | ✅ 已实现 | - |
| | ai_analysis_handler | ❌ 未实现 | 低 |
| | report_engine_handler | ❌ 未实现 | 低 |
| | business_trace_handler | ❌ 未实现 | 中 |
| | tracking_handler | ❌ 未实现 | 低 |

| **产品管理** | product_handler | ✅ 已实现 | - |
| | product_category_handler | ❌ 未实现 | 中 |

| **客户管理** | customer_handler | ✅ 已实现 | - |
| | crm_handler | ✅ 已实现 | - |

| **供应商管理** | supplier_handler | ✅ 已实现 | - |

| **审批流程** | bpm_handler | ✅ 已实现 | - |

| **多币种** | currency_handler | ✅ 已实现 | - |

---

## 二、待实现功能优先级划分

### 🔴 P0 - 紧急高优先级（核心业务）

| 序号 | 功能模块 | 文件路径 | 描述 |
|-----|---------|---------|------|
| 1 | 会计科目管理 | `/api/accountSubject.ts` / `/views/accountSubject/index.vue` | 科目增删改查、科目树管理 |
| 2 | 会计期间管理 | `/api/accountingPeriod.ts` / `/views/accountingPeriod/index.vue` | 期间设置、结账处理 |
| 3 | 凭证管理 | `/api/voucher.ts` / `/views/voucher/index.vue` | 凭证录入、审核、记账 |
| 4 | 财务报表 | `/api/financeReport.ts` / `/views/financeReport/index.vue` | 资产负债表、利润表、现金流量表 |
| 5 | 应收对账 | `/api/arReconciliation.ts` / `/views/arReconciliation/index.vue` | 应收对账处理 |
| 6 | 库存调整 | `/api/inventoryAdjustment.ts` / `/views/inventoryAdjustment/index.vue` | 库存调整单管理 |
| 7 | 库存盘点 | `/api/inventoryCount.ts` / `/views/inventoryCount/index.vue` | 盘点单管理 |
| 8 | 库存转移 | `/api/inventoryTransfer.ts` / `/views/inventoryTransfer/index.vue` | 库存调拨 |
| 9 | 采购入库 | `/api/purchaseReceipt.ts` / `/views/purchaseReceipt/index.vue` | 采购入库单 |
| 10 | 面料销售订单 | `/api/salesFabricOrder.ts` / `/views/salesFabricOrder/index.vue` | 面料行业特色订单 |

### 🟠 P1 - 高优先级（业务完整性）

| 序号 | 功能模块 | 文件路径 | 描述 |
|-----|---------|---------|------|
| 11 | 固定资产管理 | `/api/fixedAsset.ts` / `/views/fixedAsset/index.vue` | 资产新增、折旧、处置 |
| 12 | 染缸批次管理 | `/api/dyeBatch.ts` / `/views/dyeBatch/index.vue` | 染色批次跟踪 |
| 13 | 染色配方管理 | `/api/dyeRecipe.ts` / `/views/dyeRecipe/index.vue` | 配方管理、配比计算 |
| 14 | 坯布管理 | `/api/greigeFabric.ts` / `/views/greigeFabric/index.vue` | 坯布入库、库存管理 |
| 15 | 全量审计 | `/api/omniAudit.ts` / `/views/omniAudit/index.vue` | 操作日志、审计追踪 |
| 16 | 辅助核算 | `/api/assistAccounting.ts` / `/views/assistAccounting/index.vue` | 项目/部门/人员辅助核算 |
| 17 | 业务追溯 | `/api/businessTrace.ts` / `/views/businessTrace/index.vue` | 单据溯源、关联查询 |

### 🟡 P2 - 中优先级（功能增强）

| 序号 | 功能模块 | 文件路径 | 描述 |
|-----|---------|---------|------|
| 18 | 部门管理 | `/api/department.ts` / `/views/department/index.vue` | 部门组织结构管理 |
| 19 | 采购退货 | `/api/purchaseReturn.ts` / `/views/purchaseReturn/index.vue` | 采购退货单 |
| 20 | 采购检验 | `/api/purchaseInspection.ts` / `/views/purchaseInspection/index.vue` | 来料检验 |
| 21 | 采购合同 | `/api/purchaseContract.ts` / `/views/purchaseContract/index.vue` | 合同管理 |
| 22 | 销售合同 | `/api/salesContract.ts` / `/views/salesContract/index.vue` | 销售合同管理 |
| 23 | 销售分析 | `/api/salesAnalysis.ts` / `/views/salesAnalysis/index.vue` | 销售数据分析 |
| 24 | 产品分类 | `/api/productCategory.ts` / `/views/productCategory/index.vue` | 产品类别管理 |
| 25 | 五维管理 | `/api/fiveDimension.ts` / `/views/fiveDimension/index.vue` | 五维分析管理 |
| 26 | 扫码功能 | `/api/barcodeScanner.ts` / `/views/barcodeScanner/index.vue` | 条码扫描、二维码识别 |
| 27 | 用户通知设置 | `/api/userNotificationSetting.ts` | 通知偏好配置 |

### 🟢 P3 - 低优先级（扩展功能）

| 序号 | 功能模块 | 文件路径 | 描述 |
|-----|---------|---------|------|
| 28 | 租户管理 | `/api/tenant.ts` / `/views/tenant/index.vue` | 多租户管理 |
| 29 | Webhook管理 | `/api/webhook.ts` / `/views/webhook/index.vue` | 外部通知配置 |
| 30 | API密钥管理 | `/api/apiKey.ts` / `/views/apiKey/index.vue` | API访问控制 |
| 31 | AI分析 | `/api/aiAnalysis.ts` / `/views/aiAnalysis/index.vue` | 智能分析功能 |
| 32 | 报表引擎 | `/api/reportEngine.ts` / `/views/reportEngine/index.vue` | 自定义报表 |
| 33 | 系统更新 | `/api/systemUpdate.ts` | 系统升级管理 |
| 34 | 物流管理 | `/api/logistics.ts` / `/views/logistics/index.vue` | 物流追踪 |
| 35 | 分匹管理 | `/api/pieceSplit.ts` / `/views/pieceSplit/index.vue` | 面料分匹 |

---

## 三、实施顺序建议

### 阶段一：财务核心（第1-2周）
- 会计科目管理
- 会计期间管理
- 凭证管理
- 财务报表
- 应收对账

### 阶段二：供应链管理（第3-4周）
- 库存调整
- 库存盘点
- 库存转移
- 采购入库
- 采购退货

### 阶段三：面料行业特色（第5-6周）
- 面料销售订单
- 染缸批次管理
- 染色配方管理
- 坯布管理

### 阶段四：资产管理（第7周）
- 固定资产管理
- 辅助核算

### 阶段五：审计与追溯（第8周）
- 全量审计
- 业务追溯

### 阶段六：高级功能（第9-10周）
- AI分析
- 报表引擎
- 系统配置

---

## 四、关键技术规范

### API层规范
```typescript
// 文件位置: src/api/{module}.ts
export interface {Module}Entity {
  id?: number;
  // ...字段定义
}

export function list{Module}(params?: QueryParams): Promise<ApiResponse<{list: {Module}Entity[], total: number}>> {
  return request.get('/api/v1/{module}', { params });
}

export function get{Module}(id: number): Promise<ApiResponse<{Module}Entity>> {
  return request.get(`/api/v1/{module}/${id}`);
}

export function create{Module}(data: Partial<{Module}Entity>): Promise<ApiResponse<{Module}Entity>> {
  return request.post('/api/v1/{module}', data);
}

export function update{Module}(id: number, data: Partial<{Module}Entity>): Promise<ApiResponse<{Module}Entity>> {
  return request.put(`/api/v1/{module}/${id}`, data);
}

export function delete{Module}(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api/v1/{module}/${id}`);
}
```

### 页面层规范
- 统一使用 Element Plus 组件
- 表格使用 el-table，支持分页和筛选
- 表单使用 el-form，带验证规则
- 弹窗使用 el-dialog
- 状态管理使用 Pinia（如需）

### 路由配置规范
```typescript
// 文件位置: src/router/index.ts
{
  path: '/{module}',
  name: '{Module}',
  component: () => import('@/views/{module}/index.vue'),
  meta: { title: '显示名称', requiresAuth: true }
}
```

---

## 五、当前已实现功能清单

### 已完成的API文件（11个）
- `/workspace/frontend/src/api/production.ts` - 生产订单
- `/workspace/frontend/src/api/cost.ts` - 成本归集
- `/workspace/frontend/src/api/budget.ts` - 预算管理
- `/workspace/frontend/src/api/fund.ts` - 资金管理
- `/workspace/frontend/src/api/financial-analysis.ts` - 财务分析
- `/workspace/frontend/src/api/supplierEvaluation.ts` - 供应商评估
- `/workspace/frontend/src/api/customerCredit.ts` - 客户信用
- `/workspace/frontend/src/api/currency.ts` - 多币种管理
- `/workspace/frontend/src/api/notification.ts` - 通知中心
- `/workspace/frontend/src/api/dataPermission.ts` - 数据权限
- `/workspace/frontend/src/api/inventoryBatch.ts` - 批次管理

### 已完成的页面（35个）
- Dashboard、Login、403、404
- system、finance、ap、ar、fabric、inventory、sales、purchase
- customer、supplier、product、warehouse、bpm、quality
- purchase-ext、sales-ext、crm、advanced、production
- cost、budget、fund、financial-analysis
- supplierEvaluation、customerCredit、currency、notification
- dataPermission、inventoryBatch

---

## 六、后续开发建议

1. **优先级排序原则**：优先实现与财务核心、供应链、面料行业特色相关的功能
2. **代码复用**：参考现有页面结构，保持风格一致
3. **测试验证**：每个模块完成后进行构建验证
4. **文档同步**：关键业务模块需补充业务说明文档
