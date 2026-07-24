<template>
  <div class="app-container">
    <PurchaseTop
      :on-create="create.handleCreate"
      :on-print="act.handlePrint"
      :on-export="act.handleExport"
    />

    <StatCards :stats="list.stats.value" :format-currency="list.formatCurrency" />

    <PurchaseFilter
      :query-params="list.queryParams"
      :suppliers="list.suppliers.value"
      :on-query="list.handleQuery"
      :on-reset="list.handleReset"
      @update:query-params="(v) => Object.assign(list.queryParams, v)"
    />

    <PurchaseTable
      :orders="list.orders.value"
      :loading="list.loading.value"
      :total="list.total.value"
      :query-params="list.queryParams"
      :on-view="(row: PurchaseOrder) => act.handleView(row)"
      :on-approve="(row: PurchaseOrder) => act.handleApprove(row)"
      :on-receive="(row: PurchaseOrder) => rcv.handleReceive(row)"
      :on-query="list.handleQuery"
      :get-status-type="list.getStatusType"
      :get-status-text="list.getStatusText"
      :get-payment-status-type="list.getPaymentStatusType"
      :get-payment-status-text="list.getPaymentStatusText"
      @update:query-params="(v) => Object.assign(list.queryParams, v)"
    />

    <!-- 新建采购单对话框 -->
    <CreateDlg
      :model-value="create.createDialogVisible.value"
      :form="create.createForm.value"
      :rules="create.createFormRules"
      :suppliers="list.suppliers.value"
      :products="list.products.value"
      :form-ref="create.createFormRef.value"
      :on-submit="create.submitCreate"
      :on-cancel="() => (create.createDialogVisible.value = false)"
      :on-add-item="create.addItem"
      :on-remove-item="create.removeItem"
      :on-product-select="create.handleProductSelect"
      :on-calculate-subtotal="create.calculateSubtotal"
      :calculate-total="create.calculateTotal"
      @update:model-value="(v: boolean) => (create.createDialogVisible.value = v)"
      @update:form="(v) => (create.createForm.value = v)"
    />

    <!-- 收货对话框 -->
    <ReceiveDlg
      :model-value="rcv.receiveDialogVisible.value"
      :form="rcv.receiveForm.value"
      :warehouses="list.warehouses.value"
      :on-submit="rcv.submitReceive"
      :on-cancel="() => (rcv.receiveDialogVisible.value = false)"
      @update:model-value="(v: boolean) => (rcv.receiveDialogVisible.value = v)"
      @update:form="(v) => (rcv.receiveForm.value = v)"
    />

    <!-- 查看对话框 -->
    <ViewDlg
      :model-value="act.viewDialogVisible.value"
      :data="act.viewData.value"
      :get-status-type="list.getStatusType"
      :get-status-text="list.getStatusText"
      :get-payment-status-type="list.getPaymentStatusType"
      :get-payment-status-text="list.getPaymentStatusText"
      @update:model-value="(v: boolean) => (act.viewDialogVisible.value = v)"
    />
  </div>
</template>

<script setup lang="ts">
import { usePurchList } from './composables/usePurchList'
import { usePurchAct } from './composables/usePurchAct'
import { usePurchRcv } from './composables/usePurchRcv'
import { useCreate } from './composables/useCreate'
import { type PurchaseOrder } from '@/api/purchase'
import PurchaseTop from './components/PurchaseTop.vue'
import StatCards from './components/StatCards.vue'
import PurchaseFilter from './components/PurchaseFilter.vue'
import PurchaseTable from './components/PurchaseTable.vue'
import CreateDlg from './components/CreateDlg.vue'
import ReceiveDlg from './components/ReceiveDlg.vue'
import ViewDlg from './components/ViewDlg.vue'

// 列表与查询（含统计与工具函数）
const list = usePurchList()

// 业务操作（查看/审批/打印/导出）
// V15 P0-S12 修复（Batch 475b）：传入 getQueryParams，导出时传递列表筛选条件
const act = usePurchAct(
  () => list.orders.value,
  list.getStatusText,
  list.fetchData,
  () => ({ status: list.queryParams.status, supplier_id: list.queryParams.supplier_id })
)

// 收货（打开收货对话框 + 提交收货）
const rcv = usePurchRcv(list.fetchData)

// 新建采购单（表单对话框）
const create = useCreate(
  () => list.products.value,
  list.fetchData
)

// 初始化：按需懒加载数据
list.initPage()
</script>

<style scoped>
.app-container {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  background: #fff;
  padding: 20px;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
}

.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  margin: 0;
  color: #303133;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.items-table {
  width: 100%;
}

.items-header {
  display: flex;
  gap: 8px;
  padding: 8px 0;
  font-weight: 600;
  color: #303133;
  border-bottom: 1px solid #ebeef5;
}

.items-row {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;
}

.col-product {
  flex: 1;
  min-width: 200px;
}

.col-qty,
.col-price,
.col-amount {
  width: 110px;
}

.total-amount {
  font-size: 18px;
  font-weight: 700;
  color: #f56c6c;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
