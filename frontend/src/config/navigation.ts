/**
 * 侧边栏导航树：一级域 / 二级段 / 三级页面的唯一声明处。
 *
 * 调用方：`components/Layout/NavTree.vue` 递归渲染，`MainLayout.vue` 传入可见性判据。
 * 入参：无（静态声明）。存什么：叶子存路由 path 与 i18n 键，分组存 key 与 i18n 键。
 * 存哪里：仅本文件；路由 path 与权限键仍以 `router/index.ts` 为准，本表不复制权限。
 *
 * 闭合判据由 `tests/unit/navigation-closure.test.ts` 锁定：每个非 hidden、非 public 的
 * 叶子路由必须在本表出现且仅出现一次，本表每个 path 必须是已存在的叶子路由。
 */

export interface NavLeaf {
  kind: 'leaf';
  path: string;
  labelKey: string;
}

export interface NavGroup {
  kind: 'group';
  key: string;
  labelKey: string;
  icon?: string;
  children: NavNode[];
}

export type NavNode = NavLeaf | NavGroup;

const L = (path: string, labelKey: string): NavLeaf => ({ kind: 'leaf', path, labelKey });

const G = (key: string, labelKey: string, children: NavNode[], icon?: string): NavGroup => ({
  kind: 'group',
  key,
  labelKey,
  ...(icon ? { icon } : {}),
  children,
});

/** 二级段：不带图标，仅做归类 */
const S = (key: string, labelKey: string, children: NavNode[]): NavGroup => ({
  kind: 'group',
  key,
  labelKey,
  children,
});

