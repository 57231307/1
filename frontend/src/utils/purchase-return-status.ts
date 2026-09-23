import { logger } from '@/utils/logger';

/**
 * 采购退货状态（purchase_return.return_status）的单一映射源。
 *
 * 取值与后端 models/status/purchase_inventory.rs 的 purchase_return 常量逐字一致（全小写）：
 * 建单写 draft（services/purchase_return_service.rs create_return）、提交写 submitted
 * （submit_return）、审批写 approved（update_return_status_to_approved）、拒绝写 rejected
 * （reject_return）。词表没有 pending/completed，历史前端把提交态写成 pending、完成态写成
 * completed，既取不到数据也对应不上文案，故本模块只收录写入侧真实存在的四态。
 *
 * 列表筛选（purchase_return::Column::ReturnStatus.eq(status)）与门控比较点都必须用本模块原值。
 */
export const PURCHASE_RETURN_STATUS = {
  DRAFT: 'draft',
  SUBMITTED: 'submitted',
  APPROVED: 'approved',
  REJECTED: 'rejected',
} as const;

export type PurchaseReturnStatus =
  (typeof PURCHASE_RETURN_STATUS)[keyof typeof PURCHASE_RETURN_STATUS];

/** 全部合法状态值（按业务流程排序）：筛选下拉的取值来源 */
export const PURCHASE_RETURN_STATUSES: PurchaseReturnStatus[] =
  Object.values(PURCHASE_RETURN_STATUS);

export type PurchaseReturnTagType = '' | 'success' | 'warning' | 'info' | 'danger';

/** 状态 → i18n 文案键 */
export const PURCHASE_RETURN_STATUS_LABEL_KEYS: Record<PurchaseReturnStatus, string> = {
  draft: 'purchaseReturn.filter.status.draft',
  submitted: 'purchaseReturn.filter.status.submitted',
  approved: 'purchaseReturn.filter.status.approved',
  rejected: 'purchaseReturn.filter.status.rejected',
};

/** 状态 → el-tag 配色 */
export const PURCHASE_RETURN_STATUS_TAG_TYPES: Record<PurchaseReturnStatus, PurchaseReturnTagType> =
  {
    draft: 'info',
    submitted: 'warning',
    approved: 'success',
    rejected: 'danger',
  };

/**
 * 把后端返回的状态归一到已知枚举。词表外的值记错误日志并抛错——映射缺口必须让页面和日志
 * 同时报错，而不是伪装成"看起来像个状态"。
 */
export function normalizePurchaseReturnStatus(
  status: string | null | undefined
): PurchaseReturnStatus {
  if ((PURCHASE_RETURN_STATUSES as readonly string[]).includes(status ?? '')) {
    return status as PurchaseReturnStatus;
  }
  const message =
    `未识别的采购退货状态「${String(status)}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 purchase_return 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function purchaseReturnStatusLabelKey(status: string | null | undefined): string {
  return PURCHASE_RETURN_STATUS_LABEL_KEYS[normalizePurchaseReturnStatus(status)];
}

export function purchaseReturnStatusTagType(
  status: string | null | undefined
): PurchaseReturnTagType {
  return PURCHASE_RETURN_STATUS_TAG_TYPES[normalizePurchaseReturnStatus(status)];
}
