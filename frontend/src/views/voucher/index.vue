<!--
  voucher/index.vue - 凭证管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-2）：
  原 842 行"上帝组件"已拆分为以下独立子组件，
  位于 views/voucher/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 凭证列表  | tabs/VoucherListTab.vue             |

  本主入口仅承担：容器与公共样式 + 凭证编辑弹窗（跨页共用）。
-->
<template>
  <div class="voucher-page">
    <VoucherListTab />
  </div>
</template>

<script setup lang="ts">
import VoucherListTab from './tabs/VoucherListTab.vue'
</script>

<style scoped>
.voucher-page {
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
:deep(.text-red) {
  color: #f56c6c;
}
:deep(.pagination-wrapper) {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
