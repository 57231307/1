<template>
  <div class="material-shortage-page">
    <div class="page-header">
      <h2>缺料预警</h2>
      <div class="header-actions">
        <el-button type="primary" :loading="checking" @click="handleCheck">
          <el-icon><Refresh /></el-icon>
          触发检查
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card total">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">缺料总数</div>
              <div class="stat-value">{{ summary.total_shortage_count || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card critical">
          <div class="stat-content">
            <div class="stat-icon critical-icon">
              <el-icon><CircleClose /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">严重缺料</div>
              <div class="stat-value">{{ summary.critical_count || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card high">
          <div class="stat-content">
            <div class="stat-icon high-icon">
              <el-icon><WarningFilled /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">高度缺料</div>
              <div class="stat-value">{{ summary.high_count || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card info">
          <div class="stat-content">
            <div class="stat-icon info-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">最后检查时间</div>
              <div class="stat-value time-value">{{ summary.last_check_time || '-' }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" class="severity-row">
      <el-col v-for="level in severityLevels" :key="level.value" :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" :class="['severity-card', level.class]">
          <div class="severity-content">
            <div class="severity-label">{{ level.label }}</div>
            <div class="severity-value">{{ getSeverityCount(level.value) }}</div>
          </div>
          <el-progress
            :percentage="getSeverityPercentage(level.value)"
            :color="level.color"
            :stroke-width="8"
            :show-text="false"
          />
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="table-card">
      <template #header>
        <div class="card-header">
          <span>缺料预警列表</span>
          <div class="filter-actions">
            <el-select
              v-model="filterSeverity"
              placeholder="严重程度"
              clearable
              style="width: 120px"
              @change="handleFilterChange"
            >
              <el-option label="全部" value="" />
              <el-option label="严重" value="critical" />
              <el-option label="高" value="high" />
              <el-option label="中" value="medium" />
              <el-option label="低" value="low" />
            </el-select>
            <el-select
              v-model="filterStatus"
              placeholder="处理状态"
              clearable
              style="width: 120px; margin-left: 12px"
              @change="handleFilterChange"
            >
              <el-option label="全部" value="" />
              <el-option label="待处理" value="pending" />
              <el-option label="已通知" value="notified" />
              <el-option label="已解决" value="resolved" />
            </el-select>
          </div>
        </div>
      </template>

      <el-table v-loading="tableLoading" :data="shortageList" stripe style="width: 100%">
        <el-table-column prop="material_code" label="物料编码" width="130" />
        <el-table-column prop="material_name" label="物料名称" width="150" />
        <el-table-column prop="spec" label="规格" width="120" />
        <el-table-column prop="current_stock" label="当前库存" width="110">
          <template #default="{ row }">
            <span :class="{ 'stock-warning': row.current_stock < 0 }">{{ row.current_stock }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="required_quantity" label="需求数量" width="110" />
        <el-table-column prop="shortage_quantity" label="缺料数量" width="110">
          <template #default="{ row }">
            <span class="shortage-qty">{{ row.shortage_quantity }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="unit" label="单位" width="70" />
        <el-table-column prop="expected_date" label="需求日期" width="120" />
        <el-table-column prop="source_type" label="来源" width="100">
          <template #default="{ row }">
            <el-tag :type="getSourceTypeColor(row.source_type)" size="small">{{
              getSourceTypeLabel(row.source_type)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="source_no" label="来源单号" width="140" />
        <el-table-column prop="severity" label="严重程度" width="100">
          <template #default="{ row }">
            <el-tag :type="getSeverityColor(row.severity)" size="small">{{
              getSeverityLabel(row.severity)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusColor(row.status)" size="small">{{
              getStatusLabel(row.status)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              type="primary"
              link
              size="small"
              @click="handleNotify(row)"
              >通知</el-button
            >
            <el-button
              v-if="row.status !== 'resolved'"
              type="success"
              link
              size="small"
              @click="handleResolve(row)"
              >解决</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="currentPage"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50]"
        layout="total, sizes, prev, pager, next"
        class="pagination"
        @size-change="fetchShortages"
        @current-change="fetchShortages"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Warning, CircleClose, WarningFilled, Clock, Refresh } from '@element-plus/icons-vue'
import {
  materialShortageApi,
  type MaterialShortageSummary,
  type MaterialShortage,
} from '@/api/material-shortage'

const currentPage = ref(1)
const pageSize = ref(10)
const total = ref(0)
const filterSeverity = ref('')
const filterStatus = ref('')
const tableLoading = ref(false)
const checking = ref(false)

const summary = ref<MaterialShortageSummary>({} as MaterialShortageSummary)
const shortageList = ref<MaterialShortage[]>([])

const severityLevels = [
  { value: 'critical', label: '严重', color: '#f56c6c', class: 'critical' },
  { value: 'high', label: '高', color: '#e6a23c', class: 'high' },
  { value: 'medium', label: '中', color: '#409eff', class: 'medium' },
  { value: 'low', label: '低', color: '#909399', class: 'low' },
]

const getSeverityCount = (severity: string) => {
  const map: Record<string, number> = {
    critical: summary.value.critical_count || 0,
    high: summary.value.high_count || 0,
    medium: summary.value.medium_count || 0,
    low: summary.value.low_count || 0,
  }
  return map[severity] || 0
}

const getSeverityPercentage = (severity: string) => {
  const count = getSeverityCount(severity)
  const total = summary.value.total_shortage_count || 0
  if (total === 0) return 0
  return Math.round((count / total) * 100)
}

const getSeverityColor = (severity: string) => {
  const map: Record<string, string> = {
    critical: 'danger',
    high: 'warning',
    medium: '',
    low: 'info',
  }
  return map[severity] || 'info'
}

const getSeverityLabel = (severity: string) => {
  const map: Record<string, string> = { critical: '严重', high: '高', medium: '中', low: '低' }
  return map[severity] || severity
}

const getStatusColor = (status: string) => {
  const map: Record<string, string> = {
    pending: 'danger',
    notified: 'warning',
    resolved: 'success',
  }
  return map[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = { pending: '待处理', notified: '已通知', resolved: '已解决' }
  return map[status] || status
}

const getSourceTypeColor = (type: string) => {
  const map: Record<string, string> = {
    production: 'primary',
    sales: 'success',
    purchase: 'warning',
  }
  return map[type] || 'info'
}

const getSourceTypeLabel = (type: string) => {
  const map: Record<string, string> = { production: '生产', sales: '销售', purchase: '采购' }
  return map[type] || type
}

const fetchSummary = async () => {
  try {
    const res = await materialShortageApi.getSummary()
    summary.value = res.data!
  } catch (error: any) {
    ElMessage.error(error.message || '获取缺料汇总失败')
    summary.value = {}
  }
}

const fetchShortages = async () => {
  tableLoading.value = true
  try {
    const params: any = { page: currentPage.value, page_size: pageSize.value }
    if (filterSeverity.value) params.severity = filterSeverity.value
    if (filterStatus.value) params.status = filterStatus.value
    const res = await materialShortageApi.listShortages(params)
    shortageList.value = res.data!.list
    total.value = res.data!.total
  } catch (error: any) {
    ElMessage.error(error.message || '获取缺料列表失败')
    shortageList.value = []
    total.value = 0
  } finally {
    tableLoading.value = false
  }
}

const handleCheck = async () => {
  checking.value = true
  try {
    const res = await materialShortageApi.triggerCheck()
    ElMessage.success(res.data!.message || '检查完成')
    await Promise.all([fetchSummary(), fetchShortages()])
  } catch (error: any) {
    ElMessage.error(error.message || '检查失败')
  } finally {
    checking.value = false
  }
}

const handleNotify = async (row: MaterialShortage) => {
  try {
    await materialShortageApi.updateStatus(row.id, 'notified')
    ElMessage.success('已发送通知')
    await fetchShortages()
  } catch (error: any) {
    ElMessage.error(error.message || '发送通知失败')
  }
}

const handleResolve = async (row: MaterialShortage) => {
  try {
    await ElMessageBox.confirm('确认标记此缺料为已解决？', '提示', { type: 'warning' })
    await materialShortageApi.updateStatus(row.id, 'resolved')
    ElMessage.success('已标记为已解决')
    await Promise.all([fetchSummary(), fetchShortages()])
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '标记失败')
    }
  }
}

const handleFilterChange = () => {
  currentPage.value = 1
  fetchShortages()
}

onMounted(() => {
  fetchSummary()
  fetchShortages()
})
</script>

<style scoped>
.material-shortage-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.page-header h2 {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.header-actions {
  display: flex;
  align-items: center;
}

.stats-row {
  margin-bottom: 20px;
}

.severity-row {
  margin-bottom: 20px;
}

.stat-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}

.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}

.stat-card.total .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.total .stat-label,
.stat-card.total .stat-value {
  color: white;
}

.stat-card.critical .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.critical .stat-label,
.stat-card.critical .stat-value {
  color: white;
}

.stat-card.high .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.high .stat-label,
.stat-card.high .stat-value {
  color: white;
}

.stat-card.info .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.info .stat-label,
.stat-card.info .stat-value {
  color: white;
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
}

.stat-icon.total-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.critical-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
  color: white;
}

.stat-icon.high-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.stat-icon.info-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  color: white;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}

.time-value {
  font-size: 14px;
  font-weight: 500;
}

.severity-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}

.severity-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}

.severity-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.severity-label {
  font-size: 14px;
  font-weight: 500;
  color: #606266;
}

.severity-value {
  font-size: 24px;
  font-weight: 700;
}

.severity-card.critical {
  border-left: 4px solid #f56c6c;
}

.severity-card.high {
  border-left: 4px solid #e6a23c;
}

.severity-card.medium {
  border-left: 4px solid #409eff;
}

.severity-card.low {
  border-left: 4px solid #909399;
}

.table-card {
  border-radius: 12px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header span {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.filter-actions {
  display: flex;
  align-items: center;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}

.stock-warning {
  color: #f56c6c;
  font-weight: 600;
}

.shortage-qty {
  color: #e6a23c;
  font-weight: 600;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
