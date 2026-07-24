<!--
  DashboardTrend.vue - Dashboard 销售趋势 ECharts 折线柱状图
  拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>销售趋势</span>
        <el-radio-group
          :model-value="days"
          size="small"
          @update:model-value="updateDays"
        >
          <el-radio-button :value="7">近7天</el-radio-button>
          <el-radio-button :value="30">近30天</el-radio-button>
        </el-radio-group>
      </div>
    </template>
    <div ref="chartRef" class="chart-container"></div>
  </el-card>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts'
import type { ECharts } from 'echarts'
import type { SalesTrend } from '@/api/dashboard'

// 趋势数据 + 天数（v-model 双向）
const props = defineProps<{ data: SalesTrend[]; days: number }>()
const emit = defineEmits<{ 'update:days': [v: number] }>()

const updateDays = (v: number) => emit('update:days', v)

// ECharts 实例 + 容器 ref
const chartRef = ref<HTMLElement>()
let trendChart: ECharts | null = null
let resizeHandler: (() => void) | null = null

// 渲染 ECharts 折线柱状图
const renderChart = (trends: SalesTrend[]) => {
  if (!chartRef.value) return
  if (!trendChart) {
    trendChart = echarts.init(chartRef.value)
    resizeHandler = () => trendChart?.resize()
    window.addEventListener('resize', resizeHandler)
  }
  const dates = trends?.map(t => t.date) || []
  const amounts = trends?.map(t => t.amount) || []
  const counts = trends?.map(t => t.count) || []
  trendChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['销售额', '订单数'] },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', boundaryGap: false, data: dates },
    yAxis: [
      { type: 'value', name: '销售额(元)' },
      { type: 'value', name: '订单数', splitLine: { show: false } },
    ],
    series: [
      {
        name: '销售额',
        type: 'line',
        smooth: true,
        data: amounts,
        areaStyle: { color: 'rgba(102,126,234,0.15)' },
        itemStyle: { color: '#667eea' },
      },
      {
        name: '订单数',
        type: 'bar',
        yAxisIndex: 1,
        data: counts,
        itemStyle: { color: '#764ba2', borderRadius: [4, 4, 0, 0] },
      },
    ],
  })
}

// 监听数据变化
watch(
  () => props.data,
  newData => {
    renderChart(newData || [])
  },
  { immediate: true, deep: true }
)

// 挂载后渲染
onMounted(() => {
  renderChart(props.data || [])
})

// 卸载前清理
onBeforeUnmount(() => {
  trendChart?.dispose()
  trendChart = null
  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler)
    resizeHandler = null
  }
})
</script>

<style scoped>
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
</style>
