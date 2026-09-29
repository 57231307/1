/**
 * lgsFmts.ts - 物流管理格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 logistics/index.vue）
 * 提供状态类型/文本映射、运费格式化等纯函数
 */
import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';
import {
  WAYBILL_STATUS_LABEL_KEY,
  WAYBILL_STATUS_TAG_TYPE,
  type WaybillTagType,
} from '@/constants/waybill-status';

/**
 * 获取运单状态 el-tag 类型；状态机外的取值告警后按 info 展示
 */
export const getStatusType = (status: string): WaybillTagType => {
  const type = WAYBILL_STATUS_TAG_TYPE[status];
  if (!type) {
    logger.warn(
      `[lgsFmts] 未知运单状态「${status}」，不在后端状态机 IN_TRANSIT/DELIVERED/SIGNED 内`
    );
    return 'info';
  }
  return type;
};

/**
 * 运单状态文本（i18n）；无映射键的状态告警后原样显示，便于暴露脏数据
 */
export const getStatusText = (status: string): string => {
  const key = WAYBILL_STATUS_LABEL_KEY[status];
  if (!key) {
    logger.warn(`[lgsFmts] 未知运单状态「${status}」，无对应文案键`);
    return status;
  }
  return i18n.global.t(key);
};

/**
 * 格式化运费显示（带人民币符号）
 */
export const formatFreight = (fee: number | undefined | null): string => {
  return `¥${fee || 0}`;
};
