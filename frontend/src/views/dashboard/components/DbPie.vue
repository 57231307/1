<!--
  DbPie.vue - Dashboard 库存分布 ECharts 饼图
  拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <span>库存分布</span>
    </template>
    <div ref="chartRef" class="chart-container"></div>
  </el-card>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts'
import type { ECharts } from 'echarts'
import type { ChartData } from '@/api/dashboard'

// 饼图数据
const props = defineProps<{ data: ChartData[] }>()

// ECharts 实例 + 容器 ref
const chartRef = ref<HTMLElement>()
let pieChart: ECharts | null = null
let resizeHandler: (() => void) | null = null

// 渲染 ECharts 饼图
const renderChart = (distribution: ChartData[]) => {
  if (!chartRef.value) return
  if (!pieChart) {
    pieChart = echarts.init(chartRef.value)
    resizeHandler = () => pieChart?.resize()
    window.addEventListener('resize', resizeHandler)
  }
  const data = distribution?.length ? distribution : [{ label: '暂无数据', value: 0 }]
  pieChart.setOption({
    tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
    legend: { orient: 'vertical', left: 'left' },
    series: [
      {
        type: 'pie',
        radius: ['40%', '70%'],
        avoidLabelOverlap: false,
        itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
        label: { show: false, position: 'center' },
        emphasis: { label: { show: true, fontSize: 16, fontWeight: 'bold' } },
        labelLine: { show: false },
        data: data.map(d => ({ name: d.label, value: d.value })),
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
  pieChart?.dispose()
  pieChart = null
  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler)
    resizeHandler = null
  }
})
</script>

<style scoped>
.chart-container {
  height: 300px;
  width: 100%;
}
</style>
