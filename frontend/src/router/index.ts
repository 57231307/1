import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '@/components/Layout/MainLayout.vue'
import { getToken, removeToken } from '@/utils/storage'

const routes = [
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
        meta: { title: '仪表盘', requiresAuth: true },
      },
      {
        path: 'system',
        name: 'System',
        component: () => import('@/views/system/index.vue'),
        meta: { title: '系统管理', requiresAuth: true },
      },
      {
        path: 'finance',
        name: 'Finance',
        component: () => import('@/views/finance/index.vue'),
        meta: { title: '财务管理', requiresAuth: true },
      },
      {
        path: 'ap',
        name: 'AP',
        component: () => import('@/views/ap/index.vue'),
        meta: { title: '应付管理', requiresAuth: true },
      },
      {
        path: 'ar',
        name: 'AR',
        component: () => import('@/views/ar/index.vue'),
        meta: { title: '应收管理', requiresAuth: true },
      },
      {
        path: 'fabric',
        name: 'Fabric',
        component: () => import('@/views/fabric/index.vue'),
        meta: { title: '面料管理', requiresAuth: true },
      },
      {
        path: 'inventory',
        name: 'Inventory',
        component: () => import('@/views/inventory/index.vue'),
        meta: { title: '库存管理', requiresAuth: true },
      },
      {
        path: 'sales',
        name: 'Sales',
        component: () => import('@/views/sales/index.vue'),
        meta: { title: '销售管理', requiresAuth: true },
      },
      {
        path: 'purchase',
        name: 'Purchase',
        component: () => import('@/views/purchase/index.vue'),
        meta: { title: '采购管理', requiresAuth: true },
      },
      {
        path: 'customer',
        name: 'Customer',
        component: () => import('@/views/customer/index.vue'),
        meta: { title: '客户管理', requiresAuth: true },
      },
      {
        path: 'supplier',
        name: 'Supplier',
        component: () => import('@/views/supplier/index.vue'),
        meta: { title: '供应商管理', requiresAuth: true },
      },
      {
        path: 'product',
        name: 'Product',
        component: () => import('@/views/product/index.vue'),
        meta: { title: '产品管理', requiresAuth: true },
      },
      {
        path: 'warehouse',
        name: 'Warehouse',
        component: () => import('@/views/warehouse/index.vue'),
        meta: { title: '仓库管理', requiresAuth: true },
      },
      {
        path: 'departments',
        name: 'Departments',
        component: () => import('@/views/departments/index.vue'),
        meta: { title: '部门管理', requiresAuth: true },
      },
      {
        path: 'greige-fabrics',
        name: 'GreigeFabrics',
        component: () => import('@/views/greige-fabrics/index.vue'),
        meta: { title: '坯布管理', requiresAuth: true },
      },
      {
        path: 'sales-returns',
        name: 'SalesReturns',
        component: () => import('@/views/sales-returns/index.vue'),
        meta: { title: '销售退货管理', requiresAuth: true },
      },
      {
        path: 'supplier-evaluation',
        name: 'SupplierEvaluation',
        component: () => import('@/views/supplierEvaluation/index.vue'),
        meta: { title: '供应商评估', requiresAuth: true },
      },
      {
        path: 'customer-credit',
        name: 'CustomerCredit',
        component: () => import('@/views/customerCredit/index.vue'),
        meta: { title: '客户信用管理', requiresAuth: true },
      },
      {
        path: 'inventory-count',
        name: 'InventoryCount',
        component: () => import('@/views/inventoryCount/index.vue'),
        meta: { title: '库存盘点', requiresAuth: true },
      },
      {
        path: 'inventory-transfer',
        name: 'InventoryTransfer',
        component: () => import('@/views/inventoryTransfer/index.vue'),
        meta: { title: '库存调拨', requiresAuth: true },
      },
      {
        path: 'inventory-adjustment',
        name: 'InventoryAdjustment',
        component: () => import('@/views/inventoryAdjustment/index.vue'),
        meta: { title: '库存调整', requiresAuth: true },
      },
      {
        path: 'ar-reconciliation',
        name: 'ArReconciliation',
        component: () => import('@/views/arReconciliation/index.vue'),
        meta: { title: '应收对账', requiresAuth: true },
      },
      {
        path: 'finance-report',
        name: 'FinanceReport',
        component: () => import('@/views/financeReport/index.vue'),
        meta: { title: '财务报表', requiresAuth: true },
      },
      {
        path: 'purchase-receipt',
        name: 'PurchaseReceipt',
        component: () => import('@/views/purchaseReceipt/index.vue'),
        meta: { title: '采购入库', requiresAuth: true },
      },
      {
        path: 'fixed-assets',
        name: 'FixedAssets',
        component: () => import('@/views/fixed-assets/index.vue'),
        meta: { title: '固定资产管理', requiresAuth: true },
      },
      {
        path: 'bpm',
        name: 'BPM',
        component: () => import('@/views/bpm/index.vue'),
        meta: { title: '审批管理', requiresAuth: true },
      },
      {
        path: 'quality',
        name: 'Quality',
        component: () => import('@/views/quality/index.vue'),
        meta: { title: '质量管理', requiresAuth: true },
      },
      {
        path: 'purchase-ext',
        name: 'PurchaseExt',
        component: () => import('@/views/purchase-ext/index.vue'),
        meta: { title: '采购扩展', requiresAuth: true },
      },
      {
        path: 'sales-ext',
        name: 'SalesExt',
        component: () => import('@/views/sales-ext/index.vue'),
        meta: { title: '销售扩展', requiresAuth: true },
      },
      {
        path: 'crm',
        name: 'CRM',
        component: () => import('@/views/crm/index.vue'),
        meta: { title: '客户关系管理', requiresAuth: true },
      },
      {
        path: 'crm/pool',
        name: 'CRMPool',
        component: () => import('@/views/crm/pool.vue'),
        meta: { title: '公海客户池', requiresAuth: true },
      },
      {
        path: 'crm/assignment',
        name: 'CRMAssignment',
        component: () => import('@/views/crm/assignment.vue'),
        meta: { title: '客户分配', requiresAuth: true },
      },
      {
        path: 'crm/detail/:id',
        name: 'CRMDetail',
        component: () => import('@/views/crm/detail.vue'),
        meta: { title: '客户360视图', requiresAuth: true },
      },
      {
        path: 'advanced',
        name: 'Advanced',
        component: () => import('@/views/advanced/index.vue'),
        meta: { title: '高级功能', requiresAuth: true },
      },
      {
        path: 'production',
        name: 'Production',
        component: () => import('@/views/production/index.vue'),
        meta: { title: '生产计划', requiresAuth: true },
      },
      {
        path: 'bom',
        name: 'Bom',
        component: () => import('@/views/bom/index.vue'),
        meta: { title: 'BOM管理', requiresAuth: true },
      },
      {
        path: 'mrp',
        name: 'Mrp',
        component: () => import('@/views/mrp/index.vue'),
        meta: { title: 'MRP计算', requiresAuth: true },
      },
      {
        path: 'mrp/history',
        name: 'MrpHistory',
        component: () => import('@/views/mrp/history.vue'),
        meta: { title: 'MRP历史记录', requiresAuth: true },
      },
      {
        path: 'capacity',
        name: 'Capacity',
        component: () => import('@/views/capacity/index.vue'),
        meta: { title: '产能分析', requiresAuth: true },
      },
      {
        path: 'material-shortage',
        name: 'MaterialShortage',
        component: () => import('@/views/material-shortage/index.vue'),
        meta: { title: '缺料预警', requiresAuth: true },
      },
      {
        path: 'cost',
        name: 'Cost',
        component: () => import('@/views/cost/index.vue'),
        meta: { title: '成本归集', requiresAuth: true },
      },
      {
        path: 'budget',
        name: 'Budget',
        component: () => import('@/views/budget/index.vue'),
        meta: { title: '预算管理', requiresAuth: true },
      },
      {
        path: 'fund',
        name: 'Fund',
        component: () => import('@/views/fund/index.vue'),
        meta: { title: '资金管理', requiresAuth: true },
      },
      {
        path: 'financial-analysis',
        name: 'FinancialAnalysis',
        component: () => import('@/views/financial-analysis/index.vue'),
        meta: { title: '财务分析', requiresAuth: true },
      },
      {
        path: 'currency',
        name: 'Currency',
        component: () => import('@/views/currency/index.vue'),
        meta: { title: '多币种管理', requiresAuth: true },
      },
      {
        path: 'notification',
        name: 'Notification',
        component: () => import('@/views/notification/index.vue'),
        meta: { title: '通知中心', requiresAuth: true },
      },
      {
        path: 'data-permission',
        name: 'DataPermission',
        component: () => import('@/views/dataPermission/index.vue'),
        meta: { title: '数据权限管理', requiresAuth: true },
      },
      {
        path: 'inventory-batch',
        name: 'InventoryBatch',
        component: () => import('@/views/inventoryBatch/index.vue'),
        meta: { title: '批次管理', requiresAuth: true },
      },
      {
        path: 'five-dimension',
        name: 'FiveDimension',
        component: () => import('@/views/fiveDimension/index.vue'),
        meta: { title: '五维管理', requiresAuth: true },
      },
      {
        path: 'assist-accounting',
        name: 'AssistAccounting',
        component: () => import('@/views/assistAccounting/index.vue'),
        meta: { title: '辅助核算', requiresAuth: true },
      },
      {
        path: 'business-trace',
        name: 'BusinessTrace',
        component: () => import('@/views/businessTrace/index.vue'),
        meta: { title: '业务追溯', requiresAuth: true },
      },
      {
        path: 'barcode-scanner',
        name: 'BarcodeScanner',
        component: () => import('@/views/barcodeScanner/index.vue'),
        meta: { title: '扫码功能', requiresAuth: true },
      },
      {
        path: 'omni-audit',
        name: 'OmniAudit',
        component: () => import('@/views/omniAudit/index.vue'),
        meta: { title: '全量审计', requiresAuth: true },
      },
      {
        path: 'scheduling',
        name: 'Scheduling',
        component: () => import('@/views/scheduling/index.vue'),
        meta: { title: '生产排程', requiresAuth: true },
      },
      {
        path: 'scheduling/gantt',
        name: 'SchedulingGantt',
        component: () => import('@/views/scheduling/gantt.vue'),
        meta: { title: '排程甘特图', requiresAuth: true },
      },
      {
        path: 'components-demo',
        name: 'ComponentsDemo',
        component: () => import('@/views/components-demo/index.vue'),
        meta: { title: '组件示例', requiresAuth: true },
      },
      // 新增路由 - 凭证管理
      {
        path: 'voucher',
        name: 'Voucher',
        component: () => import('@/views/voucher/index.vue'),
        meta: { title: '凭证管理', requiresAuth: true },
      },
      // 新增路由 - 会计科目管理
      {
        path: 'account-subject',
        name: 'AccountSubject',
        component: () => import('@/views/accountSubject/index.vue'),
        meta: { title: '会计科目管理', requiresAuth: true },
      },
      // 新增路由 - 会计期间管理
      {
        path: 'accounting-period',
        name: 'AccountingPeriod',
        component: () => import('@/views/accountingPeriod/index.vue'),
        meta: { title: '会计期间管理', requiresAuth: true },
      },
      // 新增路由 - 交易管理
      {
        path: 'trading',
        name: 'Trading',
        component: () => import('@/views/trading/index.vue'),
        meta: { title: '交易管理', requiresAuth: true },
      },
      // 新增路由 - 报表模板
      {
        path: 'report-templates',
        name: 'ReportTemplates',
        component: () => import('@/views/report/templates.vue'),
        meta: { title: '报表模板', requiresAuth: true },
      },
      // 新增路由 - BPM流程定义
      {
        path: 'bpm/definitions',
        name: 'BPMDefinitions',
        component: () => import('@/views/bpm/definitions.vue'),
        meta: { title: '流程定义管理', requiresAuth: true },
      },
      // 新增路由 - BPM流程模板
      {
        path: 'bpm/templates',
        name: 'BPMTemplates',
        component: () => import('@/views/bpm/templates.vue'),
        meta: { title: '流程模板管理', requiresAuth: true },
      },
      // 新增路由 - 增强版应收对账
      {
        path: 'ar-reconciliation/enhanced',
        name: 'ArReconciliationEnhanced',
        component: () => import('@/views/arReconciliation/enhanced.vue'),
        meta: { title: '增强版应收对账', requiresAuth: true },
      },
      // 新增路由 - 销售分析
      {
        path: 'sales-analysis',
        name: 'SalesAnalysis',
        component: () => import('@/views/sales-analysis/index.vue'),
        meta: { title: '销售分析', requiresAuth: true },
      },
      // 新增路由 - 安全管理
      {
        path: 'security',
        name: 'Security',
        component: () => import('@/views/security/index.vue'),
        meta: { title: '安全管理', requiresAuth: true },
      },
      {
        path: 'dye-recipe',
        name: 'DyeRecipe',
        component: () => import('@/views/dye-recipe/index.vue'),
        meta: { title: '染色配方', requiresAuth: true },
      },
      {
        path: 'dye-batch',
        name: 'DyeBatch',
        component: () => import('@/views/dye-batch/index.vue'),
        meta: { title: '染色批次', requiresAuth: true },
      },
      {
        path: 'purchase-contract',
        name: 'PurchaseContract',
        component: () => import('@/views/purchase-contract/index.vue'),
        meta: { title: '采购合同', requiresAuth: true },
      },
      {
        path: 'sales-contract',
        name: 'SalesContract',
        component: () => import('@/views/sales-contract/index.vue'),
        meta: { title: '销售合同', requiresAuth: true },
      },
      {
        path: 'purchase-price',
        name: 'PurchasePrice',
        component: () => import('@/views/purchase-price/index.vue'),
        meta: { title: '采购价格', requiresAuth: true },
      },
      {
        path: 'sales-price',
        name: 'SalesPrice',
        component: () => import('@/views/sales-price/index.vue'),
        meta: { title: '销售价格', requiresAuth: true },
      },
      // 新增路由 - 邮件管理
      {
        path: 'email',
        name: 'Email',
        component: () => import('@/views/email/index.vue'),
        meta: { title: '邮件管理', requiresAuth: true },
      },
      // 新增路由 - 租户计费管理
      {
        path: 'tenant-billing',
        name: 'TenantBilling',
        component: () => import('@/views/tenant-billing/index.vue'),
        meta: { title: '租户计费管理', requiresAuth: true },
      },
      // 新增路由 - 采购检验
      {
        path: 'purchase-inspection',
        name: 'PurchaseInspection',
        component: () => import('@/views/purchase-inspection/index.vue'),
        meta: { title: '采购检验', requiresAuth: true },
      },
      // 新增路由 - 采购退货
      {
        path: 'purchase-return',
        name: 'PurchaseReturn',
        component: () => import('@/views/purchase-return/index.vue'),
        meta: { title: '采购退货', requiresAuth: true },
      },
      // 新增路由 - 物流管理
      {
        path: 'logistics',
        name: 'Logistics',
        component: () => import('@/views/logistics/index.vue'),
        meta: { title: '物流管理', requiresAuth: true },
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
    console.error('检查系统状态失败:', error)
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
    const token = getToken()
    if (!token) {
      next({ path: '/login', query: { redirect: to.fullPath } })
      return
    }

    try {
      const parts = token.split('.')
      if (parts.length === 3) {
        const tokenData = JSON.parse(atob(parts[1]))
        const currentTime = Math.floor(Date.now() / 1000)

        if (tokenData.exp && tokenData.exp < currentTime) {
          removeToken()
          next({ path: '/login', query: { redirect: to.fullPath } })
          return
        }
      }
    } catch (error) {
      console.error('Token validation failed:', error)
      removeToken()
      next({ path: '/login', query: { redirect: to.fullPath } })
      return
    }
  }

  next()
})

export default router
