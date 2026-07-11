<!--
  purchaseReceipt/index.vue - 采购入库管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 4 批
  拆分：598 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  批次 285：PrcFilter/PrcTbl 接入 useTableApi（v-model:page/page-size + @fetch + @update:queryParams）
-->
<template>
  <div class="app-container">
    <PrcFilter
      :query-params="prc.queryParams"
      :suppliers="prc.supplierOptions"
      :warehouses="prc.warehouseOptions"
      :status-options="statusOptions"
      @fetch="prcProc.handleSearch"
      @update:query-params="(v) => Object.assign(prc.queryParams, v)"
      @add="prcProc.openAddDialog"
    />

    <PrcTbl
      v-model:page="prc.page"
      v-model:page-size="prc.pageSize"
      :data="prc.tableData"
      :loading="prc.loading"
      :total="prc.total"
      @view="prcProc.openViewDialog"
      @edit="prcProc.openEditDialog"
      @approve="prcProc.handleApprove"
      @delete="prcProc.handleDelete"
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
      @update:form="(v) => (prc.form = v)"
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
  queryParams: prc.queryParams,
  page: prc.page,
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

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
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
