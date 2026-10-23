<!--
  ap/index.vue - 应付管理主入口（容器组件）
  ----------------------------------------------------------------
  本主入口只承担：页签切换 + 页签级可达性门控 + 公共样式；
  各页签内容位于 views/ap/tabs/ 目录：

  | 页签   | 子组件                          | 入口判据（后端在册细键）              |
  | ------ | ------------------------------- | ------------------------------------- |
  | 应付发票 | tabs/InvoiceTab.vue             | ap-invoices:read                      |
  | 付款管理 | tabs/PaymentTab.vue             | ap-payments:read                      |
  | 请款管理 | tabs/PaymentRequestTab.vue      | ap-payment-requests:read              |
  | 核销管理 | tabs/VerificationTab.vue        | ap-verifications:read                 |
  | 对账管理 | tabs/ReconciliationTab.vue      | ap-reconciliations:read               |
  | 报表中心 | tabs/ApReportTab.vue            | ap-aging / ap-daily / ap-monthly / ap-statistics 四枚任一 |

  门控口径：页签可见性用 v-if 配 hasRoutePermission（与 v-permission 指令同一判定函数），
  不用 v-permission 指令本身——指令移除的是面板 DOM 节点，而页签标签由 el-tabs 依已注册面板
  在另一棵子树渲染，用指令会留下"标签看得见、点开是空白"的假收敛。
  壳页 /ap 的 meta.permission 是本表六条入口判据的并集，两者由
  tests/unit/ap-shell-permission-lock.test.ts 互锁，保证"进得来 ⇒ 至少一个页签可见"。
-->
<template>
  <div class="ap-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane
        v-if="visibleTabNames.includes('invoice')"
        :label="t('apModule.tabs.invoice')"
        name="invoice"
      >
        <InvoiceTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('payment')"
        :label="t('apModule.tabs.payment')"
        name="payment"
      >
        <PaymentTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('paymentRequest')"
        :label="t('apModule.tabs.paymentRequest')"
        name="paymentRequest"
        lazy
      >
        <PaymentRequestTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('verification')"
        :label="t('apModule.tabs.verification')"
        name="verification"
      >
        <VerificationTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('reconciliation')"
        :label="t('apModule.tabs.reconciliation')"
        name="reconciliation"
      >
        <ReconciliationTab />
      </el-tab-pane>
      <el-tab-pane
        v-if="visibleTabNames.includes('report')"
        :label="t('apModule.tabs.report')"
        name="report"
        lazy
      >
        <ApReportTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { hasRoutePermission } from '@/router';
import { useUserStore } from '@/store/user';
import { AP_REPORT_TAB_PERMISSIONS, PERMISSIONS } from '@/constants/permissions';
import InvoiceTab from './tabs/InvoiceTab.vue';
import PaymentTab from './tabs/PaymentTab.vue';
import PaymentRequestTab from './tabs/PaymentRequestTab.vue';
import VerificationTab from './tabs/VerificationTab.vue';
import ReconciliationTab from './tabs/ReconciliationTab.vue';
import ApReportTab from './tabs/ApReportTab.vue';

const { t } = useI18n({ useScope: 'global' });
const userStore = useUserStore();

/** 页签登记：name 与模板里的 el-tab-pane name 一一对应，entry 为该页签的入口权限码 */
const AP_TABS = [
  { name: 'invoice', entry: PERMISSIONS.AP_INVOICE_READ },
  { name: 'payment', entry: PERMISSIONS.AP_PAYMENT_READ },
  { name: 'paymentRequest', entry: PERMISSIONS.AP_PAYMENT_REQUEST_READ },
  { name: 'verification', entry: PERMISSIONS.AP_VERIFICATION_READ },
  { name: 'reconciliation', entry: PERMISSIONS.AP_RECONCILIATION_READ },
  { name: 'report', entry: AP_REPORT_TAB_PERMISSIONS },
] as const;

/**
 * 判定口径与 v-permission 指令、路由守卫完全同源（同一个 hasRoutePermission：
 * 支持 *:* 与 resource:* 通配、read↔view 等价），不在此另写一套字符串比较。
 */
function canReach(entry: string | readonly string[]): boolean {
  // hasRoutePermission 的入参是可变的 string | string[]，常量集合按只读类型导出，取副本传入
  return hasRoutePermission(
    Array.isArray(entry) ? [...entry] : entry,
    userStore.userInfo?.permissions || []
  );
}

const visibleTabNames = computed(() =>
  AP_TABS.filter(tab => canReach(tab.entry)).map(tab => tab.name)
);

// 默认停在当前角色第一个可见页签。可见集为空只会出现在"壳页门控与页签门控分叉"时，
// 该分叉由上面登记的静态锁与路由权限防漂移门禁判红，运行期不在此处叠加第二层掩盖。
const activeTab = ref<string>(visibleTabNames.value[0] ?? '');
</script>

<style scoped>
.ap-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
</style>
