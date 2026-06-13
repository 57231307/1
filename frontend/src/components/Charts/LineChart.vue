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
import type { EChartsOption } from 'echarts'
import BaseChart from './BaseChart.vue'

interface LineData {
  name: string
  data: (number | null)[]
  smooth?: boolean
  areaStyle?: boolean
  [key: string]: any
}

interface Props {
  xAxisData?: string[]
  series?: LineData[]
  title?: string
  height?: string
  loading?: boolean
  autoResize?: boolean
  showArea?: boolean
  smooth?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  xAxisData: () => [],
  series: () => [],
  title: '',
  height: '400px',
  loading: false,
  autoResize: true,
  showArea: false,
  smooth: true,
})

const emit = defineEmits<{
  ready: [instance: any]
  click: [params: any]
}>()

const chartRef = ref()

const chartOption = computed<EChartsOption>(() => {
  const seriesConfig = props.series.map(item => ({
    name: item.name,
    type: 'line' as const,
    data: item.data,
    smooth: item.smooth ?? props.smooth,
    areaStyle: (item.areaStyle ?? props.showArea) ? {} : undefined,
  }))

  return {
    title: props.title ? { text: props.title, left: 'center' } : undefined,
    tooltip: { trigger: 'axis' },
    legend: { data: props.series.map(s => s.name), top: props.title ? 30 : 0 },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: {
      type: 'category',
      data: props.xAxisData,
      boundaryGap: false,
    },
    yAxis: { type: 'value' },
    series: seriesConfig,
  }
})

defineExpose({ getChart: () => chartRef.value?.getChart() })
</script>
