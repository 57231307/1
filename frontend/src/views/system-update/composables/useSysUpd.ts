/**
 * useSysUpd.ts - 系统更新核心 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 system-update/index.vue）
 * 提供当前版本、版本列表、更新任务、系统备份等业务状态与加载方法
 * 业务流程（确认对话框的下载/安装/回滚/恢复等）由 useSysUpdProc 提供
 * 批次 283：3 个表格接入 useTableApi，返回改为 reactive 包装
 */
import { ref, computed, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  getCurrentVersion,
  checkForUpdates,
  createSystemBackup,
  type SystemVersion,
  type UpdateTask,
  type SystemBackup,
} from '@/api/system-update'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

/** 系统更新 composable（集中管理 3 个 tab + 表单 + 详情的业务状态） */
export function useSysUpd() {
  // 当前/最新版本
  const currentVersion = ref<{ version: string; build_date: string } | null>(null)
  const latestVersion = ref<SystemVersion | null>(null)
  const hasUpdate = computed(() => {
    if (!currentVersion.value || !latestVersion.value) return false
    return currentVersion.value.version !== latestVersion.value.version
  })

  // 版本列表 - 接入 useTableApi（批次 283）
  const {
    data: versions,
    total: versionTotal,
    loading: versionLoading,
    page: versionPage,
    pageSize: versionPageSize,
    refresh: fetchVersions,
  } = useTableApi<SystemVersion>({
    url: '/system-update/versions',
    onError: (err: unknown) =>
      ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取版本列表失败'),
  })

  // 更新任务 - 接入 useTableApi（批次 283）
  const {
    data: tasks,
    total: taskTotal,
    loading: taskLoading,
    page: taskPage,
    pageSize: taskPageSize,
    refresh: fetchTasks,
  } = useTableApi<UpdateTask>({
    url: '/system-update/tasks',
    onError: (err: unknown) =>
      ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取任务列表失败'),
  })

  // 系统备份 - 接入 useTableApi（批次 283）
  const {
    data: backups,
    total: backupTotal,
    loading: backupLoading,
    page: backupPage,
    pageSize: backupPageSize,
    refresh: fetchBackups,
  } = useTableApi<SystemBackup>({
    url: '/system-update/backups',
    onError: (err: unknown) =>
      ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取备份列表失败'),
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
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '检查更新失败')
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

  // 批次 283：返回 reactive 包装（父组件通过 upd.xxx 访问）
  return reactive({
    // 当前版本
    currentVersion,
    latestVersion,
    hasUpdate,
    fetchCurrentVersion,
    handleCheckUpdate,
    // 版本列表（useTableApi 管理）
    versions,
    versionTotal,
    versionLoading,
    versionPage,
    versionPageSize,
    fetchVersions,
    // 更新任务（useTableApi 管理）
    tasks,
    taskTotal,
    taskLoading,
    taskPage,
    taskPageSize,
    fetchTasks,
    // 系统备份（useTableApi 管理）
    backups,
    backupTotal,
    backupLoading,
    backupPage,
    backupPageSize,
    fetchBackups,
    // 备份表单
    backupForm,
    backupSubmitLoading,
    resetBackupForm,
    handleBackupSubmit,
    // 版本详情
    currentVersionDetail,
    viewVersionDetail,
  })
}
