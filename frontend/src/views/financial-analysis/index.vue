<!--
  financial-analysis/index.vue - 财务分析主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-2）：
  原 NNN 行"上帝组件"已拆分为以下独立子组件，
  位于 views/financial-analysis/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 财务分析  | tabs/AnalysisListTab.vue            |

  本主入口仅承担：容器与公共样式。
-->
<template>
  <div class="financial-analysis-page">
    <AnalysisListTab />
  </div>
</template>

<script setup lang="ts">
import AnalysisListTab from './tabs/AnalysisListTab.vue'
</script>

<style scoped>
.financial-analysis-page {
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
:deep(.indicator-card) {
  margin-bottom: 16px;
}
:deep(.indicator-title) {
  font-size: 14px;
  color: #909399;
  margin-bottom: 8px;
}
:deep(.indicator-value) {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}
:deep(.trend-up) {
  color: #67c23a;
}
:deep(.trend-down) {
  color: #f56c6c;
}
</style>
