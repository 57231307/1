<!--
  inventoryTransfer/index.vue - 库存调拨主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 600 行"上帝组件"已拆分为以下结构：

  - tabs/TransferListTab.vue              （调拨列表 + 过滤 + 统计）
  - tabs/TransferFormDialogTab.vue        （新建/编辑弹窗）
  - tabs/ApproveTransferDialogTab.vue     （审批弹窗）

  本主入口仅承担：Tab 切换与公共样式。
-->
<template>
  <div class="transfer-page">
    <div class="page-header">
      <h1 class="page-title">{{ t('inventoryTransfer.index.title') }}</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">{{
          t('inventoryTransfer.index.breadcrumb.home')
        }}</el-breadcrumb-item>
        <el-breadcrumb-item>{{
          t('inventoryTransfer.index.breadcrumb.warehouse')
        }}</el-breadcrumb-item>
        <el-breadcrumb-item>{{
          t('inventoryTransfer.index.breadcrumb.current')
        }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <TransferListTab @open-form="openForm" @open-approve="openApprove" />

    <TransferFormDialogTab
      v-model="formDialogVisible"
      :current-row="currentRow"
      :warehouses="warehouses"
      :products="products"
      :mode="dialogMode"
      @submitted="handleSubmitted"
    />

    <ApproveTransferDialogTab
      v-model="approveDialogVisible"
      :current-row="approveRow"
      @submitted="handleApproveSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import { getProductList, type Product } from '@/api/product';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { logger } from '@/utils/logger';
import type { InventoryTransferEntity } from '@/api/inventory-transfer';
import TransferListTab from './tabs/TransferListTab.vue';
import TransferFormDialogTab from './tabs/TransferFormDialogTab.vue';
import ApproveTransferDialogTab from './tabs/ApproveTransferDialogTab.vue';

const { t } = useI18n({ useScope: 'global' });

const formDialogVisible = ref(false);
const dialogMode = ref<'create' | 'edit' | 'view'>('create');
const currentRow = ref<InventoryTransferEntity | null>(null);

const approveDialogVisible = ref(false);
const approveRow = ref<InventoryTransferEntity | null>(null);

const warehouses = ref<Warehouse[]>([]);
const products = ref<Product[]>([]);

const openForm = (mode: 'create' | 'edit' | 'view', row: InventoryTransferEntity | null) => {
  currentRow.value = row;
  dialogMode.value = mode;
  formDialogVisible.value = true;
};
const openApprove = (row: InventoryTransferEntity) => {
  approveRow.value = row;
  approveDialogVisible.value = true;
};
const handleSubmitted = () => {
  // 子组件已通过 emit 触发刷新
};
const handleApproveSubmitted = () => {
  // 子组件已通过 emit 触发刷新
};

const fetchWarehouses = async () => {
  try {
    const res = await getWarehouseList({ page: 1, page_size: 1000 });
    warehouses.value = (res.data?.list as Warehouse[] | undefined) || [];
  } catch (error) {
    logger.error(t('inventoryTransfer.index.fetchWarehousesFailed'), (error as Error).message);
  }
};

const fetchProducts = async () => {
  try {
    const res = await getProductList({ page: 1, page_size: 1000 });
    products.value = (res.data?.list as Product[] | undefined) || [];
  } catch (error) {
    logger.error(t('inventoryTransfer.index.fetchProductsFailed'), (error as Error).message);
  }
};

const hasLoaded = createLazyLoader();
onMounted(() => {
  loadIfNot('warehouses', fetchWarehouses, hasLoaded);
  loadIfNot('products', fetchProducts, hasLoaded);
});
</script>

<style scoped>
.transfer-page {
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
