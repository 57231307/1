<template>
  <BaseChart
    ref="chartRef"
    :option="chartOption"
    :height="height"
    :loading="loading"
    :auto-resize="autoResize"
    :aria-label="t('components.charts.barChart.pageAriaLabel')"
    @ready="emit('ready', $event)"
    @click="emit('click', $event)"
  />
</template>

<script setup lang="ts">
/**
 * BarChart 组件 - 柱状图组件
 *
 * 基于 ECharts 的柱状图封装，支持：
 * - 水平/垂直柱状图
 * - 堆叠模式
 * - dataZoom 缩放
 * - 自定义标签显示
 *
 * @example
 * <BarChart
 *   :x-axis-data="['周一', '周二', '周三']"
 *   :series="[{ name: '销量', data: [120, 200, 150] }]"
 *   height="400px"
 * />
 */
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { EChartsOption, ECharts } from 'echarts';
import BaseChart from './BaseChart.vue';

const { t } = useI18n({ useScope: 'global' });

interface BarData {
  name: string;
  data: (number | null)[];
  stack?: string;
  barWidth?: string | number;
  [key: string]: unknown;
}

interface Props {
  xAxisData?: string[];
  series?: BarData[];
  title?: string;
  height?: string;
  loading?: boolean;
  autoResize?: boolean;
  horizontal?: boolean;
  showLabel?: boolean;
  /** batch-20 P3: 是否启用 dataZoom */
  enableDataZoom?: boolean;
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
  enableDataZoom: false,
});

const emit = defineEmits<{
  ready: [instance: ECharts];
  click: [params: Record<string, unknown>];
}>();

const chartRef = ref();

const chartOption = computed<EChartsOption>(() => {
  const seriesConfig = props.series.map(item => ({
    name: item.name,
    type: 'bar' as const,
    data: item.data,
    stack: item.stack,
    barWidth: item.barWidth,
    label: props.showLabel ? { show: true, position: 'top' as const } : undefined,
  }));

  if (props.horizontal) {
    return {
      title: props.title ? { text: props.title, left: 'center' } : undefined,
      tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
      legend: { data: props.series.map(s => s.name), top: props.title ? 30 : 0 },
      grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
      xAxis: { type: 'value' },
      yAxis: { type: 'category', data: props.xAxisData },
      series: seriesConfig,
    };
  }

  return {
    title: props.title ? { text: props.title, left: 'center' } : undefined,
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    legend: { data: props.series.map(s => s.name), top: props.title ? 30 : 0 },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', data: props.xAxisData, axisTick: { alignWithLabel: true } },
    yAxis: { type: 'value' },
    series: seriesConfig,
    dataZoom: props.enableDataZoom
      ? [
          { type: 'inside', start: 0, end: 100 },
          { type: 'slider', start: 0, end: 100 },
        ]
      : undefined,
  };
});

defineExpose({ getChart: () => chartRef.value?.getChart() });
</script>
