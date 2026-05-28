<template>
  <div ref="chartRef" class="chart-container" :style="{ width: '100%', height: height }"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import * as echarts from 'echarts'
import type { EChartsOption } from 'echarts'

interface Props {
  option?: EChartsOption
  height?: string
  loading?: boolean
  autoResize?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  height: '400px',
  loading: false,
  autoResize: true,
})

const emit = defineEmits<{
  ready: [instance: echarts.ECharts]
  click: [params: any]
}>()

const chartRef = ref<HTMLDivElement>()
let chartInstance: echarts.ECharts | null = null
let resizeObserver: ResizeObserver | null = null

const defaultOption = computed<EChartsOption>(() => ({
  tooltip: { trigger: 'axis' },
  grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
  ...props.option,
}))

const initChart = () => {
  if (!chartRef.value) return
  chartInstance = echarts.init(chartRef.value)
  chartInstance.setOption(defaultOption.value)
  chartInstance.showLoading({
    text: '加载中...',
    color: '#409EFF',
    textColor: '#000',
    maskColor: 'rgba(255, 255, 255, 0.8)',
  })
  chartInstance.on('click', (params) => emit('click', params))
  emit('ready', chartInstance)
}

const updateChart = () => {
  if (!chartInstance) return
  chartInstance.setOption(props.option || {}, true)
  props.loading ? chartInstance.showLoading() : chartInstance.hideLoading()
}

const handleResize = () => {
  chartInstance?.resize()
}

watch(() => props.option, updateChart, { deep: true })
watch(
  () => props.loading,
  () => {
    if (!chartInstance) return
    props.loading ? chartInstance.showLoading() : chartInstance.hideLoading()
  }
)

onMounted(() => {
  initChart()
  if (props.autoResize) {
    resizeObserver = new ResizeObserver(handleResize)
    chartRef.value && resizeObserver.observe(chartRef.value)
  }
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
  window.removeEventListener('resize', handleResize)
  chartInstance?.dispose()
  chartInstance = null
})

defineExpose({ getChart: () => chartInstance })
</script>

<style scoped>
.chart-container {
  width: 100%;
}
</style>
