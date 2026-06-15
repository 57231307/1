<!--
  fund/index.vue - 资金管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-2）：
  原 826 行"上帝组件"已拆分为以下 2 个独立 Tab 子组件，
  位于 views/fund/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 账户管理  | tabs/AccountTab.vue                 |
  | 转账记录  | tabs/TransferTab.vue                |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="fund-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane label="账户管理" name="account">
        <AccountTab />
      </el-tab-pane>
      <el-tab-pane label="转账记录" name="transfer">
        <TransferTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import AccountTab from './tabs/AccountTab.vue'
import TransferTab from './tabs/TransferTab.vue'

const activeTab = ref('account')
</script>

<style scoped>
.fund-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
:deep(.header-card) {
  margin-bottom: 20px;
}
:deep(.header-content h2) {
  margin: 0 0 8px 0;
  color: #303133;
}
:deep(.header-content p) {
  margin: 0;
  color: #909399;
}
:deep(.table-card) {
  margin-bottom: 20px;
}
:deep(.card-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
:deep(.pagination-container) {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
:deep(.balance-positive) {
  color: #67c23a;
  font-weight: 500;
}
:deep(.balance-frozen) {
  color: #e6a23c;
  font-weight: 500;
}
:deep(.balance-available) {
  color: #409eff;
  font-weight: 500;
}
:deep(.balance-hint) {
  margin-top: 8px;
  font-size: 12px;
  color: #909399;
}
</style>
