/**
 * bpmDfFmts.ts - BPM 流程定义格式化工具
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 bpm/definitions.vue）
 * 包含状态类型、状态文本、分类文本、版本状态文本等映射
 * i18n 接入：文本映射函数改为响应式求值（接收 t 函数）
 */

/** 翻译函数类型 */
type TFunc = (key: string) => string

/**
 * 流程状态到 el-tag 类型（非文本，无需翻译）
 */
export const STATUS_TYPE_MAP: Record<string, string> = {
  draft: 'info',
  active: 'success',
  suspended: 'warning',
  deprecated: 'danger',
}

/**
 * 流程状态到 i18n key 映射
 */
const STATUS_TEXT_KEY_MAP: Record<string, string> = {
  draft: 'bpm.definitions.status.draft',
  active: 'bpm.definitions.status.active',
  suspended: 'bpm.definitions.status.suspended',
  deprecated: 'bpm.definitions.status.deprecated',
}

/**
 * 分类到 i18n key 映射
 */
const CATEGORY_TEXT_KEY_MAP: Record<string, string> = {
  finance: 'bpm.definitions.category.finance',
  hr: 'bpm.definitions.category.hr',
  purchase: 'bpm.definitions.category.purchase',
  sales: 'bpm.definitions.category.sales',
  production: 'bpm.definitions.category.production',
  inventory: 'bpm.definitions.category.inventory',
  other: 'bpm.definitions.category.other',
}

/**
 * 版本状态到 i18n key 映射
 */
const VERSION_STATUS_TEXT_KEY_MAP: Record<string, string> = {
  draft: 'bpm.definitions.versionStatus.draft',
  active: 'bpm.definitions.versionStatus.active',
  deprecated: 'bpm.definitions.versionStatus.deprecated',
}

/**
 * 获取状态类型（非文本，无需翻译）
 */
export function getStatusType(status: string): string {
  return STATUS_TYPE_MAP[status] || 'info'
}

/**
 * 获取状态文本（响应式求值）
 */
export function getStatusText(status: string, t: TFunc): string {
  const key = STATUS_TEXT_KEY_MAP[status]
  return key ? t(key) : status
}

/**
 * 获取分类文本（响应式求值）
 */
export function getCategoryText(category: string, t: TFunc): string {
  const key = CATEGORY_TEXT_KEY_MAP[category]
  return key ? t(key) : category
}

/**
 * 获取版本状态文本（响应式求值）
 */
export function getVersionStatusText(status: string, t: TFunc): string {
  const key = VERSION_STATUS_TEXT_KEY_MAP[status]
  return key ? t(key) : status
}
