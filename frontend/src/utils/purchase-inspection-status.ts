import { logger } from '@/utils/logger';

/**
 * 采购质检状态（purchase_inspection.inspection_status）的单一映射源。
 *
 * 取值与后端 models/status/purchase_inventory.rs 的 purchase_inspection 常量逐字一致（全小写）：
 * 建单写 pending（services/purchase_inspection_service.rs create_inspection）、完成写 completed
 * （complete_inspection）。词表只有这两态——历史前端把草稿态写成 draft、驳回态写成 rejected，
 * 后端永不产生这些值，导致门控按钮与筛选恒不命中。
 */
export const PURCHASE_INSPECTION_STATUS = {
  PENDING: 'pending',
  COMPLETED: 'completed',
} as const;

export type PurchaseInspectionStatus =
  (typeof PURCHASE_INSPECTION_STATUS)[keyof typeof PURCHASE_INSPECTION_STATUS];

/** 全部合法状态值（按业务流程排序）：筛选下拉的取值来源 */
export const PURCHASE_INSPECTION_STATUSES: PurchaseInspectionStatus[] = Object.values(
  PURCHASE_INSPECTION_STATUS
);

export type PurchaseInspectionTagType = '' | 'success' | 'warning' | 'info' | 'danger';

/** 状态 → i18n 文案键（复用既有筛选下拉文案键） */
export const PURCHASE_INSPECTION_STATUS_LABEL_KEYS: Record<PurchaseInspectionStatus, string> = {
  pending: 'purchaseInspection.filter.status.pending',
  completed: 'purchaseInspection.filter.status.completed',
};

/** 状态 → el-tag 配色 */
export const PURCHASE_INSPECTION_STATUS_TAG_TYPES: Record<
  PurchaseInspectionStatus,
  PurchaseInspectionTagType
> = {
  pending: 'warning',
  completed: 'success',
};

/**
 * 把后端返回的状态归一到已知枚举。
 * - null/undefined/空串（字段缺失或尚无状态，inspection_status 为 Option<String>）：
 *   返回 undefined，由调用方渲染中性占位；
 * - 词表外的非空取值（脏数据）：记错误日志并抛错。
 */
export function normalizePurchaseInspectionStatus(
  status: string | null | undefined
): PurchaseInspectionStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((PURCHASE_INSPECTION_STATUSES as readonly string[]).includes(status)) {
    return status as PurchaseInspectionStatus;
  }
  const message =
    `未识别的采购质检状态「${status}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 purchase_inspection 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function purchaseInspectionStatusLabelKey(status: string | null | undefined): string {
  const normalized = normalizePurchaseInspectionStatus(status);
  return normalized ? PURCHASE_INSPECTION_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function purchaseInspectionStatusTagType(
  status: string | null | undefined
): PurchaseInspectionTagType {
  const normalized = normalizePurchaseInspectionStatus(status);
  return normalized ? PURCHASE_INSPECTION_STATUS_TAG_TYPES[normalized] : 'info';
}
