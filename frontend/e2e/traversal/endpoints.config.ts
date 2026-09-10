/**
 * P5.7/5.9/5.11 全量端点矩阵配置
 *
 * 清单生成方式：grep backend/src/routes/*.rs 路由宏
 * - print 端点 58 个（/{id}/print 模式）
 * - export 端点 50 个（/export 及 /export/{format} 模式，剔除 export-approvals 管理端点）
 * - approve/reject 端点 48 个（/{id}/approve 与 /{id}/reject 模式）
 *
 * {id} 占位符在执行时替换为真实实体 id（由前置创建调用链提供）
 * 无前置依赖的 print 端点用 default-id 策略：id=1（CI 种子数据固定首条）
 *
 * 路径别名修正（2026-09-10 二审）：以下端点为多段 nest 前缀形态，
 * 按 grep 段名书写会 404，已改为后端 nest 装配后的真实完整路径：
 * - /certificates/{id}/print          → /export-inspections/certificates/{id}/print
 * - /issues/{id}/print                → /custom-orders/issues/{id}/print
 * - /writeoffs/{id}/print|reject      → /bad-debts/writeoffs/{id}/print|reject
 * - /{id}/after-sales/print           → /custom-orders/{id}/after-sales/print
 * - /{id}/customs-declaration/print   → /export-inspections/{id}/customs-declaration/print
 */

export const PRINT_ENDPOINTS: string[] = [
  '/inventory/adjustments/{id}/print',
  '/finance/ap/invoices/{id}/print',
  '/finance/ap/payment-requests/{id}/print',
  '/finance/ap/payments/{id}/print',
  '/finance/ap/reconciliation/{id}/print',
  '/finance/ar-reconciliations/{id}/print',
  '/finance/ar/collections/{id}/print',
  '/boms/{id}/print',
  '/export-inspections/certificates/{id}/print',
  '/chemical-requisitions/{id}/print',
  '/crm/customer-credits/{id}/print',
  '/production/dye-batches/rework/{id}/print',
  '/production/dye-batches/{id}/print',
  '/production/energy/{id}/print',
  '/finance/exchange-rates/{id}/print',
  '/export-refunds/{id}/print',
  '/production/fabric-inspections/{id}/print',
  '/finance/fixed-assets/count/{id}/print',
  '/finance/fixed-assets/{id}/print',
  '/production/flow-cards/{id}/print',
  '/custom-orders/issues/{id}/print',
  '/labor-contracts/{id}/print',
  '/logistics-tracking/waybills/{id}/print',
  '/material-shortage/{id}/print',
  '/occupational-health/hazard-monitorings/{id}/print',
  '/occupational-health/health-exams/{id}/print',
  '/occupational-health/ppe-distributions/{id}/print',
  '/sales/orders/{id}/deliveries/{delivery_id}/print',
  '/sales/orders/{id}/print',
  '/production/outsourcing-orders/{id}/print',
  '/production/outsourcing-receipts/{id}/print',
  '/pollution-monitoring/solid-waste-disposals/{id}/print',
  '/pollution-permits/{id}/print',
  '/production/process-routes/{id}/print',
  '/production/production-orders/orders/{id}/print',
  '/production/production-orders/orders/{id}/safety-accident/print',
  '/production/production-recipes/{id}/print',
  '/purchase/purchase-contracts/{id}/print',
  '/purchase/purchase/inspections/{id}/print',
  '/purchase/purchase/returns/{id}/print',
  '/production/quality/inspections/unqualified/{id}/print',
  '/production/quality/inspections/{id}/print',
  '/purchase/receipts/{id}/print',
  '/sales/sales-contracts/{id}/print',
  '/scheduling/results/{id}/print',
  '/social-insurance/{id}/print',
  '/purchase/supplier-evaluations/{id}/print',
  '/inventory/transfers/{id}/print',
  '/finance/vouchers/{id}/print',
  '/production/wages/{id}/print',
  '/inventory/write-downs/{id}/print',
  '/bad-debts/writeoffs/{id}/print',
  '/custom-orders/{id}/after-sales/print',
  '/color-cards/{id}/bulk-approval/print',
  '/export-inspections/{id}/customs-declaration/print',
  '/color-cards/{id}/issue/print',
  '/color-cards/{id}/lab-dip/print',
  '/quotations/{id}/print',
];

