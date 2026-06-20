/**
 * srFmts.ts - 销售退货格式化工具
 * 任务编号: P14 批 2 I-3 第 7 批（拆分原 sales-returns/index.vue）
 * 提供退货状态映射、货币格式化等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** 状态 → el-tag 类型 */
const STATUS_TYPE_MAP: Record<string, string> = {
  PENDING: 'warning',
  APPROVED: 'success',
  REJECTED: 'danger',
  COMPLETED: 'info',
}

/** 状态 → 中文标签 */
const STATUS_LABEL_MAP: Record<string, string> = {
  PENDING: '待审核',
  APPROVED: '已通过',
  REJECTED: '已拒绝',
  COMPLETED: '已完成',
}

/** 获取退货状态 el-tag 类型 */
export const getStatusType = (status: string) => STATUS_TYPE_MAP[status] || 'info'

/** 获取退货状态中文标签 */
export const getStatusLabel = (status: string) => STATUS_LABEL_MAP[status] || status

/** 格式化退货金额 */
export const formatAmount = (value: number) => {
  return value !== undefined && value !== null ? `¥${Number(value).toFixed(2)}` : '¥0.00'
}
