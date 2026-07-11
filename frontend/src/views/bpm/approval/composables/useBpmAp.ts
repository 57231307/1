/**
 * useBpmAp.ts - BPM 审批核心 composable
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 提供待办/已办任务列表查询、分页、统计等核心方法
 * 业务流程（审批 / 转交 / 审批链）由 useBpmApProc 提供
 * 批次 283：pending/completed 2 个表格接入 useTableApi，stats 通过 watch 自动更新
 */
import { reactive, watch } from 'vue'
import { type ApprovalTask } from '@/api/bpm-enhanced'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'
import { isOverdue } from './bpmApFmts'

/**
 * 审批主业务 composable
 * 集中管理待办/已办任务列表、分页、统计
 * 对话框状态由 useBpmApProc 单独管理
 */
export function useBpmAp() {
  // 统计
  const stats = reactive({ pending: 0, completed: 0, urgent: 0, avgTime: 0 })

  // 待办任务 - 接入 useTableApi（批次 283）
  const {
    data: pendingTasks,
    total: pendingTotal,
    loading: pendingLoading,
    page: pendingPage,
    pageSize: pendingPageSize,
    refresh: fetchPendingTasks,
  } = useTableApi<ApprovalTask>({
    url: '/bpm/tasks/pending',
    defaultPageSize: 10,
    onError: (err: unknown) => logger.error(String(err)),
  })

  // 已办任务 - 接入 useTableApi（批次 283）
  const {
    data: completedTasks,
    total: completedTotal,
    loading: completedLoading,
    page: completedPage,
    pageSize: completedPageSize,
    refresh: fetchCompletedTasks,
  } = useTableApi<ApprovalTask>({
    url: '/bpm/tasks/completed',
    defaultPageSize: 10,
    onError: (err: unknown) => logger.error(String(err)),
  })

  // 批次 283：监听待办数据变化更新统计（pending 总数 + urgent 紧急数）
  watch([pendingTasks, pendingTotal], () => {
    stats.pending = pendingTotal.value
    stats.urgent = pendingTasks.value.filter(
      t => t.priority === 'high' && !isOverdue(t.due_date || '')
    ).length
  })

  // 批次 283：监听已办数据变化更新统计（completed 总数）
  watch([completedTasks, completedTotal], () => {
    stats.completed = completedTotal.value
  })

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 统计
    stats,
    // 待办（useTableApi 管理）
    pendingTasks,
    pendingLoading,
    pendingTotal,
    pendingPage,
    pendingPageSize,
    fetchPendingTasks,
    // 已办（useTableApi 管理）
    completedTasks,
    completedLoading,
    completedTotal,
    completedPage,
    completedPageSize,
    fetchCompletedTasks,
  })
}
