import { createRouter, createWebHistory } from 'vue-router'

const routes = [
  {
    path: '/',
    redirect: '/dashboard'
  },
  {
    path: '/login',
    name: 'Login',
    component: () => import('@/views/Login.vue'),
    meta: { title: '登录' }
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('@/views/Dashboard.vue'),
    meta: { title: '仪表盘', requiresAuth: true }
  },
  {
    path: '/system',
    name: 'System',
    component: () => import('@/views/system/index.vue'),
    meta: { title: '系统管理', requiresAuth: true }
  },
  {
    path: '/finance',
    name: 'Finance',
    component: () => import('@/views/finance/index.vue'),
    meta: { title: '财务管理', requiresAuth: true }
  },
  {
    path: '/ap',
    name: 'AP',
    component: () => import('@/views/ap/index.vue'),
    meta: { title: '应付管理', requiresAuth: true }
  },
  {
    path: '/ar',
    name: 'AR',
    component: () => import('@/views/ar/index.vue'),
    meta: { title: '应收管理', requiresAuth: true }
  },
  {
    path: '/fabric',
    name: 'Fabric',
    component: () => import('@/views/fabric/index.vue'),
    meta: { title: '面料管理', requiresAuth: true }
  },
  {
    path: '/inventory',
    name: 'Inventory',
    component: () => import('@/views/inventory/index.vue'),
    meta: { title: '库存管理', requiresAuth: true }
  },
  {
    path: '/sales',
    name: 'Sales',
    component: () => import('@/views/sales/index.vue'),
    meta: { title: '销售管理', requiresAuth: true }
  },
  {
    path: '/purchase',
    name: 'Purchase',
    component: () => import('@/views/purchase/index.vue'),
    meta: { title: '采购管理', requiresAuth: true }
  },
  {
    path: '/customer',
    name: 'Customer',
    component: () => import('@/views/customer/index.vue'),
    meta: { title: '客户管理', requiresAuth: true }
  },
  {
    path: '/supplier',
    name: 'Supplier',
    component: () => import('@/views/supplier/index.vue'),
    meta: { title: '供应商管理', requiresAuth: true }
  },
  {
    path: '/product',
    name: 'Product',
    component: () => import('@/views/product/index.vue'),
    meta: { title: '产品管理', requiresAuth: true }
  },
  {
    path: '/warehouse',
    name: 'Warehouse',
    component: () => import('@/views/warehouse/index.vue'),
    meta: { title: '仓库管理', requiresAuth: true }
  },
  {
    path: '/bpm',
    name: 'BPM',
    component: () => import('@/views/bpm/index.vue'),
    meta: { title: '审批管理', requiresAuth: true }
  },
  {
    path: '/quality',
    name: 'Quality',
    component: () => import('@/views/quality/index.vue'),
    meta: { title: '质量管理', requiresAuth: true }
  },
  {
    path: '/purchase-ext',
    name: 'PurchaseExt',
    component: () => import('@/views/purchase-ext/index.vue'),
    meta: { title: '采购扩展', requiresAuth: true }
  },
  {
    path: '/sales-ext',
    name: 'SalesExt',
    component: () => import('@/views/sales-ext/index.vue'),
    meta: { title: '销售扩展', requiresAuth: true }
  },
  {
    path: '/crm',
    name: 'CRM',
    component: () => import('@/views/crm/index.vue'),
    meta: { title: '客户关系管理', requiresAuth: true }
  },
  {
    path: '/advanced',
    name: 'Advanced',
    component: () => import('@/views/advanced/index.vue'),
    meta: { title: '高级功能', requiresAuth: true }
  },
  {
    path: '/production',
    name: 'Production',
    component: () => import('@/views/production/index.vue'),
    meta: { title: '生产计划', requiresAuth: true }
  },
  {
    path: '/cost',
    name: 'Cost',
    component: () => import('@/views/cost/index.vue'),
    meta: { title: '成本归集', requiresAuth: true }
  },
  {
    path: '/budget',
    name: 'Budget',
    component: () => import('@/views/budget/index.vue'),
    meta: { title: '预算管理', requiresAuth: true }
  },
  {
    path: '/fund',
    name: 'Fund',
    component: () => import('@/views/fund/index.vue'),
    meta: { title: '资金管理', requiresAuth: true }
  },
  {
    path: '/financial-analysis',
    name: 'FinancialAnalysis',
    component: () => import('@/views/financial-analysis/index.vue'),
    meta: { title: '财务分析', requiresAuth: true }
  },
  {
    path: '/supplier-evaluation',
    name: 'SupplierEvaluation',
    component: () => import('@/views/supplierEvaluation/index.vue'),
    meta: { title: '供应商评估', requiresAuth: true }
  },
  {
    path: '/customer-credit',
    name: 'CustomerCredit',
    component: () => import('@/views/customerCredit/index.vue'),
    meta: { title: '客户信用管理', requiresAuth: true }
  },
  {
    path: '/currency',
    name: 'Currency',
    component: () => import('@/views/currency/index.vue'),
    meta: { title: '多币种管理', requiresAuth: true }
  },
  {
    path: '/notification',
    name: 'Notification',
    component: () => import('@/views/notification/index.vue'),
    meta: { title: '通知中心', requiresAuth: true }
  },
  {
    path: '/data-permission',
    name: 'DataPermission',
    component: () => import('@/views/dataPermission/index.vue'),
    meta: { title: '数据权限管理', requiresAuth: true }
  },
  {
    path: '/inventory-batch',
    name: 'InventoryBatch',
    component: () => import('@/views/inventoryBatch/index.vue'),
    meta: { title: '批次管理', requiresAuth: true }
  },
  {
    path: '/five-dimension',
    name: 'FiveDimension',
    component: () => import('@/views/fiveDimension/index.vue'),
    meta: { title: '五维管理', requiresAuth: true }
  },
  {
    path: '/assist-accounting',
    name: 'AssistAccounting',
    component: () => import('@/views/assistAccounting/index.vue'),
    meta: { title: '辅助核算', requiresAuth: true }
  },
  {
    path: '/business-trace',
    name: 'BusinessTrace',
    component: () => import('@/views/businessTrace/index.vue'),
    meta: { title: '业务追溯', requiresAuth: true }
  },
  {
    path: '/barcode-scanner',
    name: 'BarcodeScanner',
    component: () => import('@/views/barcodeScanner/index.vue'),
    meta: { title: '扫码功能', requiresAuth: true }
  },
  {
    path: '/omni-audit',
    name: 'OmniAudit',
    component: () => import('@/views/omniAudit/index.vue'),
    meta: { title: '全量审计', requiresAuth: true }
  },
  {
    path: '/403',
    name: '403',
    component: () => import('@/views/403.vue'),
    meta: { title: '无权限' }
  },
  {
    path: '/404',
    name: '404',
    component: () => import('@/views/404.vue'),
    meta: { title: '页面不存在' }
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/404'
  }
]

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes
})

router.beforeEach((to, _from, next) => {
  const title = to.meta.title as string
  if (title) {
    document.title = `${title} - 秉羲面料管理系统`
  }

  if (to.meta.requiresAuth) {
    const token = localStorage.getItem('token')
    if (!token) {
      next({ path: '/login', query: { redirect: to.fullPath } })
      return
    }
  }

  next()
})

export default router
