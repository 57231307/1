<script setup lang="ts">
/**
 * P3-4 BI 多维分析 - 销售分析主页面
 *
 * 功能：
 * 1. KPI 概览（总销售/订单数/客单价/同比/环比）
 * 2. 销售趋势图（折线图，ECharts）
 * 3. 客户排行（柱状图）
 * 4. 产品分布（饼图）
 * 5. 区域热力（柱状图）
 * 6. 利润分析
 * 7. 多维筛选 + 钻取
 */
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { echarts } from '@/utils/echarts';
import type { ECharts } from '@/utils/echarts';
import {
  getKpiSummary,
  getSalesTrend,
  getSalesByCustomer,
  getSalesByProduct,
  getSalesByRegion,
  getProfitAnalysis,
  getDrilldownYearToMonth,
  getSalesByCategory,
  getDrilldownMonthToDay,
  getDrilldownCustomerToOrder,
  getDrilldownProductToOrder,
  postSlice,
  postDice,
  postRollup,
  postPivot,
} from '@/api/bi';
import { unwrapBi } from '@/api/bi';
import type {
  KpiSummary,
  TimeSeriesPoint,
  CustomerRank,
  ProductRank,
  RegionStat,
  ProfitAnalysis,
  CategoryStat,
  DrilldownOrderItem,
} from '@/api/bi';
import logger from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

const kpi = ref<KpiSummary | null>(null);

// 同比增长率文案：后端无法计算时返回 null，显示"无同比数据"而非伪造成 0%
const yoyGrowthText = computed(() => {
  const g = kpi.value?.yoy_growth;
  return g === null || g === undefined
    ? t('biSalesAnalysis.kpi.noYoyData')
    : t('biSalesAnalysis.kpi.yoyGrowth', { value: g.toFixed(1) });
});

// 环比增长率文案：后端无法计算时返回 null，显示"无环比数据"而非伪造成 0%
const momGrowthText = computed(() => {
  const g = kpi.value?.mom_growth;
  return g === null || g === undefined
    ? t('biSalesAnalysis.kpi.noMomData')
    : t('biSalesAnalysis.kpi.momGrowth', { value: g.toFixed(1) });
});
const trend = ref<TimeSeriesPoint[]>([]);
const customers = ref<CustomerRank[]>([]);
const products = ref<ProductRank[]>([]);
const regions = ref<RegionStat[]>([]);
const profit = ref<ProfitAnalysis | null>(null);
const monthlyData = ref<TimeSeriesPoint[]>([]);

const trendChartRef = ref<HTMLDivElement>();
const customerChartRef = ref<HTMLDivElement>();
const productChartRef = ref<HTMLDivElement>();
const regionChartRef = ref<HTMLDivElement>();

let trendChart: ECharts | null = null;
let customerChart: ECharts | null = null;
let productChart: ECharts | null = null;
let regionChart: ECharts | null = null;

async function loadAll() {
  try {
    const [k, tr, c, p, r, prof] = await Promise.all([
      getKpiSummary(),
      getSalesTrend(30),
      getSalesByCustomer(10),
      getSalesByProduct(10),
      getSalesByRegion(),
      getProfitAnalysis(),
    ]);
    kpi.value = unwrapBi(k);
    trend.value = unwrapBi(tr);
    customers.value = unwrapBi(c);
    products.value = unwrapBi(p);
    regions.value = unwrapBi(r);
    profit.value = unwrapBi(prof);

    // 钻取 2026 年 → 月
    const monthly = await getDrilldownYearToMonth(2026);
    monthlyData.value = unwrapBi(monthly);

    renderCharts();
  } catch (e) {
    ElMessage.error(t('biSalesAnalysis.message.loadFailed'));
    logger.error(t('biSalesAnalysis.message.loadFailed'), e);
  }
}

