/**
 * pcFmts.ts - 采购合同格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-contract/index.vue）
 * 提供状态标签/类型映射、货币格式化等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** 状态 → el-tag 类型 */
const STATUS_TYPE_MAP: Record<string, string> = {
  draft: 'info',
  pending: 'warning',
  active: 'success',
  completed: 'success',
  cancelled: 'danger',
}

/** 状态 → 中文标签 */
const STATUS_LABEL_MAP: Record<string, string> = {
  draft: '草稿',
  pending: '待审批',
  active: '已生效',
  completed: '已完成',
  cancelled: '已取消',
}

/** 获取采购合同状态 el-tag 类型 */
export const getStatusType = (status: string) => STATUS_TYPE_MAP[status] || 'info'

/** 获取采购合同状态中文标签 */
export const getStatusLabel = (status: string) => STATUS_LABEL_MAP[status] || status

/** 格式化货币（人民币 2 位精度） */
export const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(2)}` : '¥0.00'
}
