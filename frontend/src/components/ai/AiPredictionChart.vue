<script setup lang="ts">
/**
 * AI 预测风险趋势图（P2-4）
 * 展示一段时间内质量风险评分变化与同期合格率
 * 组件文件 D13 Batch 7 缩写命名统一：AIPredictionChart → AiPredictionChart
 */
import { computed } from 'vue'

interface PeriodItem {
  period: string
  inspections: number
  avg_qualification_rate: number
}

interface Props {
  periodBreakdown: PeriodItem[]
  riskScore: number
  riskLevel: 'low' | 'medium' | 'high'
  trend: 'up' | 'flat' | 'down' | 'nodata'
}

const props = defineProps<Props>()

const trendLabel = computed(() => {
  return {
    up: '合格率上升',
    flat: '合格率平稳',
    down: '合格率下降',
    nodata: '无历史数据',
  }[props.trend]
})

const riskColor = computed(() => {
  return { low: '#67c23a', medium: '#e6a23c', high: '#f56c6c' }[props.riskLevel]
})

// 计算归一化坐标
const points = computed(() => {
  if (!props.periodBreakdown.length) return [] as string[]
  const values = props.periodBreakdown.map((p) => p.avg_qualification_rate)
  const max = Math.max(...values, 100)
  const min = Math.min(...values, 0)
  const range = Math.max(max - min, 1)
  return props.periodBreakdown.map((p, idx) => {
    const x = (idx / Math.max(props.periodBreakdown.length - 1, 1)) * 100
    const y = 100 - ((p.avg_qualification_rate - min) / range) * 100
    return `${x},${y}`
  })
})

const polyline = computed(() => points.value.join(' '))

const peak = computed(() => {
  if (!props.periodBreakdown.length) return 0
  return Math.max(...props.periodBreakdown.map((p) => p.avg_qualification_rate))
})
const trough = computed(() => {
  if (!props.periodBreakdown.length) return 0
  return Math.min(...props.periodBreakdown.map((p) => p.avg_qualification_rate))
})
</script>

<template>
  <div class="ai-prediction-chart">
    <div class="chart-header">
      <div class="left">
        <el-tag :type="riskLevel === 'high' ? 'danger' : riskLevel === 'medium' ? 'warning' : 'success'">
          风险评分 {{ riskScore }} / 100
        </el-tag>
        <span class="trend-label">{{ trendLabel }}</span>
      </div>
      <div class="right">
        <div class="stat">
          <div class="stat-label">最高合格率</div>
          <div class="stat-value" :style="{ color: riskColor }">{{ peak.toFixed(1) }}%</div>
        </div>
        <div class="stat">
          <div class="stat-label">最低合格率</div>
          <div class="stat-value" :style="{ color: riskColor }">{{ trough.toFixed(1) }}%</div>
        </div>
      </div>
    </div>

    <div v-if="periodBreakdown.length === 0" class="empty">
      <el-empty description="无历史检验数据，建议先录入至少 5 条检验记录" :image-size="80" />
    </div>

    <div v-else class="chart-body">
      <svg viewBox="0 0 100 100" preserveAspectRatio="none" class="line-svg">
        <!-- 网格 -->
        <line v-for="i in 4" :key="`g${i}`" x1="0" :y1="i * 25" x2="100" :y2="i * 25" stroke="#eee" stroke-width="0.2" />
        <!-- 折线 -->
        <polyline :points="polyline" fill="none" :stroke="riskColor" stroke-width="0.8" />
        <!-- 数据点 -->
        <circle
          v-for="(p, idx) in points"
          :key="`p${idx}`"
          :cx="p.split(',')[0]"
          :cy="p.split(',')[1]"
          r="1.2"
          :fill="riskColor"
        />
      </svg>
      <div class="x-axis">
        <span v-for="(p, idx) in periodBreakdown" :key="`x${idx}`">{{ p.period }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ai-prediction-chart {
  border: 1px solid #ebeef5;
  border-radius: 8px;
  padding: 16px;
  background: #fafbfc;
}
.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.left {
  display: flex;
  align-items: center;
  gap: 12px;
}
.trend-label {
  font-size: 14px;
  color: #606266;
}
.right {
  display: flex;
  gap: 24px;
}
.stat {
  text-align: right;
}
.stat-label {
  font-size: 12px;
  color: #909399;
}
.stat-value {
  font-size: 18px;
  font-weight: 600;
}
.chart-body {
  width: 100%;
}
.line-svg {
  width: 100%;
  height: 200px;
  display: block;
}
.x-axis {
  display: flex;
  justify-content: space-between;
  margin-top: 6px;
  font-size: 11px;
  color: #909399;
}
.empty {
  padding: 16px 0;
}
</style>
