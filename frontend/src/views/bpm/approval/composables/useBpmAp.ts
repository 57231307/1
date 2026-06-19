/**
 * useBpmAp.ts - BPM 审批核心 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 提供待办/已办任务列表查询、分页、统计等核心方法
 * 业务流程（审批 / 转交 / 审批链）由 useBpmApProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { bpmEnhancedApi, type ApprovalTask } from '@/api/bpm-enhanced'
import { logger } from '@/utils/logger'
import { isOverdue } from './bpmApFmts'

/**
 * 审批主业务 composable
 * 集中管理待办/已办任务列表、分页、统计
 * 对话框状态由 useBpmApProc 单独管理
 */
export function useBpmAp() {
  // 统计
  const stats = reactive({ pending: 0, completed: 0, urgent: 0, avgTime: 0 })

  // 待办任务
  const pendingLoading = ref(false)
  const pendingTasks = ref<ApprovalTask[]>([])
  const pendingPagination = reactive({ page: 1, page_size: 10, total: 0 })

  // 已办任务
  const completedLoading = ref(false)
  const completedTasks = ref<ApprovalTask[]>([])
  const completedPagination = reactive({ page: 1, page_size: 10, total: 0 })

  /** 获取待办任务 */
  const fetchPendingTasks = async () => {
    pendingLoading.value = true
    try {
      const res = await bpmEnhancedApi.getPendingTasks({
        page: pendingPagination.page,
        page_size: pendingPagination.page_size,
      })
      pendingTasks.value = res.data.list
      pendingPagination.total = res.data.total
      stats.pending = res.data.total
      stats.urgent = res.data.list.filter(
        t => t.priority === 'high' && !isOverdue(t.due_date || '')
      ).length
    } catch (e) {
      logger.error(String(e))
    } finally {
      pendingLoading.value = false
    }
  }

  /** 获取已办任务 */
  const fetchCompletedTasks = async () => {
    completedLoading.value = true
    try {
      const res = await bpmEnhancedApi.getCompletedTasks({
        page: completedPagination.page,
        page_size: completedPagination.page_size,
      })
      completedTasks.value = res.data.list
      completedPagination.total = res.data.total
      stats.completed = res.data.total
    } catch (e) {
      logger.error(String(e))
    } finally {
      completedLoading.value = false
    }
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 统计
    stats,
    // 待办
    pendingLoading,
    pendingTasks,
    pendingPagination,
    fetchPendingTasks,
    // 已办
    completedLoading,
    completedTasks,
    completedPagination,
    fetchCompletedTasks,
  })
}
