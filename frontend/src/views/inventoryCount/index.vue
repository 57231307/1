<!--
  inventoryCount/index.vue - 库存盘点主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 457 行"上帝组件"已拆分为以下结构：

  - tabs/CountListTab.vue         （盘点单列表 + 过滤 + 统计）
  - tabs/CountFormDialogTab.vue   （新建/编辑弹窗）
  - tabs/CountDetailDialogTab.vue （详情弹窗）

  本主入口仅承担：Tab 切换与公共样式。
-->
<template>
  <div class="count-page">
    <div class="page-header">
      <h1 class="page-title">库存盘点</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item>仓储管理</el-breadcrumb-item>
        <el-breadcrumb-item>库存盘点</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <CountListTab @open-form="openForm" @open-detail="openDetail" />

    <CountFormDialogTab
      v-model="formDialogVisible"
      :current-row="currentRow"
      :warehouses="warehouses"
      :mode="dialogMode"
      @submitted="handleSubmitted"
    />

    <CountDetailDialogTab v-model="detailDialogVisible" :current-row="detailRow" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { warehouseApi, type Warehouse } from '@/api/warehouse'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import type { InventoryCountEntity } from '@/api/inventoryCount'
import CountListTab from './tabs/CountListTab.vue'
import CountFormDialogTab from './tabs/CountFormDialogTab.vue'
import CountDetailDialogTab from './tabs/CountDetailDialogTab.vue'

const formDialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit' | 'view'>('create')
const currentRow = ref<InventoryCountEntity | null>(null)

const detailDialogVisible = ref(false)
const detailRow = ref<InventoryCountEntity | null>(null)

const warehouses = ref<Warehouse[]>([])

const openForm = (mode: 'create' | 'edit' | 'view', row: InventoryCountEntity | null) => {
  currentRow.value = row
  dialogMode.value = mode
  formDialogVisible.value = true
}

const openDetail = (row: InventoryCountEntity) => {
  detailRow.value = row
  detailDialogVisible.value = true
}

const handleSubmitted = () => {
  // 子组件已通过 emit 触发刷新
}

const fetchWarehouses = async () => {
  try {
    const res = await warehouseApi.list({ page: 1, page_size: 1000 })
    warehouses.value = (res.data?.list as Warehouse[] | undefined) || []
  } catch (error) {
    logger.error('获取仓库列表失败', (error as Error).message)
  }
}

const hasLoaded = createLazyLoader()
onMounted(() => {
  loadIfNot('warehouses', fetchWarehouses, hasLoaded)
})
</script>

<style scoped>
.count-page {
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
