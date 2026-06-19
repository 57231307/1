// sales-analysis 格式化工具集合
// 拆分自 sales-analysis/index.vue（P14 批 2 I-3 第 6 批）
// 行为完全保持一致（仅结构重构）

/** 格式化货币：数值 → "¥xxx.xx" */
export const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(2)}` : '¥0.00'
}

/** 根据完成率返回进度条颜色 */
export const getProgressColor = (percentage: number) => {
  if (percentage >= 100) return '#67c23a'
  if (percentage >= 80) return '#e6a23c'
  return '#f56c6c'
}

/** 销售目标状态 → ElTag type */
export const getTargetStatusType = (status: string) => {
  const map: Record<string, string> = {
    COMPLETED: 'success',
    IN_PROGRESS: 'warning',
    PARTIAL: 'info',
    NOT_STARTED: 'info',
  }
  return map[status] || 'info'
}

/** 销售目标状态码 → 中文标签 */
export const getTargetStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    COMPLETED: '已完成',
    IN_PROGRESS: '进行中',
    PARTIAL: '部分完成',
    NOT_STARTED: '未开始',
  }
  return map[status] || status
}
