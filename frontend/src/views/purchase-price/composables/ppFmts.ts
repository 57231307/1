/**
 * ppFmts.ts - 采购价格格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-price/index.vue）
 *
 * 价格状态词表以写入方为准：建单写 master_data::PENDING、批准写 APPROVED
 * （backend/src/services/purchase_price_service.rs:101/137）、停用写 master_data::INACTIVE
 * （update_price 透传 status）。词表模块 models/status/general.rs 的 master_data（小写）。
 * 历史前端用 active/inactive 门控与筛选，后端从不写 active ⇒ 筛选恒 0 行、编辑/停用按钮恒不可达。
 * 本模块以 pending/approved/inactive 原值为比较对象，文案走 i18n 键（purchasePrice.statusLabels.*），
 * 未知 token 抛错并记日志。
 */
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';

export const PURCHASE_PRICE_STATUS = {
  PENDING: 'pending',
  APPROVED: 'approved',
  INACTIVE: 'inactive',
} as const;

export type PurchasePriceStatus =
  (typeof PURCHASE_PRICE_STATUS)[keyof typeof PURCHASE_PRICE_STATUS];

export const PURCHASE_PRICE_STATUSES: PurchasePriceStatus[] = Object.values(PURCHASE_PRICE_STATUS);

export type PurchasePriceTagType = '' | 'success' | 'warning' | 'info' | 'danger';

export const PURCHASE_PRICE_STATUS_LABEL_KEYS: Record<PurchasePriceStatus, string> = {
  pending: 'purchasePrice.statusLabels.pending',
  approved: 'purchasePrice.statusLabels.approved',
  inactive: 'purchasePrice.statusLabels.inactive',
};

export const PURCHASE_PRICE_STATUS_TAG_TYPES: Record<PurchasePriceStatus, PurchasePriceTagType> = {
  pending: 'warning',
  approved: 'success',
  inactive: 'info',
};

export function normalizePurchasePriceStatus(
  status: string | undefined | null
): PurchasePriceStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((PURCHASE_PRICE_STATUSES as readonly string[]).includes(status)) {
    return status as PurchasePriceStatus;
  }
  const message =
    `未识别的采购价格状态「${status}」，` +
    '合法取值见后端 models/status/general.rs 的 master_data（写入侧 pending/approved）';
  logger.error(message, { status });
  throw new Error(message);
}

/** 获取价格状态 el-tag 类型 */
export const getStatusType = (status: string | undefined | null): PurchasePriceTagType => {
  const normalized = normalizePurchasePriceStatus(status);
  return normalized ? PURCHASE_PRICE_STATUS_TAG_TYPES[normalized] : 'info';
};

/** 获取采购价格状态显示文案（i18n） */
export const getStatusLabel = (status: string | undefined | null): string => {
  const normalized = normalizePurchasePriceStatus(status);
  const key = normalized ? PURCHASE_PRICE_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
  return i18n.global.t(key);
};

/** 获取价格类型标签（文案走 i18n，键 purchasePrice.form.priceType.*） */
export const getPriceTypeLabel = (type: string): string =>
  i18n.global.t(`purchasePrice.form.priceType.${type}`);

/** 格式化货币（人民币 6 位精度） */
export const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(6)}` : '¥0.000000';
};
