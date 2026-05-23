<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, View, CircleCheck, CircleClose } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import {
  autoReconcile,
  getAutoReconciliationResults,
  getAgingAnalysis,
  getReconciliationDetails,
  sendCustomerConfirmation,
  getCustomerConfirmations,
  createDispute,
  getDisputes,
  resolveDispute,
  type AutoReconciliationResult,
  type AgingAnalysisResult,
  type ReconciliationDetailItem,
  type CustomerConfirmation,
  type DisputeRecord
} from '@/api/ar-reconciliation-enhanced'
import { request } from '@/api/request'

const loading = ref(false)
const reconcileLoading = ref(false)
const tableData = ref<AutoReconciliationResult[]>([])
const total = ref(0)
const pagination = ref({ page: 1, pageSize: 20 })

const searchForm = ref({
  customer_name: '',
  match_status: '',
  start_date: '',
  end_date: ''
})

const agingData = ref<AgingAnalysisResult[]>([])
const chartRef = ref<HTMLDivElement | null>(null)
const pieChartRef = ref<HTMLDivElement | null>(null)
let barChart: echarts.ECharts | null = null
let pieChart: echarts.ECharts | null = null

const detailDialogVisible = ref(false)
const detailData = ref<ReconciliationDetailItem[]>([])
const currentReconciliation = ref<AutoReconciliationResult | null>(null)

const confirmDialogVisible = ref(false)
const confirmData = ref<CustomerConfirmation[]>([])
const confirmTotal = ref(0)

const disputeDialogVisible = ref(false)
const disputeForm = ref<Partial<DisputeRecord>>({
  dispute_type: 'amount',
  dispute_amount: 0,
  description: '',
  status: 'open'
})
const disputes = ref<DisputeRecord[]>([])
const disputesTotal = ref(0)

const matchStatusOptions = [
  { label: '全部', value: '' },
  { label: '已匹配', value: 'matched' },
  { label: '部分匹配', value: 'partial' },
  { label: '未匹配', value: 'unmatched' }
]

const disputeTypeOptions = [
  { label: '金额争议', value: 'amount' },
  { label: '质量争议', value: 'quality' },
  { label: '交付争议', value: 'delivery' },
  { label: '其他', value: 'other' }
]

const disputeStatusOptions = [
  { label: '待处理', value: 'open' },
  { label: '调查中', value: 'investigating' },
  { label: '已解决', value: 'resolved' },
  { label: '已关闭', value: 'closed' }
]

const customerOptions = ref<{ label: string; value: number }[]>([])

const loadCustomers = async () => {
  try {
    const res: any = await request.get('/customers/select')
    customerOptions.value = res.data || []
  } catch {
    console.warn('加载客户失败')
  }
}

const handleAutoReconcile = async () => {
  if (!searchForm.value.start_date || !searchForm.value.end_date) {
    ElMessage.warning('请选择对账日期范围')
    return
  }
  try {
    await ElMessageBox.confirm('确认启动自动对账？', '提示', { type: 'info' })
    reconcileLoading.value = true
    await autoReconcile({
      start_date: searchForm.value.start_date,
      end_date: searchForm.value.end_date
    })
    ElMessage.success('自动对账任务已启动')
    loadData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error('启动对账失败')
    }
  } finally {
    reconcileLoading.value = false
  }
}

