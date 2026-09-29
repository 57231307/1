/**
 * useSysUpd.ts - 系统更新核心 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 system-update/index.vue）
 * 提供当前版本、更新任务、系统备份等业务状态与加载方法
 * 业务流程（确认对话框的回滚/取消等）由 useSysUpdProc 提供
 * 更新应用为后端同步单请求：triggerUpdate 用不确定态轮询 update-status 观测进度，禁伪造百分比
 */
import { ref, reactive } from 'vue';
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import {
  getCurrentVersion,
  checkForUpdates,
  applyUpdate,
  getUpdateStatus,
  createSystemBackup,
  getSystemBackup,
  getUpdateTask,
  type CheckUpdateResult,
  type UpdateTask,
  type SystemBackup,
} from '@/api/system-update';
import { logger } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';

/** 轮询 update-status 的最大次数与间隔（同步请求下为兜底观测窗口，非进度计时器） */
const STATUS_POLL_INTERVAL_MS = 1500;
const STATUS_POLL_MAX_ATTEMPTS = 20;
const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

/** 系统更新 composable（集中管理 2 个 tab + 表单 + 详情的业务状态） */
export function useSysUpd() {
  // 当前/最新版本
  const currentVersion = ref<{ version: string; release_date: string } | null>(null);
  // 最新检查结果：直接承载后端 check 响应（含两份 release_notes 供卡片渲染）
  const latestVersion = ref<CheckUpdateResult | null>(null);
  // 是否有更新以后端权威 has_update 为准；未检查时保持 false（诚实显示"无更新"，不自算）
  const hasUpdate = ref(false);
  // 更新是否在途（不确定态）：仅由 triggerUpdate/轮询驱动，不驱动任何百分比
  const isUpdating = ref(false);

  // 更新任务 - 接入 useTableApi（后端返回 ApiResponse<PaginatedResponse<UpdateTask>>，
  // useTableApi 默认探测 items/total 键即可正确解包）
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
      ElMessage.error(
        (err instanceof Error ? err.message : String(err)) || msg.translate('loadTaskListFailed')
      ),
  });

  // 系统备份 - 接入 useTableApi（后端返回 ApiResponse<PaginatedResponse<SystemBackup>>）
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
      ElMessage.error(
        (err instanceof Error ? err.message : String(err)) || msg.translate('loadBackupListFailed')
      ),
  });

  // 备份表单
  const backupForm = reactive({
    backup_type: 'full' as 'full' | 'incremental' | 'database' | 'files',
    description: '',
  });
  const backupSubmitLoading = ref(false);

  const currentBackupDetail = ref<SystemBackup | null>(null);
  const currentTaskDetail = ref<UpdateTask | null>(null);

  /** 加载当前版本 */
  const fetchCurrentVersion = async () => {
    try {
      const res = await getCurrentVersion();
      currentVersion.value = res.data;
    } catch (error: unknown) {
      logger.error('获取当前版本失败:', error);
    }
  };

  /** 检查更新 */
  const handleCheckUpdate = async () => {
    try {
      const res = await checkForUpdates();
      latestVersion.value = res.data;
      hasUpdate.value = res.data.has_update;
      if (res.data.has_update) {
        msg.success('newVersionFound', { version: res.data.latest_version });
      } else {
        msg.info('alreadyLatestVersion');
      }
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          msg.translate('checkUpdateFailed')
      );
    }
  };

  /**
   * 轮询观测后端 is_updating。
   * 后端 apply 为同步单请求，请求返回即应用完成，故通常第一拍即观测到 false；
   * 此函数只负责「观测期间如实反映后端布尔态」，绝不参与任何百分比计算/自增。
   */
  const pollUntilIdle = async () => {
    for (let attempt = 0; attempt < STATUS_POLL_MAX_ATTEMPTS; attempt++) {
      const res = await getUpdateStatus();
      if (!res.data.is_updating) return;
      await sleep(STATUS_POLL_INTERVAL_MS);
    }
  };

  /**
   * 触发应用更新（不确定态进度）：
   * 1) 立即置 isUpdating=true → 更新按钮禁用、卡片显 indeterminate「正在更新…」，全程无百分比；
   * 2) POST /update（后端同步单请求）返回后短暂轮询 /update-status 确认回到空闲态；
   * 3) 完成后刷新任务/备份列表并重检更新。
   * 限制说明：因后端无后台化真实进度，isUpdating 的可见时长≈请求在途时长，属诚实不确定态。
   */
  const triggerUpdate = async () => {
    if (isUpdating.value) return; // 防重复触发
    isUpdating.value = true;
    try {
      await applyUpdate();
      await pollUntilIdle();
      msg.success('updateApplied');
      await Promise.all([fetchTasks(), fetchBackups(), handleCheckUpdate()]);
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          msg.translate('applyUpdateFailed')
      );
    } finally {
      isUpdating.value = false;
    }
  };

  /** 重置备份表单 */
  const resetBackupForm = () => {
    backupForm.backup_type = 'full';
    backupForm.description = '';
  };

  /** 提交备份表单 */
  const handleBackupSubmit = async () => {
    backupSubmitLoading.value = true;
    try {
      await createSystemBackup(backupForm);
      msg.success('backupTaskCreated');
      await fetchBackups();
      return true;
    } catch (error: unknown) {
      ElMessage.error(
        (error instanceof Error ? error.message : String(error)) ||
          msg.translate('createBackupFailed')
      );
      return false;
    } finally {
      backupSubmitLoading.value = false;
    }
  };

  // 备份详情回源
  const viewBackupDetail = (row: SystemBackup) => {
    currentBackupDetail.value = row;
    void getSystemBackup(row.id)
      .then(res => {
        if (res.data) currentBackupDetail.value = res.data;
      })
      .catch(error => {
        logger.error(msg.translate('loadBackupDetailFailed'), error);
      });
  };

  // 任务详情回源
  const viewTaskDetail = (row: UpdateTask) => {
    currentTaskDetail.value = row;
    void getUpdateTask(row.id)
      .then(res => {
        if (res.data) currentTaskDetail.value = res.data;
      })
      .catch(error => {
        logger.error(msg.translate('loadTaskDetailFailed'), error);
      });
  };

  // 返回 reactive 包装（父组件通过 upd.xxx 访问）
  return reactive({
    // 当前版本
    currentVersion,
    latestVersion,
    hasUpdate,
    isUpdating,
    fetchCurrentVersion,
    handleCheckUpdate,
    triggerUpdate,
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
    // 备份/任务详情
    currentBackupDetail,
    viewBackupDetail,
    currentTaskDetail,
    viewTaskDetail,
  });
}
