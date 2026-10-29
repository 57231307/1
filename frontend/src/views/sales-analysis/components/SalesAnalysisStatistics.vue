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
            <div class="stat-trend" :class="trendTone(stats.order_trend)">
              {{ trendText(stats.order_trend) }}
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
            <div class="stat-trend" :class="trendTone(stats.amount_trend)">
              {{ trendText(stats.amount_trend) }}
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
            <div class="stat-value">
              {{
                stats.gross_profit_rate === null
                  ? t('common.valueMissing')
                  : `${stats.gross_profit_rate}%`
              }}
            </div>
            <div class="stat-trend" :class="trendTone(stats.profit_trend)">
              {{ trendText(stats.profit_trend) }}
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
            <div class="stat-trend" :class="trendTone(stats.customer_trend)">
              {{ trendText(stats.customer_trend) }}
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
import type { SalesStats } from '@/api/sales-analysis';
import { formatCurrency } from '../composables/saFmts';

const { t } = useI18n({ useScope: 'global' });

// 销售分析统计数据（父组件传入的只读快照；形态直接取后端概览出参的契约类型，
// 环比与毛利率在契约里就是可空，这里不再另抄一份非空声明）
defineProps<{
  stats: SalesStats;
}>();

/**
 * 环比涨跌行的样式档位。
 * 调用方，本组件模板的四枚涨跌行。
 * 入参，后端概览出参的环比值：null 表不可比（上月桶缺失或为 0），正数表上涨，
 * 0 与负数表下跌。
 * 传给谁，仅返回 class 名给模板绑定，不参与取数、不存储、不打印。
 */
const trendTone = (trend: number | null): 'none' | 'up' | 'down' =>
  trend === null ? 'none' : trend > 0 ? 'up' : 'down';

/**
 * 环比涨跌行的可见文本。
 * 调用方，本组件模板的四枚涨跌行。
 * 入参，同 trendTone 的可空环比值；null 出"暂无可比数据"文案，不带正负号与百分号，
 * 非 null 按后端原值出数并加百分号（不做任何换算与舍入）。
 * 传给谁，仅返回字符串给模板渲染，不存储、不打印。
 */
const trendText = (trend: number | null): string =>
  trend === null ? t('salesAnalysis.stat.trendNoData') : `${trend > 0 ? '+' : ''}${trend}%`;
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

/* 环比不可比时的中性档：既不绿也不红，避免把"无数据"读成"下跌"；
   色值取本组件标签既用的 #666（12px 文本对白底约 5.7:1，达 AA），不用 --el-text-color-secondary（约 3.1:1 不达线） */
.stat-trend.none {
  color: #666;
}
</style>
