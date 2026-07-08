<!--
  面料多色号定价扩展 - 价格历史折线图组件
  使用 ECharts 展示价格历史（X=时间，Y=价格）
  创建时间: 2026-06-18
-->
<template>
  <div ref="chartRef" :style="{ width: '100%', height: height + 'px' }" />
</template>

<script setup lang="ts">
import * as echarts from 'echarts/core'
import { LineChart } from 'echarts/charts'
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
  DataZoomComponent,
  TitleComponent,
} from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { onMounted, onBeforeUnmount, ref, watch } from 'vue'
import type { PriceHistoryItem } from '@/api/color-price'
import { formatPrice } from '@/api/color-price'

echarts.use([
  LineChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  DataZoomComponent,
  TitleComponent,
  CanvasRenderer,
])

const props = defineProps<{
  historyData: PriceHistoryItem[]
  currency?: string
  height?: number
}>()

const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null

/** ECharts axis tooltip 回调参数（描述实际使用字段） */
interface AxisTooltipParam {
  name: string
  value: (string | number)[]
  data: {
    changePercent?: string
    changeType?: string
  }
}

const renderChart = () => {
  if (!chartRef.value) return
  if (!chartInstance) {
    chartInstance = echarts.init(chartRef.value)
  }
  const sorted = [...props.historyData].sort(
    (a, b) => new Date(a.operated_at).getTime() - new Date(b.operated_at).getTime(),
  )
  const data = sorted.map((item) => ({
    name: new Date(item.operated_at).toLocaleString('zh-CN'),
    value: [item.operated_at, parseFloat(item.new_price)],
    changePercent: item.change_percent,
    changeType: item.change_type,
  }))
  const currency = props.currency || 'CNY'
  chartInstance.setOption({
    title: {
      text: '价格历史趋势',
      left: 'center',
    },
    tooltip: {
      trigger: 'axis',
      formatter: (params: AxisTooltipParam[]) => {
        const p = params[0]
        const v = formatPrice(p.value[1], currency)
        const cp = p.data.changePercent
        const ct = p.data.changeType
        return `${p.name}<br/>价格: ${v}<br/>涨跌幅: ${cp || '0%'}<br/>类型: ${ct || 'manual'}`
      },
    },
    grid: {
      left: '5%',
      right: '5%',
      bottom: '15%',
      containLabel: true,
    },
    dataZoom: [
      {
        type: 'inside',
        start: 0,
        end: 100,
      },
      {
        start: 0,
        end: 100,
      },
    ],
    xAxis: {
      type: 'time',
    },
    yAxis: {
      type: 'value',
      name: '价格',
      axisLabel: {
        formatter: (v: number) => formatPrice(v, currency),
      },
    },
    series: [
      {
        name: '价格',
        type: 'line',
        data,
        smooth: true,
        symbol: 'circle',
        symbolSize: 8,
        lineStyle: { width: 2 },
        itemStyle: { color: '#1890ff' },
        areaStyle: { color: 'rgba(24, 144, 255, 0.1)' },
      },
    ],
  })
}

onMounted(() => {
  renderChart()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  chartInstance?.dispose()
  chartInstance = null
})

const handleResize = () => {
  chartInstance?.resize()
}

watch(
  () => props.historyData,
  () => renderChart(),
  { deep: true },
)
</script>
