/**
 * schMFmts.ts - 排产管理格式化工具
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
 * 提供排产管理相关的时间格式化、状态/优先级文本/类型映射工具
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 格式化日期时间（"YYYY-MM-DDTHH:mm:ss..." → "YYYY-MM-DD HH:mm"）
 */
export const formatDateTime = (t: string): string => {
  if (!t) return '-'
  return t.replace('T', ' ').slice(0, 16)
}

/**
 * 排产任务状态类型映射（用于 el-tag :type）
 */
export const getStatusType = (status: string): string => {
  const map: Record<string, string> = {
    pending: 'info',
    scheduled: 'primary',
    running: 'warning',
    completed: 'success',
    conflict: 'danger',
  }
  return map[status] || 'info'
}

/**
 * 排产任务状态文本映射（用于 el-tag 显示）
 */
export const getStatusLabel = (status: string): string => {
  const map: Record<string, string> = {
    pending: '待排程',
    scheduled: '已排程',
    running: '生产中',
    completed: '已完成',
    conflict: '冲突',
  }
  return map[status] || status
}

/**
 * 排产任务优先级类型映射（用于 el-tag :type）
 */
export const getPriorityType = (priority: number): string => {
  if (priority === 1) return 'danger'
  if (priority === 2) return 'warning'
  return 'info'
}
