/**
 * useMs.ts - 物料短缺核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 material-shortage/index.vue）
 * 提供汇总 / 列表 / 分页 / 过滤等核心方法
 * 业务流程（触发检查 / 通知 / 解决 / 筛选）由 useMsProc 提供
 * 行为完全保持一致（仅结构重构）
 * 批次 288：shortageList 接入 useTableApi，移除手写分页逻辑
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import {
  getMaterialShortageSummary,
  type MaterialShortageSummary,
  type MaterialShortage,
} from '@/api/material-shortage'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

/**
 * 物料短缺主业务 composable
 * 集中管理汇总、列表、分页、过滤
 */
export function useMs() {
  // 过滤
  const filterSeverity = ref('')
  const filterStatus = ref('')

  // 加载状态
  const checking = ref(false)

  // 列表数据接入 useTableApi
  // 缺料列表 API 返回 ApiResponse<PageResult<MaterialShortage>>，
  // PageResult 包含 list + total 字段，useTableApi detectList 检测 list（默认 listKey='list'）
  const {
    data: shortageList,
    total,
    loading: tableLoading,
    page: currentPage,
    pageSize,
    queryParams,
    refresh: fetchShortages,
  } = useTableApi<MaterialShortage>({
    url: '/material-shortage/list',
    defaultPageSize: 10,
    defaultParams: {
      severity: '',
      status: '',
    },
    onError: (err: unknown) => {
      logger.error('获取缺料列表失败', err)
      ElMessage.error('获取缺料列表失败')
    },
  })

  // 数据
  const summary = ref<MaterialShortageSummary>({} as MaterialShortageSummary)

  /**
   * 加载汇总
   */
  const fetchSummary = async () => {
    try {
      const res = await getMaterialShortageSummary()
      summary.value = (res.data || {}) as MaterialShortageSummary
    } catch (error) {
      const msg = error instanceof Error ? error.message : '获取缺料汇总失败'
      logger.error(msg)
      ElMessage.error(msg)
      summary.value = {} as MaterialShortageSummary
    }
  }

  /** 同步 filterSeverity/filterStatus 到 queryParams */
  const syncFilterToQuery = () => {
    queryParams.value = {
      ...queryParams.value,
      severity: filterSeverity.value,
      status: filterStatus.value,
    }
  }

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  // 使用 reactive 包装，父组件可直接访问字段
  return reactive({
    // 分页
    currentPage,
    pageSize,
    total,
    // 过滤
    filterSeverity,
    filterStatus,
    queryParams,
    // 加载状态
    tableLoading,
    checking,
    // 数据
    summary,
    shortageList,
    // 加载方法
    fetchSummary,
    fetchShortages,
    syncFilterToQuery,
    // 懒加载标记
    hasLoaded,
    // 兼容旧名
    loadIfNot,
  })
}
