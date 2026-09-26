import { logger } from '@/utils/logger';

/**
 * 采购入库单状态词表（单一映射源）。
 *
 * 本文件覆盖 purchase_receipt 表的两列独立状态，取值与后端逐字一致：
 *
 * 1. receipt_status（入库状态，大写）：
 *    来源 backend/src/models/status/purchase_inventory.rs 的 purchase_receipt 模块
 *    （DRAFT/CONFIRMED/COMPLETED）。写入方见 services/purchase_receipt_service.rs
 *    （建单写 DRAFT、确认写 COMPLETED）。列表/详情接口直接 to_value(Model)，出参键为
 *    `receipt_status`（非 `status`），比较点必须与该原值逐字符相同。
 *
 * 2. inspection_status（质检状态，大写）：
 *    来源 backend/src/models/status/purchase_inventory.rs 的 purchase_receipt_inspection
 *    模块（PENDING/PASSED/REJECTED）。建单初始值写 PENDING（service 第 74 行）。
 *    PENDING 的权威中文显示词取自该模块文档注释「待检验」，与 quality_inspection_result
 *    中文词族（合格/不合格）一致，禁止英文化为 PASSED/REJECTED 原值直显。
 */

/** 入库状态 → el-tag 配色类型（与 element-plus el-tag type 联合一致） */
export type PurchaseReceiptTagType = '' | 'success' | 'warning' | 'info' | 'danger';

// ============ 1. receipt_status ============

export const PURCHASE_RECEIPT_STATUS = {
  DRAFT: 'DRAFT',
  CONFIRMED: 'CONFIRMED',
  COMPLETED: 'COMPLETED',
} as const;

export type PurchaseReceiptStatus =
  (typeof PURCHASE_RECEIPT_STATUS)[keyof typeof PURCHASE_RECEIPT_STATUS];

/** 全部合法入库状态值（按业务流程排序）：筛选下拉的取值来源 */
export const PURCHASE_RECEIPT_STATUSES: PurchaseReceiptStatus[] =
  Object.values(PURCHASE_RECEIPT_STATUS);

const RECEIPT_STATUS_LABEL_KEYS: Record<PurchaseReceiptStatus, string> = {
  DRAFT: 'purchaseReceipt.statusLabels.DRAFT',
  CONFIRMED: 'purchaseReceipt.statusLabels.CONFIRMED',
  COMPLETED: 'purchaseReceipt.statusLabels.COMPLETED',
};

const RECEIPT_STATUS_TAG_TYPES: Record<PurchaseReceiptStatus, PurchaseReceiptTagType> = {
  DRAFT: 'info',
  CONFIRMED: 'warning',
  COMPLETED: 'success',
};

/**
 * 归一后端返回的入库状态。
 * - null/undefined/空串：返回 undefined，由调用方渲染中性占位；
 * - 词表外非空取值（脏数据）：记错误日志并抛错，不静默兜底。
 */
export function normalizePurchaseReceiptStatus(
  status: string | null | undefined
): PurchaseReceiptStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((PURCHASE_RECEIPT_STATUSES as readonly string[]).includes(status)) {
    return status as PurchaseReceiptStatus;
  }
  const message =
    `未识别的采购入库状态「${status}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 purchase_receipt 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function purchaseReceiptStatusLabelKey(status: string | null | undefined): string {
  const normalized = normalizePurchaseReceiptStatus(status);
  return normalized ? RECEIPT_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function purchaseReceiptStatusTagType(
  status: string | null | undefined
): PurchaseReceiptTagType {
  const normalized = normalizePurchaseReceiptStatus(status);
  return normalized ? RECEIPT_STATUS_TAG_TYPES[normalized] : 'info';
}

// ============ 2. inspection_status ============

export const PURCHASE_RECEIPT_INSPECTION_STATUS = {
  PENDING: 'PENDING',
  PASSED: 'PASSED',
  REJECTED: 'REJECTED',
} as const;

export type PurchaseReceiptInspectionStatus =
  (typeof PURCHASE_RECEIPT_INSPECTION_STATUS)[keyof typeof PURCHASE_RECEIPT_INSPECTION_STATUS];

export const PURCHASE_RECEIPT_INSPECTION_STATUSES: PurchaseReceiptInspectionStatus[] =
  Object.values(PURCHASE_RECEIPT_INSPECTION_STATUS);

const RECEIPT_INSPECTION_STATUS_LABEL_KEYS: Record<PurchaseReceiptInspectionStatus, string> = {
  PENDING: 'purchaseReceipt.inspectionLabels.PENDING',
  PASSED: 'purchaseReceipt.inspectionLabels.PASSED',
  REJECTED: 'purchaseReceipt.inspectionLabels.REJECTED',
};

const RECEIPT_INSPECTION_STATUS_TAG_TYPES: Record<
  PurchaseReceiptInspectionStatus,
  PurchaseReceiptTagType
> = {
  PENDING: 'warning',
  PASSED: 'success',
  REJECTED: 'danger',
};

/** 归一入库单的质检状态；语义与 normalizePurchaseReceiptStatus 同构。 */
export function normalizePurchaseReceiptInspectionStatus(
  status: string | null | undefined
): PurchaseReceiptInspectionStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((PURCHASE_RECEIPT_INSPECTION_STATUSES as readonly string[]).includes(status)) {
    return status as PurchaseReceiptInspectionStatus;
  }
  const message =
    `未识别的入库单质检状态「${status}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 purchase_receipt_inspection 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function purchaseReceiptInspectionStatusLabelKey(status: string | null | undefined): string {
  const normalized = normalizePurchaseReceiptInspectionStatus(status);
  return normalized ? RECEIPT_INSPECTION_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function purchaseReceiptInspectionStatusTagType(
  status: string | null | undefined
): PurchaseReceiptTagType {
  const normalized = normalizePurchaseReceiptInspectionStatus(status);
  return normalized ? RECEIPT_INSPECTION_STATUS_TAG_TYPES[normalized] : 'info';
}
