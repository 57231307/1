<!--
  crm/index.vue - CRM 客户管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 668 行"上帝组件"已拆分为以下 2 个独立 Tab 子组件，
  位于 views/crm/tabs/ 目录：

  | Tab         | 子组件                              |
  | ----------- | ----------------------------------- |
  | 客户列表    | tabs/CustomerListTab.vue            |
  | 客户分级    | tabs/RfmTab.vue                     |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="crm-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane label="客户列表" name="list">
        <CustomerListTab />
      </el-tab-pane>
      <el-tab-pane label="客户分级 (RFM)" name="rfm">
        <RfmTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import CustomerListTab from './tabs/CustomerListTab.vue'
import RfmTab from './tabs/RfmTab.vue'

// 当前激活的 Tab；数据懒加载由各子组件 onMounted 内部处理
const activeTab = ref('list')
</script>

<style scoped>
.crm-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
:deep(.page-header) {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
:deep(.header-left .page-title) {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
:deep(.header-actions) {
  display: flex;
  gap: 12px;
}
:deep(.filter-card) {
  margin-bottom: 20px;
}
:deep(.table-card) {
  margin-bottom: 20px;
}
:deep(.pagination-wrapper) {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
:deep(.table-tag) {
  border: none;
  margin-right: 4px;
}
:deep(.no-tags) {
  color: #909399;
  font-size: 12px;
}
:deep(.mb-20) {
  margin-bottom: 20px;
}
:deep(.rfm-card) {
  text-align: center;
}
:deep(.rfm-card-content) {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
:deep(.rfm-card-level) {
  font-size: 32px;
  font-weight: 700;
  color: #303133;
}
:deep(.rfm-card-count) {
  font-size: 14px;
  color: #909399;
}
</style>
