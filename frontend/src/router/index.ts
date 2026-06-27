import { createRouter, createWebHistory } from 'vue-router'
import type { RouteRecordRaw } from 'vue-router'
import MainLayout from '@/components/Layout/MainLayout.vue'
import { useUserStore } from '@/store/user'
import { logger } from '@/utils/logger'

/**
 * 路由元信息类型扩展（批次 3：前端路由 meta 补齐）
 *
 * - icon：Element Plus 图标组件名（与 MainLayout 菜单 icon 对齐）
 * - permission：访问该路由所需权限码（格式 "{resource}:{action}"，如 "inventory:read"）
 *   宽松模式：admin 角色绕过；用户未配置任何权限码时放行；支持 "*" 通配
 * - hidden：是否在菜单中隐藏（子页面如 detail/edit/create 等设为 true）
 */
declare module 'vue-router' {
  interface RouteMeta {
    title?: string
    requiresAuth?: boolean
    icon?: string
    permission?: string | string[]
    hidden?: boolean
  }
}

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    redirect: '/dashboard',
  },
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/Login.vue'),
    meta: { title: '登录' },
  },
  {
    path: '/setup',
    name: 'Setup',
    component: () => import('@/views/Setup.vue'),
    meta: { title: '系统初始化' },
  },
  {
    path: '/',
    component: MainLayout,
    meta: { requiresAuth: true },
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('@/views/Dashboard.vue'),
        meta: { title: '仪表盘', icon: 'HomeFilled', permission: 'dashboard:read', requiresAuth: true },
      },
      {
        path: 'system',
        name: 'System',
        component: () => import('@/views/system/index.vue'),
        meta: { title: '系统管理', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'system/audit-log',
        name: 'SystemAuditLog',
        component: () => import('@/views/system/audit-log/index.vue'),
        meta: { title: '审计日志', icon: 'Setting', permission: 'audit:read', requiresAuth: true },
      },
      {
        path: 'system/slow-query',
        name: 'SystemSlowQuery',
        component: () => import('@/views/system/slow-query/index.vue'),
        meta: { title: '慢查询审计', icon: 'Histogram', requiresAuth: true },
      },
      {
        path: 'finance',
        name: 'Finance',
        component: () => import('@/views/finance/index.vue'),
        meta: { title: '财务管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'ap',
        name: 'AP',
        component: () => import('@/views/ap/index.vue'),
        meta: { title: '应付管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'ar',
        name: 'AR',
        component: () => import('@/views/ar/index.vue'),
        meta: { title: '应收管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'fabric',
        name: 'Fabric',
        component: () => import('@/views/fabric/index.vue'),
        meta: { title: '面料管理', icon: 'Goods', permission: 'inventory:read', requiresAuth: true },
      },
      {
        path: 'inventory',
        name: 'Inventory',
        component: () => import('@/views/inventory/index.vue'),
        meta: { title: '库存管理', icon: 'Box', permission: 'inventory:read', requiresAuth: true },
      },
      {
        path: 'sales',
        name: 'Sales',
        component: () => import('@/views/sales/index.vue'),
        meta: { title: '销售管理', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true },
      },
      {
        path: 'purchase',
        name: 'Purchase',
        component: () => import('@/views/purchase/index.vue'),
        meta: { title: '采购管理', icon: 'ShoppingCart', permission: 'purchases:read', requiresAuth: true },
      },
      {
        path: 'customer',
        name: 'Customer',
        component: () => import('@/views/customer/index.vue'),
        meta: { title: '客户管理', icon: 'User', permission: 'customers:read', requiresAuth: true },
      },
      {
        path: 'supplier',
        name: 'Supplier',
        component: () => import('@/views/supplier/index.vue'),
        meta: { title: '供应商管理', icon: 'User', permission: 'suppliers:read', requiresAuth: true },
      },
      {
        path: 'product',
        name: 'Product',
        component: () => import('@/views/product/index.vue'),
        meta: { title: '产品管理', icon: 'Goods', permission: 'products:read', requiresAuth: true },
      },
      {
        path: 'warehouse',
        name: 'Warehouse',
        component: () => import('@/views/warehouse/index.vue'),
        meta: { title: '仓库管理', icon: 'Box', permission: 'warehouses:read', requiresAuth: true },
      },
      {
        path: 'departments',
        name: 'Departments',
        component: () => import('@/views/departments/index.vue'),
        meta: { title: '部门管理', icon: 'Setting', permission: 'users:read', requiresAuth: true },
      },
      {
        path: 'greige-fabrics',
        name: 'GreigeFabrics',
        component: () => import('@/views/greige-fabrics/index.vue'),
        meta: { title: '坯布管理', icon: 'Goods', permission: 'inventory:read', requiresAuth: true },
      },
      {
        path: 'sales-returns',
        name: 'SalesReturns',
        component: () => import('@/views/sales-returns/index.vue'),
        meta: { title: '销售退货管理', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true },
      },
      {
        path: 'supplier-evaluation',
        name: 'SupplierEvaluation',
        component: () => import('@/views/supplierEvaluation/index.vue'),
        meta: { title: '供应商评估', icon: 'User', permission: 'suppliers:read', requiresAuth: true },
      },
      {
        path: 'customer-credit',
        name: 'CustomerCredit',
        component: () => import('@/views/customerCredit/index.vue'),
        meta: { title: '客户信用管理', icon: 'User', permission: 'customers:read', requiresAuth: true },
      },
      {
        path: 'inventory-count',
        name: 'InventoryCount',
        component: () => import('@/views/inventoryCount/index.vue'),
        meta: { title: '库存盘点', icon: 'Box', permission: 'inventory:read', requiresAuth: true },
      },
      {
        path: 'inventory-transfer',
        name: 'InventoryTransfer',
        component: () => import('@/views/inventoryTransfer/index.vue'),
        meta: { title: '库存调拨', icon: 'Box', permission: 'inventory:read', requiresAuth: true },
      },
      {
        path: 'inventory-adjustment',
        name: 'InventoryAdjustment',
        component: () => import('@/views/inventoryAdjustment/index.vue'),
        meta: { title: '库存调整', icon: 'Box', permission: 'inventory:read', requiresAuth: true },
      },
      {
        path: 'ar-reconciliation',
        name: 'ArReconciliation',
        component: () => import('@/views/arReconciliation/index.vue'),
        meta: { title: '应收对账', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'finance-report',
        name: 'FinanceReport',
        component: () => import('@/views/financeReport/index.vue'),
        meta: { title: '财务报表', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'purchase-receipt',
        name: 'PurchaseReceipt',
        component: () => import('@/views/purchaseReceipt/index.vue'),
        meta: { title: '采购入库', icon: 'ShoppingCart', permission: 'purchases:read', requiresAuth: true },
      },
      {
        path: 'fixed-assets',
        name: 'FixedAssets',
        component: () => import('@/views/fixed-assets/index.vue'),
        meta: { title: '固定资产管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'bpm',
        name: 'BPM',
        component: () => import('@/views/bpm/index.vue'),
        meta: { title: '审批管理', icon: 'List', requiresAuth: true },
      },
      {
        path: 'quality',
        name: 'Quality',
        component: () => import('@/views/quality/index.vue'),
        meta: { title: '质量管理', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'purchase-ext',
        name: 'PurchaseExt',
        component: () => import('@/views/purchase-ext/index.vue'),
        meta: { title: '采购扩展', icon: 'ShoppingCart', permission: 'purchases:read', requiresAuth: true },
      },
      {
        path: 'sales-ext',
        name: 'SalesExt',
        component: () => import('@/views/sales-ext/index.vue'),
        meta: { title: '销售扩展', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true },
      },
      {
        path: 'crm',
        name: 'CRM',
        component: () => import('@/views/crm/index.vue'),
        meta: { title: '客户关系管理', icon: 'User', requiresAuth: true },
      },
      {
        path: 'crm/pool',
        name: 'CRMPool',
        component: () => import('@/views/crm/pool.vue'),
        meta: { title: '公海客户池', icon: 'User', requiresAuth: true },
      },
      {
        path: 'crm/assignment',
        name: 'CRMAssignment',
        component: () => import('@/views/crm/assignment.vue'),
        meta: { title: '客户分配', icon: 'User', requiresAuth: true },
      },
      {
        path: 'crm/detail/:id',
        name: 'CRMDetail',
        component: () => import('@/views/crm/detail.vue'),
        meta: { title: '客户360视图', icon: 'User', requiresAuth: true, hidden: true },
      },
      // 新增路由 - CRM线索管理
      {
        path: 'crm/leads',
        name: 'CRMLeads',
        component: () => import('@/views/crm/leads/index.vue'),
        meta: { title: '线索管理', icon: 'User', requiresAuth: true },
      },
      // 新增路由 - CRM商机管理
      {
        path: 'crm/opportunities',
        name: 'CRMOpportunities',
        component: () => import('@/views/crm/opportunities/index.vue'),
        meta: { title: '商机管理', icon: 'User', requiresAuth: true },
      },
      {
        path: 'advanced',
        name: 'Advanced',
        component: () => import('@/views/advanced/index.vue'),
        meta: { title: '高级功能', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'production',
        name: 'Production',
        component: () => import('@/views/production/index.vue'),
        meta: { title: '生产计划', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'bom',
        name: 'Bom',
        component: () => import('@/views/bom/index.vue'),
        meta: { title: 'BOM管理', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'mrp',
        name: 'Mrp',
        component: () => import('@/views/mrp/index.vue'),
        meta: { title: 'MRP计算', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'mrp/history',
        name: 'MrpHistory',
        component: () => import('@/views/mrp/history.vue'),
        meta: { title: 'MRP历史记录', icon: 'Cpu', requiresAuth: true, hidden: true },
      },
      {
        path: 'capacity',
        name: 'Capacity',
        component: () => import('@/views/capacity/index.vue'),
        meta: { title: '产能分析', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'material-shortage',
        name: 'MaterialShortage',
        component: () => import('@/views/material-shortage/index.vue'),
        meta: { title: '缺料预警', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'cost',
        name: 'Cost',
        component: () => import('@/views/cost/index.vue'),
        meta: { title: '成本归集', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'budget',
        name: 'Budget',
        component: () => import('@/views/budget/index.vue'),
        meta: { title: '预算管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'fund',
        name: 'Fund',
        component: () => import('@/views/fund/index.vue'),
        meta: { title: '资金管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'financial-analysis',
        name: 'FinancialAnalysis',
        component: () => import('@/views/financial-analysis/index.vue'),
        meta: { title: '财务分析', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'currency',
        name: 'Currency',
        component: () => import('@/views/currency/index.vue'),
        meta: { title: '多币种管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'notification',
        name: 'Notification',
        component: () => import('@/views/notification/index.vue'),
        meta: { title: '通知中心', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'data-permission',
        name: 'DataPermission',
        component: () => import('@/views/dataPermission/index.vue'),
        meta: { title: '数据权限管理', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'inventory-batch',
        name: 'InventoryBatch',
        component: () => import('@/views/inventoryBatch/index.vue'),
        meta: { title: '批次管理', icon: 'Box', permission: 'inventory:read', requiresAuth: true },
      },
      {
        path: 'five-dimension',
        name: 'FiveDimension',
        component: () => import('@/views/fiveDimension/index.vue'),
        meta: { title: '五维管理', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'assist-accounting',
        name: 'AssistAccounting',
        component: () => import('@/views/assistAccounting/index.vue'),
        meta: { title: '辅助核算', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      {
        path: 'business-trace',
        name: 'BusinessTrace',
        component: () => import('@/views/businessTrace/index.vue'),
        meta: { title: '业务追溯', icon: 'List', requiresAuth: true },
      },
      {
        path: 'barcode-scanner',
        name: 'BarcodeScanner',
        component: () => import('@/views/barcodeScanner/index.vue'),
        meta: { title: '扫码功能', icon: 'List', requiresAuth: true },
      },
      {
        path: 'omni-audit',
        name: 'OmniAudit',
        component: () => import('@/views/omniAudit/index.vue'),
        meta: { title: '全量审计', icon: 'Setting', permission: 'audit:read', requiresAuth: true },
      },
      {
        path: 'scheduling',
        name: 'Scheduling',
        component: () => import('@/views/scheduling/index.vue'),
        meta: { title: '生产排程', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'scheduling/gantt',
        name: 'SchedulingGantt',
        component: () => import('@/views/scheduling/gantt.vue'),
        meta: { title: '排程甘特图', icon: 'Cpu', requiresAuth: true, hidden: true },
      },
      {
        path: 'components-demo',
        name: 'ComponentsDemo',
        component: () => import('@/views/components-demo/index.vue'),
        meta: { title: '组件示例', icon: 'Setting', requiresAuth: true, hidden: true },
      },
      // 新增路由 - 凭证管理
      {
        path: 'voucher',
        name: 'Voucher',
        component: () => import('@/views/voucher/index.vue'),
        meta: { title: '凭证管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      // 新增路由 - 会计科目管理
      {
        path: 'account-subject',
        name: 'AccountSubject',
        component: () => import('@/views/accountSubject/index.vue'),
        meta: { title: '会计科目管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      // 新增路由 - 会计期间管理
      {
        path: 'accounting-period',
        name: 'AccountingPeriod',
        component: () => import('@/views/accountingPeriod/index.vue'),
        meta: { title: '会计期间管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      // 新增路由 - 交易管理
      {
        path: 'trading',
        name: 'Trading',
        component: () => import('@/views/trading/index.vue'),
        meta: { title: '交易管理', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      // 新增路由 - 报表模板
      {
        path: 'report-templates',
        name: 'ReportTemplates',
        component: () => import('@/views/report-templates/index.vue'),
        meta: { title: '报表中心', icon: 'Setting', requiresAuth: true },
      },
      // 新增路由 - BPM流程定义
      {
        path: 'bpm/definitions',
        name: 'BPMDefinitions',
        component: () => import('@/views/bpm/definitions.vue'),
        meta: { title: '流程定义管理', icon: 'List', requiresAuth: true, hidden: true },
      },
      // 新增路由 - BPM流程模板
      {
        path: 'bpm/templates',
        name: 'BPMTemplates',
        component: () => import('@/views/bpm/templates.vue'),
        meta: { title: '流程模板管理', icon: 'List', requiresAuth: true, hidden: true },
      },
      // 新增路由 - BPM审批中心
      {
        path: 'bpm/approval',
        name: 'BPMApproval',
        component: () => import('@/views/bpm/approval/index.vue'),
        meta: { title: '审批中心', icon: 'List', requiresAuth: true },
      },
      // 新增路由 - 增强版应收对账
      {
        path: 'ar-reconciliation/enhanced',
        name: 'ArReconciliationEnhanced',
        component: () => import('@/views/arReconciliation/enhanced.vue'),
        meta: { title: '增强版应收对账', icon: 'Money', permission: 'finance:read', requiresAuth: true },
      },
      // 新增路由 - 销售分析
      {
        path: 'sales-analysis',
        name: 'SalesAnalysis',
        component: () => import('@/views/sales-analysis/index.vue'),
        meta: { title: '销售分析', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true },
      },
      // 新增路由 - 安全管理
      {
        path: 'security',
        name: 'Security',
        component: () => import('@/views/security/index.vue'),
        meta: { title: '安全管理', icon: 'Setting', requiresAuth: true },
      },
      // 新增路由 - 双因素认证(修复 user-profile 死链)
      {
        path: 'security/two-factor-setup',
        name: 'SecurityTwoFactorSetup',
        component: () => import('@/views/security/TwoFactorSetup.vue'),
        meta: { title: '双因素认证', icon: 'Setting', requiresAuth: true, hidden: true },
      },
      // 新增路由 - 修改密码(修复 user-profile 死链)
      {
        path: 'security/change-password',
        name: 'SecurityChangePassword',
        component: () => import('@/views/security/ChangePassword.vue'),
        meta: { title: '修改密码', icon: 'Setting', requiresAuth: true, hidden: true },
      },
      {
        path: 'dye-recipe',
        name: 'DyeRecipe',
        component: () => import('@/views/dye-recipe/index.vue'),
        meta: { title: '染色配方', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'dye-batch',
        name: 'DyeBatch',
        component: () => import('@/views/dye-batch/index.vue'),
        meta: { title: '染色批次', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'purchase-contract',
        name: 'PurchaseContract',
        component: () => import('@/views/purchase-contract/index.vue'),
        meta: { title: '采购合同', icon: 'ShoppingCart', permission: 'purchases:read', requiresAuth: true },
      },
      {
        path: 'sales-contract',
        name: 'SalesContract',
        component: () => import('@/views/sales-contract/index.vue'),
        meta: { title: '销售合同', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true },
      },
      {
        path: 'purchase-price',
        name: 'PurchasePrice',
        component: () => import('@/views/purchase-price/index.vue'),
        meta: { title: '采购价格', icon: 'ShoppingCart', permission: 'purchases:read', requiresAuth: true },
      },
      {
        path: 'sales-price',
        name: 'SalesPrice',
        component: () => import('@/views/sales-price/index.vue'),
        meta: { title: '销售价格', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true },
      },
      // 新增路由 - 邮件管理
      {
        path: 'email',
        name: 'Email',
        component: () => import('@/views/email/index.vue'),
        meta: { title: '邮件管理', icon: 'Setting', requiresAuth: true },
      },
      // 新增路由 - 租户计费管理
      {
        path: 'tenant-billing',
        name: 'TenantBilling',
        component: () => import('@/views/tenant-billing/index.vue'),
        meta: { title: '租户计费管理', icon: 'Setting', requiresAuth: true },
      },
      // 新增路由 - 主备隔离监控
      {
        path: 'admin/failover',
        name: 'AdminFailover',
        component: () => import('@/views/admin/failover.vue'),
        meta: { title: '主备隔离监控', icon: 'Setting', requiresAuth: true },
      },
      // 新增路由 - 采购检验
      {
        path: 'purchase-inspection',
        name: 'PurchaseInspection',
        component: () => import('@/views/purchase-inspection/index.vue'),
        meta: { title: '采购检验', icon: 'ShoppingCart', permission: 'purchases:read', requiresAuth: true },
      },
      // 新增路由 - 采购退货
      {
        path: 'purchase-return',
        name: 'PurchaseReturn',
        component: () => import('@/views/purchase-return/index.vue'),
        meta: { title: '采购退货', icon: 'ShoppingCart', permission: 'purchases:read', requiresAuth: true },
      },
      // 新增路由 - 物流管理
      {
        path: 'logistics',
        name: 'Logistics',
        component: () => import('@/views/logistics/index.vue'),
        meta: { title: '物流管理', icon: 'Box', requiresAuth: true },
      },
      {
        path: 'quality-standards',
        name: 'QualityStandards',
        component: () => import('@/views/quality-standards/index.vue'),
        meta: { title: '质量标准', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'data-import',
        name: 'DataImport',
        component: () => import('@/views/data-import/index.vue'),
        meta: { title: '数据导入', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'print-templates',
        name: 'PrintTemplates',
        component: () => import('@/views/print-templates/index.vue'),
        meta: { title: '打印模板', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'api-gateway',
        name: 'ApiGateway',
        component: () => import('@/views/api-gateway/index.vue'),
        meta: { title: 'API网关', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'system-update',
        name: 'SystemUpdate',
        component: () => import('@/views/system-update/index.vue'),
        meta: { title: '系统更新', icon: 'Setting', requiresAuth: true },
      },
      {
        path: 'system/profile',
        name: 'UserProfile',
        component: () => import('@/views/user-profile/index.vue'),
        meta: { title: '个人信息', icon: 'Setting', requiresAuth: true, hidden: true },
      },
      // 报价单模块 - 列表 + 新建
      {
        path: 'quotations',
        name: 'QuotationList',
        component: () => import('@/views/quotations/list.vue'),
        meta: { title: '报价单管理', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true },
      },
      {
        path: 'quotations/new',
        name: 'QuotationCreate',
        component: () => import('@/views/quotations/create.vue'),
        meta: { title: '新建报价单', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true, hidden: true },
      },
      {
        path: 'quotations/:id',
        name: 'QuotationDetail',
        component: () => import('@/views/quotations/detail.vue'),
        meta: { title: '报价单详情', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true, hidden: true },
      },
      {
        path: 'quotations/:id/edit',
        name: 'QuotationEdit',
        component: () => import('@/views/quotations/edit.vue'),
        meta: { title: '编辑报价单', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true, hidden: true },
      },
      {
        path: 'quotations/:id/approval',
        name: 'QuotationApproval',
        component: () => import('@/views/quotations/approval.vue'),
        meta: { title: '报价单审批', icon: 'ShoppingCart', permission: 'sales:read', requiresAuth: true, hidden: true },
      },
      // 定制订单模块 - 列表 + 新建 + 详情 + 跟踪
      {
        path: 'custom-orders',
        name: 'CustomOrderList',
        component: () => import('@/views/custom-orders/list.vue'),
        meta: { title: '定制订单管理', icon: 'Cpu', requiresAuth: true },
      },
      {
        path: 'custom-orders/new',
        name: 'CustomOrderCreate',
        component: () => import('@/views/custom-orders/create.vue'),
        meta: { title: '新建定制订单', icon: 'Cpu', requiresAuth: true, hidden: true },
      },
      {
        path: 'custom-orders/:id',
        name: 'CustomOrderDetail',
        component: () => import('@/views/custom-orders/detail.vue'),
        meta: { title: '定制订单详情', icon: 'Cpu', requiresAuth: true, hidden: true },
      },
      {
        path: 'custom-orders/:id/track',
        name: 'CustomOrderTracking',
        component: () => import('@/views/custom-orders/tracking.vue'),
        meta: { title: '工艺跟踪', icon: 'Cpu', requiresAuth: true, hidden: true },
      },
      // 色卡仓储管理模块（P0-4）
      {
        path: 'color-cards/list',
        name: 'ColorCardList',
        component: () => import('@/views/color-cards/list.vue'),
        meta: { title: '色卡列表', icon: 'Goods', requiresAuth: true },
      },
      {
        path: 'color-cards/create',
        name: 'ColorCardCreate',
        component: () => import('@/views/color-cards/create.vue'),
        meta: { title: '新建色卡', icon: 'Goods', requiresAuth: true, hidden: true },
      },
      {
        path: 'color-cards/detail/:id',
        name: 'ColorCardDetail',
        component: () => import('@/views/color-cards/detail.vue'),
        meta: { title: '色卡详情', icon: 'Goods', requiresAuth: true, hidden: true },
      },
      {
        path: 'color-cards/borrow',
        name: 'ColorCardBorrow',
        component: () => import('@/views/color-cards/borrow.vue'),
        meta: { title: '色卡借出管理', icon: 'Goods', requiresAuth: true },
      },
      // 面料多色号定价扩展模块（P0-5）
      {
        path: 'color-prices/list',
        name: 'ColorPriceList',
        component: () => import('@/views/color-prices/list.vue'),
        meta: { title: '色号价格列表', icon: 'Goods', requiresAuth: true },
      },
      {
        path: 'color-prices/create',
        name: 'ColorPriceCreate',
        // 修复：原指向 list.vue（错配）→ 改为专用 create.vue
        component: () => import('@/views/color-prices/create.vue'),
        meta: { title: '新建色号价格', icon: 'Goods', requiresAuth: true, hidden: true },
      },
      {
        path: 'color-prices/detail/:id',
        name: 'ColorPriceDetail',
        component: () => import('@/views/color-prices/detail.vue'),
        meta: { title: '色号价格详情', icon: 'Goods', requiresAuth: true, hidden: true },
      },
      {
        path: 'color-prices/batch-adjust',
        name: 'ColorPriceBatchAdjust',
        component: () => import('@/views/color-prices/batch-adjust.vue'),
        meta: { title: '批量调价', icon: 'Goods', requiresAuth: true },
      },
      // P2-4 AI 分析深化（工艺优化 + 质量预测）
      {
        path: 'ai-extend',
        name: 'AiExtendOverview',
        component: () => import('@/views/ai-extend/index.vue'),
        meta: { title: 'AI 分析深化', icon: 'MagicStick', requiresAuth: true },
      },
      {
        path: 'ai-extend/process-optimization',
        name: 'AiExtendProcessOptimization',
        component: () => import('@/views/ai-extend/process-optimization.vue'),
        meta: { title: 'AI 工艺优化', icon: 'MagicStick', requiresAuth: true },
      },
      {
        path: 'ai-extend/process-detail/:id',
        name: 'AiExtendProcessDetail',
        component: () => import('@/views/ai-extend/process-detail.vue'),
        meta: { title: '工艺优化详情', icon: 'MagicStick', requiresAuth: true, hidden: true },
      },
      {
        path: 'ai-extend/quality-prediction',
        name: 'AiExtendQualityPrediction',
        component: () => import('@/views/ai-extend/quality-prediction.vue'),
        meta: { title: 'AI 质量预测', icon: 'MagicStick', requiresAuth: true },
      },
      {
        // P3-4 BI 销售多维分析
        path: 'bi/sales-analysis',
        name: 'BiSalesAnalysis',
        component: () => import('@/views/bi/SalesAnalysis.vue'),
        meta: { title: 'BI 销售多维分析', icon: 'Money', requiresAuth: true },
      },
    ],
  },
  {
    path: '/workflow',
    redirect: '/bpm',
  },
  {
    path: '/403',
    name: '403',
    component: () => import('@/views/403.vue'),
    meta: { title: '无权限' },
  },
  {
    path: '/404',
    name: '404',
    component: () => import('@/views/404.vue'),
    meta: { title: '页面不存在' },
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/404',
  },
]

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
})

let initStatus: boolean | null = null

async function checkInitStatus(): Promise<boolean> {
  if (initStatus !== null) return initStatus
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 3000)
    const response = await fetch('/api/v1/erp/init/status', {
      method: 'GET',
      headers: { Accept: 'application/json' },
      signal: controller.signal,
    })
    clearTimeout(timeout)
    if (response.ok) {
      const data = await response.json()
      // 兼容两种格式：
      // 1. setup 模式：返回 { initialized: false, message: "..." }
      // 2. 正常模式：返回 { code: 200, data: { initialized: true, ... } }
      if (data.code === 200 && data.data !== undefined) {
        initStatus = !!data.data.initialized
      } else {
        initStatus = !!data.initialized
      }
      return initStatus
    }
  } catch (error) {
    logger.error('检查系统初始化状态失败:', error)
  }
  initStatus = true
  return initStatus
}

/**
 * 重置前端缓存的初始化状态。
 *
 * 场景：用户在 setup 页面完成 `initialize-with-db` 之后，
 * 路由跳转过程中 `checkInitStatus()` 由于模块级缓存仍为 `false`
 * 会被路由守卫再次拉回 /setup。此函数用于在 Setup.vue 安装成功后
 * 主动将缓存置为 `true`，让守卫放行。
 */
export function resetInitStatus(initialized: boolean = true) {
  initStatus = initialized ? true : null
}

/**
 * 权限码匹配检查（批次 3：路由守卫权限校验）
 *
 * 宽松匹配规则（避免因后端权限码命名不统一而锁死用户）：
 * 1. admin 角色直接通过（由调用方判断，此处不查）
 * 2. 用户未配置任何权限码（permissions 为空）→ 调用方放行
 * 3. 通配符：用户持有 "resource:*" 可匹配该 resource 下的任意 action
 * 4. read/view 等价、update/edit 等价（后端两套 action 命名兼容）
 *
 * @param required 路由 meta.permission（单个权限码或数组，任一匹配即通过）
 * @param userPermissions 用户实际持有的权限码列表
 * @returns 是否拥有访问权限
 */
export function hasRoutePermission(
  required: string | string[] | undefined,
  userPermissions: string[]
): boolean {
  if (!required) return true
  const requiredList = Array.isArray(required) ? required : [required]
  return requiredList.some(req => {
    const sepIdx = req.indexOf(':')
    const reqResource = sepIdx > 0 ? req.slice(0, sepIdx) : req
    const reqAction = sepIdx > 0 ? req.slice(sepIdx + 1) : ''
    return userPermissions.some(up => {
      const upSep = up.indexOf(':')
      const upResource = upSep > 0 ? up.slice(0, upSep) : up
      const upAction = upSep > 0 ? up.slice(upSep + 1) : ''
      if (upResource !== reqResource) return false
      if (upAction === '*') return true
      if (upAction === reqAction) return true
      // 后端 action 命名不统一：read/view 等价，update/edit 等价
      if ((upAction === 'read' && reqAction === 'view') || (upAction === 'view' && reqAction === 'read')) return true
      if ((upAction === 'update' && reqAction === 'edit') || (upAction === 'edit' && reqAction === 'update')) return true
      return false
    })
  })
}

router.beforeEach(async (to, _from, next) => {
  const title = to.meta.title as string
  if (title) {
    document.title = `${title} - 面料管理系统`
  }

  if (to.path === '/setup') {
    next()
    return
  }

  if (to.path !== '/login' && to.path !== '/404' && to.path !== '/403') {
    const initialized = await checkInitStatus()
    if (!initialized) {
      next({ path: '/setup' })
      return
    }
  }

  if (to.meta.requiresAuth) {
    // Wave B-3：access_token 存于 httpOnly Cookie，JS 不可读。
    // 改用 userStore.userInfo 作为"已登录"标识；后端 getCurrentUser 失败则跳转登录。
    const userStore = useUserStore()
    if (!userStore.userInfo) {
      try {
        await userStore.fetchUserInfo()
      } catch (error) {
        logger.error('获取用户信息失败（未登录或会话已过期）:', error)
        // 401 已被 request.ts 拦截器处理并跳转登录，这里仅作为兜底
        next({ path: '/login', query: { redirect: to.fullPath } })
        return
      }
    }

    // 批次 3：路由 meta.permission 权限码校验（宽松模式）
    // - admin 角色绕过（与 v-permission 指令行为一致）
    // - 用户未配置任何权限码时放行（避免锁死未配置权限的账户）
    // - 权限码匹配支持通配符与 read/view 等价（兼容后端两套 action 命名）
    if (to.meta.permission && userStore.userInfo) {
      const user = userStore.userInfo
      if (user.role_name !== 'admin') {
        const userPerms = user.permissions || []
        if (userPerms.length > 0 && !hasRoutePermission(to.meta.permission, userPerms)) {
          logger.warn(`权限不足：访问 ${to.path} 需要权限码 ${JSON.stringify(to.meta.permission)}，用户持有 ${JSON.stringify(userPerms)}`)
          next({ path: '/403' })
          return
        }
      }
    }
  }

  next()
})

export default router
