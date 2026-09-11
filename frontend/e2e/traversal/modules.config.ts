/**
 * P5.12 全量遍历配置——全部视图模块数据驱动框架（完整版）
 *
 * 清单来源：frontend/src/router/index.ts 真实路由（120+ 条）逐模块登记
 * 分层策略：Tier A（简单 CRUD，visit+新建+填表+保存+回读）
 *           Tier B（复杂单据，显式 spec 逐个真实业务链）
 *           Tier C（只读/报表/仪表盘，visit+白屏+表头渲染断言）
 *
 * uniqueKey：幂等清理依据（跑完按 API DELETE 回收，删不掉标记 skip-cleanup）
 * formFields：必填字段安全合成值（名称带时间戳、金额 1、数量 1、下拉选首项）
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
  noCreate?: boolean;
}

/**
 * 全量模块清单（router/index.ts 真实路由映射）
 */
export const TRAVERSAL_MODULES: TraversalModule[] = [
  // ===== 核心域（router 根级）=====
  { id: 'dashboard', route: '/dashboard', domain: 'core', tier: 'C', noCreate: true },
  { id: 'system', route: '/system', domain: 'core', tier: 'C', noCreate: true },
  { id: 'departments', route: '/departments', domain: 'core', tier: 'A', listApi: '/departments' },
  { id: 'notification', route: '/notification', domain: 'core', tier: 'C', noCreate: true },
  { id: 'data-permission', route: '/data-permission', domain: 'core', tier: 'C', noCreate: true },
  { id: 'omni-audit', route: '/omni-audit', domain: 'core', tier: 'C', noCreate: true },
  { id: 'business-trace', route: '/business-trace', domain: 'core', tier: 'C', noCreate: true },
  { id: 'components-demo', route: '/components-demo', domain: 'core', tier: 'C', noCreate: true },
  { id: 'workflow', route: '/workflow', domain: 'core', tier: 'C', noCreate: true },

  // ===== system 域 =====
  { id: 'system-audit-log', route: '/system/audit-log', domain: 'system', tier: 'C', listApi: '/audit-logs', noCreate: true },
  { id: 'system-export-approvals', route: '/system/export-approvals', domain: 'system', tier: 'C', listApi: '/export-approvals', noCreate: true },
  { id: 'system-slow-query', route: '/system/slow-query', domain: 'system', tier: 'C', noCreate: true },
  { id: 'report-templates', route: '/report-templates', domain: 'system', tier: 'A', listApi: '/report-templates' },
  // print-templates 为内置模板只读展示（后端无创建端点），新建断言跳过
  { id: 'print-templates', route: '/print-templates', domain: 'system', tier: 'A', listApi: '/print-templates', noCreate: true },
  { id: 'api-gateway', route: '/api-gateway', domain: 'system', tier: 'C', noCreate: true },
  { id: 'system-update', route: '/system-update', domain: 'system', tier: 'C', noCreate: true },
  { id: 'system-profile', route: '/system/profile', domain: 'system', tier: 'C', noCreate: true },
  { id: 'admin-failover', route: '/admin/failover', domain: 'system', tier: 'C', noCreate: true },
  { id: 'email', route: '/email', domain: 'system', tier: 'A', listApi: '/email-templates' },
  { id: 'security', route: '/security', domain: 'system', tier: 'C', noCreate: true },
  { id: 'security-two-factor', route: '/security/two-factor-setup', domain: 'system', tier: 'C', noCreate: true },
  { id: 'security-change-password', route: '/security/change-password', domain: 'system', tier: 'C', noCreate: true },
  { id: 'data-import', route: '/data-import', domain: 'system', tier: 'C', noCreate: true },

  // ===== finance 域 =====
  { id: 'finance', route: '/finance', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'voucher', route: '/voucher', domain: 'finance', tier: 'B', listApi: '/vouchers' },
  { id: 'account-subject', route: '/account-subject', domain: 'finance', tier: 'A', listApi: '/finance/subjects' },
  { id: 'accounting-period', route: '/accounting-period', domain: 'finance', tier: 'A', listApi: '/accounting-periods' },
  { id: 'finance-report', route: '/finance-report', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'ar-reconciliation', route: '/ar-reconciliation', domain: 'finance', tier: 'B', listApi: '/ar-reconciliations' },
  { id: 'ar-reconciliation-enhanced', route: '/ar-reconciliation/enhanced', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'fixed-assets', route: '/fixed-assets', domain: 'finance', tier: 'B', listApi: '/fixed-assets' },
  { id: 'cost', route: '/cost', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'budget', route: '/budget', domain: 'finance', tier: 'B', listApi: '/budgets' },
  { id: 'fund', route: '/fund', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'financial-analysis', route: '/financial-analysis', domain: 'finance', tier: 'C', noCreate: true },
  { id: 'currency', route: '/currency', domain: 'finance', tier: 'A', listApi: '/exchange-rates' },
  { id: 'ap', route: '/ap', domain: 'finance', tier: 'B', listApi: '/ap/invoices' },
  { id: 'ar', route: '/ar', domain: 'finance', tier: 'B', listApi: '/ar/invoices' },
  { id: 'assist-accounting', route: '/assist-accounting', domain: 'finance', tier: 'C', noCreate: true },

  // ===== sales 域 =====
  { id: 'sales', route: '/sales', domain: 'sales', tier: 'C', noCreate: true },
  { id: 'sales-returns', route: '/sales-returns', domain: 'sales', tier: 'B', listApi: '/sales-returns' },
  { id: 'sales-contract', route: '/sales-contract', domain: 'sales', tier: 'B', listApi: '/sales-contracts' },
  { id: 'sales-price', route: '/sales-price', domain: 'sales', tier: 'A', listApi: '/sales-prices' },
  { id: 'sales-ext', route: '/sales-ext', domain: 'sales', tier: 'C', noCreate: true },
  { id: 'sales-analysis-bi', route: '/bi/sales-analysis', domain: 'sales', tier: 'C', noCreate: true },
  { id: 'quotations', route: '/quotations', domain: 'sales', tier: 'B', listApi: '/quotations' },
  { id: 'quotations-new', route: '/quotations/new', domain: 'sales', tier: 'B', noCreate: true },
  { id: 'custom-orders', route: '/custom-orders', domain: 'sales', tier: 'B', listApi: '/custom-orders' },
  { id: 'after-sales', route: '/after-sales', domain: 'sales', tier: 'B',  },
  { id: 'logistics', route: '/logistics', domain: 'sales', tier: 'C', noCreate: true },
  { id: 'trading', route: '/trading', domain: 'sales', tier: 'C', noCreate: true },

  // ===== purchase 域 =====
  { id: 'purchase', route: '/purchase', domain: 'purchase', tier: 'C', noCreate: true },
  { id: 'purchase-receipt', route: '/purchase-receipt', domain: 'purchase', tier: 'B', listApi: '/purchase/receipts' },
  { id: 'purchase-contract', route: '/purchase-contract', domain: 'purchase', tier: 'B', listApi: '/purchase-contracts' },
  { id: 'purchase-price', route: '/purchase-price', domain: 'purchase', tier: 'A', listApi: '/purchase-prices' },
  { id: 'purchase-inspection', route: '/purchase-inspection', domain: 'purchase', tier: 'B', listApi: '/purchase/inspections' },
  { id: 'purchase-return', route: '/purchase-return', domain: 'purchase', tier: 'B', listApi: '/purchase/returns' },
  { id: 'purchase-ext', route: '/purchase-ext', domain: 'purchase', tier: 'C', noCreate: true },
  { id: 'supplier-evaluation', route: '/supplier-evaluation', domain: 'purchase', tier: 'A', listApi: '/supplier-evaluations' },

  // ===== crm 域 =====
  { id: 'crm', route: '/crm', domain: 'crm', tier: 'C', noCreate: true },
  { id: 'crm-pool', route: '/crm/pool', domain: 'crm', tier: 'C', noCreate: true },
  { id: 'crm-assignment', route: '/crm/assignment', domain: 'crm', tier: 'C', noCreate: true },
  { id: 'crm-leads', route: '/crm/leads', domain: 'crm', tier: 'A', listApi: '/crm/leads' },
  { id: 'crm-opportunities', route: '/crm/opportunities', domain: 'crm', tier: 'A', listApi: '/crm/opportunities' },
  { id: 'customer', route: '/customer', domain: 'crm', tier: 'A', listApi: '/customers', uniqueKey: 'name' },
  { id: 'customer-credit', route: '/customer-credit', domain: 'crm', tier: 'C', noCreate: true },

  // ===== supplier 域 =====
  { id: 'supplier', route: '/supplier', domain: 'supplier', tier: 'A', listApi: '/suppliers', uniqueKey: 'name' },

  // ===== product/fabric 域 =====
  { id: 'product', route: '/product', domain: 'product', tier: 'A', listApi: '/products', uniqueKey: 'name' },
  { id: 'fabric', route: '/fabric', domain: 'fabric', tier: 'C', noCreate: true },
  { id: 'greige-fabrics', route: '/greige-fabrics', domain: 'fabric', tier: 'A', listApi: '/greige-fabrics' },
  { id: 'color-cards-list', route: '/color-cards/list', domain: 'fabric', tier: 'B', listApi: '/color-cards' },
  { id: 'color-cards-issues', route: '/color-cards/issues', domain: 'fabric', tier: 'B', listApi: '/color-cards/issues' },
  { id: 'color-prices-list', route: '/color-prices/list', domain: 'fabric', tier: 'A', listApi: '/color-prices' },
  { id: 'color-prices-batch', route: '/color-prices/batch-adjust', domain: 'fabric', tier: 'C', noCreate: true },
  { id: 'dye-recipe', route: '/dye-recipe', domain: 'fabric', tier: 'B', listApi: '/dye-recipes' },
  { id: 'dye-batch', route: '/dye-batch', domain: 'fabric', tier: 'B', listApi: '/dye-batches' },

  // ===== inventory 域 =====
  { id: 'inventory', route: '/inventory', domain: 'inventory', tier: 'C', noCreate: true },
  { id: 'warehouse', route: '/warehouse', domain: 'inventory', tier: 'A', listApi: '/warehouses' },
  { id: 'inventory-count', route: '/inventory-count', domain: 'inventory', tier: 'B', listApi: '/inventory/counts' },
  { id: 'inventory-transfer', route: '/inventory-transfer', domain: 'inventory', tier: 'B', listApi: '/transfers' },
  { id: 'inventory-adjustment', route: '/inventory-adjustment', domain: 'inventory', tier: 'B', listApi: '/adjustments' },
  { id: 'inventory-batch', route: '/inventory-batch', domain: 'inventory', tier: 'C', noCreate: true },
  { id: 'five-dimension', route: '/five-dimension', domain: 'inventory', tier: 'C', noCreate: true },
  { id: 'barcode-scanner', route: '/barcode-scanner', domain: 'inventory', tier: 'C', noCreate: true },

  // ===== production 域 =====
  { id: 'production', route: '/production', domain: 'production', tier: 'C', noCreate: true },
  { id: 'bom', route: '/bom', domain: 'production', tier: 'A', listApi: '/boms' },
  { id: 'mrp', route: '/mrp', domain: 'production', tier: 'C', noCreate: true },
  { id: 'mrp-history', route: '/mrp/history', domain: 'production', tier: 'C', noCreate: true },
  { id: 'capacity', route: '/capacity', domain: 'production', tier: 'C', noCreate: true },
  { id: 'material-shortage', route: '/material-shortage', domain: 'production', tier: 'C', noCreate: true },
  { id: 'scheduling', route: '/scheduling', domain: 'production', tier: 'C', noCreate: true },
  { id: 'scheduling-gantt', route: '/scheduling/gantt', domain: 'production', tier: 'C', noCreate: true },
  { id: 'process-routes', route: '/process-routes', domain: 'production', tier: 'A', listApi: '/process-routes' },

  // ===== quality 域 =====
  { id: 'quality', route: '/quality', domain: 'quality', tier: 'C', noCreate: true },
  { id: 'quality-standards', route: '/quality-standards', domain: 'quality', tier: 'A', listApi: '/quality-standards' },

  // ===== bpm 域 =====
  { id: 'bpm', route: '/bpm', domain: 'bpm', tier: 'C', noCreate: true },
  { id: 'bpm-definitions', route: '/bpm/definitions', domain: 'bpm', tier: 'C', noCreate: true },
  { id: 'bpm-templates', route: '/bpm/templates', domain: 'bpm', tier: 'C', noCreate: true },
  { id: 'bpm-approval', route: '/bpm/approval', domain: 'bpm', tier: 'C', noCreate: true },

  // ===== advanced/ai 域 =====
  { id: 'advanced', route: '/advanced', domain: 'advanced', tier: 'C', noCreate: true },
  { id: 'ai-extend', route: '/ai-extend', domain: 'advanced', tier: 'C', noCreate: true },
  { id: 'ai-extend-process-opt', route: '/ai-extend/process-optimization', domain: 'advanced', tier: 'C', noCreate: true },
  { id: 'ai-extend-quality-pred', route: '/ai-extend/quality-prediction', domain: 'advanced', tier: 'C', noCreate: true },
];

/**
 * 端点矩阵配置见 ./endpoints.config.ts（print 58 / export 50 / approve 48 全量）
 */
export {
  PRINT_ENDPOINTS,
  SENSITIVE_EXPORT_ENDPOINTS,
  NON_SENSITIVE_EXPORT_ENDPOINTS,
  APPROVE_ENDPOINTS,
} from './endpoints.config';
