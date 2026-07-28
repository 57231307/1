<template>
  <el-container class="main-layout" :aria-label="t('layout.main.pageAriaLabel')">
    <!-- V15 P1-20-2 响应式侧边栏：桌面 el-aside 固定 / 移动 el-drawer 抽屉化 -->
    <component
      :is="isMobile ? ElDrawer : ElAside"
      v-bind="sidebarAttrs"
      :class="isMobile ? 'mobile-sidebar' : 'aside'"
    >
      <div class="logo">
        <h2>{{ t('layout.brand') }}</h2>
      </div>
      <el-menu
        :default-active="activeMenu"
        class="menu"
        background-color="#304156"
        text-color="#bfcbd9"
        active-text-color="#409eff"
        router
        role="menubar"
        :aria-label="t('layout.menuAriaLabel')"
        @open="handleMenuOpen"
        @close="handleMenuClose"
      >
        <el-menu-item v-if="canAccessMenu('/dashboard')" role="menuitem" index="/dashboard">
          <el-icon><HomeFilled /></el-icon>
          <span>{{ t('layout.menu.dashboard') }}</span>
        </el-menu-item>

        <el-sub-menu
          v-if="visibleSubMenu.fabric"
          index="fabric"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('fabric')"
        >
          <template #title>
            <el-icon><Goods /></el-icon>
            <span>{{ t('layout.menu.fabric') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/fabric')" role="menuitem" index="/fabric">{{
            t('layout.menu.fabricList')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/greige-fabrics')"
            role="menuitem"
            index="/greige-fabrics"
            >{{ t('layout.menu.greigeFabrics') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/product')" role="menuitem" index="/product">{{
            t('layout.menu.product')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/color-cards/list')"
            role="menuitem"
            index="/color-cards/list"
            >{{ t('layout.menu.colorCardsList') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/color-cards/issues')"
            role="menuitem"
            index="/color-cards/issues"
            >{{ t('layout.menu.colorCardsIssue') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/color-prices/list')"
            role="menuitem"
            index="/color-prices/list"
            >{{ t('layout.menu.colorPricesList') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/color-prices/batch-adjust')"
            role="menuitem"
            index="/color-prices/batch-adjust"
            >{{ t('layout.menu.colorPricesBatchAdjust') }}</el-menu-item
          >
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.inventory"
          index="inventory"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('inventory')"
        >
          <template #title>
            <el-icon><Box /></el-icon>
            <span>{{ t('layout.menu.inventory') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/inventory')" role="menuitem" index="/inventory">{{
            t('layout.menu.inventoryList')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/warehouse')" role="menuitem" index="/warehouse">{{
            t('layout.menu.warehouse')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/inventory-batch')"
            role="menuitem"
            index="/inventory-batch"
            >{{ t('layout.menu.inventoryBatch') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/inventory-count')"
            role="menuitem"
            index="/inventory-count"
            >{{ t('layout.menu.inventoryCount') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/inventory-transfer')"
            role="menuitem"
            index="/inventory-transfer"
            >{{ t('layout.menu.inventoryTransfer') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/inventory-adjustment')"
            role="menuitem"
            index="/inventory-adjustment"
            >{{ t('layout.menu.inventoryAdjustment') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/logistics')" role="menuitem" index="/logistics">{{
            t('layout.menu.logistics')
          }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.sales"
          index="sales"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('sales')"
        >
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>{{ t('layout.menu.sales') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/sales')" role="menuitem" index="/sales">{{
            t('layout.menu.salesOrder')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/sales-returns')"
            role="menuitem"
            index="/sales-returns"
            >{{ t('layout.menu.salesReturns') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/sales-ext')" role="menuitem" index="/sales-ext">{{
            t('layout.menu.salesExt')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/customer')" role="menuitem" index="/customer">{{
            t('layout.menu.customer')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/customer-credit')"
            role="menuitem"
            index="/customer-credit"
            >{{ t('layout.menu.customerCredit') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/sales-contract')"
            role="menuitem"
            index="/sales-contract"
            >{{ t('layout.menu.salesContract') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/sales-price')" role="menuitem" index="/sales-price">{{
            t('layout.menu.salesPrice')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/sales-analysis')"
            role="menuitem"
            index="/sales-analysis"
            >{{ t('layout.menu.salesAnalysis') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/quotations')" role="menuitem" index="/quotations">{{
            t('layout.menu.quotations')
          }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.purchase"
          index="purchase"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('purchase')"
        >
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>{{ t('layout.menu.purchase') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/purchase')" role="menuitem" index="/purchase">{{
            t('layout.menu.purchaseOrder')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/purchase-receipt')"
            role="menuitem"
            index="/purchase-receipt"
            >{{ t('layout.menu.purchaseReceipt') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/purchase-ext')"
            role="menuitem"
            index="/purchase-ext"
            >{{ t('layout.menu.purchaseExt') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/supplier')" role="menuitem" index="/supplier">{{
            t('layout.menu.supplier')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/supplier-evaluation')"
            role="menuitem"
            index="/supplier-evaluation"
            >{{ t('layout.menu.supplierEvaluation') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/purchase-contract')"
            role="menuitem"
            index="/purchase-contract"
            >{{ t('layout.menu.purchaseContract') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/purchase-price')"
            role="menuitem"
            index="/purchase-price"
            >{{ t('layout.menu.purchasePrice') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/purchase-inspection')"
            role="menuitem"
            index="/purchase-inspection"
            >{{ t('layout.menu.purchaseInspection') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/purchase-return')"
            role="menuitem"
            index="/purchase-return"
            >{{ t('layout.menu.purchaseReturn') }}</el-menu-item
          >
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.crm"
          index="crm"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('crm')"
        >
          <template #title>
            <el-icon><User /></el-icon>
            <span>{{ t('layout.menu.crm') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/crm')" role="menuitem" index="/crm">{{
            t('layout.menu.crmManagement')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/crm/pool')" role="menuitem" index="/crm/pool">{{
            t('layout.menu.crmPool')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/crm/assignment')"
            role="menuitem"
            index="/crm/assignment"
            >{{ t('layout.menu.crmAssignment') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/crm/leads')" role="menuitem" index="/crm/leads">{{
            t('layout.menu.crmLeads')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/crm/opportunities')"
            role="menuitem"
            index="/crm/opportunities"
            >{{ t('layout.menu.crmOpportunities') }}</el-menu-item
          >
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.production"
          index="production"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('production')"
        >
          <template #title>
            <el-icon><Cpu /></el-icon>
            <span>{{ t('layout.menu.production') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/production')" role="menuitem" index="/production">{{
            t('layout.menu.productionPlan')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/bom')" role="menuitem" index="/bom">{{
            t('layout.menu.bom')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/mrp')" role="menuitem" index="/mrp">{{
            t('layout.menu.mrp')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/mrp/history')" role="menuitem" index="/mrp/history">{{
            t('layout.menu.mrpHistory')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/capacity')" role="menuitem" index="/capacity">{{
            t('layout.menu.capacity')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/material-shortage')"
            role="menuitem"
            index="/material-shortage"
            >{{ t('layout.menu.materialShortage') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/scheduling')" role="menuitem" index="/scheduling">{{
            t('layout.menu.scheduling')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/quality')" role="menuitem" index="/quality">{{
            t('layout.menu.quality')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/scheduling/gantt')"
            role="menuitem"
            index="/scheduling/gantt"
            >{{ t('layout.menu.gantt') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/custom-orders')"
            role="menuitem"
            index="/custom-orders"
            >{{ t('layout.menu.customOrders') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/dye-recipe')" role="menuitem" index="/dye-recipe">{{
            t('layout.menu.dyeRecipe')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/dye-batch')" role="menuitem" index="/dye-batch">{{
            t('layout.menu.dyeBatch')
          }}</el-menu-item>
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.finance"
          index="finance"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('finance')"
        >
          <template #title>
            <el-icon><Money /></el-icon>
            <span>{{ t('layout.menu.finance') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/finance')" role="menuitem" index="/finance">{{
            t('layout.menu.financeOverview')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ap')" role="menuitem" index="/ap">{{
            t('layout.menu.ap')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/ar')" role="menuitem" index="/ar">{{
            t('layout.menu.ar')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/ar-reconciliation')"
            role="menuitem"
            index="/ar-reconciliation"
            >{{ t('layout.menu.arReconciliation') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/finance-report')"
            role="menuitem"
            index="/finance-report"
            >{{ t('layout.menu.financeReport') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/cost')" role="menuitem" index="/cost">{{
            t('layout.menu.cost')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/budget')" role="menuitem" index="/budget">{{
            t('layout.menu.budget')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/fund')" role="menuitem" index="/fund">{{
            t('layout.menu.fund')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/fixed-assets')"
            role="menuitem"
            index="/fixed-assets"
            >{{ t('layout.menu.fixedAssets') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/currency')" role="menuitem" index="/currency">{{
            t('layout.menu.currency')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/financial-analysis')"
            role="menuitem"
            index="/financial-analysis"
            >{{ t('layout.menu.financialAnalysis') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/assist-accounting')"
            role="menuitem"
            index="/assist-accounting"
            >{{ t('layout.menu.assistAccounting') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/account-subject')"
            role="menuitem"
            index="/account-subject"
            >{{ t('layout.menu.accountSubject') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/accounting-period')"
            role="menuitem"
            index="/accounting-period"
            >{{ t('layout.menu.accountingPeriod') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/voucher')" role="menuitem" index="/voucher">{{
            t('layout.menu.voucher')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/trading')" role="menuitem" index="/trading">{{
            t('layout.menu.trading')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/ar-reconciliation/enhanced')"
            role="menuitem"
            index="/ar-reconciliation/enhanced"
            >{{ t('layout.menu.arReconciliationEnhanced') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/bi/sales-analysis')"
            role="menuitem"
            index="/bi/sales-analysis"
            >{{ t('layout.menu.biSalesAnalysis') }}</el-menu-item
          >
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.workflow"
          index="workflow"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('workflow')"
        >
          <template #title>
            <el-icon><List /></el-icon>
            <span>{{ t('layout.menu.workflow') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/bpm')" role="menuitem" index="/bpm">{{
            t('layout.menu.bpm')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/bpm/definitions')"
            role="menuitem"
            index="/bpm/definitions"
            >{{ t('layout.menu.bpmDefinitions') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/bpm/templates')"
            role="menuitem"
            index="/bpm/templates"
            >{{ t('layout.menu.bpmTemplates') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/bpm/approval')"
            role="menuitem"
            index="/bpm/approval"
            >{{ t('layout.menu.bpmApproval') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/business-trace')"
            role="menuitem"
            index="/business-trace"
            >{{ t('layout.menu.businessTrace') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/barcode-scanner')"
            role="menuitem"
            index="/barcode-scanner"
            >{{ t('layout.menu.barcodeScanner') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/quality-standards')"
            role="menuitem"
            index="/quality-standards"
            >{{ t('layout.menu.qualityStandards') }}</el-menu-item
          >
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.system"
          index="system"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('system')"
        >
          <template #title>
            <el-icon><Setting /></el-icon>
            <span>{{ t('layout.menu.system') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/system')" role="menuitem" index="/system">{{
            t('layout.menu.systemSettings')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/departments')" role="menuitem" index="/departments">{{
            t('layout.menu.departments')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/five-dimension')"
            role="menuitem"
            index="/five-dimension"
            >{{ t('layout.menu.fiveDimension') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/data-permission')"
            role="menuitem"
            index="/data-permission"
            >{{ t('layout.menu.dataPermission') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/report-templates')"
            role="menuitem"
            index="/report-templates"
            >{{ t('layout.menu.reportTemplates') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/data-import')" role="menuitem" index="/data-import">{{
            t('layout.menu.dataImport')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/print-templates')"
            role="menuitem"
            index="/print-templates"
            >{{ t('layout.menu.printTemplates') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/api-gateway')" role="menuitem" index="/api-gateway">{{
            t('layout.menu.apiGateway')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/system-update')"
            role="menuitem"
            index="/system-update"
            >{{ t('layout.menu.systemUpdate') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/advanced')" role="menuitem" index="/advanced">{{
            t('layout.menu.advanced')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/notification')"
            role="menuitem"
            index="/notification"
            >{{ t('layout.menu.notification') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/omni-audit')" role="menuitem" index="/omni-audit">{{
            t('layout.menu.omniAudit')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/system/audit-log')"
            role="menuitem"
            index="/system/audit-log"
            >{{ t('layout.menu.auditLog') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/system/slow-query')"
            role="menuitem"
            index="/system/slow-query"
            >{{ t('layout.menu.slowQuery') }}</el-menu-item
          >
          <el-menu-item v-if="canAccessMenu('/security')" role="menuitem" index="/security">{{
            t('layout.menu.security')
          }}</el-menu-item>
          <el-menu-item v-if="canAccessMenu('/email')" role="menuitem" index="/email">{{
            t('layout.menu.email')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/admin/failover')"
            role="menuitem"
            index="/admin/failover"
            >{{ t('layout.menu.failover') }}</el-menu-item
          >
        </el-sub-menu>

        <el-sub-menu
          v-if="visibleSubMenu.ai"
          index="ai"
          role="menuitem"
          aria-haspopup="true"
          :aria-expanded="openedMenus.includes('ai')"
        >
          <template #title>
            <el-icon><MagicStick /></el-icon>
            <span>{{ t('layout.menu.ai') }}</span>
          </template>
          <el-menu-item v-if="canAccessMenu('/ai-extend')" role="menuitem" index="/ai-extend">{{
            t('layout.menu.aiExtend')
          }}</el-menu-item>
          <el-menu-item
            v-if="canAccessMenu('/ai-extend/process-optimization')"
            role="menuitem"
            index="/ai-extend/process-optimization"
            >{{ t('layout.menu.aiProcessOptimization') }}</el-menu-item
          >
          <el-menu-item
            v-if="canAccessMenu('/ai-extend/quality-prediction')"
            role="menuitem"
            index="/ai-extend/quality-prediction"
            >{{ t('layout.menu.aiQualityPrediction') }}</el-menu-item
          >
        </el-sub-menu>
      </el-menu>
    </component>

    <el-container>
      <el-header class="header">
        <div class="header-left">
          <!-- V15 P1-20-2 移动端汉堡按钮（触屏尺寸 ≥ 44px，WCAG 2.5.5） -->
          <el-button
            v-if="isMobile"
            class="hamburger-btn"
            size="large"
            :aria-label="t('layout.main.toggleSidebarAriaLabel')"
            :title="t('layout.main.toggleSidebar')"
            text
            @click="openDrawer"
          >
            <el-icon :size="22"><Expand /></el-icon>
          </el-button>
          <el-breadcrumb separator="/" :aria-label="t('layout.main.breadcrumbAriaLabel')">
            <el-breadcrumb-item :to="{ path: '/' }">{{
              t('layout.breadcrumb.home')
            }}</el-breadcrumb-item>
            <el-breadcrumb-item>{{ currentTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <el-dropdown :aria-label="t('layout.main.userMenuAriaLabel')">
            <span class="user-info" role="button" tabindex="0">
              {{ userStore.userInfo?.username || t('layout.user.defaultName') }}
              <el-icon><ArrowDown /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item @click="$router.push('/system/profile')">{{
                  t('layout.user.profile')
                }}</el-dropdown-item>
                <el-dropdown-item divided @click="handleLogout">{{
                  t('layout.user.logout')
                }}</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <el-main class="main-content">
        <router-view v-slot="{ Component, route }">
          <ErrorBoundary :key="route.path">
            <keep-alive :include="cachedViewNames">
              <component :is="Component" :key="route.path" />
            </keep-alive>
          </ErrorBoundary>
        </router-view>
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useI18n } from 'vue-i18n';
// V15 P1-20-2 响应式侧边栏：动态组件切换需要显式导入 ElDrawer/ElAside
import { ElDrawer, ElAside } from 'element-plus';
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
  Expand,
} from '@element-plus/icons-vue';
import { useUserStore } from '@/store/user';
import ErrorBoundary from '@/components/ErrorBoundary.vue';
// 批次 6 修复（2026-06-28）：MainLayout 菜单按 permission 过滤（审计 #8 完整修复）
// 复用 router 守卫同款宽松匹配函数，保证菜单可见性与路由可达性一致。
import { hasRoutePermission } from '@/router';
// V15 P1-20-2 响应式断点 composable（md 以下视为移动端，侧边栏抽屉化）
import { useBreakpoint } from '@/composables/useBreakpoint';

const { t } = useI18n({ useScope: 'global' });
const route = useRoute();
const router = useRouter();
const userStore = useUserStore();

// V15 P1-20-2 响应式断点：md 以下（< 992px）视为移动端，侧边栏改用 el-drawer 抽屉化
const { isMobile } = useBreakpoint();
// 移动端抽屉可见性状态
const drawerVisible = ref(false);

const activeMenu = computed(() => route.path);
const currentTitle = computed(() => (route.meta.title as string) || '');

// V15 P1-20-14 keep-alive 状态保留：缓存高频切换页面（列表页 + 工作台）
// 仅缓存需要保留状态（搜索条件/分页/滚动位置）的页面，详情/编辑页不缓存
const cachedViewNames = computed<string[]>(() => [
  'Dashboard',
  'InventoryList',
  'SalesList',
  'PurchaseList',
  'CustomerList',
  'SupplierList',
  'FinanceOverview',
]);

// 批次 6：用户权限与角色响应式派生
// 批次 22 v5 P0-5：permissions 为 readonly string[]，computed 类型同步
const userPermissions = computed<readonly string[]>(() => userStore.userInfo?.permissions || []);
// P2 1-12 修复：删除 isAdmin computed（role_name === 'admin' 硬编码），统一走 *:* 通配权限

// P0 4-3 修复：维护子菜单展开状态供 aria-expanded 使用（WCAG 无障碍）
const openedMenus = ref<string[]>([]);
const handleMenuOpen = (index: string) => {
  if (!openedMenus.value.includes(index)) {
    openedMenus.value.push(index);
  }
};
const handleMenuClose = (index: string) => {
  openedMenus.value = openedMenus.value.filter(i => i !== index);
};

/**
 * V15 P1-20-2 侧边栏动态属性
 *
 * 桌面端（md 及以上）：使用 ElAside 固定侧边栏，width=220px
 * 移动端（md 以下）：使用 ElDrawer 抽屉化，direction=ltr，size=260px，
 *   withHeader=false（logo 在内容中渲染保持品牌一致），modal=true（点击遮罩关闭），
 *   appendToBody=true（避免 z-index 层级问题）
 */
const sidebarAttrs = computed<Record<string, unknown>>(() => {
  if (isMobile.value) {
    return {
      modelValue: drawerVisible.value,
      'onUpdate:modelValue': (v: boolean) => {
        drawerVisible.value = v;
      },
      title: t('layout.main.sidebarDrawerTitle'),
      direction: 'ltr' as const,
      size: '260px',
      withHeader: false,
      modal: true,
      appendToBody: true,
      destroyOnClose: false,
      closeOnPressEscape: true,
    };
  }
  return {
    width: '220px',
  };
});

// V15 P1-20-2 打开移动端抽屉
const openDrawer = () => {
  drawerVisible.value = true;
};

// V15 P1-20-2 路由切换时关闭移动端抽屉（点击菜单项导航后自动收起）
watch(
  () => route.path,
  () => {
    if (isMobile.value && drawerVisible.value) {
      drawerVisible.value = false;
    }
  }
);

// V15 P1-20-2 切换到桌面端时关闭抽屉（避免从移动端放大后抽屉残留）
watch(isMobile, mobile => {
  if (!mobile && drawerVisible.value) {
    drawerVisible.value = false;
  }
});

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
  const resolved = router.resolve(menuItemPath);
  const leafRecord = resolved.matched[resolved.matched.length - 1];
  // P1 4-1 修复（批次 64）：路由不存在 → 保守隐藏（return false）
  // 原实现 return true，菜单 path 配置错误或路由未注册时放行，菜单可见性泄露
  if (!leafRecord) return false;
  // P0 4-2 修复：hidden 路由不在菜单显示（详情/编辑/创建等子页面）
  // 必须在 admin 判断之前，否则 admin 仍会看到 hidden 路由
  if (leafRecord.meta?.hidden) return false;
  // 以下保持原权限校验逻辑
  // P2 1-12 修复：删除 isAdmin 硬编码绕过，统一走 hasRoutePermission
  // 后端为 system 角色注入 *:* 通配权限，hasRoutePermission 自动放行
  const required = leafRecord.meta?.permission as string | string[] | undefined;
  return hasRoutePermission(required, userPermissions.value);
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
    fabric: [
      '/fabric',
      '/greige-fabrics',
      '/product',
      '/color-cards/list',
      '/color-cards/issues',
      '/color-prices/list',
      '/color-prices/batch-adjust',
    ],
    inventory: [
      '/inventory',
      '/warehouse',
      '/inventory-batch',
      '/inventory-count',
      '/inventory-transfer',
      '/inventory-adjustment',
      '/logistics',
    ],
    sales: [
      '/sales',
      '/sales-returns',
      '/sales-ext',
      '/customer',
      '/customer-credit',
      '/sales-contract',
      '/sales-price',
      '/sales-analysis',
      '/quotations',
    ],
    purchase: [
      '/purchase',
      '/purchase-receipt',
      '/purchase-ext',
      '/supplier',
      '/supplier-evaluation',
      '/purchase-contract',
      '/purchase-price',
      '/purchase-inspection',
      '/purchase-return',
    ],
    crm: ['/crm', '/crm/pool', '/crm/assignment', '/crm/leads', '/crm/opportunities'],
    production: [
      '/production',
      '/bom',
      '/mrp',
      '/mrp/history',
      '/capacity',
      '/material-shortage',
      '/scheduling',
      '/quality',
      '/scheduling/gantt',
      '/custom-orders',
      '/dye-recipe',
      '/dye-batch',
    ],
    finance: [
      '/finance',
      '/ap',
      '/ar',
      '/ar-reconciliation',
      '/finance-report',
      '/cost',
      '/budget',
      '/fund',
      '/fixed-assets',
      '/currency',
      '/financial-analysis',
      '/assist-accounting',
      '/account-subject',
      '/accounting-period',
      '/voucher',
      '/trading',
      '/ar-reconciliation/enhanced',
      '/bi/sales-analysis',
    ],
    workflow: [
      '/bpm',
      '/bpm/definitions',
      '/bpm/templates',
      '/bpm/approval',
      '/business-trace',
      '/barcode-scanner',
      '/quality-standards',
    ],
    system: [
      '/system',
      '/departments',
      '/five-dimension',
      '/data-permission',
      '/report-templates',
      '/data-import',
      '/print-templates',
      '/api-gateway',
      '/system-update',
      '/advanced',
      '/notification',
      '/omni-audit',
      '/system/audit-log',
      '/system/slow-query',
      '/security',
      '/email',
      '/admin/failover',
    ],
    ai: ['/ai-extend', '/ai-extend/process-optimization', '/ai-extend/quality-prediction'],
  };
  const result: Record<string, boolean> = {};
  for (const [key, paths] of Object.entries(subMenus)) {
    // 子菜单项至少有一个可见时父级才显示
    result[key] = paths.some(p => canAccessMenu(p));
  }
  return result;
});

async function handleLogout() {
  await userStore.logout();
  router.push('/login');
}
</script>

<style scoped>
/* V15 P1-20-15 使用 CSS 变量替代硬编码颜色，支持主题切换与暗黑模式 */
.main-layout {
  --layout-aside-bg: #304156;
  --layout-aside-logo-bg: #263445;
  --layout-header-bg: #ffffff;
  --layout-content-bg: #f0f2f5;
  --layout-header-shadow: rgba(0, 21, 41, 0.08);
  --layout-text-light: #ffffff;
  --layout-shadow-blur: 4px;
  height: 100vh;
}
.aside {
  background-color: var(--layout-aside-bg);
}
.logo {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--layout-aside-logo-bg);
}
.logo h2 {
  color: var(--layout-text-light);
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
  background: var(--layout-header-bg);
  box-shadow: 0 1px var(--layout-shadow-blur) var(--layout-header-shadow);
}
.header-left {
  display: flex;
  align-items: center;
  gap: 8px;
}
.header-right .user-info {
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 4px;
}
.main-content {
  background: var(--layout-content-bg);
  padding: 20px;
}

/* V15 P1-20-2 移动端汉堡按钮：触屏目标尺寸 ≥ 44x44px（WCAG 2.5.5 Target Size） */
.hamburger-btn {
  min-height: 44px;
  min-width: 44px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

/* V15 P1-20-2 移动端抽屉内部样式：与桌面 aside 背景色保持一致 */
.mobile-sidebar :deep(.el-drawer__body) {
  padding: 0;
  background-color: var(--layout-aside-bg);
  overflow-y: auto;
}

/* V15 P1-20-2 移动端响应式适配（max-width: 991px，与 md 断点一致） */
@media (max-width: 991px) {
  /* 移动端主内容区减小内边距，释放更多内容空间 */
  .main-content {
    padding: 12px;
  }

  /* 移动端头部用户菜单触屏尺寸 ≥ 44px */
  .header-right .user-info {
    min-height: 44px;
    padding: 8px 12px;
  }

  /* 移动端面包屑简化（减少水平占用） */
  .header-left {
    gap: 4px;
  }
}

/* V15 P1-20-2 触屏按钮全局最小尺寸（移动端所有 el-button 至少 44x44px） */
@media (max-width: 991px) {
  .header :deep(.el-button) {
    min-height: 44px;
    min-width: 44px;
  }
}
</style>
