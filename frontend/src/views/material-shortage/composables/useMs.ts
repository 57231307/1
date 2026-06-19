/**
 * useMs.ts - 物料短缺核心 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 material-shortage/index.vue）
 * 提供汇总 / 列表 / 分页 / 过滤等核心方法
 * 业务流程（触发检查 / 通知 / 解决 / 筛选）由 useMsProc 提供
 * 行为完全保持一致（仅结构重构）
 *
 * 注意：返回值使用 reactive({...}) 包装，父组件可直接访问字段（自动解包 ref）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import {
  materialShortageApi,
  type MaterialShortageSummary,
  type MaterialShortage,
} from '@/api/material-shortage'
import { logger } from '@/utils/logger'

/**
 * 物料短缺主业务 composable
 * 集中管理汇总、列表、分页、过滤
 */
export function useMs() {
  // 分页
  const currentPage = ref(1)
  const pageSize = ref(10)
  const total = ref(0)

  // 过滤
  const filterSeverity = ref('')
  const filterStatus = ref('')

  // 加载状态
  const tableLoading = ref(false)
  const checking = ref(false)

  // 数据
  const summary = ref<MaterialShortageSummary>({} as MaterialShortageSummary)
  const shortageList = ref<MaterialShortage[]>([])

  /**
   * 加载汇总
   */
  const fetchSummary = async () => {
    try {
      const res = await materialShortageApi.getSummary()
      summary.value = (res.data || {}) as MaterialShortageSummary
    } catch (error) {
      const msg = error instanceof Error ? error.message : '获取缺料汇总失败'
      logger.error(msg)
      ElMessage.error(msg)
      summary.value = {} as MaterialShortageSummary
    }
  }

  /**
   * 加载缺料列表
   */
  const fetchShortages = async () => {
    tableLoading.value = true
    try {
      const params: Record<string, unknown> = {
        page: currentPage.value,
        page_size: pageSize.value,
      }
      if (filterSeverity.value) params.severity = filterSeverity.value
      if (filterStatus.value) params.status = filterStatus.value
      const res = await materialShortageApi.listShortages(params)
      const data = res.data
      shortageList.value = data ? data.list : []
      total.value = data ? data.total : 0
    } catch (error) {
      const msg = error instanceof Error ? error.message : '获取缺料列表失败'
      logger.error(msg)
      ElMessage.error(msg)
      shortageList.value = []
      total.value = 0
    } finally {
      tableLoading.value = false
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
    // 加载状态
    tableLoading,
    checking,
    // 数据
    summary,
    shortageList,
    // 加载方法
    fetchSummary,
    fetchShortages,
    // 懒加载标记
    hasLoaded,
    // 兼容旧名
    loadIfNot,
  })
}
