<template>
  <div class="dashboard">
    <div class="dashboard-header">
      <h1 class="dashboard-title">仪表盘</h1>
      <el-date-picker
        v-model="dateRange"
        type="daterange"
        range-separator="至"
        start-placeholder="开始日期"
        end-placeholder="结束日期"
        @change="handleDateChange"
      />
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon fabric-icon">
              <el-icon><Folder /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">面料总数</div>
              <div class="stat-value">{{ stats.fabricCount || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon inventory-icon">
              <el-icon><Box /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">库存总量</div>
              <div class="stat-value">{{ formatNumber(stats.inventoryTotal) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon order-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月订单</div>
              <div class="stat-value">{{ stats.monthOrders || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon customer-icon">
              <el-icon><User /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">客户总数</div>
              <div class="stat-value">{{ stats.customerCount || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon today-icon">
              <el-icon><Calendar /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">今日订单</div>
              <div class="stat-value">{{ stats.todayOrders || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">待处理订单</div>
              <div class="stat-value">{{ stats.pendingOrders || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card danger">
          <div class="stat-content">
            <div class="stat-icon alert-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">库存预警</div>
              <div class="stat-value">{{ stats.lowStockProducts || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon activity-icon">
              <el-icon><TrendCharts /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月销售额</div>
              <div class="stat-value">{{ formatCurrency(stats.monthSales) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" class="charts-row">
      <el-col :xs="24" :lg="16">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>销售趋势</span>
              <el-radio-group v-model="trendDays" size="small">
                <el-radio-button :value="7">近7天</el-radio-button>
                <el-radio-button :value="30">近30天</el-radio-button>
              </el-radio-group>
            </div>
          </template>
          <div ref="trendChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="8">
        <el-card shadow="hover">
          <template #header>
            <span>库存分布</span>
          </template>
          <div ref="pieChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20" class="activities-row">
      <el-col :span="24">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>最新活动</span>
              <el-button type="primary" link @click="refreshActivities">
                <el-icon><Refresh /></el-icon>
                刷新
              </el-button>
            </div>
          </template>
          <el-table :data="stats.recentActivities" stripe style="width: 100%">
            <el-table-column prop="time" label="时间" width="180">
              <template #default="{ row }">
                <el-icon><Clock /></el-icon>
                {{ row.time }}
              </template>
            </el-table-column>
            <el-table-column prop="type" label="类型" width="120">
              <template #default="{ row }">
                <el-tag :type="getActivityTypeColor(row.type)">{{ row.type }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="content" label="内容" />
            <el-table-column prop="user" label="操作人" width="120" />
          </el-table>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Folder,
  Box,
  Document,
  User,
  Calendar,
  Clock,
  Warning,
  TrendCharts,
  Refresh
} from '@element-plus/icons-vue'
import { dashboardApi } from '@/api/dashboard'

interface DashboardStats {
  fabricCount?: number
  inventoryTotal?: number
  monthOrders?: number
  customerCount?: number
  todayOrders?: number
  pendingOrders?: number
  lowStockProducts?: number
  monthSales?: number
  recentActivities?: Array<{
    id: number
    type: string
    content: string
    time: string
    user: string
  }>
}

const dateRange = ref<[Date, Date] | null>(null)
const trendDays = ref(7)
const trendChartRef = ref<HTMLElement>()
const pieChartRef = ref<HTMLElement>()

const stats = ref<DashboardStats>({})

const formatNumber = (num: number | undefined) => {
  if (!num) return '0'
  return num.toLocaleString()
}

const formatCurrency = (amount: number | undefined) => {
  if (!amount) return '¥0'
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
    minimumFractionDigits: 0
  }).format(amount)
}

const getActivityTypeColor = (type: string) => {
  const typeMap: Record<string, any> = {
    '订单': 'success',
    '采购': 'warning',
    '库存': 'info',
    '审批': 'primary',
    '系统': 'danger'
  }
  return typeMap[type] || 'info'
}

const fetchDashboardData = async () => {
  try {
    const res = await dashboardApi.getOverview()
    stats.value = res.data! || {}
  } catch (error) {
    stats.value = {
      fabricCount: 156,
      inventoryTotal: 12500,
      monthOrders: 89,
      customerCount: 45,
      todayOrders: 12,
      pendingOrders: 23,
      lowStockProducts: 5,
      monthSales: 1250000,
      recentActivities: [
        { id: 1, type: '订单', content: '销售订单 SO202603130001 已创建', time: '2026-05-13 10:30', user: '张三' },
        { id: 2, type: '采购', content: '采购订单 PO202603130001 已审批通过', time: '2026-05-13 09:45', user: '李四' },
        { id: 3, type: '库存', content: '面料 FB001 库存预警', time: '2026-05-13 09:20', user: '系统' },
        { id: 4, type: '审批', content: '销售订单 SO202603120005 已审批', time: '2026-05-13 08:30', user: '王五' },
        { id: 5, type: '订单', content: '客户"纺织公司A"新建订单', time: '2026-05-13 08:00', user: '赵六' }
      ]
    }
    ElMessage.info('使用演示数据')
  }
}

const refreshActivities = () => {
  fetchDashboardData()
  ElMessage.success('刷新成功')
}

const handleDateChange = () => {
  fetchDashboardData()
}

onMounted(async () => {
  await fetchDashboardData()
  await nextTick()
})
</script>

<style scoped>
.dashboard {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.dashboard-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.stats-row {
  margin-bottom: 20px;
}

.charts-row {
  margin-bottom: 20px;
}

.activities-row {
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

.stat-card.highlight {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.stat-card.highlight .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.highlight .stat-label,
.stat-card.highlight .stat-value {
  color: white;
}

.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}

.stat-card.warning .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.warning .stat-label,
.stat-card.warning .stat-value {
  color: white;
}

.stat-card.danger {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
}

.stat-card.danger .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.danger .stat-label,
.stat-card.danger .stat-value {
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

.stat-icon.fabric-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.inventory-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
  color: white;
}

.stat-icon.order-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
  color: white;
}

.stat-icon.customer-icon {
  background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
  color: white;
}

.stat-icon.today-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.pending-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.alert-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.activity-icon {
  background: linear-gradient(135deg, #30cfd0 0%, #330867 100%);
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
  height: 300px;
  width: 100%;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
