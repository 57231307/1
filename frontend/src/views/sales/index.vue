<!--
  sales/index.vue - 销售订单管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-1）：
  原 1125 行"上帝组件"已拆分为：
  | 子组件                              | 职责                |
  | ----------------------------------- | ------------------- |
  | views/OrderListView.vue             | 列表 + 过滤 + 统计  |
  | OrderFormDialog.vue                 | 订单编辑/新建对话框 |
  | OrderViewDialog.vue                 | 订单详情对话框      |
  | DeliveryDialog.vue                  | 销售发货对话框      |
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