/**
 * 敏感资源导出端点（fail-closed 校验，走审批链）
 * resource_type 映射依据 backend/src/models/export_approval_request.rs L194
 */
export const SENSITIVE_EXPORT_ENDPOINTS: Array<{ path: string; resource: string }> = [
  { path: '/crm/customers/export', resource: 'customer' },
  { path: '/purchase/suppliers/export', resource: 'supplier' },
  { path: '/products/export', resource: 'product' },
  { path: '/production/dye-recipes/export', resource: 'dye_recipe' },
  { path: '/sales/sales-prices/export', resource: 'price_list' },
  { path: '/finance/reports/trial-balance/export', resource: 'finance_report' },
  { path: '/finance/reports/balance-sheet/export', resource: 'finance_report' },
  { path: '/finance/reports/income-statement/export', resource: 'finance_report' },
  { path: '/finance/reports/cash-flow/export', resource: 'finance_report' },
  { path: '/finance/reports/general-ledger/export', resource: 'finance_report' },
  { path: '/finance/reports/subsidiary-ledger/export', resource: 'finance_report' },
  { path: '/audit-logs/export', resource: 'audit_log' },
];

/** 非敏感导出端点（直接 200 + xlsx 断言） */
export const NON_SENSITIVE_EXPORT_ENDPOINTS: string[] = [
  '/ap/invoices/export',
  '/ar/invoices/export',
  '/audit-logs/export-logs',
  '/budgets/export',
  '/cost-collections/export',
  '/dye-batches/export',
  '/energy-allocations/export',
  '/energy-consumptions/export',
  '/fixed-assets/export',
  '/issues/export',
  '/leads/export',
  '/login-logs/export',
  '/logs/export',
  '/opportunities/export',
  '/orders/export',
  '/production-orders/orders/export',
  '/quality-inspection/records/export',
  '/quality-standards/export',
  '/reports/export',
  '/reports/issue-detail/export',
  '/sales-analysis/export',
  '/sales-contracts/export',
  '/stock/export',
  '/subjects/export',
  '/vouchers/export',
  '/wage-records/export',
  '/warehouses/export',
];

/**
 * approve/reject 端点全量矩阵
 * 每项配置：路径 + 前置创建调用链（创建单据拿 id）+ 审批请求体
 * 前置调用链缺失的端点在执行时按 routes 文件内 handler 签名补齐
 */
