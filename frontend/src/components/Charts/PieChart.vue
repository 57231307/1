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

interface PieData {
  name: string
  value: number
  [key: string]: any
}

interface Props {
  data?: PieData[]
  title?: string
  height?: string
  loading?: boolean
  autoResize?: boolean
  roseType?: boolean
  showLabel?: boolean
  radius?: string | string[]
}

const props = withDefaults(defineProps<Props>(), {
  data: () => [],
  title: '',
  height: '400px',
  loading: false,
  autoResize: true,
  roseType: false,
  showLabel: true,
  radius: undefined,
})

const emit = defineEmits<{
  ready: [instance: any]
  click: [params: any]
}>()

const chartRef = ref()

const chartOption = computed<EChartsOption>(() => {
  const radius = props.radius || (props.roseType ? ['15%', '75%'] : ['35%', '65%'])

  return {
    title: props.title ? { text: props.title, left: 'center' } : undefined,
    tooltip: { trigger: 'item', formatter: '{a} <br/>{b}: {c} ({d}%)' },
    legend: {
      orient: 'vertical',
      left: 'left',
      top: 'middle',
      data: props.data.map((d) => d.name),
    },
    series: [
      {
        name: props.title || '数据',
        type: 'pie',
        radius,
        center: ['50%', '50%'],
        roseType: props.roseType ? 'area' : undefined,
        label: props.showLabel ? { show: true, formatter: '{b}: {d}%' } : { show: false },
        data: props.data,
        emphasis: {
          itemStyle: {
            shadowBlur: 10,
            shadowOffsetX: 0,
            shadowColor: 'rgba(0, 0, 0, 0.5)',
          },
        },
      },
    ],
  }
})

defineExpose({ getChart: () => chartRef.value?.getChart() })
</script>
