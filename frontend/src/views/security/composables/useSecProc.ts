// security 业务流程 composable
// 拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
// 业务领域：登录安全（解锁账户 + 导出日志 + 分页）
// 行为完全保持一致（仅结构重构）
import { ElMessage, ElMessageBox } from 'element-plus'
import { securityApi } from '@/api/security'
import { logger } from '@/utils/logger'

/** security 业务流程 composable */
export const useSecProc = () => {
  // 查询：重置页码 + 拉取日志
  const handleQuery = (sec: any) => {
    sec.queryParams.page = 1
    sec.getLoginLogs()
  }

  // 解锁账户
  const handleUnlock = async (row: any, sec: any) => {
    try {
      await ElMessageBox.confirm(`确认解锁账户 ${row.username}？`, '提示', { type: 'warning' })
      await securityApi.unlockAccount(row.id)
      ElMessage.success('解锁成功')
      sec.getLockedAccounts()
      sec.getStats()
    } catch (error) {
      logger.error('解锁失败:', error)
    }
  }

  // 导出日志
  const handleExport = async (sec: any) => {
    try {
      const res = await securityApi.exportLoginLogs(sec.queryParams)
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

  // 分页 size 变化
  const handleSizeChange = (val: number, sec: any) => {
    sec.queryParams.page_size = val
    sec.getLoginLogs()
  }

  // 分页 current 变化
  const handleCurrentChange = (val: number, sec: any) => {
    sec.queryParams.page = val
    sec.getLoginLogs()
  }

  return {
    handleQuery,
    handleUnlock,
    handleExport,
    handleSizeChange,
    handleCurrentChange,
  }
}
