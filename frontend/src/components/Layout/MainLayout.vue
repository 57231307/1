<template>
  <el-container class="main-layout">
    <el-aside width="220px" class="aside">
      <div class="logo">
        <h2>{{ $t('layout.brand') }}</h2>
      </div>
      <el-menu
        :default-active="activeMenu"
        class="menu"
        background-color="#304156"
        text-color="#bfcbd9"
        active-text-color="#409eff"
        router
        role="menubar"
        :aria-label="$t('layout.menuAriaLabel')"
        @open="handleMenuOpen"
        @close="handleMenuClose"
      >
        <el-menu-item role="menuitem" v-if="canAccessMenu('/dashboard')" index="/dashboard">
          <el-icon><HomeFilled /></el-icon>
          <span>{{ $t('layout.menu.dashboard') }}</span>
        </el-menu-item>

        <el-sub-menu v-if="visibleSubMenu.fabric" index="fabric" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('fabric')">
          <template #title>
            <el-icon><Goods /></el-icon>
            <span>{{ $t('layout.menu.fabric') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/fabric')" index="/fabric">{{ $t('layout.menu.fabricList') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/greige-fabrics')" index="/greige-fabrics">{{ $t('layout.menu.greigeFabrics') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/product')" index="/product">{{ $t('layout.menu.product') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/color-cards/list')" index="/color-cards/list">{{ $t('layout.menu.colorCardsList') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/color-cards/issues')" index="/color-cards/issues">{{ $t('layout.menu.colorCardsIssue') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/color-prices/list')" index="/color-prices/list">{{ $t('layout.menu.colorPricesList') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/color-prices/batch-adjust')" index="/color-prices/batch-adjust">{{ $t('layout.menu.colorPricesBatchAdjust') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.inventory" index="inventory" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('inventory')">
          <template #title>
            <el-icon><Box /></el-icon>
            <span>{{ $t('layout.menu.inventory') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/inventory')" index="/inventory">{{ $t('layout.menu.inventoryList') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/warehouse')" index="/warehouse">{{ $t('layout.menu.warehouse') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/inventory-batch')" index="/inventory-batch">{{ $t('layout.menu.inventoryBatch') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/inventory-count')" index="/inventory-count">{{ $t('layout.menu.inventoryCount') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/inventory-transfer')" index="/inventory-transfer">{{ $t('layout.menu.inventoryTransfer') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/inventory-adjustment')" index="/inventory-adjustment">{{ $t('layout.menu.inventoryAdjustment') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/logistics')" index="/logistics">{{ $t('layout.menu.logistics') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.sales" index="sales" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('sales')">
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>{{ $t('layout.menu.sales') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/sales')" index="/sales">{{ $t('layout.menu.salesOrder') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/sales-returns')" index="/sales-returns">{{ $t('layout.menu.salesReturns') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/sales-ext')" index="/sales-ext">{{ $t('layout.menu.salesExt') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/customer')" index="/customer">{{ $t('layout.menu.customer') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/customer-credit')" index="/customer-credit">{{ $t('layout.menu.customerCredit') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/sales-contract')" index="/sales-contract">{{ $t('layout.menu.salesContract') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/sales-price')" index="/sales-price">{{ $t('layout.menu.salesPrice') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/sales-analysis')" index="/sales-analysis">{{ $t('layout.menu.salesAnalysis') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/quotations')" index="/quotations">{{ $t('layout.menu.quotations') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.purchase" index="purchase" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('purchase')">
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>{{ $t('layout.menu.purchase') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/purchase')" index="/purchase">{{ $t('layout.menu.purchaseOrder') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/purchase-receipt')" index="/purchase-receipt">{{ $t('layout.menu.purchaseReceipt') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/purchase-ext')" index="/purchase-ext">{{ $t('layout.menu.purchaseExt') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/supplier')" index="/supplier">{{ $t('layout.menu.supplier') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/supplier-evaluation')" index="/supplier-evaluation">{{ $t('layout.menu.supplierEvaluation') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/purchase-contract')" index="/purchase-contract">{{ $t('layout.menu.purchaseContract') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/purchase-price')" index="/purchase-price">{{ $t('layout.menu.purchasePrice') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/purchase-inspection')" index="/purchase-inspection">{{ $t('layout.menu.purchaseInspection') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/purchase-return')" index="/purchase-return">{{ $t('layout.menu.purchaseReturn') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.crm" index="crm" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('crm')">
          <template #title>
            <el-icon><User /></el-icon>
            <span>{{ $t('layout.menu.crm') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/crm')" index="/crm">{{ $t('layout.menu.crmManagement') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/crm/pool')" index="/crm/pool">{{ $t('layout.menu.crmPool') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/crm/assignment')" index="/crm/assignment">{{ $t('layout.menu.crmAssignment') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/crm/leads')" index="/crm/leads">{{ $t('layout.menu.crmLeads') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/crm/opportunities')" index="/crm/opportunities">{{ $t('layout.menu.crmOpportunities') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.production" index="production" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('production')">
          <template #title>
            <el-icon><Cpu /></el-icon>
            <span>{{ $t('layout.menu.production') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/production')" index="/production">{{ $t('layout.menu.productionPlan') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/bom')" index="/bom">{{ $t('layout.menu.bom') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/mrp')" index="/mrp">{{ $t('layout.menu.mrp') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/mrp/history')" index="/mrp/history">{{ $t('layout.menu.mrpHistory') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/capacity')" index="/capacity">{{ $t('layout.menu.capacity') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/material-shortage')" index="/material-shortage">{{ $t('layout.menu.materialShortage') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/scheduling')" index="/scheduling">{{ $t('layout.menu.scheduling') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/quality')" index="/quality">{{ $t('layout.menu.quality') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/scheduling/gantt')" index="/scheduling/gantt">{{ $t('layout.menu.gantt') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/custom-orders')" index="/custom-orders">{{ $t('layout.menu.customOrders') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/dye-recipe')" index="/dye-recipe">{{ $t('layout.menu.dyeRecipe') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/dye-batch')" index="/dye-batch">{{ $t('layout.menu.dyeBatch') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.finance" index="finance" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('finance')">
          <template #title>
            <el-icon><Money /></el-icon>
            <span>{{ $t('layout.menu.finance') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/finance')" index="/finance">{{ $t('layout.menu.financeOverview') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/ap')" index="/ap">{{ $t('layout.menu.ap') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/ar')" index="/ar">{{ $t('layout.menu.ar') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/ar-reconciliation')" index="/ar-reconciliation">{{ $t('layout.menu.arReconciliation') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/finance-report')" index="/finance-report">{{ $t('layout.menu.financeReport') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/cost')" index="/cost">{{ $t('layout.menu.cost') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/budget')" index="/budget">{{ $t('layout.menu.budget') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/fund')" index="/fund">{{ $t('layout.menu.fund') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/fixed-assets')" index="/fixed-assets">{{ $t('layout.menu.fixedAssets') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/currency')" index="/currency">{{ $t('layout.menu.currency') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/financial-analysis')" index="/financial-analysis">{{ $t('layout.menu.financialAnalysis') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/assist-accounting')" index="/assist-accounting">{{ $t('layout.menu.assistAccounting') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/account-subject')" index="/account-subject">{{ $t('layout.menu.accountSubject') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/accounting-period')" index="/accounting-period">{{ $t('layout.menu.accountingPeriod') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/voucher')" index="/voucher">{{ $t('layout.menu.voucher') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/trading')" index="/trading">{{ $t('layout.menu.trading') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/ar-reconciliation/enhanced')" index="/ar-reconciliation/enhanced">{{ $t('layout.menu.arReconciliationEnhanced') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/bi/sales-analysis')" index="/bi/sales-analysis">{{ $t('layout.menu.biSalesAnalysis') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.workflow" index="workflow" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('workflow')">
          <template #title>
            <el-icon><List /></el-icon>
            <span>{{ $t('layout.menu.workflow') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/bpm')" index="/bpm">{{ $t('layout.menu.bpm') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/bpm/definitions')" index="/bpm/definitions">{{ $t('layout.menu.bpmDefinitions') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/bpm/templates')" index="/bpm/templates">{{ $t('layout.menu.bpmTemplates') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/bpm/approval')" index="/bpm/approval">{{ $t('layout.menu.bpmApproval') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/business-trace')" index="/business-trace">{{ $t('layout.menu.businessTrace') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/barcode-scanner')" index="/barcode-scanner">{{ $t('layout.menu.barcodeScanner') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/quality-standards')" index="/quality-standards">{{ $t('layout.menu.qualityStandards') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.system" index="system" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('system')">
          <template #title>
            <el-icon><Setting /></el-icon>
            <span>{{ $t('layout.menu.system') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/system')" index="/system">{{ $t('layout.menu.systemSettings') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/departments')" index="/departments">{{ $t('layout.menu.departments') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/five-dimension')" index="/five-dimension">{{ $t('layout.menu.fiveDimension') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/data-permission')" index="/data-permission">{{ $t('layout.menu.dataPermission') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/report-templates')" index="/report-templates">{{ $t('layout.menu.reportTemplates') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/data-import')" index="/data-import">{{ $t('layout.menu.dataImport') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/print-templates')" index="/print-templates">{{ $t('layout.menu.printTemplates') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/api-gateway')" index="/api-gateway">{{ $t('layout.menu.apiGateway') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/system-update')" index="/system-update">{{ $t('layout.menu.systemUpdate') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/advanced')" index="/advanced">{{ $t('layout.menu.advanced') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/notification')" index="/notification">{{ $t('layout.menu.notification') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/omni-audit')" index="/omni-audit">{{ $t('layout.menu.omniAudit') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/system/audit-log')" index="/system/audit-log">{{ $t('layout.menu.auditLog') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/system/slow-query')" index="/system/slow-query">{{ $t('layout.menu.slowQuery') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/security')" index="/security">{{ $t('layout.menu.security') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/email')" index="/email">{{ $t('layout.menu.email') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/admin/failover')" index="/admin/failover">{{ $t('layout.menu.failover') }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.ai" index="ai" role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('ai')">
          <template #title>
            <el-icon><MagicStick /></el-icon>
            <span>{{ $t('layout.menu.ai') }}</span>
          </template>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/ai-extend')" index="/ai-extend">{{ $t('layout.menu.aiExtend') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/ai-extend/process-optimization')" index="/ai-extend/process-optimization">{{ $t('layout.menu.aiProcessOptimization') }}</el-menu-item>
          <el-menu-item role="menuitem" v-if="canAccessMenu('/ai-extend/quality-prediction')" index="/ai-extend/quality-prediction">{{ $t('layout.menu.aiQualityPrediction') }}</el-menu-item>
        </el-sub-menu>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="header">
        <div class="header-left">
          <el-breadcrumb separator="/">
            <el-breadcrumb-item :to="{ path: '/' }">{{ $t('layout.breadcrumb.home') }}</el-breadcrumb-item>
            <el-breadcrumb-item>{{ currentTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <el-dropdown>
            <span class="user-info">
              {{ userStore.userInfo?.username || $t('layout.user.defaultName') }}
              <el-icon><ArrowDown /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="$router.push('/system/profile')"
                  >{{ $t('layout.user.profile') }}</el-dropdown-item
                >
                <el-dropdown-item divided @click="handleLogout">{{ $t('layout.user.logout') }}</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <el-main class="main-content">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  HomeFilled,
  Goods,
  Box,
  ShoppingCart,
  ArrowDown,
  Money,
  Setting,
  User,
  Cpu,
  List,
  MagicStick,
} from '@element-plus/icons-vue'
import { useUserStore } from '@/store/user'
// 批次 6 修复（2026-06-28）：MainLayout 菜单按 permission 过滤（审计 #8 完整修复）
// 复用 router 守卫同款宽松匹配函数，保证菜单可见性与路由可达性一致。
import { hasRoutePermission } from '@/router'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()

const activeMenu = computed(() => route.path)
const currentTitle = computed(() => (route.meta.title as string) || '')

// 批次 6：用户权限与角色响应式派生
// 批次 22 v5 P0-5：permissions 为 readonly string[]，computed 类型同步
const userPermissions = computed<readonly string[]>(() => userStore.userInfo?.permissions || [])
// P2 1-12 修复：删除 isAdmin computed（role_name === 'admin' 硬编码），统一走 *:* 通配权限

// P0 4-3 修复：维护子菜单展开状态供 aria-expanded 使用（WCAG 无障碍）
const openedMenus = ref<string[]>([])
const handleMenuOpen = (index: string) => {
  if (!openedMenus.value.includes(index)) {
    openedMenus.value.push(index)
  }
}
const handleMenuClose = (index: string) => {
  openedMenus.value = openedMenus.value.filter(i => i !== index)
}

/**
 * 批次 6（2026-06-28）：菜单项可见性判定
 *
 * 批次 22 v5 P0-7 修复：与守卫 P0-6 严格化保持一致，移除"空权限放行"。
 * - admin 角色直接通过
 * - 路由 meta.permission 不存在 → 放行（菜单 path 未配置 permission）
 * - 通过 hasRoutePermission 匹配（支持通配符、read/view 等价）
 * - 空权限码用户不再放行，与 router.beforeEach 守卫行为一致
 *
 * @param menuItemPath 菜单项 index（即路由 path，如 '/inventory'）
 * @returns 是否在菜单中显示
 */
function canAccessMenu(menuItemPath: string): boolean {
  // 通过 router.resolve 找到匹配的叶子路由 record
  const resolved = router.resolve(menuItemPath)
  const leafRecord = resolved.matched[resolved.matched.length - 1]
  // P1 4-1 修复（批次 64）：路由不存在 → 保守隐藏（return false）
  // 原实现 return true，菜单 path 配置错误或路由未注册时放行，菜单可见性泄露
  if (!leafRecord) return false
  // P0 4-2 修复：hidden 路由不在菜单显示（详情/编辑/创建等子页面）
  // 必须在 admin 判断之前，否则 admin 仍会看到 hidden 路由
  if (leafRecord.meta?.hidden) return false
  // 以下保持原权限校验逻辑
  // P2 1-12 修复：删除 isAdmin 硬编码绕过，统一走 hasRoutePermission
  // 后端为 system 角色注入 *:* 通配权限，hasRoutePermission 自动放行
  const required = leafRecord.meta?.permission as string | string[] | undefined
  return hasRoutePermission(required, userPermissions.value)
}

/**
 * 批次 6（2026-06-28）：父级子菜单可见性
 *
 * 当子菜单项全部因权限不足被隐藏时，父级 el-sub-menu 也应隐藏，
 * 避免出现"空菜单组"破坏用户体验。每个 key 对应 template 中 el-sub-menu 的 index。
 *
 * TODO(tech-debt) P3 4-7：当前 subMenus 映射为硬编码 path 列表，与 router/index.ts
 * 路由定义存在重复维护风险。后续应改为基于路由表 children 自动派生（与 4-3 侧边栏
 * 动态化一同处理）。当前实现已基于 canAccessMenu 动态计算可见性，功能正常。
 */
const visibleSubMenu = computed<Record<string, boolean>>(() => {
  // 子菜单 index 与其下属菜单项 path 的映射
  const subMenus: Record<string, string[]> = {
    fabric: ['/fabric', '/greige-fabrics', '/product', '/color-cards/list', '/color-cards/issues', '/color-prices/list', '/color-prices/batch-adjust'],
    inventory: ['/inventory', '/warehouse', '/inventory-batch', '/inventory-count', '/inventory-transfer', '/inventory-adjustment', '/logistics'],
    sales: ['/sales', '/sales-returns', '/sales-ext', '/customer', '/customer-credit', '/sales-contract', '/sales-price', '/sales-analysis', '/quotations'],
    purchase: ['/purchase', '/purchase-receipt', '/purchase-ext', '/supplier', '/supplier-evaluation', '/purchase-contract', '/purchase-price', '/purchase-inspection', '/purchase-return'],
    crm: ['/crm', '/crm/pool', '/crm/assignment', '/crm/leads', '/crm/opportunities'],
    production: ['/production', '/bom', '/mrp', '/mrp/history', '/capacity', '/material-shortage', '/scheduling', '/quality', '/scheduling/gantt', '/custom-orders', '/dye-recipe', '/dye-batch'],
    finance: ['/finance', '/ap', '/ar', '/ar-reconciliation', '/finance-report', '/cost', '/budget', '/fund', '/fixed-assets', '/currency', '/financial-analysis', '/assist-accounting', '/account-subject', '/accounting-period', '/voucher', '/trading', '/ar-reconciliation/enhanced', '/bi/sales-analysis'],
    workflow: ['/bpm', '/bpm/definitions', '/bpm/templates', '/bpm/approval', '/business-trace', '/barcode-scanner', '/quality-standards'],
    system: ['/system', '/departments', '/five-dimension', '/data-permission', '/report-templates', '/data-import', '/print-templates', '/api-gateway', '/system-update', '/advanced', '/notification', '/omni-audit', '/system/audit-log', '/system/slow-query', '/security', '/email', '/admin/failover'],
    ai: ['/ai-extend', '/ai-extend/process-optimization', '/ai-extend/quality-prediction'],
  }
  const result: Record<string, boolean> = {}
  for (const [key, paths] of Object.entries(subMenus)) {
    // 子菜单项至少有一个可见时父级才显示
    result[key] = paths.some(p => canAccessMenu(p))
  }
  return result
})

async function handleLogout() {
  await userStore.logout()
  router.push('/login')
}
</script>

<style scoped>
.main-layout {
  height: 100vh;
}
.aside {
  background-color: #304156;
}
.logo {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #263445;
}
.logo h2 {
  color: #fff;
  font-size: 18px;
  margin: 0;
}
.menu {
  border-right: none;
}
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #fff;
  box-shadow: 0 1px 4px rgba(0, 21, 41, 0.08);
}
.header-right .user-info {
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}
.main-content {
  background: #f0f2f5;
  padding: 20px;
}
</style>
