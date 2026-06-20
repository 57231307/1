<!--
  sales-returns/index.vue - 销售退货管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 7 批
  拆分：527 行 → ~120 行 + 3 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="sales-returns-page">
    <div class="header">
      <h2>销售退货管理</h2>
      <el-button type="primary" @click="onCreate">新建退货单</el-button>
    </div>

    <ReturnsTable
      :list="sr.returnList.value"
      :loading="sr.loading.value"
      @view="srProc.handleView"
      @edit="onEdit"
      @approve="srProc.handleApprove"
    />

    <ReturnDetailDialog
      v-model:visible="srProc.viewDialogVisible.value"
      :current-return="sr.currentReturn.value"
    />

    <ReturnEditDialog
      v-model:visible="editDialogVisible"
      :dialog-mode="dialogMode"
      :form-data="sr.formData"
      :sales-order-list="sr.salesOrderList.value"
      :customer-list="sr.customerList.value"
      :product-list="sr.productList.value"
      :form-rules="formRules"
      :submit-loading="submitLoading"
      @submit="onSubmit"
      @sales-order-change="sr.onSalesOrderChange"
      @add-item="sr.addItem"
      @remove-item="sr.removeItem"
      @calculate="sr.calculateTotal"
      @dialog-close="sr.onEditDialogClose"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import type { FormRules } from 'element-plus'
import { useSr } from './composables/useSr'
import { useSrProc } from './composables/useSrProc'
import ReturnsTable from './components/ReturnsTable.vue'
import ReturnDetailDialog from './components/ReturnDetailDialog.vue'
import ReturnEditDialog from './components/ReturnEditDialog.vue'

const sr = useSr()
const srProc = useSrProc(sr)

const editDialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const submitLoading = ref(false)

const formRules: FormRules = {
  salesOrderId: [{ required: true, message: '请选择销售订单', trigger: 'change' }],
  customerId: [{ required: true, message: '请选择客户', trigger: 'change' }],
  returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
  reason: [{ required: true, message: '请选择退货原因', trigger: 'change' }],
}

const onCreate = () => {
  dialogMode.value = 'create'
  srProc.handleCreate()
  editDialogVisible.value = true
}

const onEdit = (row: any) => {
  dialogMode.value = 'edit'
  srProc.handleEdit(row)
  editDialogVisible.value = true
}

const onSubmit = async () => {
  submitLoading.value = true
  try {
    const ok = await srProc.handleSubmit(dialogMode.value)
    if (ok) {
      editDialogVisible.value = false
      await sr.loadReturns()
    }
  } finally {
    submitLoading.value = false
  }
}

onMounted(() => {
  sr.initLoad()
})
</script>

<style scoped>
.sales-returns-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>
