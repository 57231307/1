/**
 * bpmDfFmts.ts - BPM 流程定义格式化工具
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 bpm/definitions.vue）
 * 包含状态类型、状态文本、分类文本、版本状态文本等映射
 * 行为完全保持一致（仅结构重构）
 */

/**
 * 流程状态到 el-tag 类型
 */
export const STATUS_TYPE_MAP: Record<string, string> = {
  draft: 'info',
  active: 'success',
  suspended: 'warning',
  deprecated: 'danger',
}

/**
 * 流程状态到中文标签
 */
export const STATUS_TEXT_MAP: Record<string, string> = {
  draft: '草稿',
  active: '已发布',
  suspended: '已暂停',
  deprecated: '已废弃',
}

/**
 * 分类中文映射
 */
export const CATEGORY_TEXT_MAP: Record<string, string> = {
  finance: '财务',
  hr: '人事',
  purchase: '采购',
  sales: '销售',
  production: '生产',
  inventory: '库存',
  other: '其他',
}

/**
 * 版本状态中文映射
 */
export const VERSION_STATUS_TEXT_MAP: Record<string, string> = {
  draft: '草稿',
  active: '激活',
  deprecated: '废弃',
}

/**
 * 获取状态类型
 */
export function getStatusType(status: string): string {
  return STATUS_TYPE_MAP[status] || 'info'
}

/**
 * 获取状态文本
 */
export function getStatusText(status: string): string {
  return STATUS_TEXT_MAP[status] || status
}

/**
 * 获取分类文本
 */
export function getCategoryText(category: string): string {
  return CATEGORY_TEXT_MAP[category] || category
}

/**
 * 获取版本状态文本
 */
export function getVersionStatusText(status: string): string {
  return VERSION_STATUS_TEXT_MAP[status] || status
}
