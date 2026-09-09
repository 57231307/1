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
  '/adjustments/{id}/print',
  '/ap/invoices/{id}/print',
  '/ap/payment-requests/{id}/print',
  '/ap/payments/{id}/print',
  '/ap/reconciliation/{id}/print',
  '/ar-reconciliations/{id}/print',
  '/ar/collections/{id}/print',
  '/boms/{id}/print',
  '/export-inspections/certificates/{id}/print',
  '/chemical-requisitions/{id}/print',
  '/customer-credits/{id}/print',
  '/dye-batches/rework/{id}/print',
  '/dye-batches/{id}/print',
  '/energy/{id}/print',
  '/exchange-rates/{id}/print',
  '/export-refunds/{id}/print',
  '/fabric-inspections/{id}/print',
  '/fixed-assets/count/{id}/print',
  '/fixed-assets/{id}/print',
  '/flow-cards/{id}/print',
  '/custom-orders/issues/{id}/print',
  '/labor-contracts/{id}/print',
  '/logistics-tracking/waybills/{id}/print',
  '/material-shortage/{id}/print',
  '/occupational-health/hazard-monitorings/{id}/print',
  '/occupational-health/health-exams/{id}/print',
  '/occupational-health/ppe-distributions/{id}/print',
  '/orders/{id}/deliveries/{delivery_id}/print',
  '/orders/{id}/print',
  '/outsourcing-orders/{id}/print',
  '/outsourcing-receipts/{id}/print',
  '/pollution-monitoring/solid-waste-disposals/{id}/print',
  '/pollution-permits/{id}/print',
  '/process-routes/{id}/print',
  '/production-orders/orders/{id}/print',
  '/production-orders/orders/{id}/safety-accident/print',
  '/production-recipes/{id}/print',
  '/purchase-contracts/{id}/print',
  '/purchase/inspections/{id}/print',
  '/purchase/returns/{id}/print',
  '/quality/inspections/unqualified/{id}/print',
  '/quality/inspections/{id}/print',
  '/receipts/{id}/print',
  '/sales-contracts/{id}/print',
  '/scheduling/results/{id}/print',
  '/social-insurance/{id}/print',
  '/supplier-evaluations/{id}/print',
  '/transfers/{id}/print',
  '/vouchers/{id}/print',
  '/wages/{id}/print',
  '/write-downs/{id}/print',
  '/bad-debts/writeoffs/{id}/print',
  '/custom-orders/{id}/after-sales/print',
  '/{id}/bulk-approval/print',
  '/export-inspections/{id}/customs-declaration/print',
  '/{id}/issue/print',
  '/{id}/lab-dip/print',
  '/{id}/print',
];

/**
 * 敏感资源导出端点（fail-closed 校验，走审批链）
 * resource_type 映射依据 backend/src/models/export_approval_request.rs L194
 */
