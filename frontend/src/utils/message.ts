/**
 * i18n 感知的 ElMessage 包装器（V15 批次 07 P1-9 修复）
 * 解决 ElMessage 硬编码中文问题，集中管理高频提示短语。
 * 用法：
 *   import { msg } from '@/utils/message'
 *   msg.success()       // 操作成功
 *   msg.saveOk()        // 保存成功
 *   msg.deleteOk()      // 删除成功
 *   msg.approveOk()     // 审批成功
 *   msg.error('loadFailed', '账单')  // 加载账单失败
 */
import { ElMessage } from 'element-plus'
import { i18n } from '@/i18n'

/** 从 message 命名空间翻译 key */
function t(key: string, named?: Record<string, unknown>): string {
  const fullKey = named ? `message.${key}` : `message.${key}`
  const translated = i18n.global.t(fullKey, named ?? {})
  return translated
}

/** 显示成功提示（默认"操作成功"） */
function success(key = 'operationSuccess', named?: Record<string, unknown>): void {
  ElMessage.success(t(key, named))
}

/** 显示错误提示（默认"操作失败"） */
function error(key = 'operationFailed', named?: Record<string, unknown>): void {
  ElMessage.error(t(key, named))
}

/** 显示警告提示 */
function warning(key: string, named?: Record<string, unknown>): void {
  ElMessage.warning(t(key, named))
}

/** 显示信息提示 */
function info(key: string, named?: Record<string, unknown>): void {
  ElMessage.info(t(key, named))
}

/** 高频操作快捷方法 */
export const msg = {
  success,
  error,
  warning,
  info,
  // 翻译 message 命名空间下的 key（用于动态拼接场景）
  translate: (key: string, named?: Record<string, unknown>) => t(key, named),
  // 通用操作
  operationOk: () => success('operationSuccess'),
  operationFail: () => error('operationFailed'),
  // CRUD
  saveOk: () => success('saveSuccess'),
  saveFail: () => error('saveFailed'),
  createOk: () => success('createSuccess'),
  createFail: () => error('createFailed'),
  updateOk: () => success('updateSuccess'),
  updateFail: () => error('updateFailed'),
  deleteOk: () => success('deleteSuccess'),
  deleteFail: () => error('deleteFailed'),
  // 业务流程
  approveOk: () => success('approveSuccess'),
  approveFail: () => error('approveFailed'),
  submitOk: () => success('submitSuccess'),
  submitFail: () => error('submitFailed'),
  auditOk: () => success('auditSuccess'),
  auditFail: () => error('auditFailed'),
  cancelOk: () => success('cancelSuccess'),
  cancelFail: () => error('cancelFailed'),
  refreshOk: () => success('refreshSuccess'),
  refreshFail: () => error('refreshFailed'),
  // 导入导出
  importOk: () => success('importSuccess'),
  importFail: () => error('importFailed'),
  exportOk: () => success('exportSuccess'),
  exportFail: () => error('exportFailed'),
  // 复制
  copyOk: () => success('copySuccess'),
  copyFail: () => error('copyFailed'),
  // 打印
  printOpened: () => success('printWindowOpened'),
  printBlocked: () => error('printWindowBlocked'),
  // 加载失败（带业务名）
  loadFail: (entity?: string) =>
    error('loadFailed', entity ? { entity } : undefined),
  // 网络异常
  networkError: () => error('networkError'),
  // 权限
  permissionDenied: () => error('permissionDenied'),
  sessionExpired: () => error('sessionExpired'),
}

export default msg
