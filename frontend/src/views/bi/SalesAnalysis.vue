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
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import * as echarts from 'echarts'
import {
  getKpiSummary,
  getSalesTrend,
  getSalesByCustomer,
  getSalesByProduct,
  getSalesByRegion,
  getProfitAnalysis,
  getDrilldownYearToMonth,
} from '@/api/bi'
import type { KpiSummary, TimeSeriesPoint, CustomerRank, ProductRank, RegionStat, ProfitAnalysis } from '@/api/bi'

const kpi = ref<KpiSummary | null>(null)
const trend = ref<TimeSeriesPoint[]>([])
const customers = ref<CustomerRank[]>([])
const products = ref<ProductRank[]>([])
const regions = ref<RegionStat[]>([])
const profit = ref<ProfitAnalysis | null>(null)
const monthlyData = ref<TimeSeriesPoint[]>([])

const trendChartRef = ref<HTMLDivElement>()
const customerChartRef = ref<HTMLDivElement>()
const productChartRef = ref<HTMLDivElement>()
const regionChartRef = ref<HTMLDivElement>()

let trendChart: echarts.ECharts | null = null
let customerChart: echarts.ECharts | null = null
let productChart: echarts.ECharts | null = null
let regionChart: echarts.ECharts | null = null

async function loadAll() {
  try {
    const [k, t, c, p, r, prof] = await Promise.all([
      getKpiSummary(),
      getSalesTrend(30),
      getSalesByCustomer(10),
      getSalesByProduct(10),
      getSalesByRegion(),
      getProfitAnalysis(),
    ])
    kpi.value = k.data
    trend.value = t.data
    customers.value = c.data
    products.value = p.data
    regions.value = r.data
    profit.value = prof.data

    // 钻取 2026 年 → 月
    const monthly = await getDrilldownYearToMonth(2026)
    monthlyData.value = monthly.data

    renderCharts()
  } catch (e) {
    ElMessage.error('加载 BI 数据失败')
    console.error(e)
  }
}

function renderCharts() {
  // 1. 销售趋势
  if (trendChartRef.value) {
    trendChart = echarts.init(trendChartRef.value)
    trendChart.setOption({
      title: { text: '销售趋势（最近 30 天）', left: 'center' },
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'category', data: trend.value.map((p) => p.period) },
      yAxis: { type: 'value' },
      series: [
        { name: '销售额', data: trend.value.map((p) => p.total_amount), type: 'line', smooth: true, itemStyle: { color: '#409EFF' } },
        { name: '利润', data: trend.value.map((p) => p.profit_amount), type: 'line', smooth: true, itemStyle: { color: '#67C23A' } },
      ],
    })
  }

  // 2. 客户排行
  if (customerChartRef.value) {
    customerChart = echarts.init(customerChartRef.value)
    customerChart.setOption({
      title: { text: '客户销售排行（Top 10）', left: 'center' },
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'value' },
      yAxis: { type: 'category', data: customers.value.map((c) => c.customer_name).reverse() },
      series: [
        { type: 'bar', data: customers.value.map((c) => c.total_amount).reverse(), itemStyle: { color: '#E6A23C' } },
      ],
    })
  }

  // 3. 产品分布
  if (productChartRef.value) {
    productChart = echarts.init(productChartRef.value)
    productChart.setOption({
      title: { text: '产品销售分布', left: 'center' },
      tooltip: { trigger: 'item' },
      series: [
        {
          name: '销售额',
          type: 'pie',
          radius: '50%',
          data: products.value.map((p) => ({ name: p.product_name, value: p.total_amount })),
        },
      ],
    })
  }

  // 4. 区域热力
  if (regionChartRef.value) {
    regionChart = echarts.init(regionChartRef.value)
    regionChart.setOption({
      title: { text: '区域销售分布', left: 'center' },
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'category', data: regions.value.map((r) => r.region) },
      yAxis: { type: 'value' },
      series: [
        { type: 'bar', data: regions.value.map((r) => r.total_amount), itemStyle: { color: '#F56C6C' } },
      ],
    })
  }
}

function formatCurrency(n: number | undefined) {
  if (n === undefined) return '—'
  return `¥${n.toLocaleString('zh-CN', { maximumFractionDigits: 2 })}`
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', resizeCharts)
})

onBeforeUnmount(() => {
  // 清理 window resize 监听器，防止组件卸载后内存泄漏
  // 多次进入 BI 销售分析页面时，旧的 listener 不释放会持续累积，导致内存占用线性增长
  window.removeEventListener('resize', resizeCharts)
})

function resizeCharts() {
  trendChart?.resize()
  customerChart?.resize()
  productChart?.resize()
  regionChart?.resize()
}
</script>

<template>
  <div class="bi-sales-analysis">
    <h2>BI 销售多维分析（P3-4 关键路径 demo）</h2>

    <!-- 1. KPI 概览 -->
    <div class="kpi-row">
      <el-card class="kpi-card">
        <div class="kpi-label">总销售额</div>
        <div class="kpi-value">{{ formatCurrency(kpi?.total_sales) }}</div>
        <div class="kpi-trend up">同比 +{{ kpi?.yoy_growth?.toFixed(1) }}%</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">订单数</div>
        <div class="kpi-value">{{ kpi?.order_count ?? '—' }}</div>
        <div class="kpi-trend up">环比 +{{ kpi?.mom_growth?.toFixed(1) }}%</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">客户数</div>
        <div class="kpi-value">{{ kpi?.customer_count ?? '—' }}</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">客单价</div>
        <div class="kpi-value">{{ formatCurrency(kpi?.avg_order_value) }}</div>
      </el-card>
      <el-card class="kpi-card">
        <div class="kpi-label">毛利率</div>
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
      <h3>2026 年月度销售（钻取：年 → 月）</h3>
      <el-table :data="monthlyData" stripe>
        <el-table-column prop="period" label="月份" width="120" />
        <el-table-column prop="total_amount" label="销售额">
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column prop="order_count" label="订单数" width="120" />
        <el-table-column prop="quantity" label="数量" width="120" />
        <el-table-column prop="profit_amount" label="利润">
          <template #default="{ row }">{{ formatCurrency(row.profit_amount) }}</template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<style scoped>
.bi-sales-analysis { padding: 20px; }
.bi-sales-analysis h2 { margin-bottom: 20px; }
.kpi-row { display: flex; gap: 16px; margin-bottom: 20px; flex-wrap: wrap; }
.kpi-card { flex: 1; min-width: 180px; text-align: center; }
.kpi-label { color: #909399; font-size: 14px; }
.kpi-value { font-size: 24px; font-weight: bold; color: #303133; margin: 8px 0; }
.kpi-trend.up { color: #67c23a; font-size: 12px; }
.kpi-trend.down { color: #f56c6c; font-size: 12px; }
.chart-card { margin-bottom: 16px; }
</style>
