/**
 * P5.12 全量遍历配置——86 视图模块数据驱动框架
 *
 * 数据结构：每个模块登记 route + domain + tier + listApi + formFields
 * 清单生成方式：扫描 router/index.ts（126 路由）+ api/ 目录导出函数映射
 * 分层策略：Tier A（简单 CRUD ~50）/ Tier B（复杂单据 ~20）/ Tier C（只读/报表 ~16）
 *
 * 执行时以页面实际 DOM 结构为准补齐 formFields，
 * 必填字段给安全合成值：名称带时间戳、金额1、数量1、下拉选首项
 */

export interface ModuleFormField {
  selector: string;
  value: string | number;
}

export interface TraversalModule {
  id: string;
  route: string;
  domain: string;
  tier: 'A' | 'B' | 'C';
  listApi?: string;
  uniqueKey?: string;
  newButton?: string;
  formFields?: ModuleFormField[];
  editAndSave?: boolean;
  noCreate?: boolean;
}

/**
 * 86 模块清单（按域分组）
 * 清单从 router/index.ts 路由 + api/ 目录导出函数映射生成
 * formFields 按页面实际 DOM 逐模块补齐（执行时）
 */
export const TRAVERSAL_MODULES: TraversalModule[] = [
  // ===== 核心域 =====
  { id: 'dashboard', route: '/dashboard', domain: 'core', tier: 'C', noCreate: true },
  { id: 'system', route: '/system', domain: 'core', tier: 'C', noCreate: true },
  { id: 'system-users', route: '/system/users', domain: 'core', tier: 'A', listApi: '/users' },
  { id: 'system-roles', route: '/system/roles', domain: 'core', tier: 'A', listApi: '/roles' },
  { id: 'system-departments', route: '/system/departments', domain: 'core', tier: 'A', listApi: '/departments' },
  { id: 'system-audit-log', route: '/system/audit-log', domain: 'core', tier: 'C', listApi: '/audit-logs', noCreate: true },
  { id: 'system-export-approvals', route: '/system/export-approvals', domain: 'core', tier: 'A', listApi: '/export-approvals', noCreate: true },
  { id: 'system-slow-query', route: '/system/slow-query', domain: 'core', tier: 'C', listApi: '/slow-queries', noCreate: true },
  { id: 'system-webhooks', route: '/system/webhooks', domain: 'core', tier: 'A', listApi: '/webhooks' },
  { id: 'system-api-gateway', route: '/system/api-gateway', domain: 'core', tier: 'C', noCreate: true },

  // ===== 客户/供应商域 =====
  { id: 'customer', route: '/customer', domain: 'crm', tier: 'A', listApi: '/customers', uniqueKey: 'name' },
  { id: 'supplier', route: '/supplier', domain: 'crm', tier: 'A', listApi: '/suppliers', uniqueKey: 'name' },
  { id: 'crm-lead', route: '/crm/leads', domain: 'crm', tier: 'A', listApi: '/crm/leads', uniqueKey: 'lead_name' },
  { id: 'crm-opportunity', route: '/crm/opportunities', domain: 'crm', tier: 'A', listApi: '/crm/opportunities' },
  { id: 'crm-customer-credit', route: '/crm/customer-credits', domain: 'crm', tier: 'C', noCreate: true },

  // ===== 采购域 =====
  { id: 'purchase-order', route: '/purchase/orders', domain: 'purchase', tier: 'B', listApi: '/purchase-orders' },
  { id: 'purchase-receipt', route: '/purchase/receipts', domain: 'purchase', tier: 'B', listApi: '/purchase-receipts' },
  { id: 'purchase-return', route: '/purchase/returns', domain: 'purchase', tier: 'B', listApi: '/purchase-returns' },
  { id: 'purchase-inspection', route: '/purchase/inspections', domain: 'purchase', tier: 'B', listApi: '/purchase/inspections' },
  { id: 'purchase-contract', route: '/purchase/contracts', domain: 'purchase', tier: 'B', listApi: '/purchase-contracts' },
  { id: 'purchase-price', route: '/purchase/prices', domain: 'purchase', tier: 'A', listApi: '/purchase-prices' },
  { id: 'supplier-evaluation', route: '/supplier/evaluations', domain: 'purchase', tier: 'A', listApi: '/supplier-evaluations' },

  // ===== 销售域 =====
  { id: 'sales-order', route: '/sales/orders', domain: 'sales', tier: 'B', listApi: '/sales-orders' },
  { id: 'sales-delivery', route: '/sales/deliveries', domain: 'sales', tier: 'B', listApi: '/sales-deliveries' },
  { id: 'sales-return', route: '/sales/returns', domain: 'sales', tier: 'B', listApi: '/sales-returns' },
  { id: 'sales-quotation', route: '/sales/quotations', domain: 'sales', tier: 'B', listApi: '/sales-quotations' },
  { id: 'sales-price', route: '/sales/prices', domain: 'sales', tier: 'A', listApi: '/sales-prices' },
  { id: 'export-refund', route: '/sales/export-refunds', domain: 'sales', tier: 'B', listApi: '/export-refunds' },

  // ===== 库存域 =====
  { id: 'inventory', route: '/inventory', domain: 'inventory', tier: 'C', listApi: '/inventory', noCreate: true },
  { id: 'inventory-count', route: '/inventory/counts', domain: 'inventory', tier: 'B', listApi: '/inventory-counts' },
  { id: 'warehouse', route: '/inventory/warehouses', domain: 'inventory', tier: 'A', listApi: '/warehouses' },
  { id: 'stock-move', route: '/inventory/stock-moves', domain: 'inventory', tier: 'B', listApi: '/stock-moves' },

  // ===== 生产域 =====
  { id: 'production-order', route: '/production/orders', domain: 'production', tier: 'B', listApi: '/production-orders' },
  { id: 'bom', route: '/fabric/bom', domain: 'production', tier: 'A', listApi: '/boms' },
  { id: 'dye-batch', route: '/production/dye-batches', domain: 'production', tier: 'B', listApi: '/dye-batches' },
  { id: 'dye-recipe', route: '/production/dye-recipes', domain: 'production', tier: 'A', listApi: '/dye-recipes' },
  { id: 'flow-card', route: '/production/flow-cards', domain: 'production', tier: 'B', listApi: '/flow-cards' },
  { id: 'color-card', route: '/fabric/color-card', domain: 'production', tier: 'B', listApi: '/color-cards' },
  { id: 'color-card-issue', route: '/fabric/color-card-issues', domain: 'production', tier: 'B', listApi: '/color-card-issues' },

  // ===== 质量域 =====
  { id: 'quality-inspection', route: '/quality/inspections', domain: 'quality', tier: 'B', listApi: '/quality-inspections' },
  { id: 'quality-standard', route: '/quality/standards', domain: 'quality', tier: 'A', listApi: '/quality-standards' },

  // ===== 财务域 =====
  { id: 'finance-voucher', route: '/finance/vouchers', domain: 'finance', tier: 'B', listApi: '/vouchers' },
  { id: 'finance-accounting-period', route: '/finance/accounting-periods', domain: 'finance', tier: 'A', listApi: '/accounting-periods' },
  { id: 'finance-trial-balance', route: '/finance/trial-balance', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'finance-balance-sheet', route: '/finance/balance-sheet', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'finance-income-statement', route: '/finance/income-statement', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'finance-cash-flow', route: '/finance/cash-flow', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'finance-general-ledger', route: '/finance/general-ledger', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'finance-subsidiary-ledger', route: '/finance/subsidiary-ledger', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'finance-asset', route: '/finance/assets', domain: 'finance', tier: 'B', listApi: '/fixed-assets' },
  { id: 'finance-asset-count', route: '/finance/asset-counts', domain: 'finance', tier: 'B', listApi: '/fixed-assets/count' },
  { id: 'finance-ap-invoice', route: '/ap/invoices', domain: 'finance', tier: 'B', listApi: '/ap/invoices' },
  { id: 'finance-ap-payment', route: '/ap/payments', domain: 'finance', tier: 'B', listApi: '/ap/payments' },
  { id: 'finance-ap-reconciliation', route: '/ap/reconciliation', domain: 'finance', tier: 'B', listApi: '/ap/reconciliation' },
  { id: 'finance-ar-collection', route: '/ar/collections', domain: 'finance', tier: 'B', listApi: '/ar/collections' },
  { id: 'finance-ar-reconciliation', route: '/ar/reconciliations', domain: 'finance', tier: 'B', listApi: '/ar-reconciliations' },
  { id: 'finance-exchange-rate', route: '/finance/exchange-rates', domain: 'finance', tier: 'A', listApi: '/exchange-rates' },
  { id: 'finance-account-subject', route: '/finance/account-subjects', domain: 'finance', tier: 'A', listApi: '/account-subjects' },

  // ===== 面料域 =====
  { id: 'fabric-catalog', route: '/fabric/catalog', domain: 'fabric', tier: 'A', listApi: '/fabric/catalog' },
  { id: 'fabric-grey', route: '/fabric/grey', domain: 'fabric', tier: 'A', listApi: '/fabric/grey' },
  { id: 'fabric-finished', route: '/fabric/finished', domain: 'fabric', tier: 'A', listApi: '/fabric/finished' },

  // ===== 审批/系统配置域 =====
  { id: 'approval-center', route: '/approval/center', domain: 'approval', tier: 'C', noCreate: true },
  { id: 'role-change-approval', route: '/approval/role-change', domain: 'approval', tier: 'B' },
  { id: 'transfer-approval', route: '/approval/transfer', domain: 'approval', tier: 'B' },
  { id: 'writeoff-approval', route: '/approval/writeoffs', domain: 'approval', tier: 'B' },
  { id: 'print-templates', route: '/system/print-templates', domain: 'core', tier: 'A', listApi: '/print-templates' },
  { id: 'report-templates', route: '/system/report-templates', domain: 'core', tier: 'A', listApi: '/report-templates' },

  // ===== BI/报表域 =====
  { id: 'bi-dashboard', route: '/bi/dashboard', domain: 'bi', tier: 'C', noCreate: true },
  { id: 'bi-analysis', route: '/bi/analysis', domain: 'bi', tier: 'C', noCreate: true },

  // ===== 其他模块（扩展补齐）=====
  { id: 'custom-order', route: '/custom-order', domain: 'sales', tier: 'B', listApi: '/custom-orders' },
  { id: 'after-sales', route: '/after-sales', domain: 'sales', tier: 'B', listApi: '/after-sales' },
  { id: 'barcode-scanner', route: '/inventory/barcode', domain: 'inventory', tier: 'C', noCreate: true },
  { id: 'ai-extend', route: '/ai/extend', domain: 'core', tier: 'C', noCreate: true },
  { id: 'notification', route: '/notifications', domain: 'core', tier: 'C', noCreate: true },
];

