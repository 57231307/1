/**
 * lgsFmts.ts - 物流管理格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 logistics/index.vue）
 * 提供状态类型/文本映射、运费格式化等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** el-tag 类型联合（与 element-plus TagProps.type 对齐） */
type TagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'

/**
 * 状态对应的 el-tag 类型
 */
const STATUS_TYPE_MAP: Record<string, TagType> = {
  pending: 'info',
  shipped: 'warning',
  in_transit: 'primary',
  delivered: 'success',
  cancelled: 'danger',
}

/**
 * 获取运单状态 el-tag 类型
 */
export const getStatusType = (status: string): TagType => {
  return STATUS_TYPE_MAP[status] || 'info'
}

/**
 * 状态中文文本
 */
const STATUS_TEXT_MAP: Record<string, string> = {
  pending: '待发货',
  shipped: '已发货',
  in_transit: '运输中',
  delivered: '已签收',
  cancelled: '已取消',
}

/**
 * 获取运单状态中文文本
 */
export const getStatusText = (status: string): string => {
  return STATUS_TEXT_MAP[status] || status
}

/**
 * 格式化运费显示（带人民币符号）
 */
export const formatFreight = (fee: number | undefined | null): string => {
  return `¥${fee || 0}`
}
