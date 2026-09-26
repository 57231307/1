<!--
  sales/views/OrderListView.vue - 销售订单列表（拆分重构版）
  任务编号: P14 批 2 I-3 第 3 批
  拆分：644 行 → ~150 行 + 3 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
  说明：本文件保留原路径 views/（B3-1 拆分时已迁入），对话框子组件保留 OrderFormDialog / OrderViewDialog / DeliveryDialog
-->
<template>
  <div class="sales-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('sales.indexPage.title') }}</h2>
      <el-button v-permission="PERMISSIONS.SALES_ORDER_CREATE" type="primary" @click="onCreate">
        <el-icon><Plus /></el-icon> {{ t('sales.indexPage.newOrder') }}
      </el-button>
    </div>

    <SalesOrderStat :stats="olv.stats" />

    <SalesOrderFilter
      :filter-form="olv.filterForm"
      @query="olv.handleQuery"
      @reset="olv.handleReset"
      @update:filter-form="v => Object.assign(olv.filterForm, v)"
    />

    <SalesOrderTable
      :columns="olv.columns"
      :data="olv.data"
      :loading="olv.loading"
      :page="olv.page"
      :page-size="olv.pageSize"
      :total="olv.total"
      @page-change="olv.handlePageChange"
      @size-change="olv.handleSizeChange"
      @view="onView"
      @approve="olvProc.handleApprove"
      @delivery="onDelivery"
      @cancel="olvProc.handleCancel"
      @submit-order="olvProc.handleSubmitOrder"
      @reject="olvProc.handleReject"
      @delete-order="olvProc.handleDelete"
      @detail="olvProc.handleDetail"
    />

    <!-- 拆分后的对话框子组件 -->
    <OrderFormDialog
      v-model:visible="formDialogVisible"
      :title="olv.formDialogTitle"
      :form-data="olv.formData"
      :customers="olv.customers"
      :products="olv.products"
      :submitting="olv.submitting"
      @submit="onFormSubmit"
    />

    <OrderViewDialog v-model:visible="olv.viewDialogVisible" :order="olv.currentOrder" />

    <DeliveryDialog
      v-model:visible="olv.deliveryDialogVisible"
      :form="olv.deliveryForm"
      :warehouses="olv.warehouses"
      :stock-rows="olv.deliveryStockRows"
      :submitting="olvDeliverySubmitting"
      @update:form="v => Object.assign(olv.deliveryForm, v)"
      @warehouse-change="id => olv.loadDeliveryStockRows(id)"
      @submit="onDeliverySubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { Plus } from '@element-plus/icons-vue';
import type { SalesOrder } from '@/api/sales';
import { useOlv, type OrderForm } from '../composables/useOlv';
import { useOlvProc } from '../composables/useOlvProc';
import SalesOrderStat from '../components/SalesOrderStat.vue';
import SalesOrderFilter from '../components/SalesOrderFilter.vue';
import SalesOrderTable from '../components/SalesOrderTable.vue';
import OrderFormDialog from '../OrderFormDialog.vue';
import OrderViewDialog from '../OrderViewDialog.vue';
import DeliveryDialog from '../DeliveryDialog.vue';
// Batch 468 P0-S28：引入权限码常量，与后端 sales-orders 资源对齐
import { PERMISSIONS } from '@/constants/permissions';

const { t } = useI18n({ useScope: 'global' });

const olv = useOlv();
const olvProc = useOlvProc({
  refresh: olv.refresh,
});

// 表单对话框可见性本地 ref
const formDialogVisible = ref(false);

/** 新建订单 */
const onCreate = () => {
  olv.prepareCreate();
  formDialogVisible.value = true;
};

/** 查看详情 */
const onView = (row: SalesOrder) => {
  olv.currentOrder = row;
  olv.viewDialogVisible = true;
};

/** 打开发货对话框 */
const onDelivery = (row: SalesOrder) => {
  olv.prepareDelivery(row);
  olv.deliveryDialogVisible = true;
};

/** 发货提交（出库四维扣减：warehouse_code 从已选仓库带出，后端按编码查仓） */
const olvDeliverySubmitting = ref(false);
const onDeliverySubmit = async (form: typeof olv.deliveryForm) => {
  const warehouse = olv.warehouses.find(w => w.id === form.warehouse_id);
  if (!warehouse?.warehouse_code) {
    ElMessage.warning(t('sales.delivery.warehouseCodeMissing'));
    return;
  }
  olvDeliverySubmitting.value = true;
  try {
    const ok = await olvProc.handleDeliverySubmit(form, warehouse.warehouse_code);
    if (ok) olv.deliveryDialogVisible = false;
  } finally {
    olvDeliverySubmitting.value = false;
  }
};

/** 提交订单表单：使用 OrderFormDialog emit 的本地编辑副本（localData），
 *  而非父组件的 olv.formData——dialog 只在挂载时同步 props→localData，
 *  后续用户编辑只落在 localData；若仍提交 olv.formData 会把未同步的初始值
 *  （required_date: ''）发出，后端 serde 解析 `Option<DateTime<Utc>>` 时
 *  对 `""` 报 "premature end of input" 422，销售订单建单永远失败。 */
const onFormSubmit = async (data: OrderForm) => {
  const ok = await olvProc.handleFormSubmit(data);
  if (ok) formDialogVisible.value = false;
};

onMounted(() => {
  olv.initLoad();
});
</script>

<style scoped>
.sales-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>
