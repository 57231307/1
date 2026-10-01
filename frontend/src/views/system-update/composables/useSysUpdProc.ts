/**
 * useSysUpdProc.ts - 系统更新流程操作 composable
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 system-update/index.vue）
 * 封装回滚/取消等任务流程性方法。
 * 备份删除/恢复/下载：后端未注册 /system-update/backups/{id}（DELETE）、
 * {id}/restore、{id}/download 端点，UI 对应按钮已置灰并注明「暂不支持」，
 * 故本文件不再提供指向不存在端点的调用封装（restore 属真不可逆动作，
 * 将来 HTTP 化须独立设计审计+二次确认后再补）。
 */
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { cancelUpdateTask, rollbackUpdate, type UpdateTask } from '@/api/system-update';

/** 刷新回调 */
interface RefreshCallbacks {
  fetchTasks: () => Promise<void>;
}

/**
 * 系统更新流程操作方法集合
 */
export function useSysUpdProc(refresh: RefreshCallbacks) {
  /** 取消任务 */
  const handleCancelTask = async (row: UpdateTask) => {
    try {
      await ElMessageBox.confirm('确定要取消此任务吗？', '确认取消', { type: 'warning' });
      await cancelUpdateTask(row.id);
      msg.success('taskCancelled');
      await refresh.fetchTasks();
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel')
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) || msg.translate('cancelFailed')
        );
    }
  };

  /** 回滚 */
  const handleRollback = async (row: UpdateTask) => {
    try {
      await ElMessageBox.confirm(`确定要回滚到版本 ${row.from_version} 吗？`, '确认回滚', {
        type: 'warning',
      });
      await rollbackUpdate(row.from_version);
      msg.success('rollbackTaskCreated');
      await refresh.fetchTasks();
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      if (error !== 'cancel')
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) ||
            msg.translate('rollbackFailed')
        );
    }
  };

  return {
    handleCancelTask,
    handleRollback,
  };
}
