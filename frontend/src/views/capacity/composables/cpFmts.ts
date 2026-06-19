// capacity 格式化工具集合
// 拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
// 行为完全保持一致（仅结构重构）

/** 工作中心状态码 → ElTag type */
export const getStatusType = (status: string) => {
  const map: Record<string, string> = { normal: 'success', busy: 'warning', overload: 'danger' }
  return map[status] || 'info'
}

/** 工作中心状态码 → 中文标签 */
export const getStatusLabel = (status: string) => {
  const map: Record<string, string> = { normal: '正常', busy: '繁忙', overload: '超负荷' }
  return map[status] || status
}

/** 负荷率 → ElTag type */
export const getLoadRateType = (rate: number) => {
  if (rate >= 1) return 'danger'
  if (rate >= 0.8) return 'warning'
  return 'success'
}
