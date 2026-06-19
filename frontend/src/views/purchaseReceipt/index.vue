<!--
  purchaseReceipt/index.vue - 采购入库管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 4 批
  拆分：598 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="app-container">
    <PrcFilter
      :form="prc.searchForm"
      :suppliers="prc.supplierOptions"
      :warehouses="prc.warehouseOptions"
      :status-options="statusOptions"
      @search="prcProc.handleSearch"
      @reset="prcProc.handleReset"
      @add="prcProc.openAddDialog"
    />

    <PrcTbl
      :data="prc.tableData"
      :loading="prc.loading"
      :total="prc.total"
      :pagination="prc.pagination"
      @view="prcProc.openViewDialog"
      @edit="prcProc.openEditDialog"
      @approve="prcProc.handleApprove"
      @delete="prcProc.handleDelete"
      @size-change="prcProc.handlePageSizeChange"
      @current-change="prcProc.handlePageChange"
    />

    <PrcForm
      v-model:visible="prc.dialogVisible"
      :title="prc.dialogTitle"
      :form="prc.form"
      :rules="prc.formRules"
      :suppliers="prc.supplierOptions"
      :warehouses="prc.warehouseOptions"
      :products="prc.productOptions"
      @add-item="prcProc.addItem"
      @remove-item="prcProc.removeItem"
      @calc-amount="prcProc.calculateItemAmount"
      @submit="prcProc.handleSubmit"
    />

    <PrcDetail
      v-model:visible="prc.viewDialogVisible"
      :data="prc.viewData"
      :items="prc.detailData"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { usePrc } from './composables/usePrc'
import { usePrcProc } from './composables/usePrcProc'
import { STATUS_OPTIONS } from './composables/prcFmts'
import PrcFilter from './components/PrcFilter.vue'
import PrcTbl from './components/PrcTbl.vue'
import PrcForm from './components/PrcForm.vue'
import PrcDetail from './components/PrcDetail.vue'

// 业务状态
const prc = usePrc()
const prcProc = usePrcProc({
  searchForm: prc.searchForm,
  pagination: prc.pagination,
  dialogVisible: prc.dialogVisible,
  dialogTitle: prc.dialogTitle,
  form: prc.form,
  viewDialogVisible: prc.viewDialogVisible,
  viewData: prc.viewData,
  detailData: prc.detailData,
  loadData: prc.loadData,
})

// 状态选项
const statusOptions = STATUS_OPTIONS

// 懒加载标记
const hasLoaded = createLazyLoader()

onMounted(() => {
  prc.loadData()
  loadIfNot('suppliers', prc.loadSuppliers, hasLoaded)
  loadIfNot('warehouses', prc.loadWarehouses, hasLoaded)
  loadIfNot('products', prc.loadProducts, hasLoaded)
})
</script>

<style scoped>
.app-container {
  padding: 20px;
}
</style>
