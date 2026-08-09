<template>
  <div
    ref="chartRef"
    class="chart-container"
    role="img"
    :aria-label="chartAriaLabel"
    :style="{ width: '100%', height: height }"
  ></div>
</template>

<script setup lang="ts">
/**
 * BaseChart 组件 - ECharts 图表基础组件
 * 
 * 所有图表组件（LineChart、BarChart、PieChart）的基类，提供：
 * - 自动响应式调整
 * - 加载状态显示
 * - ARIA 无障碍支持
 * - 国际化图表标题
 * 
 * @example
 * <BaseChart :option="chartOption" height="400px" />
 */
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue';
import echarts from '@/utils/echarts';
import type { ECharts, EChartsOption } from '@/utils/echarts';
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

/** BaseChart 组件属性 */
interface Props {
  /** ECharts 配置选项 */
  option?: EChartsOption;
  /** 图表高度，默认 400px */
  height?: string;
  /** 是否显示加载状态 */
  loading?: boolean;
  /** 是否自动响应容器大小变化 */
  autoResize?: boolean;
  /** 图表描述（用于无障碍访问） */
  alt?: string;
}

const props = withDefaults(defineProps<Props>(), {
  height: '400px',
  loading: false,
  autoResize: true,
});

const emit = defineEmits<{
  ready: [instance: ECharts];
  click: [params: Record<string, unknown>];
}>();

const chartRef = ref<HTMLDivElement>();
let chartInstance: ECharts | null = null;
let resizeObserver: ResizeObserver | null = null;

const defaultOption = computed<EChartsOption>(() => ({
  tooltip: { trigger: 'axis' },
  grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
  ...props.option,
}));

/// 图表 aria-label：优先使用 alt prop，否则从 option.title 提取文本，无 title 时降级为"数据图表"
const chartAriaLabel = computed<string>(() => {
  if (props.alt) return props.alt;
  const title = props.option?.title;
  if (!title) return t('components.charts.baseChart.defaultAriaLabel');
  if (typeof title === 'string') return title;
  if (typeof title === 'object' && 'text' in title && typeof title.text === 'string') {
    return title.text;
  }
  return t('components.charts.baseChart.defaultAriaLabel');
});

const initChart = () => {
  if (!chartRef.value) return;
  chartInstance = echarts.init(chartRef.value);
  chartInstance.setOption(defaultOption.value);
  chartInstance.showLoading({
    text: t('components.charts.baseChart.loading'),
    color: '#409EFF',
    textColor: '#000',
    maskColor: 'rgba(255, 255, 255, 0.8)',
  });
  chartInstance.on('click', params => emit('click', params));
  emit('ready', chartInstance);
};

const updateChart = () => {
  if (!chartInstance) return;
  chartInstance.setOption(props.option || {}, true);
  props.loading ? chartInstance.showLoading() : chartInstance.hideLoading();
};

const handleResize = () => {
  chartInstance?.resize();
};

watch(() => props.option, updateChart, { deep: true });
watch(
  () => props.loading,
  () => {
    if (!chartInstance) return;
    props.loading ? chartInstance.showLoading() : chartInstance.hideLoading();
  }
);

onMounted(() => {
  initChart();
  if (props.autoResize) {
    resizeObserver = new ResizeObserver(handleResize);
    chartRef.value && resizeObserver.observe(chartRef.value);
  }
  window.addEventListener('resize', handleResize);
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  window.removeEventListener('resize', handleResize);
  chartInstance?.dispose();
  chartInstance = null;
});

defineExpose({ getChart: () => chartInstance });
</script>

<style scoped>
.chart-container {
  width: 100%;
}
</style>
