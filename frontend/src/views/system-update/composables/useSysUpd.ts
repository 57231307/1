/**
 * useSysUpd.ts - 系统更新核心 composable
 * 提供当前版本、更新任务、系统备份等业务状态与加载方法
 * 业务流程（确认对话框的回滚/取消等）由 useSysUpdProc 提供
 * 更新应用为后端同步单请求：POST /update 返回即已落库应用版本；triggerUpdate 随后轮询
 * update-status 的真实生效字段（applied_version / restart_pending）判定是否已生效，
 * 并把后端权威重启结论 message 如实呈现，绝不伪造百分比或谎称服务已重启。
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
import { logger, logAuxLoadFailure } from '@/utils/logger';
import { useTableApi } from '@/composables/useTableApi';

/** 轮询 update-status 真实生效态的最大次数与间隔（含重启窗口容错，非进度计时器） */
const STATUS_POLL_INTERVAL_MS = 1500;
const STATUS_POLL_MAX_ATTEMPTS = 20;
const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

/** 系统更新 composable（集中管理 2 个 tab + 表单 + 详情的业务状态） */
export function useSysUpd() {
  // 当前/最新版本（release_date 与 changelog 来自后端真列值，未知为 null，不以默认值掩盖缺键）
  const currentVersion = ref<{
    version: string;
    release_date: string | null;
    changelog: string | null;
  } | null>(null);
  // 最新检查结果：直接承载后端 check 响应（含两份 release_notes 供卡片渲染）
  const latestVersion = ref<CheckUpdateResult | null>(null);
  // 是否有更新以后端权威 has_update 为准；未检查时保持 false（诚实显示"无更新"，不自算）
  const hasUpdate = ref(false);
  // 更新是否在途（不确定态）：仅由 triggerUpdate/轮询驱动，不驱动任何百分比
  const isUpdating = ref(false);
  // 检查更新是否在途：驱动头部按钮的 loading（Element Plus 在 loading 时即禁用，防重复提交）
  const isChecking = ref(false);
  // 应用更新结果（真实生效态，来自 POST /update 响应 + update-status 轮询）：
  // 后端重启结论 message（权威文本，原样呈现）、已落库应用版本、是否待重启生效。
  // 未触发过应用时 applyMessage 为空串，卡片不渲染结果区（诚实空态，不兜底假文案）。
  const applyMessage = ref('');
  const appliedVersion = ref<string | null>(null);
  const restartPending = ref(false);

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
      logAuxLoadFailure('获取当前版本失败:', error);
    }
  };

  /** 检查更新 */
  const handleCheckUpdate = async () => {
    isChecking.value = true;
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
    } finally {
      isChecking.value = false;
    }
  };

  /**
   * 轮询 update-status 观测真实生效态。
   * 判据（逐条对齐后端 applied_version / restart_pending 语义，不臆测）：
   * - 目标版本（apply 响应的 new_version）已落库为 applied_version 且 restart_pending 转 false
   *   → 新编译版本已在运行进程加载，视为已生效，提前结束；
   * - 若 new_version 为空（后端未能确定目标版本），仅以 restart_pending 转 false 为结束条件；
   * - 其余情况（含服务正处于重启窗口、轮询请求失败）继续重试直至观测窗口用尽，
   *   保留最后一次读到的 applied_version / restart_pending 交人工据实判断，绝不静默吞错。
   */
  const pollUntilEffective = async (targetVersion: string | null) => {
    for (let attempt = 0; attempt < STATUS_POLL_MAX_ATTEMPTS; attempt++) {
      try {
        const res = await getUpdateStatus();
        appliedVersion.value = res.data.applied_version;
        restartPending.value = res.data.restart_pending;
        const versionMatched = targetVersion === null || res.data.applied_version === targetVersion;
        if (!res.data.restart_pending && versionMatched) return;
      } catch (error: unknown) {
        // 重启窗口内后端可能短暂不可达：这不是业务失败，记录后继续观测，不当作已生效/已失败
        logAuxLoadFailure('更新生效状态轮询失败（可能处于重启窗口）:', error);
      }
      await sleep(STATUS_POLL_INTERVAL_MS);
    }
  };

  /**
   * 触发应用更新（真实生效判定）：
   * 1) 置 isUpdating=true → 更新按钮禁用、卡片显 indeterminate「正在更新…」，全程无百分比；
   * 2) POST /update（后端同步单请求）返回，取 data.message（权威重启结论）与 data.new_version；
   * 3) 轮询 update-status 直至 applied_version==目标版本且 restart_pending 转 false 或超时；
   * 4) 依真实 restart_pending 分支给出诚实成功提示，并把后端结论原文与生效字段交给卡片展示；
   * 5) 刷新任务/备份列表并重检更新。
   * 说明：后端按配置门控与监督器探测决定是否自退，故生效结果可能是「等待重启」或「已受控重启」，
   * 前端只如实转述，不替后端断言服务已重启。
   */
  const triggerUpdate = async () => {
    if (isUpdating.value) return; // 防重复触发
    isUpdating.value = true;
    // 清空上一次结果，避免旧结论与新一次应用混叠（诚实：每次触发都重头采集）
    applyMessage.value = '';
    appliedVersion.value = null;
    restartPending.value = false;
    try {
      const res = await applyUpdate();
      applyMessage.value = res.data.message;
      const targetVersion = res.data.new_version;
      await pollUntilEffective(targetVersion);
      if (restartPending.value) {
        // 已应用但新进程尚未加载：等待重启生效（受控自退未拉起或需人工重启）
        msg.success('updateApplied');
      } else {
        // restart_pending 已转 false：新编译版本已在运行进程加载，受控重启并生效
        msg.success('updateAppliedActive');
      }
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
    isChecking,
    applyMessage,
    appliedVersion,
    restartPending,
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
