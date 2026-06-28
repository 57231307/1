<template>
  <el-container class="main-layout">
    <el-aside width="220px" class="aside">
      <div class="logo">
        <h2>面料管理</h2>
      </div>
      <el-menu
        :default-active="activeMenu"
        class="menu"
        background-color="#304156"
        text-color="#bfcbd9"
        active-text-color="#409eff"
        router
      >
        <el-menu-item v-if="canAccessMenu('/dashboard')" index="/dashboard">
          <el-icon><HomeFilled /></el-icon>
          <span>仪表盘</span>
        </el-menu-item>

        <el-sub-menu v-if="visibleSubMenu.fabric" index="fabric">
          <template #title>
            <el-icon><Goods /></el-icon>
            <span>面料管理</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/fabric')" index="/fabric">面料列表</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/greige-fabrics')" index="/greige-fabrics">坯布管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/product')" index="/product">产品管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/color-cards/list')" index="/color-cards/list">色卡列表</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/color-cards/borrow')" index="/color-cards/borrow">色卡借出</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/color-prices/list')" index="/color-prices/list">色号价格</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/color-prices/batch-adjust')" index="/color-prices/batch-adjust">批量调价</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.inventory" index="inventory">
          <template #title>
            <el-icon><Box /></el-icon>
            <span>库存管理</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/inventory')" index="/inventory">库存列表</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/warehouse')" index="/warehouse">仓库管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/inventory-batch')" index="/inventory-batch">批次管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/inventory-count')" index="/inventory-count">库存盘点</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/inventory-transfer')" index="/inventory-transfer">库存调拨</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/inventory-adjustment')" index="/inventory-adjustment">库存调整</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/logistics')" index="/logistics">物流管理</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.sales" index="sales">
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>销售管理</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/sales')" index="/sales">销售订单</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/sales-returns')" index="/sales-returns">销售退货</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/sales-ext')" index="/sales-ext">销售扩展</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/customer')" index="/customer">客户管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/customer-credit')" index="/customer-credit">客户信用</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/sales-contract')" index="/sales-contract">销售合同</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/sales-price')" index="/sales-price">销售价格</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/sales-analysis')" index="/sales-analysis">销售分析</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/quotations')" index="/quotations">报价单管理</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.purchase" index="purchase">
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>采购管理</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/purchase')" index="/purchase">采购订单</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/purchase-receipt')" index="/purchase-receipt">采购入库</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/purchase-ext')" index="/purchase-ext">采购扩展</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/supplier')" index="/supplier">供应商管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/supplier-evaluation')" index="/supplier-evaluation">供应商评估</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/purchase-contract')" index="/purchase-contract">采购合同</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/purchase-price')" index="/purchase-price">采购价格</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/purchase-inspection')" index="/purchase-inspection">采购检验</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/purchase-return')" index="/purchase-return">采购退货</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.crm" index="crm">
          <template #title>
            <el-icon><User /></el-icon>
            <span>客户关系</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/crm')" index="/crm">CRM 管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/crm/pool')" index="/crm/pool">公海客户池</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/crm/assignment')" index="/crm/assignment">客户分配</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/crm/leads')" index="/crm/leads">线索管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/crm/opportunities')" index="/crm/opportunities">商机管理</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.production" index="production">
          <template #title>
            <el-icon><Cpu /></el-icon>
            <span>生产管理</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/production')" index="/production">生产计划</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/bom')" index="/bom">BOM 管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/mrp')" index="/mrp">MRP 计算</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/mrp/history')" index="/mrp/history">MRP 历史记录</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/capacity')" index="/capacity">产能分析</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/material-shortage')" index="/material-shortage">缺料预警</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/scheduling')" index="/scheduling">排产管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/quality')" index="/quality">质量管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/scheduling/gantt')" index="/scheduling/gantt">甘特图</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/custom-orders')" index="/custom-orders">定制订单</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/dye-recipe')" index="/dye-recipe">染色配方</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/dye-batch')" index="/dye-batch">染色批次</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.finance" index="finance">
          <template #title>
            <el-icon><Money /></el-icon>
            <span>财务管理</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/finance')" index="/finance">财务总览</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ap')" index="/ap">应付管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ar')" index="/ar">应收管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ar-reconciliation')" index="/ar-reconciliation">应收对账</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/finance-report')" index="/finance-report">财务报表</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/cost')" index="/cost">成本归集</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/budget')" index="/budget">预算管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/fund')" index="/fund">资金管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/fixed-assets')" index="/fixed-assets">固定资产</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/currency')" index="/currency">多币种</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/financial-analysis')" index="/financial-analysis">财务分析</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/assist-accounting')" index="/assist-accounting">辅助核算</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/account-subject')" index="/account-subject">会计科目</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/accounting-period')" index="/accounting-period">会计期间</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/voucher')" index="/voucher">凭证管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/trading')" index="/trading">交易管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ar-reconciliation/enhanced')" index="/ar-reconciliation/enhanced">增强版应收对账</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/bi/sales-analysis')" index="/bi/sales-analysis">BI 销售分析</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.workflow" index="workflow">
          <template #title>
            <el-icon><List /></el-icon>
            <span>工作流</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/bpm')" index="/bpm">审批管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/bpm/definitions')" index="/bpm/definitions">流程定义</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/bpm/templates')" index="/bpm/templates">流程模板</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/bpm/approval')" index="/bpm/approval">审批中心</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/business-trace')" index="/business-trace">业务追溯</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/barcode-scanner')" index="/barcode-scanner">扫码功能</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/quality-standards')" index="/quality-standards">质量标准</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.system" index="system">
          <template #title>
            <el-icon><Setting /></el-icon>
            <span>系统管理</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/system')" index="/system">系统设置</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/departments')" index="/departments">部门管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/five-dimension')" index="/five-dimension">五维管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/data-permission')" index="/data-permission">数据权限</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/report-templates')" index="/report-templates">报表中心</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/data-import')" index="/data-import">数据导入</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/print-templates')" index="/print-templates">打印模板</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/api-gateway')" index="/api-gateway">API 网关</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/system-update')" index="/system-update">系统更新</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/advanced')" index="/advanced">高级功能</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/notification')" index="/notification">通知中心</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/omni-audit')" index="/omni-audit">全量审计</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/system/audit-log')" index="/system/audit-log">审计日志</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/system/slow-query')" index="/system/slow-query">慢查询审计</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/security')" index="/security">安全管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/email')" index="/email">邮件管理</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/admin/failover')" index="/admin/failover">主备监控</el-menu-item>
        </el-sub-menu>

        <el-sub-menu v-if="visibleSubMenu.ai" index="ai">
          <template #title>
            <el-icon><MagicStick /></el-icon>
            <span>AI 智能</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/ai-extend')" index="/ai-extend">AI 分析深化</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ai-extend/process-optimization')" index="/ai-extend/process-optimization">AI 工艺优化</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ai-extend/quality-prediction')" index="/ai-extend/quality-prediction">AI 质量预测</el-menu-item>
        </el-sub-menu>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="header">
        <div class="header-left">
          <el-breadcrumb separator="/">
            <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
            <el-breadcrumb-item>{{ currentTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <el-dropdown>
            <span class="user-info">
              {{ userStore.userInfo?.username || '用户' }}
              <el-icon><ArrowDown /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="$router.push('/system/profile')"
                  >个人信息</el-dropdown-item
                >
                <el-dropdown-item divided @click="handleLogout">退出登录</el-dropdown-item>
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
import { computed } from 'vue'
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
const userPermissions = computed<string[]>(() => userStore.userInfo?.permissions || [])
const isAdmin = computed<boolean>(() => userStore.userInfo?.role_name === 'admin')
// 与守卫一致：用户未配置任何权限码（permissions 为空数组）时放行
const bypassByEmptyPerms = computed<boolean>(() => userPermissions.value.length === 0)

/**
 * 批次 6（2026-06-28）：菜单项可见性判定
 *
 * 与 router.beforeEach 守卫一致的宽松匹配规则：
 * 1. admin 角色直接通过
 * 2. 用户未配置任何权限码（permissions 为空）→ 放行
 * 3. 路由 meta.permission 不存在 → 放行（菜单 path 未配置 permission）
 * 4. 通过 hasRoutePermission 匹配（支持通配符、read/view 等价）
 *
 * @param menuItemPath 菜单项 index（即路由 path，如 '/inventory'）
 * @returns 是否在菜单中显示
 */
function canAccessMenu(menuItemPath: string): boolean {
  if (isAdmin.value || bypassByEmptyPerms.value) return true
  // 通过 router.resolve 找到匹配的叶子路由 record
  const resolved = router.resolve(menuItemPath)
  const leafRecord = resolved.matched[resolved.matched.length - 1]
  // 路由不存在或 meta.permission 未配置 → 放行（避免菜单异常消失）
  if (!leafRecord) return true
  const required = leafRecord.meta?.permission as string | string[] | undefined
  return hasRoutePermission(required, userPermissions.value)
}

/**
 * 批次 6（2026-06-28）：父级子菜单可见性
 *
 * 当子菜单项全部因权限不足被隐藏时，父级 el-sub-menu 也应隐藏，
 * 避免出现"空菜单组"破坏用户体验。每个 key 对应 template 中 el-sub-menu 的 index。
 */
const visibleSubMenu = computed<Record<string, boolean>>(() => {
  // 子菜单 index 与其下属菜单项 path 的映射
  const subMenus: Record<string, string[]> = {
    fabric: ['/fabric', '/greige-fabrics', '/product', '/color-cards/list', '/color-cards/borrow', '/color-prices/list', '/color-prices/batch-adjust'],
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
