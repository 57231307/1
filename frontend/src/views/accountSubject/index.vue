<!--
  accountSubject/index.vue - 会计科目主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-2）：
  原 536 行"上帝组件"已拆分为以下独立子组件，
  位于 views/accountSubject/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 会计科目  | tabs/SubjectListTab.vue             |

  本主入口仅承担：容器与公共样式。
-->
<template>
  <div class="account-subject-page">
    <SubjectListTab />
  </div>
</template>

<script setup lang="ts">
import SubjectListTab from './tabs/SubjectListTab.vue'
</script>

<style scoped>
.account-subject-page {
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
