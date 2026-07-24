/**
 * arRecFmts.ts - AR 对账共享格式化工具
 * 任务编号: P14 批 1 B3 I-2（拆分原 arReconciliation/enhanced.vue）
 * 提供匹配状态/争议类型/状态/确认状态的标签与类型映射
 * 行为完全保持一致（仅结构重构）
 * i18n：label 值改为 i18n key 字符串，由调用方通过 t() 翻译
 */

/** 匹配状态 → el-tag 类型 */
const MATCH_TYPE_MAP: Record<string, string> = {
  matched: 'success',
  partial: 'warning',
  unmatched: 'danger',
}

/** 匹配状态 → i18n key */
const MATCH_LABEL_MAP: Record<string, string> = {
  matched: 'arReconciliationModule.matchStatusMatched',
  partial: 'arReconciliationModule.matchStatusPartial',
  unmatched: 'arReconciliationModule.matchStatusUnmatched',
}

/** 争议状态 → el-tag 类型 */
const DISP_TYPE_MAP: Record<string, string> = {
  open: 'info',
  investigating: 'warning',
  resolved: 'success',
  closed: 'info',
}

/** 确认状态 → i18n key */
const CONFIRM_LABEL_MAP: Record<string, string> = {
  pending: 'arReconciliationModule.confirmStatusPending',
  confirmed: 'arReconciliationModule.confirmStatusConfirmed',
  disputed: 'arReconciliationModule.confirmStatusDisputed',
}

/** 确认状态 → el-tag 类型 */
const CONFIRM_TYPE_MAP: Record<string, string> = {
  pending: 'warning',
  confirmed: 'success',
  disputed: 'danger',
}

/** 获取匹配状态 i18n key */
export const getMatchLabel = (status: string) => MATCH_LABEL_MAP[status] || status

/** 获取匹配状态对应 el-tag 类型 */
export const getMatchType = (status: string) => MATCH_TYPE_MAP[status] || 'info'

/** 获取争议状态对应 el-tag 类型 */
export const getDisputeType = (status: string) => DISP_TYPE_MAP[status] || 'info'

/** 获取确认状态 i18n key */
export const getConfirmLabel = (status: string) => CONFIRM_LABEL_MAP[status] || status

/** 获取确认状态对应 el-tag 类型 */
export const getConfirmType = (status: string) => CONFIRM_TYPE_MAP[status] || 'info'

/** 匹配状态下拉选项（label 为 i18n key） */
export const MATCH_OPTIONS = [
  { label: 'arReconciliationModule.matchOptionAll', value: '' },
  { label: 'arReconciliationModule.matchStatusMatched', value: 'matched' },
  { label: 'arReconciliationModule.matchStatusPartial', value: 'partial' },
  { label: 'arReconciliationModule.matchStatusUnmatched', value: 'unmatched' },
]

/** 争议类型下拉选项（label 为 i18n key） */
export const DISPUTE_TYPE_OPTIONS = [
  { label: 'arReconciliationModule.disputeTypeAmount', value: 'amount' },
  { label: 'arReconciliationModule.disputeTypeQuality', value: 'quality' },
  { label: 'arReconciliationModule.disputeTypeDelivery', value: 'delivery' },
  { label: 'arReconciliationModule.disputeTypeOther', value: 'other' },
]

/** 争议状态下拉选项（label 为 i18n key） */
export const DISPUTE_STATUS_OPTIONS = [
  { label: 'arReconciliationModule.disputeStatusOpen', value: 'open' },
  { label: 'arReconciliationModule.disputeStatusInvestigating', value: 'investigating' },
  { label: 'arReconciliationModule.disputeStatusResolved', value: 'resolved' },
  { label: 'arReconciliationModule.disputeStatusClosed', value: 'closed' },
]

/** 账龄分析柱状图配色 */
export const AGING_COLORS = ['#67c23a', '#e6a23c', '#f56c6c', '#909399']
