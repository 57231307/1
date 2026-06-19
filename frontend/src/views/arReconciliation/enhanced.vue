<!--
  arReconciliation/enhanced.vue - AR 对账增强（拆分重构版）
  任务编号: P14 批 1 B3 I-2
  拆分：789 行 → ~130 行 + 4 composable + 1 工具 + 6 子组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="app-container">
    <ArFilter
      :search-form="arrec.searchForm.value"
      :reconcile-loading="arrec.reconcileLoading.value"
      @search="onSearch"
      @reset="arrec.handleReset"
      @auto-reconcile="arrec.handleAutoReconcile"
      @view-confirmations="arrec.handleViewConfirmations"
      @open-dispute="onOpenDispute"
    />

    <ArCharts :chart-ref="arChart.chartRef" :pie-chart-ref="arChart.pieChartRef" />

    <ArTbl
      :data="arrec.tableData.value"
      :loading="arrec.loading.value"
      :total="arrec.total.value"
      :pagination="arrec.pagination.value"
      @view-detail="arrec.handleViewDetail"
      @send-confirmation="arrec.handleSendConfirmation"
      @open-dispute="ardisp.openDisputeDialog"
      @page-change="arrec.handlePageChange"
      @page-size-change="arrec.handlePageSizeChange"
    />

    <ArDetail
      v-model:visible="arrec.detailDialogVisible.value"
      :current-reconciliation="arrec.currentReconciliation.value"
      :detail-data="arrec.detailData.value"
    />

    <ArConfirm
      v-model:visible="arrec.confirmDialogVisible.value"
      :data="arrec.confirmData.value"
      @confirm-status="arrec.handleConfirmStatus"
    />

    <ArDispute
      v-model:visible="ardisp.disputeDialogVisible.value"
      :form="ardisp.disputeForm.value"
      :disputes="ardisp.disputes.value"
      @submit="ardisp.handleSubmitDispute"
      @resolve="ardisp.handleResolveDispute"
    />
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useArRec } from './composables/useArRec'
import { useArDisp } from './composables/useArDisp'
import { useArChart } from './composables/useArChart'
import { type AutoReconciliationResult } from '@/api/ar-reconciliation-enhanced'
import { request } from '@/api/request'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

const arrec = useArRec()
const ardisp = useArDisp(arrec.loadData)
const arChart = useArChart()

// 客户下拉选项（保留原版数据结构与懒加载行为，模板未直接引用）
const customerOptions = ref<{ label: string; value: number }[]>([])

const hasLoaded = createLazyLoader()

/** 触发搜索并刷新账龄分析 */
const onSearch = async () => {
  await arrec.handleSearch()
  await arChart.loadAgingAnalysis(arrec.searchForm.value.end_date)
}

/** 顶部按钮触发争议对话框（无行数据） */
const onOpenDispute = () => {
  ardisp.openDisputeDialog({ id: 0 } as unknown as AutoReconciliationResult)
}

/** 加载客户下拉数据 */
const loadCustomers = async () => {
  try {
    const res: any = await request.get('/customers/select')
    customerOptions.value = res.data || []
  } catch {
    logger.warn('加载客户失败')
  }
}

onBeforeUnmount(() => {
  arChart.disposeCharts()
})

onMounted(() => {
  arrec.loadData()
  loadIfNot(
    'agingAnalysis',
    () => arChart.loadAgingAnalysis(arrec.searchForm.value.end_date),
    hasLoaded
  )
  loadIfNot('customers', loadCustomers, hasLoaded)
})
</script>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-container {
  margin-bottom: 20px;
  background: #fff;
  padding: 20px;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
}

.filter-actions {
  margin-top: 16px;
  display: flex;
  gap: 8px;
}

.chart-row {
  margin-bottom: 20px;
}

.chart-container {
  width: 100%;
}

.table-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.detail-header {
  margin-bottom: 16px;
}

.w-100 {
  width: 100%;
}
</style>
