/**
 * sysUpdFmts.ts - 系统更新格式化工具
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 system-update/index.vue）
 * 提供版本/任务/备份状态标签与类型映射、文件大小格式化
 * 行为完全保持一致（仅结构重构）
 */

/** 版本状态 → 中文标签 */
export const VERSION_STATUS_LABEL: Record<string, string> = {
  available: '可下载',
  downloading: '下载中',
  downloaded: '已下载',
  installing: '安装中',
  installed: '已安装',
  failed: '失败',
}

/** 版本状态 → el-tag 类型 */
export const VERSION_STATUS_TYPE: Record<string, string> = {
  available: 'info',
  downloading: 'warning',
  downloaded: 'success',
  installing: 'warning',
  installed: 'success',
  failed: 'danger',
}

/** 更新任务状态 → 中文标签 */
export const TASK_STATUS_LABEL: Record<string, string> = {
  pending: '待处理',
  downloading: '下载中',
  downloaded: '已下载',
  installing: '安装中',
  completed: '已完成',
  failed: '失败',
  rolled_back: '已回滚',
}

/** 更新任务状态 → el-tag 类型 */
export const TASK_STATUS_TYPE: Record<string, string> = {
  pending: 'info',
  downloading: 'warning',
  downloaded: 'success',
  installing: 'warning',
  completed: 'success',
  failed: 'danger',
  rolled_back: 'info',
}

/** 备份类型 → 中文标签 */
export const BACKUP_TYPE_LABEL: Record<string, string> = {
  full: '完整备份',
  incremental: '增量备份',
  database: '数据库备份',
  files: '文件备份',
}

/** 备份状态 → 中文标签 */
export const BACKUP_STATUS_LABEL: Record<string, string> = {
  creating: '创建中',
  completed: '已完成',
  failed: '失败',
}

/** 备份状态 → el-tag 类型 */
export const BACKUP_STATUS_TYPE: Record<string, string> = {
  creating: 'warning',
  completed: 'success',
  failed: 'danger',
}

/** 备份类型下拉选项 */
export const BACKUP_TYPE_OPTIONS = [
  { label: '完整备份', value: 'full' },
  { label: '增量备份', value: 'incremental' },
  { label: '数据库备份', value: 'database' },
  { label: '文件备份', value: 'files' },
]

/** 获取版本状态中文标签 */
export const getVersionStatusLabel = (status: string) => VERSION_STATUS_LABEL[status] || status

/** 获取版本状态 el-tag 类型 */
export const getVersionStatusType = (status: string) => VERSION_STATUS_TYPE[status] || 'info'

/** 获取任务状态中文标签 */
export const getTaskStatusLabel = (status: string) => TASK_STATUS_LABEL[status] || status

/** 获取任务状态 el-tag 类型 */
export const getTaskStatusType = (status: string) => TASK_STATUS_TYPE[status] || 'info'

/** 获取备份类型中文标签 */
export const getBackupTypeLabel = (type: string) => BACKUP_TYPE_LABEL[type] || type

/** 获取备份状态中文标签 */
export const getBackupStatusLabel = (status: string) => BACKUP_STATUS_LABEL[status] || status

/** 获取备份状态 el-tag 类型 */
export const getBackupStatusType = (status: string) => BACKUP_STATUS_TYPE[status] || 'info'

/** 格式化文件大小（B/KB/MB/GB） */
export const formatFileSize = (bytes: number) => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}
