/**
 * prdFmts.ts - 生产管理格式化工具
 * 任务编号: P14 批 2 I-3 第 4 批（拆分原 production/index.vue）
 * 提供状态文本格式化等纯函数
 * 行为完全保持一致（仅结构重构）
 */
import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';
import { PRODUCTION_ORDER_STATUS, type ProductionOrderStatusValue } from '@/api/production';

type TagType = 'primary' | 'success' | 'warning' | 'info' | 'danger';

const entry = (status: string) =>
  PRODUCTION_ORDER_STATUS[status as ProductionOrderStatusValue] as
    { labelKey: string; type: string } | undefined;

/**
 * 生产订单状态文本；状态机之外的取值告警后原样显示，
 * 不静默套用默认文案（那会把脏数据伪装成正常状态）
 */
export const getStatusLabel = (status: string): string => {
  const config = entry(status);
  if (!config) {
    logger.warn(`[prdFmts] 未知生产订单状态「${status}」，不在后端状态机取值内`);
    return status;
  }
  return i18n.global.t(config.labelKey);
};

/** 状态对应的 el-tag 配色 */
export const getStatusType = (status: string): TagType => {
  const config = entry(status);
  if (!config) {
    logger.warn(`[prdFmts] 未知生产订单状态「${status}」，无法取配色`);
    return 'info';
  }
  return config.type as TagType;
};
