<script setup lang="ts">
/**
 * purchase/index.vue - 采购管理页
 * 任务编号: P13 批 1 B3 I-1（拆分原 957 行大 .vue）
 * 拆分后：4 个 composable + 7 个展示子组件
 * 行为完全保持一致（仅结构重构）
 */
import { onMounted } from 'vue'
import { usePurchList } from './composables/usePurchList'
import { usePurchAct } from './composables/usePurchAct'
import { usePurchRcv } from './composables/usePurchRcv'
import { useCreate } from './composables/useCreate'
import PurchTop from './components/PurchTop.vue'
import StatCards from './components/StatCards.vue'
import PurchFilter from './components/PurchFilter.vue'
import PurchTbl from './components/PurchTbl.vue'
import CreateDlg from './components/CreateDlg.vue'
import ReceiveDlg from './components/ReceiveDlg.vue'
import ViewDlg from './components/ViewDlg.vue'
import type { PurchaseOrder } from '@/api/purchase'

// 列表与查询（含统计与工具函数）
const list = usePurchList()

// 业务操作（查看/审批/打印/导出）
const act = usePurchAct(
  () => list.orders.value,
  list.getStatusText,
  list.fetchData
)

// 收货（打开收货对话框 + 提交收货）
const rcv = usePurchRcv(list.fetchData)

// 新建采购单
const create = useCreate(
  () => list.suppliers.value,
  () => list.products.value,
  list.fetchData
)

onMounted(() => {
  list.initPage()
})
</script>

<template>
  <div class="app-container">
    <PurchTop
      :on-create="create.handleCreate"
      :on-print="act.handlePrint"
      :on-export="act.handleExport"
    />

    <StatCards :stats="list.stats.value" :format-currency="list.formatCurrency" />

    <PurchFilter
      :query-params="list.queryParams"
      :suppliers="list.suppliers.value"
      :on-query="list.handleQuery"
      :on-reset="list.handleReset"
    />

    <PurchTbl
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
    />

    <!-- 新建采购单对话框 -->
    <CreateDlg
      :model-value="create.createDialogVisible.value"
      :form="create.createForm.value"
      :rules="create.createFormRules"
      :suppliers="list.suppliers.value"
      :products="list.products.value"
      :form-ref="create.createFormRef"
      :on-submit="create.submitCreate"
      :on-cancel="() => (create.createDialogVisible.value = false)"
      :on-add-item="create.addItem"
      :on-remove-item="create.removeItem"
      :on-product-select="create.handleProductSelect"
      :on-calculate-subtotal="create.calculateSubtotal"
      :calculate-total="create.calculateTotal"
      @update:model-value="(v: boolean) => (create.createDialogVisible.value = v)"
    />

    <!-- 收货对话框 -->
    <ReceiveDlg
      :model-value="rcv.receiveDialogVisible.value"
      :form="rcv.receiveForm.value"
      :warehouses="list.warehouses.value"
      :on-submit="rcv.submitReceive"
      :on-cancel="() => (rcv.receiveDialogVisible.value = false)"
      @update:model-value="(v: boolean) => (rcv.receiveDialogVisible.value = v)"
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

.header-left {
  display: flex;
  align-items: center;
  gap: 20px;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  color: #303133;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-card {
  transition: transform 0.2s, box-shadow 0.2s;
}

.stat-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.08);
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: #fff;
}

.order-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}

.amount-icon {
  background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
}

.pending-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}

.supplier-icon {
  background: linear-gradient(135deg, #a8edea 0%, #fed6e3 100%);
  color: #5c6bc0 !important;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 8px;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
}

.filter-card,
.table-card {
  margin-bottom: 20px;
  border-radius: 8px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.amount {
  font-weight: 500;
  color: #f56c6c;
}

.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.items-table {
  width: 100%;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  padding: 12px;
}

.items-header {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr 80px;
  gap: 8px;
  margin-bottom: 8px;
  font-weight: 600;
  color: #606266;
}

.items-row {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr 80px;
  gap: 8px;
  margin-bottom: 8px;
  align-items: center;
}

.col-product,
.col-qty,
.col-price,
.col-amount {
  width: 100%;
}

.total-amount {
  font-size: 18px;
  font-weight: 600;
  color: #f56c6c;
}
</style>
