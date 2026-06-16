<!--
  purchase-ext/index.vue - 采购扩展主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-1）：
  原 1151 行"上帝组件"已拆分为以下 3 个独立 Tab 子组件，
  位于 views/purchase-ext/tabs/ 目录：

  | Tab       | 子组件                              |
  | --------- | ----------------------------------- |
  | 采购合同  | tabs/ContractTab.vue                |
  | 采购价格  | tabs/PriceTab.vue                   |
  | 采购退货  | tabs/ReturnTab.vue                  |

  本主入口仅承担：Tab 切换 + 公共样式。
-->
<template>
  <div class="purchase-ext-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => (activeTab = String(tab))">
      <el-tab-pane label="采购合同" name="contract">
        <ContractTab />
      </el-tab-pane>
      <el-tab-pane label="采购价格" name="price">
        <PriceTab />
      </el-tab-pane>
      <el-tab-pane label="采购退货" name="return">
        <ReturnTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import ContractTab from './tabs/ContractTab.vue'
import PriceTab from './tabs/PriceTab.vue'
import ReturnTab from './tabs/ReturnTab.vue'

// 当前激活的 Tab；数据懒加载由各子组件 onMounted 内部处理
const activeTab = ref('contract')
</script>

<style scoped>
.purchase-ext-page {
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
</style>
