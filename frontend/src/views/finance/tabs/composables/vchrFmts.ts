/**
 * vchrFmts.ts - 凭证格式化工具
 * 任务编号: P14 批 1 B3 I-2（拆分原 VoucherTab.vue）
 * 提供金额格式化、状态标签、状态类型映射等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** 金额格式化（保留 2 位小数） */
export const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

/** 凭证状态 → 中文标签 */
const STATUS_LABEL_MAP: Record<string, string> = {
  draft: '草稿',
  submitted: '已提交',
  reviewed: '已审核',
  posted: '已过账',
}

/** 凭证状态 → el-tag 类型 */
const STATUS_TYPE_MAP: Record<string, string> = {
  draft: 'info',
  submitted: 'warning',
  reviewed: 'success',
  posted: 'primary',
}

/** 获取凭证状态中文标签 */
export const getVchrStatusLabel = (status?: string) => {
  return STATUS_LABEL_MAP[status || ''] || status || ''
}

/** 获取凭证状态对应 el-tag 类型 */
export const getVchrStatusType = (status?: string) => {
  return STATUS_TYPE_MAP[status || ''] || 'info'
}
