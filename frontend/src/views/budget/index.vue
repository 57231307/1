<!--
  budget/index.vue - 预算管理主入口（方案头 → 明细行 两级容器）
  ----------------------------------------------------------------
  两级导航（Q3 决策：方案头 + 明细行，财务按月/季期间分解）：

  | 层级     | 子组件                     |
  | -------- | -------------------------- |
  | 方案头   | tabs/BudgetPlanTab.vue     |
  | 明细行   | tabs/BudgetItemTab.vue     |

  本主入口承担两级切换状态（当前选中的预算方案），不承载业务列表逻辑。
-->
<template>
  <div class="budget-page" :aria-label="t('budget.index.pageAriaLabel')">
    <BudgetItemTab v-if="selectedPlan" :plan="selectedPlan" @back="selectedPlan = null" />
    <BudgetPlanTab v-else @select="selectedPlan = $event" />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useI18n } from 'vue-i18n';
import BudgetPlanTab from './tabs/BudgetPlanTab.vue';
import BudgetItemTab from './tabs/BudgetItemTab.vue';
import type { BudgetPlan } from '@/api/budget';

const { t } = useI18n({ useScope: 'global' });

const selectedPlan = ref<BudgetPlan | null>(null);
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
</style>
