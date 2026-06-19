/**
 * prRtnFmts.ts - 采购退货格式化工具
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 * 提供采购退货相关的状态文本/类型映射、金额格式化等工具函数
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 采购退货状态类型映射（用于 el-tag :type）
 */
export const getStatusType = (status: string): string => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'success',
    rejected: 'danger',
    completed: 'success',
  }
  return map[status] || 'info'
}

/**
 * 采购退货状态文本映射（用于 el-tag 显示）
 */
export const getStatusText = (status: string): string => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审批',
    approved: '已审批',
    rejected: '已拒绝',
    completed: '已完成',
  }
  return map[status] || status
}
