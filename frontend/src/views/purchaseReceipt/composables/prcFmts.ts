/**
 * prcFmts.ts - 采购入库格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 purchaseReceipt/index.vue）
 * 提供状态标签/状态 css 类等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 状态选项
 */
const STATUS_OPTIONS: { label: string; value: string }[] = [
  { label: '全部', value: '' },
  { label: '草稿', value: 'draft' },
  { label: '已审核', value: 'approved' },
]

/**
 * 获取入库单状态中文标签
 */
export const getStatusLabel = (value: string): string => {
  return STATUS_OPTIONS.find(s => s.value === value)?.label || value
}

/**
 * 获取入库单状态 css 类名
 */
export const getStatusClass = (value: string): string => {
  return value === 'draft' ? 'status-draft' : 'status-approved'
}

/**
 * 暴露状态选项
 */
export { STATUS_OPTIONS }
