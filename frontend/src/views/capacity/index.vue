<template>
  <div class="capacity-page">
    <div class="page-header">
      <h2>产能分析</h2>
      <div class="header-actions">
        <el-date-picker
          v-model="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @change="handleDateChange"
        />
        <el-select v-model="selectedWorkCenter" placeholder="选择工作中心" clearable style="width: 200px; margin-left: 12px" @change="handleWorkCenterChange">
          <el-option label="全部" :value="undefined" />
          <el-option v-for="wc in workCenters" :key="wc.id" :label="wc.name" :value="wc.id" />
        </el-select>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><OfficeBuilding /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">工作中心总数</div>
              <div class="stat-value">{{ summary.total_work_centers || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card normal">
          <div class="stat-content">
            <div class="stat-icon normal-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">正常运行</div>
              <div class="stat-value">{{ summary.normal_count || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card busy">
          <div class="stat-content">
            <div class="stat-icon busy-icon">
              <el-icon><Loading /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">繁忙状态</div>
              <div class="stat-value">{{ summary.busy_count || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card overload">
          <div class="stat-content">
            <div class="stat-icon overload-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">超负荷</div>
              <div class="stat-value">{{ summary.overload_count || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" class="chart-row">
      <el-col :span="24">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>产能负荷趋势</span>
              <el-radio-group v-model="trendDays" size="small" @change="handleTrendDaysChange">
                <el-radio-button :value="7">近7天</el-radio-button>
                <el-radio-button :value="30">近30天</el-radio-button>
              </el-radio-group>
            </div>
          </template>
          <div ref="capacityChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" class="table-row">
      <el-col :xs="24" :lg="16">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>工作中心列表</span>
              <el-button type="primary" link @click="fetchWorkCenters">
                <el-icon><Refresh /></el-icon>
                刷新
              </el-button>
            </div>
          </template>
          <el-table :data="workCenters" stripe style="width: 100%" v-loading="tableLoading">
            <el-table-column prop="code" label="编号" width="120" />
            <el-table-column prop="name" label="名称" width="150" />
            <el-table-column prop="capacity_hours" label="产能工时" width="120" />
            <el-table-column prop="used_hours" label="已用工时" width="120" />
            <el-table-column prop="load_rate" label="负荷率" width="120">
              <template #default="{ row }">
                <el-tag :type="getLoadRateType(row.load_rate)">{{ (row.load_rate * 100).toFixed(1) }}%</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="bottleneck" label="瓶颈" width="80">
              <template #default="{ row }">
                <el-tag v-if="row.bottleneck" type="danger" size="small">是</el-tag>
                <span v-else>-</span>
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
            @size-change="fetchWorkCenters"
            @current-change="fetchWorkCenters"
          />
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="8">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>瓶颈识别</span>
              <el-tag type="danger">{{ bottlenecks.length }} 个瓶颈</el-tag>
            </div>
          </template>
          <div class="bottleneck-list" v-loading="bottleneckLoading">
            <div v-if="bottlenecks.length === 0" class="empty-state">
              <el-icon><CircleCheck /></el-icon>
              <p>暂无瓶颈工作中心</p>
            </div>
            <div v-for="item in bottlenecks" :key="item.id" class="bottleneck-item">
              <div class="bottleneck-header">
                <span class="bottleneck-name">{{ item.name }}</span>
                <el-tag type="danger" size="small">瓶颈</el-tag>
              </div>
              <div class="bottleneck-info">
                <span>负荷率: <strong>{{ (item.load_rate * 100).toFixed(1) }}%</strong></span>
                <span>已用工时: {{ item.used_hours }} / {{ item.capacity_hours }}</span>
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import {
  OfficeBuilding,
  CircleCheck,
  Loading,
  Warning,
  Refresh
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import type { ECharts } from 'echarts'
import { capacityApi, type CapacitySummary, type WorkCenter, type CapacityTrend } from '@/api/capacity'

const dateRange = ref<[Date, Date] | null>(null)
const trendDays = ref(7)
const selectedWorkCenter = ref<number | undefined>(undefined)
const currentPage = ref(1)
const pageSize = ref(10)
const total = ref(0)

const summary = ref<CapacitySummary>({} as CapacitySummary)
const workCenters = ref<WorkCenter[]>([])
const bottlenecks = ref<WorkCenter[]>([])
const tableLoading = ref(false)
const bottleneckLoading = ref(false)

const capacityChartRef = ref<HTMLElement>()
let capacityChart: ECharts | null = null

const getStatusType = (status: string) => {
  const map: Record<string, string> = { normal: 'success', busy: 'warning', overload: 'danger' }
  return map[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = { normal: '正常', busy: '繁忙', overload: '超负荷' }
  return map[status] || status
}

const getLoadRateType = (rate: number) => {
  if (rate >= 1) return 'danger'
  if (rate >= 0.8) return 'warning'
  return 'success'
}

const fetchSummary = async () => {
  try {
    const res = await capacityApi.getSummary()
    summary.value = res.data!
  } catch {
    summary.value = {
      total_work_centers: 12,
      normal_count: 7,
      busy_count: 3,
      overload_count: 2,
      bottleneck_count: 2,
      avg_load_rate: 0.78
    }
    ElMessage.info('使用演示数据')
  }
}

const fetchTrendData = async () => {
  try {
    const res = await capacityApi.getTrend({ days: trendDays.value, work_center_id: selectedWorkCenter.value })
    const data = res.data!
    renderCapacityChart(data)
  } catch {
    const mockData: CapacityTrend[] = []
    const days = trendDays.value
    for (let i = days - 1; i >= 0; i--) {
      const d = new Date()
      d.setDate(d.getDate() - i)
      mockData.push({
        date: `${d.getMonth() + 1}-${d.getDate()}`,
        planned_hours: Math.floor(Math.random() * 40 + 60),
        actual_hours: Math.floor(Math.random() * 30 + 50),
        capacity_hours: 100
      })
    }
    renderCapacityChart(mockData)
  }
}

const renderCapacityChart = (data: CapacityTrend[]) => {
  if (!capacityChartRef.value) return
  if (!capacityChart) {
    capacityChart = echarts.init(capacityChartRef.value)
  }
  const option = {
    tooltip: { trigger: 'axis' },
    legend: { data: ['计划工时', '实际工时', '产能工时'], bottom: 0 },
    grid: { left: '3%', right: '4%', bottom: '15%', top: '10%', containLabel: true },
    xAxis: { type: 'category', data: data.map(d => d.date), axisLine: { lineStyle: { color: '#909399' } } },
    yAxis: { type: 'value', name: '工时', axisLine: { lineStyle: { color: '#909399' } }, splitLine: { lineStyle: { color: '#ebeef5' } } },
    series: [
      { name: '计划工时', type: 'line', data: data.map(d => d.planned_hours), smooth: true, itemStyle: { color: '#409eff' }, areaStyle: { color: 'rgba(64, 158, 255, 0.1)' } },
      { name: '实际工时', type: 'line', data: data.map(d => d.actual_hours), smooth: true, itemStyle: { color: '#67c23a' } },
      { name: '产能工时', type: 'line', data: data.map(d => d.capacity_hours), smooth: true, itemStyle: { color: '#e6a23c' }, lineStyle: { type: 'dashed' } }
    ]
  }
  capacityChart.setOption(option)
}

const fetchWorkCenters = async () => {
  tableLoading.value = true
  try {
    const res = await capacityApi.listWorkCenters({ page: currentPage.value, page_size: pageSize.value })
    workCenters.value = res.data!.list
    total.value = res.data!.total
  } catch {
    workCenters.value = [
      { id: 1, name: '裁剪中心', code: 'WC001', capacity_hours: 100, used_hours: 85, load_rate: 0.85, status: 'busy', bottleneck: false },
      { id: 2, name: '缝纫中心', code: 'WC002', capacity_hours: 120, used_hours: 115, load_rate: 0.96, status: 'overload', bottleneck: true },
      { id: 3, name: '印染中心', code: 'WC003', capacity_hours: 80, used_hours: 72, load_rate: 0.90, status: 'busy', bottleneck: false },
      { id: 4, name: '包装中心', code: 'WC004', capacity_hours: 90, used_hours: 45, load_rate: 0.50, status: 'normal', bottleneck: false },
      { id: 5, name: '质检中心', code: 'WC005', capacity_hours: 60, used_hours: 58, load_rate: 0.97, status: 'overload', bottleneck: true }
    ]
    total.value = 12
  } finally {
    tableLoading.value = false
  }
}

const fetchBottlenecks = async () => {
  bottleneckLoading.value = true
  try {
    const res = await capacityApi.getBottlenecks()
    bottlenecks.value = res.data!
  } catch {
    bottlenecks.value = [
      { id: 2, name: '缝纫中心', code: 'WC002', capacity_hours: 120, used_hours: 115, load_rate: 0.96, status: 'overload', bottleneck: true },
      { id: 5, name: '质检中心', code: 'WC005', capacity_hours: 60, used_hours: 58, load_rate: 0.97, status: 'overload', bottleneck: true }
    ]
  } finally {
    bottleneckLoading.value = false
  }
}

const handleDateChange = () => {
  fetchTrendData()
}

const handleWorkCenterChange = () => {
  fetchTrendData()
}

const handleTrendDaysChange = () => {
  fetchTrendData()
}

onMounted(async () => {
  await Promise.all([fetchSummary(), fetchTrendData(), fetchWorkCenters(), fetchBottlenecks()])
  await nextTick()
  window.addEventListener('resize', () => capacityChart?.resize())
})
</script>

<style scoped>
.capacity-page {
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

.chart-row {
  margin-bottom: 20px;
}

.table-row {
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

.stat-card.normal .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.normal .stat-label,
.stat-card.normal .stat-value {
  color: white;
}

.stat-card.busy .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.busy .stat-label,
.stat-card.busy .stat-value {
  color: white;
}

.stat-card.overload .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.overload .stat-label,
.stat-card.overload .stat-value {
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

.stat-icon.normal-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
  color: white;
}

.stat-icon.busy-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.stat-icon.overload-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
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

.chart-container {
  height: 350px;
  width: 100%;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}

.bottleneck-list {
  min-height: 200px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
  color: #909399;
}

.empty-state .el-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.bottleneck-item {
  padding: 12px;
  border-bottom: 1px solid #ebeef5;
}

.bottleneck-item:last-child {
  border-bottom: none;
}

.bottleneck-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.bottleneck-name {
  font-weight: 600;
  color: #303133;
}

.bottleneck-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
  color: #606266;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
