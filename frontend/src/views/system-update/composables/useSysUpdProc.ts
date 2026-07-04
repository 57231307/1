/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useSysUpdProc.ts - 系统更新流程操作 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 system-update/index.vue）
 * 封装下载/安装/回滚/取消/恢复/下载备份/删除等流程性方法
 * 行为完全保持一致（仅结构重构）
 */
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  downloadUpdate,
  installUpdate,
  cancelUpdateTask,
  rollbackUpdate,
  deleteSystemBackup,
  restoreFromBackup,
  downloadBackup,
  type SystemVersion,
  type UpdateTask,
  type SystemBackup,
} from '@/api/system-update'

/** 刷新回调 */
interface RefreshCallbacks {
  fetchVersions: () => Promise<void>
  fetchTasks: () => Promise<void>
  fetchBackups: () => Promise<void>
}

/**
 * 系统更新流程操作方法集合
 */
export function useSysUpdProc(refresh: RefreshCallbacks) {
  /** 下载更新 */
  const handleDownload = async (row: SystemVersion) => {
    try {
      await ElMessageBox.confirm(`确定要下载版本 ${row.version} 吗？`, '确认下载', {
        type: 'warning',
      })
      await downloadUpdate(row.id)
      ElMessage.success('下载任务已创建')
      await refresh.fetchVersions()
      await refresh.fetchTasks()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '下载失败')
    }
  }

  /** 安装更新 */
  const handleInstall = async (row: SystemVersion) => {
    try {
      await ElMessageBox.confirm(
        `确定要安装版本 ${row.version} 吗？安装过程中系统可能会重启。`,
        '确认安装',
        { type: 'warning' }
      )
      await installUpdate(row.id)
      ElMessage.success('安装任务已创建')
      await refresh.fetchVersions()
      await refresh.fetchTasks()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '安装失败')
    }
  }

  /** 取消任务 */
  const handleCancelTask = async (row: UpdateTask) => {
    try {
      await ElMessageBox.confirm('确定要取消此任务吗？', '确认取消', { type: 'warning' })
      await cancelUpdateTask(row.id)
      ElMessage.success('任务已取消')
      await refresh.fetchTasks()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '取消失败')
    }
  }

  /** 回滚 */
  const handleRollback = async (row: UpdateTask) => {
    try {
      await ElMessageBox.confirm(`确定要回滚到版本 ${row.from_version} 吗？`, '确认回滚', {
        type: 'warning',
      })
      await rollbackUpdate(row.from_version)
      ElMessage.success('回滚任务已创建')
      await refresh.fetchTasks()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '回滚失败')
    }
  }

  /** 删除备份 */
  const handleDeleteBackup = async (row: SystemBackup) => {
    try {
      await ElMessageBox.confirm('确定要删除此备份吗？', '确认删除', { type: 'warning' })
      await deleteSystemBackup(row.id)
      ElMessage.success('删除成功')
      await refresh.fetchBackups()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '删除失败')
    }
  }

  /** 从备份恢复 */
  const handleRestore = async (row: SystemBackup) => {
    try {
      await ElMessageBox.confirm('确定要从此备份恢复系统吗？此操作不可撤销。', '确认恢复', {
        type: 'warning',
      })
      await restoreFromBackup(row.id)
      ElMessage.success('恢复任务已创建')
      await refresh.fetchTasks()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '恢复失败')
    }
  }

  /** 下载备份文件 */
  const handleDownloadBackup = async (row: SystemBackup) => {
    try {
      const blob = await downloadBackup(row.id)
      const link = document.createElement('a')
      link.href = URL.createObjectURL(blob)
      link.download = `backup_${row.backup_code}.zip`
      link.click()
      ElMessage.success('备份下载成功')
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '下载失败')
    }
  }

  return {
    handleDownload,
    handleInstall,
    handleCancelTask,
    handleRollback,
    handleDeleteBackup,
    handleRestore,
    handleDownloadBackup,
  }
}
