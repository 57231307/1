/**
 * useArChart.ts - AR 对账账龄图表 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供 ECharts 柱状图与饼图的初始化、配置、销毁等方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, nextTick } from 'vue'
import * as echarts from 'echarts'
import { ElMessage } from 'element-plus'
import { i18n } from '@/i18n'
import { getAgingAnalysis, type AgingAnalysisResult, type AgingBucket } from '@/api/ar-reconciliation-enhanced'
import { AGING_COLORS } from './arRecFmts'

const t = i18n.global.t.bind(i18n.global)

/**
 * 账龄分析图表 composable
 */
export function useArChart() {
  const agingData = ref<AgingAnalysisResult[]>([])
  const chartRef = ref<HTMLDivElement | null>(null)
  const pieChartRef = ref<HTMLDivElement | null>(null)
  let barChart: echarts.ECharts | null = null
  let pieChart: echarts.ECharts | null = null

  /** 默认账龄分桶（无数据时填充，label 为 i18n key） */
  const DEFAULT_BUCKETS = [
    { label: 'arReconciliationModule.agingBucket030', range: '0-30', amount: 0, percentage: 0, count: 0 },
    { label: 'arReconciliationModule.agingBucket3160', range: '31-60', amount: 0, percentage: 0, count: 0 },
    { label: 'arReconciliationModule.agingBucket6190', range: '61-90', amount: 0, percentage: 0, count: 0 },
    { label: 'arReconciliationModule.agingBucket90Plus', range: '90+', amount: 0, percentage: 0, count: 0 },
  ]

  const loadAgingAnalysis = async (endDate?: string) => {
    try {
      // v11 批次 182 P2-1 修复：const res: any 改为 as 具体类型
      const res = (await getAgingAnalysis({
        customer_id: undefined,
        as_of_date: endDate || undefined,
      })) as { data?: AgingAnalysisResult[] }
      agingData.value = res.data || []
      await nextTick()
      renderCharts()
    } catch {
      ElMessage.error(t('arReconciliationModule.loadAgingFailed'))
    }
  }

  const renderCharts = () => {
    if (!chartRef.value || !pieChartRef.value) return
    if (!barChart) {
      barChart = echarts.init(chartRef.value)
    }
    if (!pieChart) {
      pieChart = echarts.init(pieChartRef.value)
    }

    const buckets: AgingBucket[] =
      agingData.value.length > 0 ? agingData.value[0].buckets : DEFAULT_BUCKETS

    const barOption = {
      title: { text: t('arReconciliationModule.agingBarTitle'), left: 'center' },
      tooltip: { trigger: 'axis', formatter: (params: { name: string; value: number }) => `${params.name}: ${params.value} ${t('arReconciliationModule.yuan')}` },
      xAxis: { type: 'category', data: buckets.map((b: AgingBucket) => b.label.startsWith('arReconciliationModule.') ? t(b.label) : b.label) },
      yAxis: { type: 'value', name: t('arReconciliationModule.amountWithUnit') },
      series: [
        {
          type: 'bar',
          data: buckets.map((b: AgingBucket) => b.amount),
          itemStyle: {
            // v11 批次 182 P2-1 修复：(params: any) 改为 echarts 的回调参数类型
            color: (params: { dataIndex: number }) => {
              return AGING_COLORS[params.dataIndex] || '#409eff'
            },
          },
          label: { show: true, position: 'top', formatter: '{c}' },
        },
      ],
      grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    }

    const pieOption = {
      title: { text: t('arReconciliationModule.agingPieTitle'), left: 'center' },
      tooltip: { trigger: 'item', formatter: (params: { name: string; value: number; percent: number }) => `${params.name}: ${params.value}${t('arReconciliationModule.yuan')} (${params.percent}%)` },
      legend: { bottom: '0%' },
      series: [
        {
          type: 'pie',
          radius: ['40%', '70%'],
          avoidLabelOverlap: false,
          itemStyle: { borderRadius: 10, borderColor: '#fff', borderWidth: 2 },
          label: { show: true, formatter: '{b}: {d}%' },
          data: buckets.map((b: AgingBucket, i: number) => ({
            value: b.amount,
            name: b.label.startsWith('arReconciliationModule.') ? t(b.label) : b.label,
            itemStyle: { color: AGING_COLORS[i] },
          })),
        },
      ],
    }

    barChart.setOption(barOption)
    pieChart.setOption(pieOption)
  }

  const disposeCharts = () => {
    barChart?.dispose()
    barChart = null
    pieChart?.dispose()
    pieChart = null
  }

  return {
    chartRef,
    pieChartRef,
    agingData,
    loadAgingAnalysis,
    renderCharts,
    disposeCharts,
  }
}
