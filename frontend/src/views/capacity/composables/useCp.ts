// capacity 主业务 composable
// 拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：产能管理（summary/trend/workCenters/bottlenecks + 分页 + 过滤）
// 行为完全保持一致（仅结构重构）
// 批次 288：workCenters 接入 useTableApi，移除手写分页逻辑
import { reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { getCapacitySummary, getCapacityTrend, getCapacityBottlenecks } from '@/api/capacity'
import type { CapacitySummary, WorkCenter, CapacityTrend } from '@/api/capacity'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

/** capacity 主业务 composable（返回 reactive 包装的字段，父组件可直接 .字段 解包） */
export const useCp = () => {
  // 日期范围 + 趋势天数 + 工作中心筛选
  const dateRange = ref<[Date, Date] | null>(null)
  const trendDays = ref(7)
  const selectedWorkCenter = ref<number | undefined>(undefined)

  // 列表数据接入 useTableApi
  // 工作中心 API 返回 ApiResponse<PageResult<WorkCenter>>，
  // PageResult 包含 list + total 字段，useTableApi detectList 检测 list（默认 listKey='list'）
  const {
    data: workCenters,
    total,
    loading: tableLoading,
    page: currentPage,
    pageSize,
    queryParams,
    refresh: fetchWorkCenters,
  } = useTableApi<WorkCenter>({
    url: '/capacity/work-centers',
    defaultPageSize: 10,
    onError: (err: unknown) => {
      logger.error('获取工作中心列表失败:', err)
      ElMessage.error('获取工作中心列表失败')
    },
  })

  // 业务数据
  const summary = ref<CapacitySummary>({} as CapacitySummary)
  const bottlenecks = ref<WorkCenter[]>([])
  const bottleneckLoading = ref(false)

  // 趋势数据（传给 CpTrend 用于渲染 ECharts）
  const trendData = ref<CapacityTrend[]>([])

  // 获取概览
  const fetchSummary = async () => {
    try {
      const res = await getCapacitySummary()
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) summary.value = res.data
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '获取产能概览失败')
      summary.value = {} as CapacitySummary
    }
  }

  // 获取趋势数据
  const fetchTrendData = async () => {
    try {
      const res = await getCapacityTrend({
        days: trendDays.value,
        work_center_id: selectedWorkCenter.value,
      })
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) trendData.value = res.data
    } catch (error: unknown) {
      ElMessage.error((error instanceof Error ? error.message : '') || '获取产能趋势失败')
    }
  }

  // 获取瓶颈分析
  const fetchBottlenecks = async () => {
    bottleneckLoading.value = true
    try {
      const res = await getCapacityBottlenecks()
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (res.data) bottlenecks.value = res.data
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

  // 初始化挂载：工作中心列表由 useTableApi setup 自动加载，此处仅拉取辅助数据
  const initOnMount = async () => {
    await Promise.all([fetchSummary(), fetchTrendData(), fetchBottlenecks()])
  }

  return reactive({
    dateRange,
    trendDays,
    selectedWorkCenter,
    currentPage,
    pageSize,
    total,
    queryParams,
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