export const APPROVE_ENDPOINTS: Array<{
  path: string;
  entity: string;
  createApi?: string;
  createBody?: Record<string, unknown>;
}> = [
  { path: '/inventory/adjustments/{id}/approve', entity: 'adjustment' },
  { path: '/inventory/adjustments/{id}/reject', entity: 'adjustment' },
  { path: '/ai-models/ai-models/versions/{version_id}/approve', entity: 'ai_model_version' },
  { path: '/finance/ap/invoices/{id}/approve', entity: 'ap_invoice', createApi: '/ap/invoices' },
  { path: '/finance/ap/payment-requests/{id}/approve', entity: 'ap_payment_request', createApi: '/ap/payment-requests' },
  { path: '/finance/ap/payment-requests/{id}/reject', entity: 'ap_payment_request', createApi: '/ap/payment-requests' },
  { path: '/finance/ar/invoices/{id}/approve', entity: 'ar_invoice', createApi: '/ar/invoices' },
  { path: '/boms/{id}/approve', entity: 'bom', createApi: '/boms' },
  { path: '/bpm/tasks/approve', entity: 'bpm_task' },
  { path: '/finance/budgets/adjust/{id}/approve', entity: 'budget_adjust' },
  { path: '/finance/budgets/adjust/{id}/reject', entity: 'budget_adjust' },
  { path: '/finance/budgets/plans/{id}/approve', entity: 'budget_plan' },
  { path: '/finance/budgets/plans/{id}/reject', entity: 'budget_plan' },
  { path: '/finance/budgets/versions/{id}/approve', entity: 'budget_version' },
  { path: '/finance/budgets/{id}/approve', entity: 'budget' },
  { path: '/chemical-requisitions/{id}/approve', entity: 'chemical_requisition' },
  { path: '/inventory/counts/{id}/approve', entity: 'inventory_count', createApi: '/inventory-counts' },
  { path: '/inventory/counts/{id}/reject', entity: 'inventory_count', createApi: '/inventory-counts' },
  { path: '/production/dye-batch-reworks/{id}/approve', entity: 'dye_batch_rework' },
  { path: '/production/dye-recipes/{id}/approve', entity: 'dye_recipe', createApi: '/dye-recipes' },
  { path: '/export-approvals/{id}/approve', entity: 'export_approval', createApi: '/export-approvals' },
  { path: '/export-approvals/{id}/reject', entity: 'export_approval', createApi: '/export-approvals' },
  { path: '/sales/fabric-orders/{id}/approve', entity: 'fabric_order' },
  { path: '/finance/fixed-assets/depreciation-policy-changes/{id}/approve', entity: 'depreciation_policy_change' },
  { path: '/finance/fixed-assets/impairment-tests/{id}/approve', entity: 'impairment_test' },
  { path: '/finance/fund-management/transfers/{id}/approve', entity: 'fund_transfer' },
  { path: '/finance/fund-management/transfers/{id}/reject', entity: 'fund_transfer' },
  { path: '/finance/invoices/{id}/approve', entity: 'invoice' },
  { path: '/production/lab-dip/requests/{id}/approve', entity: 'lab_dip_request' },
  { path: '/production/lab-dip/requests/{id}/reject', entity: 'lab_dip_request' },
  { path: '/sales/orders/{id}/approve', entity: 'order', createApi: '/orders' },
  { path: '/sales/orders/{id}/reject', entity: 'order', createApi: '/orders' },
  { path: '/production/production-orders/orders/{id}/approve', entity: 'production_order', createApi: '/production-orders/orders' },
  { path: '/production/production-recipes/additions/{id}/approve', entity: 'production_recipe_addition' },
  { path: '/production/production-recipes/{id}/approve', entity: 'production_recipe' },
  { path: '/purchase/purchase-contracts/{id}/approve', entity: 'purchase_contract', createApi: '/purchase-contracts' },
  { path: '/purchase/purchase-prices/{id}/approve', entity: 'purchase_price' },
  { path: '/quality-standards/{id}/approve', entity: 'quality_standard', createApi: '/quality-standards' },
  { path: '/quality-standards/{id}/reject', entity: 'quality_standard', createApi: '/quality-standards' },
  { path: '/purchase/returns/{id}/approve', entity: 'return' },
  { path: '/purchase/returns/{id}/reject', entity: 'return' },
  { path: '/role-change-approvals/{id}/reject', entity: 'role_change_approval' },
  { path: '/sales/sales-contracts/{id}/approve', entity: 'sales_contract', createApi: '/sales-contracts' },
  { path: '/sales/sales-prices/{id}/approve', entity: 'sales_price' },
  { path: '/inventory/transfers/{id}/approve', entity: 'transfer' },
  { path: '/bad-debts/writeoffs/{id}/reject', entity: 'writeoff' },
  { path: '/quotations/{id}/approve', entity: 'generic' },
  { path: '/quotations/{id}/reject', entity: 'generic' },
];
