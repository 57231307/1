<!--
  inventoryBatch/index.vue - 批次管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 517 行"上帝组件"已拆分为以下结构：

  - tabs/BatchListTab.vue         （批次列表 + 过滤 + 分页）
  - tabs/BatchFormDialogTab.vue   （新建/编辑弹窗）

  本主入口仅承担：Tab 切换与公共样式。
-->
<template>
  <div class="inventory-batch-page">
    <div class="page-header">
      <h1 class="page-title">批次管理</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item>仓储管理</el-breadcrumb-item>
        <el-breadcrumb-item>批次管理</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <BatchListTab @open-form="openForm" />

    <BatchFormDialogTab
      v-model="formDialogVisible"
      :current-row="currentRow"
      @submitted="handleSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import type { InventoryBatch } from '@/api/inventoryBatch'
import BatchListTab from './tabs/BatchListTab.vue'
import BatchFormDialogTab from './tabs/BatchFormDialogTab.vue'

const formDialogVisible = ref(false)
const currentRow = ref<InventoryBatch | null>(null)

const openForm = (row: InventoryBatch | null) => {
  currentRow.value = row
  formDialogVisible.value = true
}

const handleSubmitted = () => {
  // 子组件已通过 emit 触发刷新
}
</script>

<style scoped>
.inventory-batch-page {
  padding: 24px;
  background: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
</style>
