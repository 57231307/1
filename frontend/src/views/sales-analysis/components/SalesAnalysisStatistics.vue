<!--
  SalesAnalysisStatistics.vue - 销售分析 4 统计卡片
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
            <div class="stat-label">{{ t('salesAnalysis.stat.labelMonthOrders') }}</div>
            <div class="stat-value">{{ stats.month_orders }}</div>
            <div class="stat-trend" :class="stats.order_trend > 0 ? 'up' : 'down'">
              {{ stats.order_trend > 0 ? '+' : '' }}{{ stats.order_trend }}%
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
            <div class="stat-label">{{ t('salesAnalysis.stat.labelMonthAmount') }}</div>
            <div class="stat-value">{{ formatCurrency(stats.month_amount) }}</div>
            <div class="stat-trend" :class="stats.amount_trend > 0 ? 'up' : 'down'">
              {{ stats.amount_trend > 0 ? '+' : '' }}{{ stats.amount_trend }}%
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
            <div class="stat-label">{{ t('salesAnalysis.stat.labelGrossProfitRate') }}</div>
            <div class="stat-value">{{ stats.gross_profit_rate === '' ? t('common.valueMissing') : `${stats.gross_profit_rate}%` }}</div>
            <div class="stat-trend" :class="stats.profit_trend > 0 ? 'up' : 'down'">
              {{ stats.profit_trend > 0 ? '+' : '' }}{{ stats.profit_trend }}%
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
            <div class="stat-label">{{ t('salesAnalysis.stat.labelActiveCustomers') }}</div>
            <div class="stat-value">{{ stats.active_customers }}</div>
            <div class="stat-trend" :class="stats.customer_trend > 0 ? 'up' : 'down'">
              {{ stats.customer_trend > 0 ? '+' : '' }}{{ stats.customer_trend }}%
            </div>
          </div>
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { Document, Money, TrendCharts, User } from '@element-plus/icons-vue';
import { formatCurrency } from '../composables/saFmts';

const { t } = useI18n({ useScope: 'global' });

// 销售分析统计数据（reactive 对象，父组件传入）
defineProps<{
  stats: {
    month_orders: number;
    month_amount: string;
    gross_profit_rate: string;
    active_customers: number;
    order_trend: number;
    amount_trend: number;
    profit_trend: number;
    customer_trend: number;
  };
}>();
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
