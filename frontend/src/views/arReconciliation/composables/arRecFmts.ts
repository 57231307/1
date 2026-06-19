/**
 * arRecFmts.ts - AR 对账共享格式化工具
 * 任务编号: P14 批 1 B3 I-2（拆分原 arReconciliation/enhanced.vue）
 * 提供匹配状态/争议类型/状态/确认状态的标签与类型映射
 * 行为完全保持一致（仅结构重构）
 */

/** 匹配状态 → el-tag 类型 */
const MATCH_TYPE_MAP: Record<string, string> = {
  matched: 'success',
  partial: 'warning',
  unmatched: 'danger',
}

/** 匹配状态 → 中文标签 */
const MATCH_LABEL_MAP: Record<string, string> = {
  matched: '已匹配',
  partial: '部分匹配',
  unmatched: '未匹配',
}

/** 争议状态 → el-tag 类型 */
const DISP_TYPE_MAP: Record<string, string> = {
  open: 'info',
  investigating: 'warning',
  resolved: 'success',
  closed: 'info',
}

/** 确认状态 → 中文标签 */
const CONFIRM_LABEL_MAP: Record<string, string> = {
  pending: '待确认',
  confirmed: '已确认',
  disputed: '有争议',
}

/** 确认状态 → el-tag 类型 */
const CONFIRM_TYPE_MAP: Record<string, string> = {
  pending: 'warning',
  confirmed: 'success',
  disputed: 'danger',
}

/** 获取匹配状态中文标签 */
export const getMatchLabel = (status: string) => MATCH_LABEL_MAP[status] || status

/** 获取匹配状态对应 el-tag 类型 */
export const getMatchType = (status: string) => MATCH_TYPE_MAP[status] || 'info'

/** 获取争议状态对应 el-tag 类型 */
export const getDisputeType = (status: string) => DISP_TYPE_MAP[status] || 'info'

/** 获取确认状态中文标签 */
export const getConfirmLabel = (status: string) => CONFIRM_LABEL_MAP[status] || status

/** 获取确认状态对应 el-tag 类型 */
export const getConfirmType = (status: string) => CONFIRM_TYPE_MAP[status] || 'info'

/** 匹配状态下拉选项 */
export const MATCH_OPTIONS = [
  { label: '全部', value: '' },
  { label: '已匹配', value: 'matched' },
  { label: '部分匹配', value: 'partial' },
  { label: '未匹配', value: 'unmatched' },
]

/** 争议类型下拉选项 */
export const DISPUTE_TYPE_OPTIONS = [
  { label: '金额争议', value: 'amount' },
  { label: '质量争议', value: 'quality' },
  { label: '交付争议', value: 'delivery' },
  { label: '其他', value: 'other' },
]

/** 争议状态下拉选项 */
export const DISPUTE_STATUS_OPTIONS = [
  { label: '待处理', value: 'open' },
  { label: '调查中', value: 'investigating' },
  { label: '已解决', value: 'resolved' },
  { label: '已关闭', value: 'closed' },
]

/** 账龄分析柱状图配色 */
export const AGING_COLORS = ['#67c23a', '#e6a23c', '#f56c6c', '#909399']
