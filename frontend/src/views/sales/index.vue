<!--
  sales/index.vue - 销售管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-17 P1-3-Batch-1）：
  原 1070 行"上帝组件"已拆分为以下独立子组件：
  - OrderListView（V2Table 迁移版） + 3 个对话框子组件（已存在）
  - SalesStatsCards（销售统计卡片） - tabs/SalesStatsCards.vue
  - SalesOrderFilter（销售订单筛选） - tabs/SalesOrderFilter.vue

  本主入口仅承担：页面布局 + 容器化展示。
  业务逻辑已全部迁入子组件，通过 props/emit 通信。
-->
<template>
  <div class="sales-page">
    <OrderListView />
  </div>
</template>

<script setup lang="ts">
import OrderListView from './views/OrderListView.vue'
</script>

<style scoped>
.sales-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
</style>
