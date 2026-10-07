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
  /**
   * 集合列表 GET 路径（/api/v1/erp 之后、以 route-snapshot.txt 为准）。
   * 消费语义：凡配置了 listApi 的模块（不分 tier）都由 42a-42d 经
   * ./list-api-probe.ts 的 strict 健康判据真实探测（2xx 判绿，401/403/404/
   * 其它 4xx/5xx 一律判红）——没有"配了但不发请求"的死配置档位。
   */
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
  // listApi 必须是页面真正消费的那个端点：本页读 /reports/enhanced/templates（DB 表），
  // 而 /report-templates 是 report_engine 的"预置模板"只读视图，指它会验到一条 UI 已不再走的链路。
  { id: 'report-templates', route: '/report-templates', domain: 'system', tier: 'A', listApi: '/reports/enhanced/templates' },
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
  // 科目集合 GET 注册在根级而非 /finance 前缀下：gl()（finance.rs:187 `.route("/subjects", get)`）
  // 经 sub_routes()（finance.rs:1163/1165 merge）在 mod.rs:513 挂 /api/v1/erp
  //（route-snapshot.txt:930 `GET /api/v1/erp/subjects`）。
  { id: 'account-subject', route: '/account-subject', domain: 'finance', tier: 'A', listApi: '/subjects' },
  // route-snapshot.txt:494 `GET /api/v1/erp/finance/accounting-periods`（账期挂在 /finance 前缀下）
  { id: 'accounting-period', route: '/accounting-period', domain: 'finance', tier: 'A', listApi: '/finance/accounting-periods' },
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
  // 三个 sales 子资源的集合 GET 均在 /sales 组下注册（mod.rs:462 nest /api/v1/erp/sales；
  // sales_return_handler.rs:43 为 check-route-mount 白名单内的 handler 委托 router）：
  // route-snapshot.txt:900 /sales/sales-returns、:890 /sales/sales-contracts、:895 /sales/sales-prices
  { id: 'sales-returns', route: '/sales-returns', domain: 'sales', tier: 'B', listApi: '/sales/sales-returns' },
  { id: 'sales-contract', route: '/sales-contract', domain: 'sales', tier: 'B', listApi: '/sales/sales-contracts' },
  { id: 'sales-price', route: '/sales-price', domain: 'sales', tier: 'A', listApi: '/sales/sales-prices' },
  { id: 'sales-analysis-bi', route: '/bi/sales-analysis', domain: 'sales', tier: 'C', noCreate: true },
  { id: 'quotations', route: '/quotations', domain: 'sales', tier: 'B', listApi: '/quotations' },
  { id: 'quotations-new', route: '/quotations/new', domain: 'sales', tier: 'B', noCreate: true },
  { id: 'custom-orders', route: '/custom-orders', domain: 'sales', tier: 'B', listApi: '/custom-orders' },
  // after-sales（/after-sales）登记已撤销：src/router/index.ts 无该路由（人人 404，
  // CI 判责 P1「幽灵登记」），用户裁定口径 = 删登记而非补路由。
  // 售后业务面真实入口是 /sales-returns 与 /custom-orders/{id}/after-sales/*（后端端点，
  // 见 endpoints.config 打印族），若产品补 /after-sales 页面路由，届时再登记本条。
  { id: 'logistics', route: '/logistics', domain: 'sales', tier: 'C', noCreate: true },

  // ===== purchase 域 =====
  { id: 'purchase', route: '/purchase', domain: 'purchase', tier: 'C', noCreate: true },
  { id: 'purchase-receipt', route: '/purchase-receipt', domain: 'purchase', tier: 'B', listApi: '/purchase/receipts' },
  // 两个 purchase 子资源集合 GET 在 /purchase 组下（mod.rs:511 nest；purchase.rs:227 合同
  // get(list_contracts)、:273 价格 get(list_prices)）：route-snapshot.txt:798 /purchase/purchase-contracts、
  // :802 /purchase/purchase-prices
  { id: 'purchase-contract', route: '/purchase-contract', domain: 'purchase', tier: 'B', listApi: '/purchase/purchase-contracts' },
  { id: 'purchase-price', route: '/purchase-price', domain: 'purchase', tier: 'A', listApi: '/purchase/purchase-prices' },
  { id: 'purchase-inspection', route: '/purchase-inspection', domain: 'purchase', tier: 'B', listApi: '/purchase/inspections' },
  { id: 'purchase-return', route: '/purchase-return', domain: 'purchase', tier: 'B', listApi: '/purchase/returns' },
  {
    id: 'supplier-evaluation',
    route: '/supplier-evaluation',
    domain: 'purchase',
    tier: 'A',
    // 真实注册路径在 /purchase 组下（route-snapshot.txt:820，且 src/api/supplier-evaluation.ts
    // 的 13 处调用全部带前缀）；不带前缀会被权限中间件的白名单层
    // （middleware/permission.rs:121 validate_route_whitelist）判 403——该层早于
    // check_permission 内的 admin 短路（同文件 :497/:544），且 seg3 白名单是
    // utils/path_utils.rs:12 is_module_prefix + :177 is_direct_resource 两个词表
    // （:284-299 的枚举里有 suppliers 而无 supplier-evaluations），词表里没登记的
    // 模块连 admin 也一样 403 ⇒ 与角色授权无关，补权限播种解不了这条红。
    listApi: '/purchase/supplier-evaluations',
  },

  // ===== crm 域 =====
  { id: 'crm', route: '/crm', domain: 'crm', tier: 'C', noCreate: true },
  { id: 'crm-pool', route: '/crm/pool', domain: 'crm', tier: 'C', noCreate: true },
  { id: 'crm-assignment', route: '/crm/assignment', domain: 'crm', tier: 'C', noCreate: true },
  { id: 'crm-leads', route: '/crm/leads', domain: 'crm', tier: 'A', listApi: '/crm/leads' },
  { id: 'crm-opportunities', route: '/crm/opportunities', domain: 'crm', tier: 'A', listApi: '/crm/opportunities' },
  // 客户集合 GET 在 /crm 组下（mod.rs:515 nest /api/v1/erp/crm）：route-snapshot.txt:359
  { id: 'customer', route: '/customer', domain: 'crm', tier: 'A', listApi: '/crm/customers', uniqueKey: 'name' },
  { id: 'customer-credit', route: '/customer-credit', domain: 'crm', tier: 'C', noCreate: true },

  // ===== supplier 域 =====
  // 供应商集合 GET 在 /purchase 组下（route-snapshot.txt:834）；根级只注册了
  // /suppliers/select 别名（snapshot:934），不存在根级集合 GET——不带前缀的 /suppliers
  // 打 GET 落 404，且 seg3 "suppliers" 虽在白名单（path_utils 词表）也无 GET 契约可寻。
  { id: 'supplier', route: '/supplier', domain: 'supplier', tier: 'A', listApi: '/purchase/suppliers', uniqueKey: 'name' },

  // ===== product/fabric 域 =====
  { id: 'product', route: '/product', domain: 'product', tier: 'A', listApi: '/products', uniqueKey: 'name' },
  { id: 'fabric', route: '/fabric', domain: 'fabric', tier: 'C', noCreate: true },
  // 坯布/染整三资源集合 GET 在 /production 组下（mod.rs:514 nest；route-snapshot.txt:722
  // /production/greige-fabrics、:687 /production/dye-recipes、:681 /production/dye-batches）
  { id: 'greige-fabrics', route: '/greige-fabrics', domain: 'fabric', tier: 'A', listApi: '/production/greige-fabrics' },
  { id: 'color-cards-list', route: '/color-cards/list', domain: 'fabric', tier: 'B', listApi: '/color-cards' },
  { id: 'color-cards-issues', route: '/color-cards/issues', domain: 'fabric', tier: 'B', listApi: '/color-cards/issues' },
  { id: 'color-prices-list', route: '/color-prices/list', domain: 'fabric', tier: 'A', listApi: '/color-prices' },
  { id: 'color-prices-batch', route: '/color-prices/batch-adjust', domain: 'fabric', tier: 'C', noCreate: true },
  { id: 'dye-recipe', route: '/dye-recipe', domain: 'fabric', tier: 'B', listApi: '/production/dye-recipes' },
  { id: 'dye-batch', route: '/dye-batch', domain: 'fabric', tier: 'B', listApi: '/production/dye-batches' },

  // ===== inventory 域 =====
  { id: 'inventory', route: '/inventory', domain: 'inventory', tier: 'C', noCreate: true },
  { id: 'warehouse', route: '/warehouse', domain: 'inventory', tier: 'A', listApi: '/warehouses' },
  { id: 'inventory-count', route: '/inventory-count', domain: 'inventory', tier: 'B', listApi: '/inventory/counts' },
  // 调拨/调整集合 GET 在 /inventory 组下（route-snapshot.txt:576 /inventory/transfers、
  // :552 /inventory/adjustments；根级无同名 GET——/transfers 只出现在 /fund-management/transfers(:540)，不同资源）
  { id: 'inventory-transfer', route: '/inventory-transfer', domain: 'inventory', tier: 'B', listApi: '/inventory/transfers' },
  { id: 'inventory-adjustment', route: '/inventory-adjustment', domain: 'inventory', tier: 'B', listApi: '/inventory/adjustments' },
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
  // 注：process-routes 已从 UI 遍历清单移除。前端 router/index.ts 无 /process-routes
  // 路由、src/views 下亦无对应页面组件（grep 精确匹配数=0），page.goto('/process-routes')
  // 落到 catch-all/空白路由，故 Tier A 的「新建」按钮断言必然 waitFor 超时。
  // 后端 /production/process-routes 端点确实存在（production.rs:225-231），属"有后端、
  // 无管理 UI"，不是可遍历的前端页面；若产品需补 UI，届时再登记本条并按实际按钮文案调整。

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