const loadData = async () => {
  loading.value = true
  try {
    const res: any = await getAutoReconciliationResults({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      customer_name: searchForm.value.customer_name || undefined,
      status: searchForm.value.match_status || undefined,
      start_date: searchForm.value.start_date || undefined,
      end_date: searchForm.value.end_date || undefined
    })
    tableData.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch {
    ElMessage.error('加载对账结果失败')
  } finally {
    loading.value = false
  }
}

const loadAgingAnalysis = async () => {
  try {
    const res: any = await getAgingAnalysis({
      customer_id: undefined,
      as_of_date: searchForm.value.end_date || undefined
    })
    agingData.value = res.data || []
    await nextTick()
    renderCharts()
  } catch {
    ElMessage.error('加载账龄分析失败')
  }
}

const renderCharts = () => {
  if (!chartRef.value || !pieChartRef.value) return
  if (!barChart) {
    barChart = echarts.init(chartRef.value)
  }
  if (!pieChart) {
    pieChart = echarts.init(pieChartRef.value)
  }

  const buckets = agingData.value.length > 0
    ? agingData.value[0].buckets
    : [
        { label: '0-30天', range: '0-30', amount: 0, percentage: 0, count: 0 },
        { label: '31-60天', range: '31-60', amount: 0, percentage: 0, count: 0 },
        { label: '61-90天', range: '61-90', amount: 0, percentage: 0, count: 0 },
        { label: '90天以上', range: '90+', amount: 0, percentage: 0, count: 0 }
      ]

  const barOption = {
    title: { text: '账龄分析柱状图', left: 'center' },
    tooltip: { trigger: 'axis', formatter: '{b}: {c} 元' },
    xAxis: { type: 'category', data: buckets.map(b => b.label) },
    yAxis: { type: 'value', name: '金额（元）' },
    series: [{
      type: 'bar',
      data: buckets.map(b => b.amount),
      itemStyle: {
        color: (params: any) => {
          const colors = ['#67c23a', '#e6a23c', '#f56c6c', '#909399']
          return colors[params.dataIndex] || '#409eff'
        }
      },
      label: { show: true, position: 'top', formatter: '{c}' }
    }],
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true }
  }

  const pieOption = {
    title: { text: '账龄分布占比', left: 'center' },
    tooltip: { trigger: 'item', formatter: '{b}: {c}元 ({d}%)' },
    legend: { bottom: '0%' },
    series: [{
      type: 'pie',
      radius: ['40%', '70%'],
      avoidLabelOverlap: false,
      itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
      label: { show: true, formatter: '{b}: {d}%' },
      data: buckets.map((b, i) => ({
        value: b.amount,
        name: b.label,
        itemStyle: {
          color: ['#67c23a', '#e6a23c', '#f56c6c', '#909399'][i]
        }
      }))
    }]
  }

  barChart.setOption(barOption)
  pieChart.setOption(pieOption)
}

const handleSearch = () => {
  pagination.value.page = 1
  loadData()
  loadAgingAnalysis()
}

const handleReset = () => {
  searchForm.value = { customer_name: '', match_status: '', start_date: '', end_date: '' }
  handleSearch()
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadData()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  loadData()
}

const handleViewDetail = async (row: AutoReconciliationResult) => {
  try {
    const res: any = await getReconciliationDetails(row.id)
    detailData.value = res.data || []
    currentReconciliation.value = row
    detailDialogVisible.value = true
  } catch {
    ElMessage.error('获取对账明细失败')
  }
}

const handleSendConfirmation = async (row: AutoReconciliationResult) => {
  try {
    await ElMessageBox.confirm('确认向客户发送对账确认请求？', '提示', { type: 'info' })
    await sendCustomerConfirmation(row.id)
    ElMessage.success('确认请求已发送')
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error('发送确认请求失败')
    }
  }
}

const handleViewConfirmations = async () => {
  try {
    const res: any = await getCustomerConfirmations({
      page: 1,
      pageSize: 20
    })
    confirmData.value = res.data?.list || []
    confirmTotal.value = res.data?.total || 0
    confirmDialogVisible.value = true
  } catch {
    ElMessage.error('加载确认记录失败')
  }
}

const handleConfirmStatus = async (row: CustomerConfirmation, status: 'confirmed' | 'disputed') => {
  const msg = status === 'confirmed' ? '确认此对账记录？' : '标记为争议？'
  try {
    await ElMessageBox.confirm(msg, '提示', { type: 'warning' })
    await updateConfirmationStatus(row.id, { status })
    ElMessage.success('操作成功')
    handleViewConfirmations()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error('操作失败')
    }
  }
}

const updateConfirmationStatus = async (id: number, data: { status: 'confirmed' | 'disputed'; remark?: string }) => {
  const { updateConfirmationStatus: api } = await import('@/api/ar-reconciliation-enhanced')
  return api(id, data)
}

const openDisputeDialog = async (row: AutoReconciliationResult) => {
  disputeForm.value = {
    dispute_type: 'amount',
    dispute_amount: 0,
    description: '',
    status: 'open',
    reconciliation_id: row.id
  }
  disputes.value = []
  try {
    const res: any = await getDisputes({ page: 1, pageSize: 10 })
    disputes.value = res.data?.list || []
    disputesTotal.value = res.data?.total || 0
  } catch {
    console.warn('加载争议记录失败')
  }
  disputeDialogVisible.value = true
}

const handleSubmitDispute = async () => {
  if (!disputeForm.value.description) {
    ElMessage.warning('请填写争议描述')
    return
  }
  try {
    await createDispute(disputeForm.value)
    ElMessage.success('争议已提交')
    disputeDialogVisible.value = false
    loadData()
  } catch {
    ElMessage.error('提交争议失败')
  }
}

const handleResolveDispute = async (row: DisputeRecord) => {
  try {
    const { value } = await ElMessageBox.prompt('请输入解决方案', '解决争议', {
      inputType: 'textarea',
      inputValidator: (v) => !v ? '解决方案不能为空' : true
    })
    await resolveDispute(row.id, { resolution: value })
    ElMessage.success('争议已解决')
    openDisputeDialog({ id: row.reconciliation_id } as AutoReconciliationResult)
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error('解决争议失败')
    }
  }
}

