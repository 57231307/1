// capacity 主业务 composable
// 拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：产能管理（summary/trend/workCenters/bottlenecks + 分页 + 过滤）
// 行为完全保持一致（仅结构重构）
import { reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { capacityApi } from '@/api/capacity'
import type { CapacitySummary, WorkCenter, CapacityTrend } from '@/api/capacity'

/** capacity 主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useCp = () => {
  // 日期范围 + 趋势天数 + 工作中心筛选
  const dateRange = ref<[Date, Date] | null>(null)
  const trendDays = ref(7)
  const selectedWorkCenter = ref<number | undefined>(undefined)

  // 分页
  const currentPage = ref(1)
  const pageSize = ref(10)
  const total = ref(0)

  // 业务数据
  const summary = ref<CapacitySummary>({} as CapacitySummary)
  const workCenters = ref<WorkCenter[]>([])
  const bottlenecks = ref<WorkCenter[]>([])
  const tableLoading = ref(false)
  const bottleneckLoading = ref(false)

  // 趋势数据（传给 CpTrend 用于渲染 ECharts）
  const trendData = ref<CapacityTrend[]>([])

  // 获取概览
  const fetchSummary = async () => {
    try {
      const res = await capacityApi.getSummary()
      summary.value = res.data!
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '获取产能概览失败')
      summary.value = {} as CapacitySummary
    }
  }

  // 获取趋势数据
  const fetchTrendData = async () => {
    try {
      const res = await capacityApi.getTrend({
        days: trendDays.value,
        work_center_id: selectedWorkCenter.value,
      })
      trendData.value = res.data!
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '获取产能趋势失败')
    }
  }

  // 获取工作中心列表
  const fetchWorkCenters = async () => {
    tableLoading.value = true
    try {
      const res = await capacityApi.listWorkCenters({
        page: currentPage.value,
        page_size: pageSize.value,
      })
      workCenters.value = res.data!.list
      total.value = res.data!.total
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '获取工作中心列表失败')
      workCenters.value = []
      total.value = 0
    } finally {
      tableLoading.value = false
    }
  }

  // 获取瓶颈分析
  const fetchBottlenecks = async () => {
    bottleneckLoading.value = true
    try {
      const res = await capacityApi.getBottlenecks()
      bottlenecks.value = res.data!
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '获取瓶颈分析失败')
      bottlenecks.value = []
    } finally {
      bottleneckLoading.value = false
    }
  }

  // 日期变化 → 重新拉取趋势
  const handleDateChange = () => {
    fetchTrendData()
  }

  // 工作中心变化 → 重新拉取趋势
  const handleWorkCenterChange = () => {
    fetchTrendData()
  }

  // 趋势天数变化 → 重新拉取趋势
  const handleTrendDaysChange = () => {
    fetchTrendData()
  }

  // 初始化挂载：拉取全部数据
  const initOnMount = async () => {
    await Promise.all([fetchSummary(), fetchTrendData(), fetchWorkCenters(), fetchBottlenecks()])
  }

  return reactive({
    dateRange,
    trendDays,
    selectedWorkCenter,
    currentPage,
    pageSize,
    total,
    summary,
    workCenters,
    bottlenecks,
    tableLoading,
    bottleneckLoading,
    trendData,
    fetchSummary,
    fetchTrendData,
    fetchWorkCenters,
    fetchBottlenecks,
    handleDateChange,
    handleWorkCenterChange,
    handleTrendDaysChange,
    initOnMount,
  })
}
