// security 格式化工具集合
// 拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
// 行为完全保持一致（仅结构重构）

/** 登录类型 → 中文标签 */
export const getTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    LOGIN: '登录',
    LOGOUT: '登出',
  }
  return map[type] || type
}

/** 登录状态 → ElTag type */
export const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    SUCCESS: 'success',
    FAILED: 'danger',
  }
  return map[status] || 'info'
}

/** 登录状态码 → 中文标签 */
export const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    SUCCESS: '成功',
    FAILED: '失败',
  }
  return map[status] || status
}

/** 告警类型 → ElTag type */
export const getAlertType = (type: string) => {
  const map: Record<string, string> = {
    BRUTE_FORCE: 'danger',
    SUSPICIOUS_IP: 'warning',
    MULTIPLE_FAILURES: 'warning',
    UNUSUAL_LOCATION: 'info',
  }
  return map[type] || 'info'
}

/** 告警类型码 → 中文标签 */
export const getAlertLabel = (type: string) => {
  const map: Record<string, string> = {
    BRUTE_FORCE: '暴力破解',
    SUSPICIOUS_IP: '可疑IP',
    MULTIPLE_FAILURES: '多次失败',
    UNUSUAL_LOCATION: '异常地点',
  }
  return map[type] || type
}

/** 告警状态 → ElTag type */
export const getAlertStatusType = (status: string) => {
  const map: Record<string, string> = {
    PENDING: 'warning',
    PROCESSING: 'primary',
    RESOLVED: 'success',
    IGNORED: 'info',
  }
  return map[status] || 'info'
}

/** 告警状态码 → 中文标签 */
export const getAlertStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    PENDING: '待处理',
    PROCESSING: '处理中',
    RESOLVED: '已解决',
    IGNORED: '已忽略',
  }
  return map[status] || status
}
