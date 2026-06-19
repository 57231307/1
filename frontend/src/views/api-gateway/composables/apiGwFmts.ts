/**
 * apiGwFmts.ts - API 网关共享格式化工具
 * 任务编号: P14 批 1 B3 I-2（拆分原 api-gateway/index.vue）
 * 提供 HTTP 方法、端点状态、密钥状态的标签/类型映射
 * 行为完全保持一致（仅结构重构）
 */

/** HTTP 方法 → el-tag 类型 */
export const METHOD_TYPE_MAP: Record<string, string> = {
  GET: 'success',
  POST: 'primary',
  PUT: 'warning',
  DELETE: 'danger',
  PATCH: 'info',
}

/** 接口状态 → 中文标签 */
export const EP_STATUS_LABEL_MAP: Record<string, string> = {
  active: '启用',
  inactive: '停用',
  deprecated: '废弃',
}

/** 接口状态 → el-tag 类型 */
export const EP_STATUS_TYPE_MAP: Record<string, string> = {
  active: 'success',
  inactive: 'info',
  deprecated: 'warning',
}

/** 密钥状态 → 中文标签 */
export const KEY_STATUS_LABEL_MAP: Record<string, string> = {
  active: '启用',
  inactive: '停用',
  expired: '已过期',
}

/** 密钥状态 → el-tag 类型 */
export const KEY_STATUS_TYPE_MAP: Record<string, string> = {
  active: 'success',
  inactive: 'info',
  expired: 'warning',
}

/** API Key 脱敏（仅保留前 4 后 4） */
export const maskApiKey = (key: string) => {
  if (!key || key.length < 8) return '***'
  return key.substring(0, 4) + '****' + key.substring(key.length - 4)
}