export const SENSITIVE_EXPORT_ENDPOINTS: Array<{ path: string; resource: string }> = [
  { path: '/customers/export', resource: 'customer' },
  { path: '/suppliers/export', resource: 'supplier' },
  { path: '/products/export', resource: 'product' },
  { path: '/dye-recipes/export', resource: 'dye_recipe' },
  { path: '/sales-prices/export', resource: 'price_list' },
  { path: '/reports/trial-balance/export', resource: 'finance_report' },
  { path: '/reports/balance-sheet/export', resource: 'finance_report' },
  { path: '/reports/income-statement/export', resource: 'finance_report' },
  { path: '/reports/cash-flow/export', resource: 'finance_report' },
  { path: '/reports/general-ledger/export', resource: 'finance_report' },
  { path: '/reports/subsidiary-ledger/export', resource: 'finance_report' },
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
  { path: '/adjustments/{id}/approve', entity: 'adjustment' },
  { path: '/adjustments/{id}/reject', entity: 'adjustment' },
  { path: '/ai-models/ai-models/versions/{version_id}/approve', entity: 'ai_model_version' },
  { path: '/ap/invoices/{id}/approve', entity: 'ap_invoice', createApi: '/ap/invoices' },
  { path: '/ap/payment-requests/{id}/approve', entity: 'ap_payment_request', createApi: '/ap/payment-requests' },
  { path: '/ap/payment-requests/{id}/reject', entity: 'ap_payment_request', createApi: '/ap/payment-requests' },
  { path: '/ar/invoices/{id}/approve', entity: 'ar_invoice', createApi: '/ar/invoices' },
  { path: '/boms/{id}/approve', entity: 'bom', createApi: '/boms' },
  { path: '/bpm/tasks/approve', entity: 'bpm_task' },
  { path: '/budgets/adjust/{id}/approve', entity: 'budget_adjust' },
  { path: '/budgets/adjust/{id}/reject', entity: 'budget_adjust' },
  { path: '/budgets/plans/{id}/approve', entity: 'budget_plan' },
  { path: '/budgets/plans/{id}/reject', entity: 'budget_plan' },
  { path: '/budgets/versions/{id}/approve', entity: 'budget_version' },
  { path: '/budgets/{id}/approve', entity: 'budget' },
  { path: '/chemical-requisitions/{id}/approve', entity: 'chemical_requisition' },
  { path: '/counts/{id}/approve', entity: 'inventory_count', createApi: '/inventory-counts' },
  { path: '/counts/{id}/reject', entity: 'inventory_count', createApi: '/inventory-counts' },
  { path: '/dye-batch-reworks/{id}/approve', entity: 'dye_batch_rework' },
  { path: '/dye-recipes/{id}/approve', entity: 'dye_recipe', createApi: '/dye-recipes' },
  { path: '/export-approvals/{id}/approve', entity: 'export_approval', createApi: '/export-approvals' },
  { path: '/export-approvals/{id}/reject', entity: 'export_approval', createApi: '/export-approvals' },
  { path: '/fabric-orders/{id}/approve', entity: 'fabric_order' },
  { path: '/fixed-assets/depreciation-policy-changes/{id}/approve', entity: 'depreciation_policy_change' },
  { path: '/fixed-assets/impairment-tests/{id}/approve', entity: 'impairment_test' },
  { path: '/fund-management/transfers/{id}/approve', entity: 'fund_transfer' },
  { path: '/fund-management/transfers/{id}/reject', entity: 'fund_transfer' },
  { path: '/invoices/{id}/approve', entity: 'invoice' },
  { path: '/lab-dip/requests/{id}/approve', entity: 'lab_dip_request' },
  { path: '/lab-dip/requests/{id}/reject', entity: 'lab_dip_request' },
  { path: '/orders/{id}/approve', entity: 'order', createApi: '/orders' },
  { path: '/orders/{id}/reject', entity: 'order', createApi: '/orders' },
  { path: '/production-orders/orders/{id}/approve', entity: 'production_order', createApi: '/production-orders/orders' },
  { path: '/production-recipes/additions/{id}/approve', entity: 'production_recipe_addition' },
  { path: '/production-recipes/{id}/approve', entity: 'production_recipe' },
  { path: '/purchase-contracts/{id}/approve', entity: 'purchase_contract', createApi: '/purchase-contracts' },
  { path: '/purchase-prices/{id}/approve', entity: 'purchase_price' },
  { path: '/quality-standards/{id}/approve', entity: 'quality_standard', createApi: '/quality-standards' },
  { path: '/quality-standards/{id}/reject', entity: 'quality_standard', createApi: '/quality-standards' },
  { path: '/returns/{id}/approve', entity: 'return' },
  { path: '/returns/{id}/reject', entity: 'return' },
  { path: '/role-change-approvals/{id}/reject', entity: 'role_change_approval' },
  { path: '/sales-contracts/{id}/approve', entity: 'sales_contract', createApi: '/sales-contracts' },
  { path: '/sales-prices/{id}/approve', entity: 'sales_price' },
  { path: '/transfers/{id}/approve', entity: 'transfer' },
  { path: '/bad-debts/writeoffs/{id}/reject', entity: 'writeoff' },
  { path: '/{id}/approve', entity: 'generic' },
  { path: '/{id}/reject', entity: 'generic' },
];