function renderCharts() {
  // 1. 销售趋势
  if (trendChartRef.value) {
    trendChart = echarts.init(trendChartRef.value);
    trendChart.setOption({
      title: { text: t('biSalesAnalysis.chart.salesTrendTitle'), left: 'center' },
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'category', data: trend.value.map(p => p.period) },
      yAxis: { type: 'value' },
      series: [
        {
          name: t('biSalesAnalysis.chart.salesAmount'),
          data: trend.value.map(p => p.total_amount),
          type: 'line',
          smooth: true,
          itemStyle: { color: '#409EFF' },
        },
        {
          name: t('biSalesAnalysis.chart.profit'),
          data: trend.value.map(p => p.profit_amount),
          type: 'line',
          smooth: true,
          itemStyle: { color: '#67C23A' },
        },
      ],
    });
  }

  // 2. 客户排行
  if (customerChartRef.value) {
    customerChart = echarts.init(customerChartRef.value);
    customerChart.setOption({
      title: { text: t('biSalesAnalysis.chart.customerRankTitle'), left: 'center' },
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'value' },
      yAxis: { type: 'category', data: customers.value.map(c => c.customer_name).reverse() },
      series: [
        {
          type: 'bar',
          data: customers.value.map(c => c.total_amount).reverse(),
          itemStyle: { color: '#E6A23C' },
        },
      ],
    });
  }

  // 3. 产品分布
  if (productChartRef.value) {
    productChart = echarts.init(productChartRef.value);
    productChart.setOption({
      title: { text: t('biSalesAnalysis.chart.productDistTitle'), left: 'center' },
      tooltip: { trigger: 'item' },
      series: [
        {
          name: t('biSalesAnalysis.chart.salesAmount'),
          type: 'pie',
          radius: '50%',
          data: products.value.map(p => ({ name: p.product_name, value: p.total_amount })),
        },
      ],
    });
  }

  // 4. 区域热力
  if (regionChartRef.value) {
    regionChart = echarts.init(regionChartRef.value);
    regionChart.setOption({
      title: { text: t('biSalesAnalysis.chart.regionDistTitle'), left: 'center' },
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'category', data: regions.value.map(r => r.region) },
      yAxis: { type: 'value' },
      series: [
        {
          type: 'bar',
          data: regions.value.map(r => r.total_amount),
          itemStyle: { color: '#F56C6C' },
        },
      ],
    });
  }
}

function formatCurrency(n: number | undefined) {
  if (n === undefined) return '—';
  return `¥${n.toLocaleString('zh-CN', { maximumFractionDigits: 2 })}`;
}

// ===== 品类统计 =====
const categories = ref<CategoryStat[]>([]);
const categoryLoading = ref(false);

async function loadCategory() {
  categoryLoading.value = true;
  try {
    const res = await getSalesByCategory();
    categories.value = unwrapBi(res);
  } catch (e) {
    ElMessage.error('获取品类统计失败');
    logger.error('获取品类统计失败', e);
  } finally {
    categoryLoading.value = false;
  }
}

// ===== 钻取中心 =====
const monthDrill = reactive({ year: '2026', month: '1' });
const customerDrillId = ref('');
const productDrillId = ref('');
const monthToDayData = ref<TimeSeriesPoint[]>([]);
const orderDrillData = ref<DrilldownOrderItem[]>([]);

async function loadMonthToDay() {
  const year = Number(monthDrill.year);
  const month = Number(monthDrill.month);
  if (!year || !month || month < 1 || month > 12) {
    ElMessage.warning('请输入有效年份与月份');
    return;
  }
  orderDrillData.value = [];
  try {
    const res = await getDrilldownMonthToDay(year, month);
    monthToDayData.value = unwrapBi(res);
  } catch (e) {
    ElMessage.error('获取月→日钻取数据失败');
    logger.error('获取月→日钻取数据失败', e);
  }
}

async function loadCustomerToOrder() {
  const id = Number(customerDrillId.value);
  if (!id) {
    ElMessage.warning('请输入客户ID');
    return;
  }
  monthToDayData.value = [];
  try {
    const res = await getDrilldownCustomerToOrder(id);
    orderDrillData.value = unwrapBi(res);
  } catch (e) {
    ElMessage.error('获取客户→订单钻取数据失败');
    logger.error('获取客户→订单钻取数据失败', e);
  }
}

async function loadProductToOrder() {
  const id = Number(productDrillId.value);
  if (!id) {
    ElMessage.warning('请输入产品ID');
    return;
  }
  monthToDayData.value = [];
  try {
    const res = await getDrilldownProductToOrder(id);
    orderDrillData.value = unwrapBi(res);
  } catch (e) {
    ElMessage.error('获取产品→订单钻取数据失败');
    logger.error('获取产品→订单钻取数据失败', e);
  }
}

// ===== 多维分析：切片/切块/上卷/透视 =====
const multiForm = reactive({ dimension: '', filters: '{}' });
const rollupForm = reactive({ from: '', to: '' });
const pivotForm = reactive({ row: '', col: '', measure: 'total_amount' });
const multiResult = ref('');

function parseFilters(): Record<string, unknown> | null {
  try {
    const parsed = JSON.parse(multiForm.filters || '{}');
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) return parsed;
    ElMessage.warning('过滤条件需为 JSON 对象');
    return null;
  } catch {
    ElMessage.warning('过滤条件 JSON 格式有误');
    return null;
  }
}

async function runSlice() {
  if (!multiForm.dimension) {
    ElMessage.warning('请输入切片维度');
    return;
  }
  const filters = parseFilters();
  if (!filters) return;
  try {
    const res = await postSlice(multiForm.dimension, filters);
    multiResult.value = JSON.stringify(res.data, null, 2);
  } catch (e) {
    ElMessage.error('切片查询失败');
    logger.error('切片查询失败', e);
  }
}

