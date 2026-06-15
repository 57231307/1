<!--
  inventoryAdjustment/index.vue - 库存调整主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 557 行"上帝组件"已拆分为以下结构：

  - tabs/AdjustmentListTab.vue        （调整单列表 + 过滤 + 统计）
  - tabs/AdjustmentFormDialogTab.vue  （新建/编辑弹窗）
  - tabs/ApproveDialogTab.vue         （审批弹窗）

  本主入口仅承担：Tab 切换与公共样式。
-->
<template>
  <div class="adjustment-page">
    <div class="page-header">
      <h1 class="page-title">库存调整</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item>仓储管理</el-breadcrumb-item>
        <el-breadcrumb-item>库存调整</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <AdjustmentListTab @open-form="openForm" @open-approve="openApprove" />

    <AdjustmentFormDialogTab
      v-model="formDialogVisible"
      :current-row="currentRow"
      :warehouses="warehouses"
      :products="products"
      :mode="dialogMode"
      @submitted="handleSubmitted"
    />

    <ApproveDialogTab
      v-model="approveDialogVisible"
      :current-row="approveRow"
      @submitted="handleApproveSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { warehouseApi, type Warehouse } from '@/api/warehouse'
import { productApi, type Product } from '@/api/product'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import type { InventoryAdjustmentEntity } from '@/api/inventoryAdjustment'
import AdjustmentListTab from './tabs/AdjustmentListTab.vue'
import AdjustmentFormDialogTab from './tabs/AdjustmentFormDialogTab.vue'
import ApproveDialogTab from './tabs/ApproveDialogTab.vue'

const formDialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit' | 'view'>('create')
const currentRow = ref<InventoryAdjustmentEntity | null>(null)

const approveDialogVisible = ref(false)
const approveRow = ref<InventoryAdjustmentEntity | null>(null)

const warehouses = ref<Warehouse[]>([])
const products = ref<Product[]>([])

const openForm = (mode: 'create' | 'edit' | 'view', row: InventoryAdjustmentEntity | null) => {
  currentRow.value = row
  dialogMode.value = mode
  formDialogVisible.value = true
}
const openApprove = (row: InventoryAdjustmentEntity) => {
  approveRow.value = row
  approveDialogVisible.value = true
}
const handleSubmitted = () => {
  // 子组件已通过 emit 触发刷新
}
const handleApproveSubmitted = () => {
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

const fetchProducts = async () => {
  try {
    const res = await productApi.list({ page: 1, page_size: 1000 })
    products.value = (res.data?.list as Product[] | undefined) || []
  } catch (error) {
    logger.error('获取产品列表失败', (error as Error).message)
  }
}

const hasLoaded = createLazyLoader()
onMounted(() => {
  loadIfNot('warehouses', fetchWarehouses, hasLoaded)
  loadIfNot('products', fetchProducts, hasLoaded)
})
</script>

<style scoped>
.adjustment-page {
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
