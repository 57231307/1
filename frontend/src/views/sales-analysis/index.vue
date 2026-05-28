<template>
  <div class="sales-analysis-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">销售分析</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>销售管理</el-breadcrumb-item>
          <el-breadcrumb-item>销售分析</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出报表
        </el-button>
      </div>
    </div>

    <!-- 统计概览 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon order-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月订单数</div>
              <div class="stat-value">{{ stats.monthOrders }}</div>
              <div class="stat-trend" :class="stats.orderTrend > 0 ? 'up' : 'down'">
                {{ stats.orderTrend > 0 ? '+' : '' }}{{ stats.orderTrend }}%
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon amount-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月销售额</div>
              <div class="stat-value">{{ formatCurrency(stats.monthAmount) }}</div>
              <div class="stat-trend" :class="stats.amountTrend > 0 ? 'up' : 'down'">
                {{ stats.amountTrend > 0 ? '+' : '' }}{{ stats.amountTrend }}%
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon profit-icon">
              <el-icon><TrendCharts /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">毛利率</div>
              <div class="stat-value">{{ stats.grossProfitRate }}%</div>
              <div class="stat-trend" :class="stats.profitTrend > 0 ? 'up' : 'down'">
                {{ stats.profitTrend > 0 ? '+' : '' }}{{ stats.profitTrend }}%
              </div>
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
              <div class="stat-label">活跃客户数</div>
              <div class="stat-value">{{ stats.activeCustomers }}</div>
              <div class="stat-trend" :class="stats.customerTrend > 0 ? 'up' : 'down'">
                {{ stats.customerTrend > 0 ? '+' : '' }}{{ stats.customerTrend }}%
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 趋势分析 -->
    <el-row :gutter="20" class="chart-row">
      <el-col :xs="24" :lg="16">
        <el-card shadow="hover" class="chart-card">
          <template #header>
            <div class="card-header">
              <span>销售趋势</span>
              <el-radio-group v-model="trendPeriod" size="small">
                <el-radio-button label="week">本周</el-radio-button>
                <el-radio-button label="month">本月</el-radio-button>
                <el-radio-button label="quarter">本季度</el-radio-button>
                <el-radio-button label="year">本年</el-radio-button>
              </el-radio-group>
            </div>
          </template>
          <div class="chart-container">
            <!-- 趋势图表 -->
            <div class="chart-placeholder">
              <el-icon><TrendCharts /></el-icon>
              <p>销售趋势图表</p>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="8">
        <el-card shadow="hover" class="chart-card">
          <template #header>
            <span>销售构成</span>
          </template>
          <div class="chart-container">
            <!-- 饼图 -->
            <div class="chart-placeholder">
              <el-icon><PieChart /></el-icon>
              <p>销售构成图表</p>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 排名分析 -->
    <el-row :gutter="20" class="ranking-row">
      <el-col :xs="24" :lg="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>产品销售排名</span>
              <el-select v-model="productRankType" size="small" style="width: 100px">
                <el-option label="按金额" value="amount" />
                <el-option label="按数量" value="quantity" />
              </el-select>
            </div>
          </template>
          <el-table :data="productRanking" size="small">
            <el-table-column type="index" label="排名" width="60" align="center" />
            <el-table-column
              prop="product_name"
              label="产品名称"
              min-width="150"
              show-overflow-tooltip
            />
            <el-table-column prop="amount" label="销售额" width="120" align="right">
              <template #default="{ row }">
                {{ formatCurrency(row.amount) }}
              </template>
            </el-table-column>
            <el-table-column prop="quantity" label="销售数量" width="100" align="right" />
            <el-table-column prop="percentage" label="占比" width="80" align="center">
              <template #default="{ row }"> {{ row.percentage }}% </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>客户销售排名</span>
              <el-select v-model="customerRankType" size="small" style="width: 100px">
                <el-option label="按金额" value="amount" />
                <el-option label="按订单数" value="orders" />
              </el-select>
            </div>
          </template>
          <el-table :data="customerRanking" size="small">
            <el-table-column type="index" label="排名" width="60" align="center" />
            <el-table-column
              prop="customer_name"
              label="客户名称"
              min-width="150"
              show-overflow-tooltip
            />
            <el-table-column prop="amount" label="销售额" width="120" align="right">
              <template #default="{ row }">
                {{ formatCurrency(row.amount) }}
              </template>
            </el-table-column>
            <el-table-column prop="order_count" label="订单数" width="80" align="right" />
            <el-table-column prop="percentage" label="占比" width="80" align="center">
              <template #default="{ row }"> {{ row.percentage }}% </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <!-- 目标管理 -->
    <el-card shadow="hover" class="target-card">
      <template #header>
        <div class="card-header">
          <span>销售目标</span>
          <el-button type="primary" size="small" @click="handleEditTarget">
            <el-icon><Edit /></el-icon>
            编辑目标
          </el-button>
        </div>
      </template>
      <el-table :data="salesTargets" border>
        <el-table-column prop="period" label="周期" width="120" align="center" />
        <el-table-column prop="target_amount" label="目标金额" width="150" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.target_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="actual_amount" label="实际金额" width="150" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.actual_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="completion_rate" label="完成率" width="120" align="center">
          <template #default="{ row }">
            <el-progress
              :percentage="row.completion_rate"
              :color="getProgressColor(row.completion_rate)"
            />
          </template>
        </el-table-column>
        <el-table-column prop="variance" label="差异" width="150" align="right">
          <template #default="{ row }">
            <span :class="row.variance >= 0 ? 'text-success' : 'text-danger'">
              {{ row.variance >= 0 ? '+' : '' }}{{ formatCurrency(row.variance) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getTargetStatusType(row.status)">{{
              getTargetStatusLabel(row.status)
            }}</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Download, Edit, TrendCharts, PieChart } from '@element-plus/icons-vue'

