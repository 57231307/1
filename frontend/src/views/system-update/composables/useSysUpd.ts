/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useSysUpd.ts - 系统更新核心 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 system-update/index.vue）
 * 提供当前版本、版本列表、更新任务、系统备份等业务状态与加载方法
 * 业务流程（确认对话框的下载/安装/回滚/恢复等）由 useSysUpdProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, computed, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getCurrentVersion,
  checkForUpdates,
  listSystemVersions,
  listUpdateTasks,
  listSystemBackups,
  createSystemBackup,
  type SystemVersion,
  type UpdateTask,
  type SystemBackup,
} from '@/api/system-update'
import { logger } from '@/utils/logger'

/**
 * 系统更新 composable
 * 集中管理 3 个 tab + 表单 + 详情的业务状态
 * 对话框可见性由父组件本地 ref 管理
 */
export function useSysUpd() {
  // 当前/最新版本
  const currentVersion = ref<{ version: string; build_date: string } | null>(null)
  const latestVersion = ref<SystemVersion | null>(null)
  const hasUpdate = computed(() => {
    if (!currentVersion.value || !latestVersion.value) return false
    return currentVersion.value.version !== latestVersion.value.version
  })

  // 版本列表
  const versions = ref<SystemVersion[]>([])
  const versionTotal = ref(0)
  const versionLoading = ref(false)
  const versionQuery = reactive({
    page: 1,
    page_size: 20,
  })

  // 更新任务
  const tasks = ref<UpdateTask[]>([])
  const taskTotal = ref(0)
  const taskLoading = ref(false)
  const taskQuery = reactive({
    page: 1,
    page_size: 20,
  })

  // 系统备份
  const backups = ref<SystemBackup[]>([])
  const backupTotal = ref(0)
  const backupLoading = ref(false)
  const backupQuery = reactive({
    page: 1,
    page_size: 20,
  })

  // 备份表单
  const backupForm = reactive({
    backup_type: 'full' as 'full' | 'incremental' | 'database' | 'files',
    description: '',
  })
  const backupSubmitLoading = ref(false)

  // 版本详情
  const currentVersionDetail = ref<SystemVersion | null>(null)

  /** 加载当前版本 */
  const fetchCurrentVersion = async () => {
    try {
      const res = await getCurrentVersion()
      currentVersion.value = res.data
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      logger.error('获取当前版本失败:', error)
    }
  }

  /** 检查更新 */
  const handleCheckUpdate = async () => {
    try {
      const res = await checkForUpdates()
      latestVersion.value = res.data
      if (hasUpdate.value) {
        ElMessage.success(`发现新版本: ${res.data.version}`)
      } else {
        ElMessage.info('当前已是最新版本')
      }
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '检查更新失败')
    }
  }

  /** 加载版本列表 */
  const fetchVersions = async () => {
    versionLoading.value = true
    try {
      const res = await listSystemVersions(versionQuery)
      versions.value = res.data || []
      versionTotal.value = res.total || 0
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '获取版本列表失败')
    } finally {
      versionLoading.value = false
    }
  }

  /** 加载更新任务 */
  const fetchTasks = async () => {
    taskLoading.value = true
    try {
      const res = await listUpdateTasks(taskQuery)
      tasks.value = res.data || []
      taskTotal.value = res.total || 0
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '获取任务列表失败')
    } finally {
      taskLoading.value = false
    }
  }

  /** 加载系统备份 */
  const fetchBackups = async () => {
    backupLoading.value = true
    try {
      const res = await listSystemBackups(backupQuery)
      backups.value = res.data || []
      backupTotal.value = res.total || 0
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '获取备份列表失败')
    } finally {
      backupLoading.value = false
    }
  }

  /** 重置备份表单 */
  const resetBackupForm = () => {
    backupForm.backup_type = 'full'
    backupForm.description = ''
  }

  /** 提交备份表单 */
  const handleBackupSubmit = async () => {
    backupSubmitLoading.value = true
    try {
      await createSystemBackup(backupForm)
      ElMessage.success('备份任务已创建')
      await fetchBackups()
      return true
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '创建备份失败')
      return false
    } finally {
      backupSubmitLoading.value = false
    }
  }

  /** 打开版本详情（父组件需自行打开对话框） */
  const viewVersionDetail = (row: SystemVersion) => {
    currentVersionDetail.value = row
  }

  // 直接返回 ref/function/computed 集合（不再用 reactive 包装），方便父组件解构到顶层
  // 这样 template 里可以直接用 `versions` 而不用 `upd.versions`
  return {
    // 当前版本
    currentVersion,
    latestVersion,
    hasUpdate,
    fetchCurrentVersion,
    handleCheckUpdate,
    // 版本列表
    versions,
    versionTotal,
    versionLoading,
    versionQuery,
    fetchVersions,
    // 更新任务
    tasks,
    taskTotal,
    taskLoading,
    taskQuery,
    fetchTasks,
    // 系统备份
    backups,
    backupTotal,
    backupLoading,
    backupQuery,
    fetchBackups,
    // 备份表单
    backupForm,
    backupSubmitLoading,
    resetBackupForm,
    handleBackupSubmit,
    // 版本详情
    currentVersionDetail,
    viewVersionDetail,
  }
}
