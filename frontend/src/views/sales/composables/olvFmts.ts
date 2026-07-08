/**
 * olvFmts.ts - 销售订单列表格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 sales/views/OrderListView.vue）
 * 提供状态类型/标签/金额格式化等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** el-tag 类型联合（与 element-plus TagType 对齐） */
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger'

/** 状态 → el-tag 类型 */
const STATUS_TYPE_MAP: Record<string, TagType> = {
  pending: 'warning',
  approved: 'primary',
  shipped: 'success',
  completed: 'info',
  cancelled: 'danger',
}

/** 状态 → 中文标签 */
const STATUS_TEXT_MAP: Record<string, string> = {
  pending: '待审批',
  approved: '已审批',
  shipped: '已发货',
  completed: '已完成',
  cancelled: '已取消',
}

/** 获取销售订单状态 el-tag 类型 */
export const getStatusType = (status: string): TagType => STATUS_TYPE_MAP[status] || 'info'

/** 获取销售订单状态中文标签 */
export const getStatusText = (status: string) => STATUS_TEXT_MAP[status] || status

/** 格式化金额（人民币 + 千分位） */
export const formatAmount = (value: number) => `¥${(value || 0).toLocaleString()}`
