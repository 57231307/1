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
        <el-menu-item index="/dashboard">
          <el-icon><HomeFilled /></el-icon>
          <span>仪表盘</span>
        </el-menu-item>

        <el-sub-menu index="fabric">
          <template #title>
            <el-icon><Goods /></el-icon>
            <span>面料管理</span>
          </template>
          <el-menu-item index="/fabric">面料列表</el-menu-item>
          <el-menu-item index="/greige-fabrics">坯布管理</el-menu-item>
          <el-menu-item index="/product">产品管理</el-menu-item>
          <el-menu-item index="/color-cards/list">色卡列表</el-menu-item>
          <el-menu-item index="/color-cards/borrow">色卡借出</el-menu-item>
          <el-menu-item index="/color-prices/list">色号价格</el-menu-item>
          <el-menu-item index="/color-prices/batch-adjust">批量调价</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="inventory">
          <template #title>
            <el-icon><Box /></el-icon>
            <span>库存管理</span>
          </template>
          <el-menu-item index="/inventory">库存列表</el-menu-item>
          <el-menu-item index="/warehouse">仓库管理</el-menu-item>
          <el-menu-item index="/inventory-batch">批次管理</el-menu-item>
          <el-menu-item index="/inventory-count">库存盘点</el-menu-item>
          <el-menu-item index="/inventory-transfer">库存调拨</el-menu-item>
          <el-menu-item index="/inventory-adjustment">库存调整</el-menu-item>
          <el-menu-item index="/logistics">物流管理</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="sales">
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>销售管理</span>
          </template>
          <el-menu-item index="/sales">销售订单</el-menu-item>
          <el-menu-item index="/sales-returns">销售退货</el-menu-item>
          <el-menu-item index="/sales-ext">销售扩展</el-menu-item>
          <el-menu-item index="/customer">客户管理</el-menu-item>
          <el-menu-item index="/customer-credit">客户信用</el-menu-item>
          <el-menu-item index="/sales-contract">销售合同</el-menu-item>
          <el-menu-item index="/sales-price">销售价格</el-menu-item>
          <el-menu-item index="/sales-analysis">销售分析</el-menu-item>
          <el-menu-item index="/quotations">报价单管理</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="purchase">
          <template #title>
            <el-icon><ShoppingCart /></el-icon>
            <span>采购管理</span>
          </template>
          <el-menu-item index="/purchase">采购订单</el-menu-item>
          <el-menu-item index="/purchase-receipt">采购入库</el-menu-item>
          <el-menu-item index="/purchase-ext">采购扩展</el-menu-item>
          <el-menu-item index="/supplier">供应商管理</el-menu-item>
          <el-menu-item index="/supplier-evaluation">供应商评估</el-menu-item>
          <el-menu-item index="/purchase-contract">采购合同</el-menu-item>
          <el-menu-item index="/purchase-price">采购价格</el-menu-item>
          <el-menu-item index="/purchase-inspection">采购检验</el-menu-item>
          <el-menu-item index="/purchase-return">采购退货</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="crm">
          <template #title>
            <el-icon><User /></el-icon>
            <span>客户关系</span>
          </template>
          <el-menu-item index="/crm">CRM 管理</el-menu-item>
          <el-menu-item index="/crm/pool">公海客户池</el-menu-item>
          <el-menu-item index="/crm/assignment">客户分配</el-menu-item>
          <el-menu-item index="/crm/leads">线索管理</el-menu-item>
          <el-menu-item index="/crm/opportunities">商机管理</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="production">
          <template #title>
            <el-icon><Cpu /></el-icon>
            <span>生产管理</span>
          </template>
          <el-menu-item index="/production">生产计划</el-menu-item>
          <el-menu-item index="/bom">BOM 管理</el-menu-item>
          <el-menu-item index="/mrp">MRP 计算</el-menu-item>
          <el-menu-item index="/mrp/history">MRP 历史记录</el-menu-item>
          <el-menu-item index="/capacity">产能分析</el-menu-item>
          <el-menu-item index="/material-shortage">缺料预警</el-menu-item>
          <el-menu-item index="/scheduling">排产管理</el-menu-item>
          <el-menu-item index="/quality">质量管理</el-menu-item>
          <el-menu-item index="/scheduling/gantt">甘特图</el-menu-item>
          <el-menu-item index="/custom-orders">定制订单</el-menu-item>
          <el-menu-item index="/dye-recipe">染色配方</el-menu-item>
          <el-menu-item index="/dye-batch">染色批次</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="finance">
          <template #title>
            <el-icon><Money /></el-icon>
            <span>财务管理</span>
          </template>
          <el-menu-item index="/finance">财务总览</el-menu-item>
          <el-menu-item index="/ap">应付管理</el-menu-item>
          <el-menu-item index="/ar">应收管理</el-menu-item>
          <el-menu-item index="/ar-reconciliation">应收对账</el-menu-item>
          <el-menu-item index="/finance-report">财务报表</el-menu-item>
          <el-menu-item index="/cost">成本归集</el-menu-item>
          <el-menu-item index="/budget">预算管理</el-menu-item>
          <el-menu-item index="/fund">资金管理</el-menu-item>
          <el-menu-item index="/fixed-assets">固定资产</el-menu-item>
          <el-menu-item index="/currency">多币种</el-menu-item>
          <el-menu-item index="/financial-analysis">财务分析</el-menu-item>
          <el-menu-item index="/assist-accounting">辅助核算</el-menu-item>
          <el-menu-item index="/account-subject">会计科目</el-menu-item>
          <el-menu-item index="/accounting-period">会计期间</el-menu-item>
          <el-menu-item index="/voucher">凭证管理</el-menu-item>
          <el-menu-item index="/trading">交易管理</el-menu-item>
          <el-menu-item index="/ar-reconciliation/enhanced">增强版应收对账</el-menu-item>
          <el-menu-item index="/bi/sales-analysis">BI 销售分析</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="workflow">
          <template #title>
            <el-icon><List /></el-icon>
            <span>工作流</span>
          </template>
          <el-menu-item index="/bpm">审批管理</el-menu-item>
          <el-menu-item index="/bpm/definitions">流程定义</el-menu-item>
          <el-menu-item index="/bpm/templates">流程模板</el-menu-item>
          <el-menu-item index="/bpm/approval">审批中心</el-menu-item>
          <el-menu-item index="/business-trace">业务追溯</el-menu-item>
          <el-menu-item index="/barcode-scanner">扫码功能</el-menu-item>
          <el-menu-item index="/quality-standards">质量标准</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="system">
          <template #title>
            <el-icon><Setting /></el-icon>
            <span>系统管理</span>
          </template>
          <el-menu-item index="/system">系统设置</el-menu-item>
          <el-menu-item index="/departments">部门管理</el-menu-item>
          <el-menu-item index="/five-dimension">五维管理</el-menu-item>
          <el-menu-item index="/data-permission">数据权限</el-menu-item>
          <el-menu-item index="/report-templates">报表中心</el-menu-item>
          <el-menu-item index="/data-import">数据导入</el-menu-item>
          <el-menu-item index="/print-templates">打印模板</el-menu-item>
          <el-menu-item index="/api-gateway">API 网关</el-menu-item>
          <el-menu-item index="/system-update">系统更新</el-menu-item>
          <el-menu-item index="/advanced">高级功能</el-menu-item>
          <el-menu-item index="/notification">通知中心</el-menu-item>
          <el-menu-item index="/omni-audit">全量审计</el-menu-item>
          <el-menu-item index="/system/audit-log">审计日志</el-menu-item>
          <el-menu-item index="/system/slow-query">慢查询审计</el-menu-item>
          <el-menu-item index="/security">安全管理</el-menu-item>
          <el-menu-item index="/email">邮件管理</el-menu-item>
          <el-menu-item index="/tenant-billing">租户计费</el-menu-item>
          <el-menu-item index="/admin/failover">主备监控</el-menu-item>
        </el-sub-menu>

        <el-sub-menu index="ai">
          <template #title>
            <el-icon><MagicStick /></el-icon>
            <span>AI 智能</span>
          </template>
          <el-menu-item index="/ai-extend">AI 分析深化</el-menu-item>
          <el-menu-item index="/ai-extend/process-optimization">AI 工艺优化</el-menu-item>
          <el-menu-item index="/ai-extend/quality-prediction">AI 质量预测</el-menu-item>
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

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()

const activeMenu = computed(() => route.path)
const currentTitle = computed(() => (route.meta.title as string) || '')

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
