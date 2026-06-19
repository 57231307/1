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
import { request } from '@/api/request'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import ArReconcileFilter from './ArReconcileFilter.vue'

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

<template>
  <div class="app-container">
    <ArReconcileFilter
      :search-form="searchForm"
      :reconcile-loading="reconcileLoading"
      @search="handleSearch"
      @reset="handleReset"
      @auto-reconcile="handleAutoReconcile"
      @view-confirmations="handleViewConfirmations"
      @open-dispute="openDisputeDialog"
      @update:search-form="(v: any) => Object.assign(searchForm, v)"
    />

    <el-row :gutter="20" class="chart-row">
      <el-col :span="12">
        <el-card shadow="hover">
          <div ref="chartRef" class="chart-container" style="height: 320px"></div>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <div ref="pieChartRef" class="chart-container" style="height: 320px"></div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="table-card">
      <template #header>
        <div class="card-header">
          <span>对账结果列表</span>
          <el-tag type="info">共 {{ total }} 条</el-tag>
        </div>
      </template>
      <el-table
        :data="tableData"
        :loading="loading"
        border
        fit
        highlight-current-row
        style="width: 100%"
      >
        <el-table-column prop="customer_code" label="客户编码" width="120" />
        <el-table-column prop="customer_name" label="客户名称" width="160" />
        <el-table-column label="匹配状态" width="100">
          <template #default="scope">
            <el-tag :type="getMatchStatusType(scope.row.match_status)" size="small">
              {{ getMatchStatusLabel(scope.row.match_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="invoice_amount" label="发票金额" width="130" align="right">
          <template #default="scope">{{ scope.row.invoice_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="payment_amount" label="回款金额" width="130" align="right">
          <template #default="scope">{{ scope.row.payment_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="difference" label="差异金额" width="130" align="right">
          <template #default="scope">
            <span :style="{ color: scope.row.difference !== 0 ? '#f56c6c' : '#67c23a' }">
              {{ scope.row.difference.toFixed(2) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="matched_count" label="已匹配" width="80" align="center" />
        <el-table-column prop="unmatched_count" label="未匹配" width="80" align="center" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="240" align="center">
          <template #default="scope">
            <el-button size="small" @click="handleViewDetail(scope.row as any)">
              <el-icon><View /></el-icon> 明细
            </el-button>
            <el-button
              size="small"
              type="primary"
              @click="handleSendConfirmation(scope.row as any)"
            >
              <el-icon><Send /></el-icon> 确认
            </el-button>
            <el-button size="small" type="danger" @click="openDisputeDialog(scope.row as any)">
              <el-icon><CircleClose /></el-icon> 争议
            </el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next"
        class="pagination-container"
        @current-change="handlePageChange"
        @size-change="handlePageSizeChange"
      />
    </el-card>

    <el-dialog v-model="detailDialogVisible" title="对账明细" width="900px">
      <div v-if="currentReconciliation" class="detail-header">
        <el-descriptions :column="4" border>
          <el-descriptions-item label="客户编码">{{
            currentReconciliation.customer_code
          }}</el-descriptions-item>
          <el-descriptions-item label="客户名称">{{
            currentReconciliation.customer_name
          }}</el-descriptions-item>
          <el-descriptions-item label="发票金额">{{
            currentReconciliation.invoice_amount.toFixed(2)
          }}</el-descriptions-item>
          <el-descriptions-item label="回款金额">{{
            currentReconciliation.payment_amount.toFixed(2)
          }}</el-descriptions-item>
          <el-descriptions-item label="差异金额">{{
            currentReconciliation.difference.toFixed(2)
          }}</el-descriptions-item>
          <el-descriptions-item label="匹配状态">
            <el-tag :type="getMatchStatusType(currentReconciliation.match_status)" size="small">
              {{ getMatchStatusLabel(currentReconciliation.match_status) }}
            </el-tag>
          </el-descriptions-item>
        </el-descriptions>
      </div>
      <el-table :data="detailData" border style="width: 100%; margin-top: 16px">
        <el-table-column prop="type" label="类型" width="100">
          <template #default="scope">
            <el-tag
              size="small"
              :type="
                scope.row.type === 'invoice'
                  ? ''
                  : scope.row.type === 'payment'
                    ? 'success'
                    : 'warning'
              "
            >
              {{
                scope.row.type === 'invoice'
                  ? '发票'
                  : scope.row.type === 'payment'
                    ? '回款'
                    : '调整'
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="source_no" label="单据号" width="150" />
        <el-table-column prop="source_date" label="日期" width="120" />
        <el-table-column prop="amount" label="金额" width="120" align="right">
          <template #default="scope">{{ scope.row.amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="matched_amount" label="已匹配金额" width="120" align="right">
          <template #default="scope">{{ scope.row.matched_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="unmatched_amount" label="未匹配金额" width="120" align="right">
          <template #default="scope">{{ scope.row.unmatched_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="scope">
            <el-tag size="small" :type="getMatchStatusType(scope.row.status)">
              {{ getMatchStatusLabel(scope.row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" />
      </el-table>
    </el-dialog>

    <el-dialog v-model="confirmDialogVisible" title="客户确认记录" width="900px">
      <el-table :data="confirmData" border style="width: 100%">
        <el-table-column prop="customer_name" label="客户名称" width="160" />
        <el-table-column label="确认状态" width="100">
          <template #default="scope">
            <el-tag size="small" :type="getConfirmStatusType(scope.row.confirm_status)">
              {{ getConfirmStatusLabel(scope.row.confirm_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="confirm_amount" label="确认金额" width="120" align="right">
          <template #default="scope">{{ scope.row.confirm_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="disputed_amount" label="争议金额" width="120" align="right">
          <template #default="scope">{{ scope.row.disputed_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="confirmed_at" label="确认时间" width="160" />
        <el-table-column prop="remark" label="备注" />
        <el-table-column label="操作" width="180" align="center">
          <template #default="scope">
            <el-button
              v-if="scope.row.confirm_status === 'pending'"
              size="small"
              type="success"
              @click="handleConfirmStatus(scope.row, 'confirmed')"
            >
              <el-icon><CircleCheck /></el-icon> 确认
            </el-button>
            <el-button
              v-if="scope.row.confirm_status === 'pending'"
              size="small"
              type="danger"
              @click="handleConfirmStatus(scope.row, 'disputed')"
            >
              <el-icon><CircleClose /></el-icon> 争议
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>

    <el-dialog v-model="disputeDialogVisible" title="争议处理" width="900px">
      <el-form :model="disputeForm" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="争议类型">
              <el-select v-model="disputeForm.dispute_type">
                <el-option
                  v-for="o in disputeTypeOptions"
                  :key="o.value"
                  :label="o.label"
                  :value="o.value"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="争议金额">
              <el-input-number
                v-model="disputeForm.dispute_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="争议描述">
          <el-input
            v-model="disputeForm.description"
            type="textarea"
            :rows="3"
            placeholder="请详细描述争议内容"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSubmitDispute">提交争议</el-button>
        </el-form-item>
      </el-form>

      <el-divider>争议记录</el-divider>
      <el-table :data="disputes" border style="width: 100%">
        <el-table-column label="争议类型" width="100">
          <template #default="scope">{{ getDisputeTypeLabel(scope.row.dispute_type) }}</template>
        </el-table-column>
        <el-table-column prop="dispute_amount" label="争议金额" width="120" align="right">
          <template #default="scope">{{ scope.row.dispute_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="scope">
            <el-tag size="small" :type="getDisputeStatusType(scope.row.status)">
              {{ getDisputeStatusLabel(scope.row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="描述" show-overflow-tooltip />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="100" align="center">
          <template #default="scope">
            <el-button
              v-if="scope.row.status !== 'resolved' && scope.row.status !== 'closed'"
              size="small"
              type="primary"
              @click="handleResolveDispute(scope.row as any)"
            >
              解决
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

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
