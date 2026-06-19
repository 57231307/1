/**
 * piFmts.ts - 采购验货格式化工具
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 * 包含状态/结果的类型与文本映射
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 检验单状态到 el-tag 类型
 */
export const STATUS_TYPE_MAP: Record<string, string> = {
  draft: 'info',
  pending: 'warning',
  completed: 'success',
  rejected: 'danger',
}

/**
 * 检验单状态到中文标签
 */
export const STATUS_TEXT_MAP: Record<string, string> = {
  draft: '草稿',
  pending: '待检验',
  completed: '已完成',
  rejected: '已拒绝',
}

/**
 * 检验结果到 el-tag 类型
 */
export const RESULT_TYPE_MAP: Record<string, string> = {
  pass: 'success',
  fail: 'danger',
  partial: 'warning',
}

/**
 * 检验结果到中文标签
 */
export const RESULT_TEXT_MAP: Record<string, string> = {
  pass: '合格',
  fail: '不合格',
  partial: '部分合格',
}

/**
 * 获取状态类型
 */
export function getStatusType(status: string): string {
  return STATUS_TYPE_MAP[status] || 'info'
}

/**
 * 获取状态文本
 */
export function getStatusText(status: string): string {
  return STATUS_TEXT_MAP[status] || status
}

/**
 * 获取结果类型
 */
export function getResultType(result: string): string {
  return RESULT_TYPE_MAP[result] || 'info'
}

/**
 * 获取结果文本
 */
export function getResultText(result: string): string {
  return RESULT_TEXT_MAP[result] || result
}
