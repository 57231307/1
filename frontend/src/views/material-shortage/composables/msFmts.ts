/**
 * msFmts.ts - 物料缺料格式化工具
 * 提供级别 / 状态的 el-tag 配色与 i18n 文案，取值域来自 @/constants/shortage
 * 越界取值（脏数据）告警后原样显示，不静默掩盖
 */
import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';
import {
  REPLENISHMENT_PRIORITY_LABEL_KEY,
  REPLENISHMENT_PRIORITY_TAG_TYPE,
  SHORTAGE_ALERT_STATUS_LABEL_KEY,
  SHORTAGE_ALERT_STATUS_TAG_TYPE,
  SHORTAGE_LEVEL_LABEL_KEY,
  SHORTAGE_LEVEL_TAG_TYPE,
  type ShortageTagType,
} from '@/constants/shortage';

/** 缺料级别 el-tag 类型 */
export const getLevelTagType = (level: string): ShortageTagType => {
  const type = SHORTAGE_LEVEL_TAG_TYPE[level];
  if (!type) {
    logger.warn(
      `[msFmts] 未知缺料级别「${level}」，不在后端取值 Critical/Severe/Warning/Normal 内`
    );
    return 'info';
  }
  return type;
};

/** 缺料级别文案 */
export const getLevelText = (level: string): string => {
  const key = SHORTAGE_LEVEL_LABEL_KEY[level];
  if (!key) {
    logger.warn(`[msFmts] 未知缺料级别「${level}」，无对应文案键`);
    return level;
  }
  return i18n.global.t(key);
};

/** 缺料预警状态 el-tag 类型 */
export const getStatusTagType = (status: string | null | undefined): ShortageTagType => {
  if (!status) return 'info';
  const type = SHORTAGE_ALERT_STATUS_TAG_TYPE[status];
  if (!type) {
    logger.warn(
      `[msFmts] 未知缺料预警状态「${status}」，不在后端状态机 identified/purchase_request/purchase_order/received/resolved 内`
    );
    return 'info';
  }
  return type;
};

/** 缺料预警状态文案；未落库（status 为空）时明确提示而非伪造状态 */
export const getStatusText = (status: string | null | undefined): string => {
  if (!status) return i18n.global.t('materialShortage.table.statusUnsaved');
  const key = SHORTAGE_ALERT_STATUS_LABEL_KEY[status];
  if (!key) {
    logger.warn(`[msFmts] 未知缺料预警状态「${status}」，无对应文案键`);
    return status;
  }
  return i18n.global.t(key);
};

/** 补货建议优先级 el-tag 类型 */
export const getPriorityTagType = (priority: string): ShortageTagType => {
  const type = REPLENISHMENT_PRIORITY_TAG_TYPE[priority];
  if (!type) {
    logger.warn(`[msFmts] 未知补货优先级「${priority}」，不在后端取值 URGENT/HIGH/MEDIUM/LOW 内`);
    return 'info';
  }
  return type;
};

/** 补货建议优先级文案 */
export const getPriorityText = (priority: string): string => {
  const key = REPLENISHMENT_PRIORITY_LABEL_KEY[priority];
  if (!key) {
    logger.warn(`[msFmts] 未知补货优先级「${priority}」，无对应文案键`);
    return priority;
  }
  return i18n.global.t(key);
};

/** 时间显示（后端为 RFC3339，按浏览器 locale 展示）；空值返回空串由模板决定占位 */
export const formatDateTime = (value: string | null | undefined): string => {
  if (!value) return '';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    logger.warn(`[msFmts] 无法解析时间值「${value}」`);
    return value;
  }
  return date.toLocaleString();
};

/** 数量显示：整数不带小数，小数保留两位 */
export const formatQuantity = (value: number | string | null | undefined): string => {
  const num = Number(value);
  if (!Number.isFinite(num)) {
    logger.warn(`[msFmts] 缺料数量不是合法数值：${String(value)}`);
    return String(value ?? '');
  }
  return Number.isInteger(num) ? String(num) : num.toFixed(2);
};

/** 缺口率显示（后端已是百分比数值） */
export const formatDeficitRate = (rate: number | string | null | undefined): string => {
  const num = Number(rate);
  if (!Number.isFinite(num)) {
    logger.warn(`[msFmts] 缺口率不是合法数值：${String(rate)}`);
    return String(rate ?? '');
  }
  return `${num.toFixed(1)}%`;
};
