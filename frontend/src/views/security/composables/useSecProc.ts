// security 业务流程 composable
// 拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：登录安全（解锁账户 + 导出日志 + 查询）
// 批次 282：移除 handleSizeChange/handleCurrentChange（useTableApi watch 自动处理分页）
import { ElMessage, ElMessageBox } from 'element-plus'
import { unlockAccount, exportLoginLogs, type LockedAccount, type SecurityQueryParams } from '@/api/security'
import { logger } from '@/utils/logger'

// v11 批次 181 P2-1 修复：定义 SecContext 接口替代 any
// 批次 282：适配 useTableApi（page 独立 ref，queryParams 为 Record<string, unknown>）
interface SecContext {
  page: number
  queryParams: Record<string, unknown>
  getLoginLogs: () => Promise<void>
  getLockedAccounts: () => Promise<void>
  getStats: () => Promise<void>
}

/** security 业务流程 composable */
export const useSecProc = () => {
  // 查询：重置页码 + 拉取日志（批次 282：page 独立 ref，refresh 别名 getLoginLogs）
  const handleQuery = (sec: SecContext) => {
    sec.page = 1
    sec.getLoginLogs()
  }

  // 解锁账户
  const handleUnlock = async (row: LockedAccount, sec: SecContext) => {
    try {
      await ElMessageBox.confirm(`确认解锁账户 ${row.username}？`, '提示', { type: 'warning' })
      await unlockAccount(row.id)
      ElMessage.success('解锁成功')
      sec.getLockedAccounts()
      sec.getStats()
    } catch (error) {
      logger.error('解锁失败:', error)
    }
  }

  // 导出日志
  const handleExport = async (sec: SecContext) => {
    try {
      const res = await exportLoginLogs(sec.queryParams as SecurityQueryParams)
      const url = window.URL.createObjectURL(new Blob([res]))
      const link = document.createElement('a')
      link.href = url
      link.setAttribute('download', '登录日志.xlsx')
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      window.URL.revokeObjectURL(url)
      ElMessage.success('导出成功')
    } catch (error) {
      logger.error('导出失败:', error)
    }
  }

  return {
    handleQuery,
    handleUnlock,
    handleExport,
  }
}
