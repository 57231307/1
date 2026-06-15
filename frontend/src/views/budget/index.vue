<!--
  budget/index.vue - 预算管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-2）：
  原 NNN 行"上帝组件"已拆分为以下独立子组件，
  位于 views/budget/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 预算管理  | tabs/BudgetListTab.vue              |

  本主入口仅承担：容器与公共样式。
-->
<template>
  <div class="budget-page">
    <BudgetListTab />
  </div>
</template>

<script setup lang="ts">
import BudgetListTab from './tabs/BudgetListTab.vue'
</script>

<style scoped>
.budget-page {
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
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: #303133;
}
:deep(.filter-card) {
  margin-bottom: 20px;
}
:deep(.pagination-wrapper) {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
:deep(.text-red) {
  color: #f56c6c;
}
:deep(.text-orange) {
  color: #e6a23c;
}
:deep(.text-green) {
  color: #67c23a;
}
</style>