async function runDice() {
  const filters = parseFilters();
  if (!filters) return;
  try {
    const res = await postDice(filters);
    multiResult.value = JSON.stringify(res.data, null, 2);
  } catch (e) {
    ElMessage.error('切块查询失败');
    logger.error('切块查询失败', e);
  }
}

async function runRollup() {
  if (!rollupForm.from || !rollupForm.to) {
    ElMessage.warning('请输入上卷起始与目标维度');
    return;
  }
  try {
    const res = await postRollup(rollupForm.from, rollupForm.to);
    multiResult.value = JSON.stringify(res.data, null, 2);
  } catch (e) {
    ElMessage.error('上卷查询失败');
    logger.error('上卷查询失败', e);
  }
}

async function runPivot() {
  if (!pivotForm.row || !pivotForm.col || !pivotForm.measure) {
    ElMessage.warning('请填写行/列维度与度量');
    return;
  }
  try {
    const res = await postPivot(pivotForm.row, pivotForm.col, pivotForm.measure);
    multiResult.value = JSON.stringify(res.data, null, 2);
  } catch (e) {
    ElMessage.error('透视查询失败');
    logger.error('透视查询失败', e);
  }
}

onMounted(() => {
  loadAll();
  window.addEventListener('resize', resizeCharts);
});

onBeforeUnmount(() => {
  // 清理 window resize 监听器，防止组件卸载后内存泄漏
  // 多次进入 BI 销售分析页面时，旧的 listener 不释放会持续累积，导致内存占用线性增长
  window.removeEventListener('resize', resizeCharts);
});

function resizeCharts() {
  trendChart?.resize();
  customerChart?.resize();
  productChart?.resize();
  regionChart?.resize();
}
</script>