const getMatchStatusType = (status: string) => {
  const map: Record<string, string> = { matched: 'success', partial: 'warning', unmatched: 'danger' }
  return map[status] || 'info'
}

const getMatchStatusLabel = (status: string) => {
  const map: Record<string, string> = { matched: '已匹配', partial: '部分匹配', unmatched: '未匹配' }
  return map[status] || status
}

const getDisputeTypeLabel = (type: string) => {
  return disputeTypeOptions.find(o => o.value === type)?.label || type
}

const getDisputeStatusLabel = (status: string) => {
  return disputeStatusOptions.find(o => o.value === status)?.label || status
}

const getDisputeStatusType = (status: string) => {
  const map: Record<string, string> = { open: 'info', investigating: 'warning', resolved: 'success', closed: 'info' }
  return map[status] || 'info'
}

const getConfirmStatusLabel = (status: string) => {
  const map: Record<string, string> = { pending: '待确认', confirmed: '已确认', disputed: '有争议' }
  return map[status] || status
}

const getConfirmStatusType = (status: string) => {
  const map: Record<string, string> = { pending: 'warning', confirmed: 'success', disputed: 'danger' }
  return map[status] || 'info'
}

onMounted(() => {
  loadData()
  loadAgingAnalysis()
  loadCustomers()
})
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <el-row :gutter="20">
        <el-col :span="6">
          <el-input v-model="searchForm.customer_name" placeholder="客户名称" clearable @keyup.enter="handleSearch" />
        </el-col>
        <el-col :span="5">
          <el-select v-model="searchForm.match_status" placeholder="匹配状态" clearable>
            <el-option v-for="s in matchStatusOptions" :key="s.value" :label="s.label" :value="s.value" />
          </el-select>
        </el-col>
        <el-col :span="5">
          <el-date-picker v-model="searchForm.start_date" type="date" placeholder="开始日期" class="w-100" />
        </el-col>
        <el-col :span="5">
          <el-date-picker v-model="searchForm.end_date" type="date" placeholder="结束日期" class="w-100" />
        </el-col>
        <el-col :span="3">
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-col>
      </el-row>
      <div class="filter-actions">
        <el-button type="success" :loading="reconcileLoading" @click="handleAutoReconcile">
          <Refresh /> 自动对账
        </el-button>
        <el-button type="warning" @click="handleViewConfirmations">
          <Send /> 客户确认
        </el-button>
        <el-button type="danger" @click="openDisputeDialog({ id: 0 } as any)">
          <CircleClose /> 争议处理
        </el-button>
      </div>
    </div>

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
      <el-table :data="tableData" :loading="loading" border fit highlight-current-row style="width: 100%">
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
            <el-button size="small" @click="handleViewDetail(scope.row)">
              <el-icon><View /></el-icon> 明细
            </el-button>
            <el-button size="small" type="primary" @click="handleSendConfirmation(scope.row)">
              <el-icon><Send /></el-icon> 确认
            </el-button>
            <el-button size="small" type="danger" @click="openDisputeDialog(scope.row)">
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
        @current-change="handlePageChange"
        @size-change="handlePageSizeChange"
        class="pagination-container"
      />
    </el-card>

    <el-dialog v-model="detailDialogVisible" title="对账明细" width="900px">
      <div v-if="currentReconciliation" class="detail-header">
        <el-descriptions :column="4" border>
          <el-descriptions-item label="客户编码">{{ currentReconciliation.customer_code }}</el-descriptions-item>
          <el-descriptions-item label="客户名称">{{ currentReconciliation.customer_name }}</el-descriptions-item>
          <el-descriptions-item label="发票金额">{{ currentReconciliation.invoice_amount.toFixed(2) }}</el-descriptions-item>
          <el-descriptions-item label="回款金额">{{ currentReconciliation.payment_amount.toFixed(2) }}</el-descriptions-item>
          <el-descriptions-item label="差异金额">{{ currentReconciliation.difference.toFixed(2) }}</el-descriptions-item>
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
            <el-tag size="small" :type="scope.row.type === 'invoice' ? '' : scope.row.type === 'payment' ? 'success' : 'warning'">
              {{ scope.row.type === 'invoice' ? '发票' : scope.row.type === 'payment' ? '回款' : '调整' }}
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
                <el-option v-for="o in disputeTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="争议金额">
              <el-input-number v-model="disputeForm.dispute_amount" :min="0" :precision="2" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="争议描述">
          <el-input v-model="disputeForm.description" type="textarea" :rows="3" placeholder="请详细描述争议内容" />
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
              @click="handleResolveDispute(scope.row)"
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
