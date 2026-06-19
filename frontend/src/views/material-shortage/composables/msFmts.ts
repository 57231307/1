/**
 * msFmts.ts - 物料短缺格式化工具
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 material-shortage/index.vue）
 * 包含严重程度、状态、来源类型的类型与文本映射
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 严重程度元数据列表
 */
export interface SeverityLevel {
  value: 'critical' | 'high' | 'medium' | 'low'
  label: string
  color: string
  class: string
}

/**
 * 严重程度元数据（用于严重程度进度卡片）
 */
export const SEVERITY_LEVELS: SeverityLevel[] = [
  { value: 'critical', label: '严重', color: '#f56c6c', class: 'critical' },
  { value: 'high', label: '高', color: '#e6a23c', class: 'high' },
  { value: 'medium', label: '中', color: '#409eff', class: 'medium' },
  { value: 'low', label: '低', color: '#909399', class: 'low' },
]

/**
 * 严重程度到 el-tag 类型
 */
export const SEVERITY_COLOR_MAP: Record<string, string> = {
  critical: 'danger',
  high: 'warning',
  medium: '',
  low: 'info',
}

/**
 * 严重程度到中文标签
 */
export const SEVERITY_LABEL_MAP: Record<string, string> = {
  critical: '严重',
  high: '高',
  medium: '中',
  low: '低',
}

/**
 * 状态到 el-tag 类型
 */
export const STATUS_COLOR_MAP: Record<string, string> = {
  pending: 'danger',
  notified: 'warning',
  resolved: 'success',
}

/**
 * 状态到中文标签
 */
export const STATUS_LABEL_MAP: Record<string, string> = {
  pending: '待处理',
  notified: '已通知',
  resolved: '已解决',
}

/**
 * 来源类型到 el-tag 类型
 */
export const SOURCE_TYPE_COLOR_MAP: Record<string, string> = {
  production: 'primary',
  sales: 'success',
  purchase: 'warning',
}

/**
 * 来源类型到中文标签
 */
export const SOURCE_TYPE_LABEL_MAP: Record<string, string> = {
  production: '生产',
  sales: '销售',
  purchase: '采购',
}

/**
 * 获取严重程度类型
 */
export function getSeverityColor(severity: string): string {
  return SEVERITY_COLOR_MAP[severity] || 'info'
}

/**
 * 获取严重程度文本
 */
export function getSeverityLabel(severity: string): string {
  return SEVERITY_LABEL_MAP[severity] || severity
}

/**
 * 获取状态类型
 */
export function getStatusColor(status: string): string {
  return STATUS_COLOR_MAP[status] || 'info'
}

/**
 * 获取状态文本
 */
export function getStatusLabel(status: string): string {
  return STATUS_LABEL_MAP[status] || status
}

/**
 * 获取来源类型类型
 */
export function getSourceTypeColor(type: string): string {
  return SOURCE_TYPE_COLOR_MAP[type] || 'info'
}

/**
 * 获取来源类型文本
 */
export function getSourceTypeLabel(type: string): string {
  return SOURCE_TYPE_LABEL_MAP[type] || type
}
