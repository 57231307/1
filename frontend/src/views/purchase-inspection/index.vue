<!--
  purchase-inspection/index.vue - 采购验货管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 5 批
  拆分：594 行 → ~120 行 + 5 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="purchase-inspection">
    <div class="page-header">
      <h2>采购检验</h2>
      <el-button type="primary" @click="piProc.handleCreate">
        <el-icon><Plus /></el-icon>
        新建检验单
      </el-button>
    </div>

    <PiStat :stats="pi.stats" />

    <PiFilter
      :params="pi.queryParams"
      :date-range="pi.dateRange"
      :suppliers="pi.suppliers"
      @query="piProc.handleQuery"
      @reset="piProc.handleReset"
      @date-change="(v: [Date, Date] | null) => (pi.dateRange = v)"
      @update:params="(v) => Object.assign(pi.queryParams, v)"
    />

    <PiTbl
      :data="pi.tableData"
      :loading="pi.loading"
      :total="pi.total"
      :pagination="pi.queryParams"
      @view="piProc.handleView"
      @edit="piProc.handleEdit"
      @complete="piProc.handleComplete"
      @reload="pi.fetchData"
      @update:page="(v: number) => (pi.queryParams.page = v)"
      @update:size="(v: number) => (pi.queryParams.page_size = v)"
    />

    <PiForm
      v-model:visible="pi.dialogVisible"
      :is-edit="pi.isEdit"
      :form-data="pi.formData"
      :rules="pi.formRules"
      :submit-loading="pi.submitLoading"
      :receipts="pi.receipts"
      @receipt-change="(v: number) => pi.handleReceiptChange(v)"
      @submit="piProc.handleSubmit"
      @update:form-data="(v) => Object.assign(pi.formData, v)"
    />

    <PiDetail
      v-model:visible="pi.detailDialogVisible"
      :data="pi.detailData"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { loadIfNot } from '@/utils/lazy-loader'
import { Plus } from '@element-plus/icons-vue'
import { usePi } from './composables/usePi'
import { usePiProc } from './composables/usePiProc'
import PiStat from './components/PiStat.vue'
import PiFilter from './components/PiFilter.vue'
import PiTbl from './components/PiTbl.vue'
import PiForm from './components/PiForm.vue'
import PiDetail from './components/PiDetail.vue'

// 业务状态
const pi = usePi()
const piProc = usePiProc({
  tableData: pi.tableData,
  loading: pi.loading,
  total: pi.total,
  dateRange: pi.dateRange,
  queryParams: pi.queryParams,
  suppliers: pi.suppliers,
  receipts: pi.receipts,
  dialogVisible: pi.dialogVisible,
  isEdit: pi.isEdit,
  submitLoading: pi.submitLoading,
  formData: pi.formData,
  detailDialogVisible: pi.detailDialogVisible,
  detailData: pi.detailData,
  fetchData: pi.fetchData,
  handleReceiptChange: pi.handleReceiptChange,
})

onMounted(() => {
  pi.fetchData()
  loadIfNot('suppliers', pi.fetchSuppliers, pi.hasLoaded)
  loadIfNot('receipts', pi.fetchReceipts, pi.hasLoaded)
})
</script>

<style scoped>
.purchase-inspection {
  padding: 20px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}
</style>
