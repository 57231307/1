/**
 * useMsProc.ts - 物料短缺流程操作 composable
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 material-shortage/index.vue）
 * 封装触发检查 / 通知 / 解决 / 筛选等流程性方法
 * 行为完全保持一致（仅结构重构）
 *
 * 设计说明：通过 callbacks 接收 useMs 的状态引用（Reactive 包装层）
 */
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import {
  triggerMaterialShortageCheck,
  updateMaterialShortageStatus,
  type MaterialShortage,
  type MaterialShortageSummary,
} from '@/api/material-shortage';
import { logger } from '@/utils/logger';

/**
 * 流程回调（接收 useMs 返回的状态，自动解包后的值类型）
 */
interface MsCallbacks {
  // 分页
  currentPage: number;
  pageSize: number;
  total: number;
  // 过滤
  filterSeverity: string;
  filterStatus: string;
  // 加载状态
  tableLoading: boolean;
  checking: boolean;
  // 数据
  summary: MaterialShortageSummary;
  shortageList: MaterialShortage[];
  // 方法
  fetchSummary: () => Promise<void>;
  fetchShortages: () => Promise<void>;
  syncFilterToQuery: () => void;
}

/**
 * 物料短缺流程操作方法集合
 */
export function useMsProc(cb: MsCallbacks) {
  /**
   * 触发检查
   */
  const handleCheck = async () => {
    cb.checking = true;
    try {
      const res = await triggerMaterialShortageCheck();
      const data = (res.data || {}) as { message?: string };
      ElMessage.success(data.message || msg.translate('checkComplete'));
      await Promise.all([cb.fetchSummary(), cb.fetchShortages()]);
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : msg.translate('checkFailed');
      logger.error(errMsg);
      ElMessage.error(errMsg);
    } finally {
      cb.checking = false;
    }
  };

  /**
   * 发送通知
   */
  const handleNotify = async (row: MaterialShortage) => {
    try {
      await updateMaterialShortageStatus(row.id, 'notified');
      msg.success('notificationSent');
      await cb.fetchShortages();
    } catch (error) {
      const errMsg =
        error instanceof Error ? error.message : msg.translate('sendNotificationFailed');
      logger.error(errMsg);
      ElMessage.error(errMsg);
    }
  };

  /**
   * 标记为已解决
   */
  const handleResolve = async (row: MaterialShortage) => {
    try {
      await ElMessageBox.confirm('确认标记此缺料为已解决？', '提示', { type: 'warning' });
      await updateMaterialShortageStatus(row.id, 'resolved');
      msg.success('markedAsResolved');
      await Promise.all([cb.fetchSummary(), cb.fetchShortages()]);
    } catch (error) {
      if (error !== 'cancel') {
        const errMsg = error instanceof Error ? error.message : msg.translate('markFailed');
        logger.error(errMsg);
        ElMessage.error(errMsg);
      }
    }
  };

  /**
   * 过滤变化：同步筛选条件，重置页码，触发加载
   */
  const handleFilterChange = () => {
    cb.syncFilterToQuery();
    cb.currentPage = 1;
    cb.fetchShortages();
  };

  // 使用 reactive 包装，访问字段时自动解包 ref
  return {
    handleCheck,
    handleNotify,
    handleResolve,
    handleFilterChange,
  };
}
