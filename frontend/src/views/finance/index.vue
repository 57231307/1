<!--
  finance/index.vue - 财务管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-2）：
  原 867 行"上帝组件"已拆分为以下 2 个独立 Tab 子组件，
  位于 views/finance/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 科目管理  | tabs/SubjectTab.vue                 |
  | 凭证管理  | tabs/VoucherTab.vue                 |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="finance-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane label="科目管理" name="subject">
        <SubjectTab />
      </el-tab-pane>
      <el-tab-pane label="凭证管理" name="voucher">
        <VoucherTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import SubjectTab from './tabs/SubjectTab.vue'
import VoucherTab from './tabs/VoucherTab.vue'

// 当前激活的 Tab；数据懒加载由各子组件 onMounted 内部处理
const activeTab = ref('subject')
</script>

<style scoped>
.finance-page {
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
:deep(.entry-footer) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 16px;
}
:deep(.entry-summary) {
  display: flex;
  gap: 24px;
  font-weight: 500;
}
:deep(.text-red) {
  color: #f56c6c;
}
</style>
