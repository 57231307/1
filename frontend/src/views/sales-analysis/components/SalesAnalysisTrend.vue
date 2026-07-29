<!--
  SalesAnalysisTrend.vue - 销售趋势折线图 + 销售构成饼图
  拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
  批次 95 P3-20 修复：接入 ECharts（原为占位面板未渲染图表）
-->
<template>
  <el-row :gutter="20" class="chart-row">
    <el-col :xs="24" :lg="16">
      <el-card shadow="hover" class="chart-card">
        <template #header>
          <div class="card-header">
            <span>{{ t('salesAnalysis.trend.cardTitleTrend') }}</span>
            <el-radio-group :model-value="period" size="small" @update:model-value="updatePeriod">
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
import type { SalesTrendResult, ProductRanking } from '@/api/sales-analysis';

const { t } = useI18n({ useScope: 'global' });

// 趋势周期 + 趋势数据 + 构成数据（v-model 通过 model-value + update:period 实现）
const props = defineProps<{
  period: string;
  data: SalesTrendResult[];
  composition: ProductRanking[];
}>();
const emit = defineEmits<{ 'update:period': [v: string] }>();

const updatePeriod = (v: string) => emit('update:period', v);

// ECharts 实例 + 容器 ref（批次 95 P3-20 修复）
const trendChartRef = ref<HTMLElement>();
const pieChartRef = ref<HTMLElement>();
let trendChart: ECharts | null = null;
let pieChart: ECharts | null = null;
let resizeHandler: (() => void) | null = null;

// 渲染销售趋势折线柱状图（参考 DashboardTrend.vue：销售额折线 + 订单数柱状双轴）
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
  const periods = data?.map(item => item.period) || [];
  const amounts = data?.map(item => item.amount) || [];
  const counts = data?.map(item => item.order_count) || [];
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

// 监听趋势数据变化
watch(
  () => props.data,
  newData => renderTrendChart(newData || []),
  { immediate: true, deep: true }
);

// 监听构成数据变化
watch(
  () => props.composition,
  newData => renderPieChart(newData || []),
  { immediate: true, deep: true }
);

// 挂载后渲染
onMounted(() => {
  renderTrendChart(props.data || []);
  renderPieChart(props.composition || []);
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
