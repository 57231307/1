/**
 * schGFmts.ts - 排产甘特图格式化工具
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
 * 提供排产甘特图相关的时间格式化、状态颜色/标签映射工具
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 排产任务状态颜色映射（用于 ECharts 渲染）
 */
export const statusColorMap: Record<string, string> = {
  pending: '#909399',
  scheduled: '#409eff',
  running: '#e6a23c',
  completed: '#67c23a',
  conflict: '#f56c6c',
}

/**
 * 排产任务状态文本映射（用于图例/工具提示）
 */
export const statusLabelMap: Record<string, string> = {
  pending: '待排程',
  scheduled: '已排程',
  running: '生产中',
  completed: '已完成',
  conflict: '冲突',
}

/**
 * 格式化时间（"YYYY-MM-DDTHH:mm:ss..." → "YYYY-MM-DD HH:mm"）
 */
export const formatTime = (t: string): string => {
  if (!t) return '-'
  return t.replace('T', ' ').slice(0, 16)
}
