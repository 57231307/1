<template>
  <BaseChart
    ref="chartRef"
    :option="chartOption"
    :height="height"
    :loading="loading"
    :auto-resize="autoResize"
    @ready="emit('ready', $event)"
    @click="emit('click', $event)"
  />
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { EChartsOption, ECharts } from 'echarts'
import BaseChart from './BaseChart.vue'

interface BarData {
  name: string
  data: (number | null)[]
  stack?: string
  barWidth?: string | number
  [key: string]: unknown
}

interface Props {
  xAxisData?: string[]
  series?: BarData[]
  title?: string
  height?: string
  loading?: boolean
  autoResize?: boolean
  horizontal?: boolean
  showLabel?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  xAxisData: () => [],
  series: () => [],
  title: '',
  height: '400px',
  loading: false,
  autoResize: true,
  horizontal: false,
  showLabel: false,
})

const emit = defineEmits<{
  ready: [instance: ECharts]
  click: [params: Record<string, unknown>]
}>()

const chartRef = ref()

const chartOption = computed<EChartsOption>(() => {
  const seriesConfig = props.series.map(item => ({
    name: item.name,
    type: 'bar' as const,
    data: item.data,
    stack: item.stack,
    barWidth: item.barWidth,
    label: props.showLabel ? { show: true, position: 'top' as const } : undefined,
  }))

  if (props.horizontal) {
    return {
      title: props.title ? { text: props.title, left: 'center' } : undefined,
      tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
      legend: { data: props.series.map(s => s.name), top: props.title ? 30 : 0 },
      grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
      xAxis: { type: 'value' },
      yAxis: { type: 'category', data: props.xAxisData },
      series: seriesConfig,
    }
  }

  return {
    title: props.title ? { text: props.title, left: 'center' } : undefined,
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    legend: { data: props.series.map(s => s.name), top: props.title ? 30 : 0 },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', data: props.xAxisData, axisTick: { alignWithLabel: true } },
    yAxis: { type: 'value' },
    series: seriesConfig,
  }
})

defineExpose({ getChart: () => chartRef.value?.getChart() })
</script>