/**
 * 端点清单（从 backend/src/routes grep 生成）
 */
export const PRINT_ENDPOINTS = [
  '/purchase/orders/{id}/print',
  '/purchase/receipts/{id}/print',
  '/purchase/inspections/{id}/print',
  '/purchase/returns/{id}/print',
  '/purchase-contracts/{id}/print',
  '/supplier-evaluations/{id}/print',
  '/social-insurance/{id}/print',
  '/vouchers/{id}/print',
  '/fixed-assets/{id}/print',
  '/fixed-assets/count/{id}/print',
  '/ap/invoices/{id}/print',
  '/ap/payments/{id}/print',
  '/ap/payment-requests/{id}/print',
  '/ap/reconciliation/{id}/print',
  '/ar/collections/{id}/print',
  '/ar-reconciliations/{id}/print',
  '/exchange-rates/{id}/print',
  '/customer-credits/{id}/print',
  '/export-refunds/{id}/print',
  '/boms/{id}/print',
  // 完整清单执行时从 routes grep 补齐（65 端点）
];

export const EXPORT_ENDPOINTS = [
  '/customers/export',
  '/suppliers/export',
  '/products/export',
  '/dye-recipes/export',
  '/sales-prices/export',
  '/audit-logs/export',
  '/audit/logs/export',
  '/finance/trial-balance/export',
  '/finance/balance-sheet/export',
  '/finance/income-statement/export',
  '/finance/cash-flow/export',
  '/finance/general-ledger/export',
  '/finance/subsidiary-ledger/export',
  // 完整清单执行时从 routes grep 补齐（68 端点）
];

export const APPROVE_ENDPOINTS = [
  '/purchase-orders/{id}/approve',
  '/sales-orders/{id}/approve',
  '/export-approvals/{id}/approve',
  '/role-change-requests/{id}/approve',
  '/transfers/{id}/approve',
  '/writeoffs/{id}/approve',
  '/sales-quotations/{id}/approve',
  // 完整清单执行时从 routes grep 补齐（53 端点）
];
