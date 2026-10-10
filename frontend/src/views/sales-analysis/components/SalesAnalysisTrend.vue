<!--
  SalesAnalysisTrend.vue - 销售趋势折线图 + 销售构成饼图
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  趋势折线消费后端按粒度分桶的时间序列（GET /crm/sales-analysis/trends，现算 sales_orders）
-->
<template>
  <el-row :gutter="20" class="chart-row">
    <el-col :xs="24" :lg="16">
      <el-card shadow="hover" class="chart-card">
        <template #header>
          <div class="card-header">
            <span>{{ t('salesAnalysis.trend.cardTitleTrend') }}</span>
            <el-radio-group
              :model-value="granularity"
              size="small"
              @update:model-value="updateGranularity"
            >
              <el-radio-button label="week">{{
                t('salesAnalysis.trend.periodWeek')
              }}</el-radio-button>
              <el-radio-button label="month">{{
                t('salesAnalysis.trend.periodMonth')
              }}</el-radio-button>
              <el-radio-button label="quarter">{{
                t('salesAnalysis.trend.periodQuarter')
              }}</el-radio-button>
              <el-radio-button label="year">{{
                t('salesAnalysis.trend.periodYear')
              }}</el-radio-button>
            </el-radio-group>
          </div>
        </template>
        <div ref="trendChartRef" class="chart-container"></div>
      </el-card>
    </el-col>
    <el-col :xs="24" :lg="8">
      <el-card shadow="hover" class="chart-card">
        <template #header>
          <span>{{ t('salesAnalysis.trend.cardTitleComposition') }}</span>
        </template>
        <div ref="pieChartRef" class="chart-container"></div>
      </el-card>
    </el-col>
  </el-row>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { echarts } from '@/utils/echarts';
import type { ECharts } from '@/utils/echarts';
import { logger } from '@/utils/logger';
import type { SalesTrendGranularity, SalesTrendResult, ProductRanking } from '@/api/sales-analysis';

const { t } = useI18n({ useScope: 'global' });

// 趋势分桶粒度 + 趋势桶序列 + 构成数据（v-model 通过 model-value + update:granularity 实现）
const props = defineProps<{
  granularity: SalesTrendGranularity;
  data: SalesTrendResult[];
  composition: ProductRanking[];
}>();
const emit = defineEmits<{ 'update:granularity': [v: SalesTrendGranularity] }>();

// 控件值域即上面四个 label 字面量（week/month/quarter/year），与后端词表同源；
// day 档后端支持但本卡片控件未提供，不在可发集合内。
const GRANULARITY_LABELS: SalesTrendGranularity[] = ['week', 'month', 'quarter', 'year'];
const updateGranularity = (v: string | number | boolean | undefined) => {
  const hit = GRANULARITY_LABELS.find(g => g === v);
  if (!hit) {
    // 词表外值在控件上不可达；真到达即为控件被改坏的异常形态，显式留痕而非静默下发
    logger.warn(`销售趋势粒度控件收到词表外值: ${String(v)}，已忽略本次切换`);
    return;
  }
  emit('update:granularity', hit);
};

// ECharts 实例 + 容器 ref（批次 95 P3-20 修复）
const trendChartRef = ref<HTMLElement>();
const pieChartRef = ref<HTMLElement>();
let trendChart: ECharts | null = null;
let pieChart: ECharts | null = null;
let resizeHandler: (() => void) | null = null;

// 渲染销售趋势折线柱状图（销售额折线 + 订单数柱状双轴，x 轴为分桶键序列）。
// amount 是后端 Decimal 口径的字符串（如 "3000.75"），喂 ECharts 前逐点 Number() 归一；
// data 由契约保证为数组，不做 `|| []` 兜底（兜底会把缺键契约破坏伪装成空态）。
const renderTrendChart = (data: SalesTrendResult[]) => {
  if (!trendChartRef.value) return;
  if (!trendChart) {
    trendChart = echarts.init(trendChartRef.value);
    resizeHandler = () => {
      trendChart?.resize();
      pieChart?.resize();
    };
    window.addEventListener('resize', resizeHandler);
  }
  const periods = data.map(item => item.period);
  const amounts = data.map(item => Number(item.amount));
  const counts = data.map(item => item.order_count);
  trendChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: {
      data: [t('salesAnalysis.trend.seriesAmount'), t('salesAnalysis.trend.seriesOrderCount')],
    },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', boundaryGap: false, data: periods },
    yAxis: [
      { type: 'value', name: t('salesAnalysis.trend.yAxisAmount') },
      { type: 'value', name: t('salesAnalysis.trend.yAxisOrderCount'), splitLine: { show: false } },
    ],
    series: [
      {
        name: t('salesAnalysis.trend.seriesAmount'),
        type: 'line',
        smooth: true,
        data: amounts,
        areaStyle: { color: 'rgba(102,126,234,0.15)' },
        itemStyle: { color: '#667eea' },
      },
      {
        name: t('salesAnalysis.trend.seriesOrderCount'),
        type: 'bar',
        yAxisIndex: 1,
        data: counts,
        itemStyle: { color: '#764ba2', borderRadius: [4, 4, 0, 0] },
      },
    ],
  });
};

// 渲染销售构成饼图（参考 DashboardPie.vue，数据源为产品排名按金额占比）
const renderPieChart = (composition: ProductRanking[]) => {
  if (!pieChartRef.value) return;
  if (!pieChart) {
    pieChart = echarts.init(pieChartRef.value);
  }
  const data = composition?.length
    ? composition.map(c => ({ name: c.product_name, value: c.amount }))
    : [{ name: t('salesAnalysis.trend.noData'), value: 0 }];
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
        data,
      },
    ],
  });
};

// 监听趋势数据变化（props.data 由契约保证为数组，直传不做兜底）
watch(
  () => props.data,
  newData => renderTrendChart(newData),
  { immediate: true, deep: true }
);

// 监听构成数据变化
watch(
  () => props.composition,
  newData => renderPieChart(newData),
  { immediate: true, deep: true }
);

// 挂载后渲染
onMounted(() => {
  renderTrendChart(props.data);
  renderPieChart(props.composition);
});

// 卸载前清理
onBeforeUnmount(() => {
  trendChart?.dispose();
  pieChart?.dispose();
  trendChart = null;
  pieChart = null;
  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler);
    resizeHandler = null;
  }
});
</script>

<style scoped>
.chart-row {
  margin-bottom: 20px;
}

.chart-card {
  height: 100%;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.chart-container {
  height: 300px;
  width: 100%;
}
</style>
