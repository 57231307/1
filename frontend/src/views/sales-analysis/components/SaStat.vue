<!--
  SaStat.vue - 销售分析 4 统计卡片
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
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
</template>

<script setup lang="ts">
import { Document, Money, TrendCharts, User } from '@element-plus/icons-vue'
import { formatCurrency } from '../composables/saFmts'

// 销售分析统计数据（reactive 对象，父组件传入）
defineProps<{
  stats: {
    monthOrders: number
    monthAmount: number
    grossProfitRate: number
    activeCustomers: number
    orderTrend: number
    amountTrend: number
    profitTrend: number
    customerTrend: number
  }
}>()
</script>

<style scoped>
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
</style>
