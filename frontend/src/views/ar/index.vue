<!--
  ar/index.vue - 应收管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明：原"上帝组件"已拆分为以下 6 个页签子组件，位于 views/ar/tabs/ 目录。

  | 页签     | 子组件                         | 入口判据（后端在册细键）                          |
  | -------- | ------------------------------ | ------------------------------------------------- |
  | 应收发票 | tabs/InvoiceTab.vue            | ar-invoices:read                                  |
  | 收款管理 | tabs/PaymentTab.vue            | ar-payments:read                                  |
  | 核销管理 | tabs/VerificationTab.vue       | ar-verifications:read                             |
  | 应收对账 | tabs/ReconciliationTab.vue     | ar-reconciliations:read                           |
  | 资金账户 | tabs/FundTab.vue               | fund-management:read                              |
  | 报表中心 | tabs/ArReportTab.vue           | ar-aging / ar-daily / ar-monthly / ar-statistics 四枚任一 |

  本主入口承担：Tab 切换 + 页签级可达性门控 + 公共样式。

  门控口径与应付域完全同形：页签可见性用 v-if 配 hasRoutePermission（与 v-permission
  指令同一判定函数，不另写第二套字符串比较），不用 v-permission 指令本身——指令移除的是
  面板 DOM 节点，而页签标签由 el-tabs 依已注册面板在另一棵子树渲染，用指令会留下
  「标签看得见、点开是空白」的假收敛。
  壳页 /ar 的 meta.permission 是本表六条入口判据的并集，两者由
  tests/unit/ar-shell-permission-lock.test.ts 与 constants/permissions.ts 的
  AR_SHELL_ENTRY_PERMISSIONS 互锁，保证「进得来 ⇒ 至少一个页签可见」，
  也保证页签可见面不宽于后端放行面（点开不是一枚必 403 的请求）。
-->
<template>
  <div class="ar-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane
        v-if="visibleTabNames.includes('invoice')"
        :label="t('arModule.tabs.invoice')"
        name="invoice"
      >
        <InvoiceTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('payment')"
        :label="t('arModule.tabs.payment')"
        name="payment"
        lazy
      >
        <PaymentTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('verification')"
        :label="t('arModule.tabs.verification')"
        name="verification"
        lazy
      >
        <VerificationTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('reconciliation')"
        :label="t('arModule.tabs.reconciliation')"
        name="reconciliation"
      >
        <ReconciliationTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('fund')"
        :label="t('arModule.tabs.fund')"
        name="fund"
      >
        <FundTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('report')"
        :label="t('arModule.tabs.report')"
        name="report"
        lazy
      >
        <ArReportTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { hasRoutePermission } from '@/router';
import { useUserStore } from '@/store/user';
import { AR_REPORT_TAB_PERMISSIONS, PERMISSIONS } from '@/constants/permissions';
import InvoiceTab from './tabs/InvoiceTab.vue';
import PaymentTab from './tabs/PaymentTab.vue';
import VerificationTab from './tabs/VerificationTab.vue';
import ReconciliationTab from './tabs/ReconciliationTab.vue';
import FundTab from './tabs/FundTab.vue';
import ArReportTab from './tabs/ArReportTab.vue';

const { t } = useI18n({ useScope: 'global' });
const userStore = useUserStore();

/** 页签登记：name 与模板里的 el-tab-pane name 一一对应，entry 为该页签的入口权限码 */
const AR_TABS = [
  { name: 'invoice', entry: PERMISSIONS.AR_INVOICE_READ },
  { name: 'payment', entry: PERMISSIONS.AR_PAYMENT_READ },
  { name: 'verification', entry: PERMISSIONS.AR_VERIFICATION_READ },
  { name: 'reconciliation', entry: PERMISSIONS.AR_RECONCILIATION_READ },
  { name: 'fund', entry: PERMISSIONS.FUND_MANAGEMENT_READ },
  { name: 'report', entry: AR_REPORT_TAB_PERMISSIONS },
] as const;

/**
 * 判定口径与 v-permission 指令、路由守卫完全同源（同一个 hasRoutePermission：
 * 支持 *:* 与 resource:* 通配、read 与 view 等价），不在此另写一套字符串比较。
 */
function canReach(entry: string | readonly string[]): boolean {
  // hasRoutePermission 的形参是可变的 string 或 string[]，常量集合按只读类型导出，取副本传入。
  // 判据先用 typeof 收窄：Array.isArray 不能把 readonly string[] 从并集里摘出去，
  // 那样传给形参会留下 readonly 分支的类型错误。
  const code: string | string[] = typeof entry === 'string' ? entry : [...entry];
  return hasRoutePermission(code, userStore.userInfo?.permissions || []);
}

const visibleTabNames = computed(() => AR_TABS.filter(tab => canReach(tab.entry)).map(tab => tab.name));

// 默认停在当前角色第一个可见页签。可见集为空只会出现在「壳页门控与页签门控分叉」时，
// 该分叉由上面登记的静态锁与路由权限防漂移门禁判红，运行期不在此处叠加第二层掩盖。
const activeTab = ref<string>(visibleTabNames.value[0] ?? '');
</script>

<style scoped>
.ar-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
:deep(.page-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
:deep(.page-title) {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
:deep(.filter-card) {
  margin-bottom: 20px;
}
</style>
