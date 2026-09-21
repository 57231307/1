/**
 * useMsProc.ts - 物料缺料流程操作 composable
 * 封装触发检查 / 预警状态推进 / 筛选三类流程方法
 *
 * 设计说明：通过 callbacks 接收 useMs 的状态引用（Reactive 包装层）
 */
import { ElMessage, ElMessageBox } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
import {
  triggerMaterialShortageCheck,
  updateMaterialShortageStatus,
  type MaterialShortageAlert,
  type MaterialShortageSummary,
} from '@/api/material-shortage';
import {
  SHORTAGE_ALERT_STATUS_LABEL_KEY,
  type ShortageAlertStatusValue,
} from '@/constants/shortage';
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
  filterLevel: string;
  filterStatus: string;
  // 加载状态
  tableLoading: boolean;
  checking: boolean;
  // 数据
  summary: MaterialShortageSummary;
  shortageList: MaterialShortageAlert[];
  // 方法
  fetchSummary: () => Promise<void>;
  fetchShortages: () => Promise<void>;
  syncFilterToQuery: () => void;
}

/**
 * 物料缺料流程操作方法集合
 */
export function useMsProc(cb: MsCallbacks) {
  /**
   * 触发一次实时缺料检测（后端同步落库预警快照并推送 Critical 通知）
   */
  const handleCheck = async () => {
    cb.checking = true;
    try {
      const res = await triggerMaterialShortageCheck();
      const data = (res.data || {}) as { shortage_count?: number };
      ElMessage.success(
        i18n.global.t('materialShortage.checkResult', {
          count: data.shortage_count ?? 0,
        })
      );
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
   * 推进缺料预警状态（identified → purchase_request → purchase_order → received → resolved）
   * 路径参数为 material_id，取值域与后端状态机一致，越界由后端 400 拦截
   */
  const handleStatusChange = async (
    row: MaterialShortageAlert,
    status: ShortageAlertStatusValue
  ) => {
    const labelKey = SHORTAGE_ALERT_STATUS_LABEL_KEY[status];
    const label = labelKey ? i18n.global.t(labelKey) : status;
    try {
      await ElMessageBox.confirm(
        i18n.global.t('materialShortage.confirmStatusChange', {
          material: row.material_name,
          status: label,
        }),
        i18n.global.t('materialShortage.common.tips'),
        { type: 'warning' }
      );
    } catch {
      // 用户取消，不发请求
      await cb.fetchShortages();
      return;
    }

    try {
      await updateMaterialShortageStatus(row.material_id, status);
      msg.success('saveSuccess');
      await Promise.all([cb.fetchSummary(), cb.fetchShortages()]);
    } catch (error) {
      const errMsg = error instanceof Error ? error.message : msg.translate('saveFailed');
      logger.error(errMsg);
      ElMessage.error(errMsg);
      await cb.fetchShortages();
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
    handleStatusChange,
    handleFilterChange,
  };
}
