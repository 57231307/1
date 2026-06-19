/**
 * bpmApFmts.ts - BPM 审批格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 bpm/approval.vue）
 * 提供优先级映射 / 节点状态 / 节点类型 / 逾期判断等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 优先级类型（el-tag type）
 */
const PRIORITY_TYPE_MAP: Record<string, string> = {
  high: 'danger',
  medium: 'warning',
  low: 'info',
}

/**
 * 优先级文本
 */
const PRIORITY_TEXT_MAP: Record<string, string> = {
  high: '高',
  medium: '中',
  low: '低',
}

/**
 * 获取优先级对应的 el-tag 类型
 */
export const getPriorityType = (priority: string): string => {
  return PRIORITY_TYPE_MAP[priority] || 'info'
}

/**
 * 获取优先级显示文本
 */
export const getPriorityText = (priority: string): string => {
  return PRIORITY_TEXT_MAP[priority] || priority
}

/**
 * 判断是否逾期（截止时间 < 当前时间）
 */
export const isOverdue = (dueDate: string): boolean => {
  return new Date(dueDate) < new Date()
}

/**
 * 节点状态对应的 css 类名
 */
const NODE_STATUS_CLASS_MAP: Record<string, string> = {
  pending: 'status-pending',
  approved: 'status-approved',
  rejected: 'status-rejected',
  skipped: 'status-skipped',
}

/**
 * 获取审批链节点状态 css 类
 */
export const getNodeStatusClass = (status: string): string => {
  return NODE_STATUS_CLASS_MAP[status] || ''
}

/**
 * 节点类型中文名
 */
const NODE_TYPE_NAME_MAP: Record<string, string> = {
  start: '开始',
  end: '结束',
  approval: '审批',
  condition: '条件',
  notify: '通知',
}

/**
 * 获取审批链节点类型中文名
 */
export const getNodeTypeName = (type: string): string => {
  return NODE_TYPE_NAME_MAP[type] || type
}
