/**
 * ppFmts.ts - 采购价格格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-price/index.vue）
 * 提供价格类型/状态映射、货币格式化等纯函数
 * 行为完全保持一致（仅结构重构）
 */

/** 价格类型 → 中文标签 */
const PRICE_TYPE_LABEL_MAP: Record<string, string> = {
  STANDARD: '标准价',
  AGREED: '协议价',
  PROMOTION: '促销价',
}

/** 状态 → el-tag 类型 */
const STATUS_TYPE_MAP: Record<string, string> = {
  active: 'success',
  inactive: 'danger',
}

/** 状态 → 中文标签 */
const STATUS_LABEL_MAP: Record<string, string> = {
  active: '已生效',
  inactive: '已停用',
}

/** 获取价格类型中文标签 */
export const getPriceTypeLabel = (type: string) => PRICE_TYPE_LABEL_MAP[type] || type

/** 获取采购价格状态 el-tag 类型 */
export const getStatusType = (status: string) => STATUS_TYPE_MAP[status] || 'info'

/** 获取采购价格状态中文标签 */
export const getStatusLabel = (status: string) => STATUS_LABEL_MAP[status] || status

/** 格式化货币（人民币 6 位精度） */
export const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(6)}` : '¥0.000000'
}