<template>
  <div class="bi-sales-analysis">
    <h2>{{ $t('biSalesAnalysis.title') }}</h2>

    <!-- 1. KPI 概览 -->
    <div class="kpi-row">
      <el-card class="kpi-card">
        <div class="kpi-label">{{ $t('biSalesAnalysis.kpi.totalSales') }}</div>
        <div class="kpi-value">{{ formatCurrency(kpi?.total_sales) }}</div>
        <div class="kpi-trend up">{{ yoyGrowthText }}</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">{{ $t('biSalesAnalysis.kpi.orderCount') }}</div>
        <div class="kpi-value">{{ kpi?.order_count ?? '—' }}</div>
        <div class="kpi-trend up">{{ momGrowthText }}</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">{{ $t('biSalesAnalysis.kpi.customerCount') }}</div>
        <div class="kpi-value">{{ kpi?.customer_count ?? '—' }}</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">{{ $t('biSalesAnalysis.kpi.avgOrderValue') }}</div>
        <div class="kpi-value">{{ formatCurrency(kpi?.avg_order_value) }}</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">{{ $t('biSalesAnalysis.kpi.grossMargin') }}</div>
        <div class="kpi-value">{{ profit?.gross_margin?.toFixed(1) ?? '—' }}%</div>
      </el-card>
    </div>

    <!-- 2. 销售趋势 -->
    <el-card class="chart-card">
      <div ref="trendChartRef" style="width: 100%; height: 320px"></div>
    </el-card>

    <!-- 3. 客户排行 + 产品分布 -->
    <el-row :gutter="16">
      <el-col :span="12">
        <el-card class="chart-card">
          <div ref="customerChartRef" style="width: 100%; height: 360px"></div>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card class="chart-card">
          <div ref="productChartRef" style="width: 100%; height: 360px"></div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 4. 区域热力 -->
    <el-card class="chart-card">
      <div ref="regionChartRef" style="width: 100%; height: 320px"></div>
    </el-card>

    <!-- 5. 月度钻取 -->
    <el-card class="chart-card">
      <h3>{{ $t('biSalesAnalysis.monthly.title', { year: 2026 }) }}</h3>
      <el-table
        :data="monthlyData"
        stripe
        :aria-label="$t('biSalesAnalysis.monthly.tableAriaLabel')"
      >
        <el-table-column prop="period" :label="$t('biSalesAnalysis.monthly.period')" width="120" />
        <el-table-column prop="total_amount" :label="$t('biSalesAnalysis.monthly.totalAmount')">
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column
          prop="order_count"
          :label="$t('biSalesAnalysis.monthly.orderCount')"
          width="120"
        />
        <el-table-column
          prop="quantity"
          :label="$t('biSalesAnalysis.monthly.quantity')"
          width="120"
        />
        <el-table-column prop="profit_amount" :label="$t('biSalesAnalysis.monthly.profitAmount')">
          <template #default="{ row }">{{ formatCurrency(row.profit_amount) }}</template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 6. 品类统计 -->
    <el-card class="chart-card">
      <div class="section-toolbar">
        <h3>品类统计</h3>
        <el-button
          type="primary"
          plain
          size="small"
          :loading="categoryLoading"
          @click="loadCategory"
        >
          查询品类统计
        </el-button>
      </div>
      <el-table v-if="categories.length" :data="categories" stripe>
        <el-table-column prop="category" label="品类" min-width="140" />
        <el-table-column prop="total_amount" label="销售额">
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column prop="percentage" label="占比 %" width="120" />
      </el-table>
    </el-card>

    <!-- 7. 钻取中心：月→日 / 客户→订单 / 产品→订单 -->
    <el-card class="chart-card">
      <h3>数据钻取</h3>
      <div class="drill-toolbar">
        <el-input v-model="monthDrill.year" placeholder="年（如 2026）" style="width: 120px" />
        <el-input v-model="monthDrill.month" placeholder="月（1-12）" style="width: 120px" />
        <el-button type="primary" plain @click="loadMonthToDay">钻取 月→日</el-button>
        <el-input v-model="customerDrillId" placeholder="客户ID" style="width: 120px" />
        <el-button type="primary" plain @click="loadCustomerToOrder">钻取 客户→订单</el-button>
        <el-input v-model="productDrillId" placeholder="产品ID" style="width: 120px" />
        <el-button type="primary" plain @click="loadProductToOrder">钻取 产品→订单</el-button>
      </div>

      <el-table v-if="monthToDayData.length" :data="monthToDayData" stripe class="drill-table">
        <el-table-column prop="period" label="日期" width="130" />
        <el-table-column prop="total_amount" label="销售额">
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column prop="order_count" label="订单数" width="110" />
        <el-table-column prop="quantity" label="数量" width="110" />
      </el-table>

      <el-table v-if="orderDrillData.length" :data="orderDrillData" stripe class="drill-table">
        <el-table-column prop="order_no" label="订单号" min-width="150" />
        <el-table-column prop="order_date" label="订单日期" width="120" />
        <el-table-column prop="total_amount" label="订单金额">
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column prop="quantity" label="数量" width="110" />
        <el-table-column prop="status" label="状态" width="110" />
      </el-table>
    </el-card>

    <!-- 8. 多维分析：切片 / 切块 / 上卷 / 透视 -->
    <el-card class="chart-card">
      <h3>多维分析</h3>
      <div class="drill-toolbar">
        <el-input v-model="multiForm.dimension" placeholder="切片维度" style="width: 130px" />
        <el-input
          v-model="multiForm.filters"
          placeholder='过滤条件 JSON，如 {"region":"华东"}'
          style="width: 260px"
        />
        <el-button type="primary" plain @click="runSlice">切片</el-button>
        <el-button type="primary" plain @click="runDice">切块</el-button>
        <el-input v-model="rollupForm.from" placeholder="上卷起始维度" style="width: 130px" />
        <el-input v-model="rollupForm.to" placeholder="目标维度" style="width: 130px" />
        <el-button type="primary" plain @click="runRollup">上卷</el-button>
      </div>
      <div class="drill-toolbar">
        <el-input v-model="pivotForm.row" placeholder="行维度" style="width: 130px" />
        <el-input v-model="pivotForm.col" placeholder="列维度" style="width: 130px" />
        <el-input
          v-model="pivotForm.measure"
          placeholder="度量（如 total_amount）"
          style="width: 200px"
        />
        <el-button type="primary" plain @click="runPivot">透视</el-button>
      </div>
      <pre v-if="multiResult" class="multi-result">{{ multiResult }}</pre>
    </el-card>
  </div>
</template>

<style scoped>
.bi-sales-analysis {
  padding: 20px;
}
.bi-sales-analysis h2 {
  margin-bottom: 20px;
}
.kpi-row {
  display: flex;
  gap: 16px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}
.kpi-card {
  flex: 1;
  min-width: 180px;
  text-align: center;
}
.kpi-label {
  color: #909399;
  font-size: 14px;
}
.kpi-value {
  font-size: 24px;
  font-weight: bold;
  color: #303133;
  margin: 8px 0;
}
.kpi-trend.up {
  color: #67c23a;
  font-size: 12px;
}
.kpi-trend.down {
  color: #f56c6c;
  font-size: 12px;
}
.chart-card {
  margin-bottom: 16px;
}
.section-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.section-toolbar h3 {
  margin: 0;
}
.drill-toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.drill-table {
  margin-top: 8px;
}
.multi-result {
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 12px;
  font-size: 12px;
  max-height: 360px;
  overflow: auto;
  white-space: pre-wrap;
  margin: 0;
}
</style>