// 统计数据
const stats = reactive({
  monthOrders: 0,
  monthAmount: 0,
  grossProfitRate: 0,
  activeCustomers: 0,
  orderTrend: 0,
  amountTrend: 0,
  profitTrend: 0,
  customerTrend: 0,
})

// 趋势周期
const trendPeriod = ref('month')

// 排名类型
const productRankType = ref('amount')
const customerRankType = ref('amount')

// 产品排名
const productRanking = ref([])

// 客户排名
const customerRanking = ref([])

// 销售目标
const salesTargets = ref([])

// 获取统计数据
const getStats = async () => {
  try {
    // TODO: 调用API获取统计数据
    stats.monthOrders = 156
    stats.monthAmount = 2580000
    stats.grossProfitRate = 32.5
    stats.activeCustomers = 89
    stats.orderTrend = 12.5
    stats.amountTrend = 8.3
    stats.profitTrend = -2.1
    stats.customerTrend = 5.2
  } catch (error) {
    console.error('获取统计数据失败:', error)
  }
}

// 获取产品排名
const getProductRanking = async () => {
  try {
    // TODO: 调用API获取产品排名
    productRanking.value = [
      { product_name: '纯棉面料A', amount: 580000, quantity: 12000, percentage: 22.5 },
      { product_name: '涤纶面料B', amount: 420000, quantity: 8500, percentage: 16.3 },
      { product_name: '混纺面料C', amount: 350000, quantity: 7200, percentage: 13.6 },
    ]
  } catch (error) {
    console.error('获取产品排名失败:', error)
  }
}

// 获取客户排名
const getCustomerRanking = async () => {
  try {
    // TODO: 调用API获取客户排名
    customerRanking.value = [
      { customer_name: '客户A', amount: 680000, order_count: 25, percentage: 26.4 },
      { customer_name: '客户B', amount: 450000, order_count: 18, percentage: 17.4 },
      { customer_name: '客户C', amount: 320000, order_count: 12, percentage: 12.4 },
    ]
  } catch (error) {
    console.error('获取客户排名失败:', error)
  }
}

// 获取销售目标
const getSalesTargets = async () => {
  try {
    // TODO: 调用API获取销售目标
    salesTargets.value = [
      {
        period: '2026年1月',
        target_amount: 3000000,
        actual_amount: 2800000,
        completion_rate: 93.3,
        variance: -200000,
        status: 'PARTIAL',
      },
      {
        period: '2026年2月',
        target_amount: 2800000,
        actual_amount: 3100000,
        completion_rate: 110.7,
        variance: 300000,
        status: 'COMPLETED',
      },
      {
        period: '2026年3月',
        target_amount: 3200000,
        actual_amount: 2580000,
        completion_rate: 80.6,
        variance: -620000,
        status: 'IN_PROGRESS',
      },
    ]
  } catch (error) {
    console.error('获取销售目标失败:', error)
  }
}

// 格式化货币
const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(2)}` : '¥0.00'
}

// 获取进度条颜色
const getProgressColor = (percentage: number) => {
  if (percentage >= 100) return '#67c23a'
  if (percentage >= 80) return '#e6a23c'
  return '#f56c6c'
}

// 获取目标状态类型
const getTargetStatusType = (status: string) => {
  const map: Record<string, string> = {
    COMPLETED: 'success',
    IN_PROGRESS: 'warning',
    PARTIAL: 'info',
    NOT_STARTED: 'info',
  }
  return map[status] || 'info'
}

// 获取目标状态标签
const getTargetStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    COMPLETED: '已完成',
    IN_PROGRESS: '进行中',
    PARTIAL: '部分完成',
    NOT_STARTED: '未开始',
  }
  return map[status] || status
}

// 编辑目标
const handleEditTarget = () => {
  ElMessage.info('编辑目标功能开发中')
}

// 导出报表
const handleExport = () => {
  ElMessage.success('导出成功')
}

onMounted(() => {
  getStats()
  getProductRanking()
  getCustomerRanking()
  getSalesTargets()
})
</script>

<style scoped>
.sales-analysis-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-card {
  height: 100%;
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 15px;
}

.stat-icon {
  width: 50px;
  height: 50px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
}

.order-icon {
  background: #e6f7ff;
  color: #1890ff;
}

.amount-icon {
  background: #fff7e6;
  color: #fa8c16;
}

.profit-icon {
  background: #f6ffed;
  color: #52c41a;
}

.customer-icon {
  background: #f9f0ff;
  color: #722ed1;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #666;
  margin-bottom: 5px;
}

.stat-value {
  font-size: 24px;
  font-weight: 600;
  color: #333;
}

.stat-trend {
  font-size: 12px;
  margin-top: 5px;
}

.stat-trend.up {
  color: #52c41a;
}

.stat-trend.down {
  color: #f5222d;
}

.chart-row {
  margin-bottom: 20px;
}

.chart-card {
  height: 100%;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chart-container {
  height: 300px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.chart-placeholder {
  text-align: center;
  color: #999;
}

.chart-placeholder .el-icon {
  font-size: 48px;
  margin-bottom: 10px;
}

.ranking-row {
  margin-bottom: 20px;
}

.target-card {
  margin-bottom: 20px;
}

.text-success {
  color: #52c41a;
}

.text-danger {
  color: #f5222d;
}
</style>
