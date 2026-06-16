<!--
  ar/index.vue - 应收管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-2）：
  原 967 行"上帝组件"已拆分为以下 3 个独立 Tab 子组件，
  位于 views/ar/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 应收发票  | tabs/InvoiceTab.vue                 |
  | 应收对账  | tabs/ReconciliationTab.vue          |
  | 资金账户  | tabs/FundTab.vue                    |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="ar-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane label="应收发票" name="invoice">
        <InvoiceTab />
      </el-tab-pane>
      <el-tab-pane label="应收对账" name="reconciliation">
        <ReconciliationTab />
      </el-tab-pane>
      <el-tab-pane label="资金账户" name="fund">
        <FundTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import InvoiceTab from './tabs/InvoiceTab.vue'
import ReconciliationTab from './tabs/ReconciliationTab.vue'
import FundTab from './tabs/FundTab.vue'

// 当前激活的 Tab；数据懒加载由各子组件 onMounted 内部处理
const activeTab = ref('invoice')
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
