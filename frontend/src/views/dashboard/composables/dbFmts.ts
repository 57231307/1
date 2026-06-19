// Dashboard 格式化工具集合
// 拆分自 Dashboard.vue（P14 批 2 I-3 第 6 批）
// 行为完全保持一致（仅结构重构）

/** 数字千分位格式化 */
export const formatNumber = (num: number | undefined) => {
  if (!num) return '0'
  return num.toLocaleString()
}

/** 货币格式化（Intl 人民币） */
export const formatCurrency = (amount: number | undefined) => {
  if (!amount) return '¥0'
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
    minimumFractionDigits: 0,
  }).format(amount)
}

/** 活动类型 → ElTag type */
export const getActivityTypeColor = (type: string) => {
  const typeMap: Record<string, string> = {
    订单: 'success',
    采购: 'warning',
    库存: 'info',
    审批: 'primary',
    系统: 'danger',
  }
  return typeMap[type] || 'info'
}