export const NAV_TREE: NavNode[] = [
  L('/dashboard', 'layout.menu.dashboard'),

  G(
    'marketing',
    'layout.group.marketing',
    [
      S('crm', 'layout.section.crm', [
        L('/customer', 'layout.menu.customer'),
        L('/customer-credit', 'layout.menu.customerCredit'),
        L('/crm', 'layout.menu.crmManagement'),
        L('/crm/pool', 'layout.menu.crmPool'),
        L('/crm/assignment', 'layout.menu.crmAssignment'),
        L('/crm/leads', 'layout.menu.crmLeads'),
        L('/crm/opportunities', 'layout.menu.crmOpportunities'),
        L('/crm-enhanced', 'layout.menu.crmEnhanced'),
        L('/customer-collab', 'layout.menu.customerCollab'),
      ]),
      S('order', 'layout.section.order', [
        L('/quotations', 'layout.menu.quotations'),
        L('/sales', 'layout.menu.salesOrder'),
        L('/sales-returns', 'layout.menu.salesReturns'),
        L('/sales-contract', 'layout.menu.salesContract'),
        L('/custom-orders', 'layout.menu.customOrders'),
      ]),
      S('color', 'layout.section.color', [
        L('/sales-price', 'layout.menu.salesPrice'),
        L('/color-cards/list', 'layout.menu.colorCardsList'),
        L('/color-cards/issues', 'layout.menu.colorCardsIssue'),
        L('/color-prices/list', 'layout.menu.colorPricesList'),
        L('/color-prices/batch-adjust', 'layout.menu.colorPricesBatchAdjust'),
      ]),
      S('analysis', 'layout.section.analysis', [
        L('/sales-analysis', 'layout.menu.salesAnalysis'),
        L('/bi/sales-analysis', 'layout.menu.biSalesAnalysis'),
      ]),
    ],
    'ShoppingCart'
  ),

  G(
    'supplychain',
    'layout.group.supplychain',
    [
      S('supplier', 'layout.section.supplier', [
        L('/supplier', 'layout.menu.supplier'),
        L('/supplier-evaluation', 'layout.menu.supplierEvaluation'),
        L('/supplier-enhanced', 'layout.menu.supplierEnhanced'),
      ]),
      S('purchase', 'layout.section.purchase', [
        L('/purchase', 'layout.menu.purchaseOrder'),
        L('/purchase-receipt', 'layout.menu.purchaseReceipt'),
        L('/purchase-inspection', 'layout.menu.purchaseInspection'),
        L('/purchase-return', 'layout.menu.purchaseReturn'),
        L('/purchase-contract', 'layout.menu.purchaseContract'),
        L('/purchase-price', 'layout.menu.purchasePrice'),
        L('/outsourcing', 'layout.menu.outsourcing'),
      ]),
      S('catalog', 'layout.section.catalog', [
        L('/product', 'layout.menu.product'),
        L('/purchase/sku-mapping', 'layout.menu.skuMapping'),
        L('/purchase/supplier-product', 'layout.menu.supplierProduct'),
        L('/fabric', 'layout.menu.fabricList'),
        L('/greige-fabrics', 'layout.menu.greigeFabrics'),
      ]),
    ],
    'Sell'
  ),

  G(
    'production',
    'layout.group.production',
    [
      S('plan', 'layout.section.plan', [
        L('/production', 'layout.menu.productionPlan'),
        L('/bom', 'layout.menu.bom'),
        L('/mrp', 'layout.menu.mrp'),
        L('/mrp/history', 'layout.menu.mrpHistory'),
        L('/capacity', 'layout.menu.capacity'),
        L('/material-shortage', 'layout.menu.materialShortage'),
        L('/scheduling', 'layout.menu.scheduling'),
        L('/scheduling/gantt', 'layout.menu.gantt'),
      ]),
      S('dyeing', 'layout.section.dyeing', [
        L('/dye-recipe', 'layout.menu.dyeRecipe'),
        L('/dye-batch', 'layout.menu.dyeBatch'),
        L('/lab-dip', 'layout.menu.labDip'),
        L('/bulk-color-approval', 'layout.menu.bulkColorApproval'),
        L('/production-recipes', 'layout.menu.productionRecipes'),
        L('/flow-cards', 'layout.menu.flowCards'),
        L('/chemicals', 'layout.menu.chemicals'),
      ]),
      S('workforce', 'layout.section.workforce', [
        L('/wage', 'layout.menu.wage'),
        L('/labor-contracts', 'layout.menu.laborContracts'),
        L('/social-insurance', 'layout.menu.socialInsurance'),
        L('/occupational-health', 'layout.menu.occupationalHealth'),
      ]),
    ],
    'Cpu'
  ),

  G(
    'inventory',
    'layout.group.inventory',
    [
      L('/inventory', 'layout.menu.inventoryList'),
      L('/warehouse', 'layout.menu.warehouse'),
      L('/inventory-batch', 'layout.menu.inventoryBatch'),
      L('/inventory-count', 'layout.menu.inventoryCount'),
      L('/inventory-transfer', 'layout.menu.inventoryTransfer'),
      L('/inventory-adjustment', 'layout.menu.inventoryAdjustment'),
      L('/logistics', 'layout.menu.logistics'),
      L('/barcode-scanner', 'layout.menu.barcodeScanner'),
      L('/five-dimension', 'layout.menu.fiveDimension'),
    ],
    'Box'
  ),

  G(
    'finance',
    'layout.group.finance',
    [
      S('ledger', 'layout.section.ledger', [
        L('/finance', 'layout.menu.financeOverview'),
        L('/voucher', 'layout.menu.voucher'),
        L('/account-subject', 'layout.menu.accountSubject'),
        L('/accounting-period', 'layout.menu.accountingPeriod'),
        L('/periods', 'layout.menu.accountingPeriods'),
        L('/period-adjustments', 'layout.menu.periodAdjustments'),
        L('/assist-accounting', 'layout.menu.assistAccounting'),
      ]),
      S('counterparty', 'layout.section.counterparty', [
        L('/ar', 'layout.menu.ar'),
        L('/ap', 'layout.menu.ap'),
        L('/ar-reconciliation', 'layout.menu.arReconciliation'),
        L('/ar-reconciliation/enhanced', 'layout.menu.arReconciliationEnhanced'),
        L('/bad-debts', 'layout.menu.badDebts'),
        L('/invoice-details', 'layout.menu.invoiceDetails'),
      ]),
      S('assetFundBudget', 'layout.section.assetFundBudget', [
        L('/fixed-assets', 'layout.menu.fixedAssets'),
        L('/fund', 'layout.menu.fund'),
        L('/budget', 'layout.menu.budget'),
        L('/budgets', 'layout.menu.budgetApprovals'),
        L('/cost', 'layout.menu.cost'),
        L('/currency', 'layout.menu.currency'),
      ]),
      S('report', 'layout.section.report', [
        L('/finance-report', 'layout.menu.financeReport'),
        L('/report-templates', 'layout.menu.reportTemplates'),
        L('/financial-analysis', 'layout.menu.financialAnalysis'),
      ]),
    ],
    'Money'
  ),

  G(
    'qualityCompliance',
    'layout.group.qualityCompliance',
    [
      S('inspection', 'layout.section.inspection', [
        L('/quality', 'layout.menu.quality'),
        L('/quality-8d', 'layout.menu.quality8d'),
        L('/quality-standards', 'layout.menu.qualityStandards'),
        L('/fabric-inspections', 'layout.menu.fabricInspections'),
      ]),
      S('compliance', 'layout.section.compliance', [
        L('/business-trace', 'layout.menu.businessTrace'),
        L('/export-compliance', 'layout.menu.exportCompliance'),
        L('/environmental-compliance', 'layout.menu.environmentalCompliance'),
      ]),
    ],
    'CircleCheck'
  ),

  G(
    'workflowIntelligence',
    'layout.group.workflowIntelligence',
    [
      S('bpm', 'layout.section.bpm', [
        L('/bpm', 'layout.menu.bpm'),
        L('/bpm/definitions', 'layout.menu.bpmDefinitions'),
        L('/bpm/templates', 'layout.menu.bpmTemplates'),
        L('/bpm/approval', 'layout.menu.bpmApproval'),
      ]),
      S('ai', 'layout.section.ai', [
        L('/advanced', 'layout.menu.advanced'),
        L('/ai-extend', 'layout.menu.aiExtend'),
        L('/ai-extend/process-optimization', 'layout.menu.aiProcessOptimization'),
        L('/ai-extend/quality-prediction', 'layout.menu.aiQualityPrediction'),
      ]),
      S('audit', 'layout.section.audit', [L('/omni-audit', 'layout.menu.omniAudit')]),
    ],
    'MagicStick'
  ),

  G(
    'system',
    'layout.group.system',
    [
      S('organization', 'layout.section.organization', [
        L('/system', 'layout.menu.systemSettings'),
        L('/departments', 'layout.menu.departments'),
        L('/security', 'layout.menu.security'),
      ]),
      S('governance', 'layout.section.governance', [
        L('/data-permission', 'layout.menu.dataPermission'),
        L('/system-governance', 'layout.menu.systemGovernance'),
      ]),
      S('printExport', 'layout.section.printExport', [
        L('/print-templates', 'layout.menu.printTemplates'),
        L('/print-targets', 'layout.menu.printTargets'),
        L('/system/export-approvals', 'layout.menu.exportApprovals'),
      ]),
      S('integration', 'layout.section.integration', [
        L('/api-gateway', 'layout.menu.apiGateway'),
        L('/data-import', 'layout.menu.dataImport'),
        L('/notification', 'layout.menu.notification'),
        L('/system/oa-announcements', 'layout.menu.oaAnnouncements'),
        L('/email', 'layout.menu.email'),
      ]),
      S('ops', 'layout.section.ops', [
        L('/system-update', 'layout.menu.systemUpdate'),
        L('/admin/failover', 'layout.menu.failover'),
        L('/system/audit-log', 'layout.menu.auditLog'),
        L('/system/slow-query', 'layout.menu.slowQuery'),
      ]),
    ],
    'Setting'
  ),
];
