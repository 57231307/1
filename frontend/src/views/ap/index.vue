<!--
  ap/index.vue - 应付管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-1）：
  原 1035 行"上帝组件"已拆分为以下 4 个独立 Tab 子组件，
  位于 views/ap/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 应付发票  | tabs/InvoiceTab.vue                 |
  | 付款管理  | tabs/PaymentTab.vue                 |
  | 核销管理  | tabs/VerificationTab.vue            |
  | 对账管理  | tabs/ReconciliationTab.vue          |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="ap-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane label="应付发票" name="invoice">
        <InvoiceTab />
      </el-tab-pane>
      <el-tab-pane label="付款管理" name="payment">
        <PaymentTab />
      </el-tab-pane>
      <el-tab-pane label="核销管理" name="verification">
        <VerificationTab />
      </el-tab-pane>
      <el-tab-pane label="对账管理" name="reconciliation">
        <ReconciliationTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import InvoiceTab from './tabs/InvoiceTab.vue'
import PaymentTab from './tabs/PaymentTab.vue'
import VerificationTab from './tabs/VerificationTab.vue'
import ReconciliationTab from './tabs/ReconciliationTab.vue'

const activeTab = ref('invoice')
</script>

<style scoped>
.ap-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
</style>
