/**
 * vchrLstFmts.ts - 凭证列表格式化工具
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 VoucherListTab.vue）
 * 提供状态标签/类型映射/格式化金额/凭证类型选项等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** 状态 → 中文标签 */
const STATUS_LABEL_MAP: Record<string, string> = {
  draft: '草稿',
  approved: '已审核',
  posted: '已记账',
}

/** 状态 → CSS 类名（保留原版 class） */
const STATUS_CLASS_MAP: Record<string, string> = {
  draft: 'status-draft',
  approved: 'status-approved',
  posted: 'status-posted',
}

/** 凭证类型 → 中文标签（通用/自定义） */
const TYPE_LABEL_MAP: Record<string, string> = {
  general: '通用',
  customized: '自定义',
}

/** 状态过滤下拉选项 */
export const STATUS_OPTIONS = [
  { label: '全部', value: '' },
  { label: '草稿', value: 'draft' },
  { label: '已审核', value: 'approved' },
  { label: '已记账', value: 'posted' },
]

/** 获取状态中文标签 */
export const getStatusLabel = (value: string) => STATUS_LABEL_MAP[value] || value

/** 获取状态 CSS 类名 */
export const getStatusClass = (value: string) => STATUS_CLASS_MAP[value] || ''

/** 获取凭证类型中文标签（兼容 general/其它） */
export const getTypeLabel = (type: string) => {
  if (TYPE_LABEL_MAP[type]) return TYPE_LABEL_MAP[type]
  return type
}

/** 金额格式化（保留 2 位小数） */
export const formatAmount = (amount: number | null | undefined) => {
  return (amount ?? 0).toFixed(2)
}
