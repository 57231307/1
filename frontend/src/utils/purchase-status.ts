import { logger } from '@/utils/logger';

/**
 * 采购订单状态（purchase_order.order_status）的单一映射源。
 *
 * 取值与后端 models/status/purchase_inventory.rs 的 purchase_order 常量逐字一致（全大写）：
 * 建单写 DRAFT（services/po/order_ops/crud.rs）、提交写 PENDING_APPROVAL、审批写
 * APPROVED/REJECTED（services/po/contract.rs）、收货回写 PARTIAL_RECEIVED/COMPLETED
 * （services/purchase_receipt_private.rs）、关闭写 CLOSED（order_ops/lifecycle.rs）、
 * 取消写 CANCELLED（services/po/contract.rs）。SUBMITTED 后端已声明常量但当前无写入点，
 * 仍保留映射。
 *
 * 列表/导出接口用 OrderStatus.eq(status) 精确匹配、不做大小写归一也不做取值校验
 * （services/po/order_ops/crud.rs 的 list_orders），因此「筛选提交值」「行内操作按钮的
 * 状态门槛」「文案映射键」三处都必须使用本模块的原值，任何其他形态（小写、译文）
 * 在后端既筛不出数据也对应不上文案。
 *
 * Record<PurchaseOrderStatus, …> 让「后端词表新增状态必须补全映射」由编译器保证。
 */
export const PURCHASE_ORDER_STATUS = {
  DRAFT: 'DRAFT',
  PENDING_APPROVAL: 'PENDING_APPROVAL',
  SUBMITTED: 'SUBMITTED',
  APPROVED: 'APPROVED',
  PARTIAL_RECEIVED: 'PARTIAL_RECEIVED',
  COMPLETED: 'COMPLETED',
  CLOSED: 'CLOSED',
  REJECTED: 'REJECTED',
  CANCELLED: 'CANCELLED',
} as const;

export type PurchaseOrderStatus =
  (typeof PURCHASE_ORDER_STATUS)[keyof typeof PURCHASE_ORDER_STATUS];

/** 全部合法状态值（按业务流程排序）：筛选下拉的取值来源 */
export const PURCHASE_ORDER_STATUSES: PurchaseOrderStatus[] = Object.values(PURCHASE_ORDER_STATUS);

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐，'' 为默认主题色） */
export type PurchaseTagType = '' | 'success' | 'warning' | 'info' | 'danger';

/** 状态 → i18n 文案键（locales 的 purchase.statusLabels.*，键名即后端原值） */
export const PURCHASE_STATUS_LABEL_KEYS: Record<PurchaseOrderStatus, string> = {
  DRAFT: 'purchase.statusLabels.DRAFT',
  PENDING_APPROVAL: 'purchase.statusLabels.PENDING_APPROVAL',
  SUBMITTED: 'purchase.statusLabels.SUBMITTED',
  APPROVED: 'purchase.statusLabels.APPROVED',
  PARTIAL_RECEIVED: 'purchase.statusLabels.PARTIAL_RECEIVED',
  COMPLETED: 'purchase.statusLabels.COMPLETED',
  CLOSED: 'purchase.statusLabels.CLOSED',
  REJECTED: 'purchase.statusLabels.REJECTED',
  CANCELLED: 'purchase.statusLabels.CANCELLED',
};

/** 状态 → el-tag 配色 */
export const PURCHASE_STATUS_TAG_TYPES: Record<PurchaseOrderStatus, PurchaseTagType> = {
  DRAFT: 'info',
  PENDING_APPROVAL: 'warning',
  SUBMITTED: 'warning',
  APPROVED: '',
  PARTIAL_RECEIVED: 'info',
  COMPLETED: 'success',
  CLOSED: 'info',
  REJECTED: 'danger',
  CANCELLED: 'danger',
};

/**
 * 把后端返回的状态归一到已知枚举。
 * - null/undefined/空串（字段缺失或尚无状态）：返回 undefined，由调用方渲染中性占位；
 * - 词表外的非空取值（脏数据）：记错误日志并抛错，映射缺口必须让页面和日志同时报错。
 */
export function normalizePurchaseOrderStatus(
  status: string | undefined | null
): PurchaseOrderStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((PURCHASE_ORDER_STATUSES as readonly string[]).includes(status)) {
    return status as PurchaseOrderStatus;
  }
  const message =
    `未识别的采购订单状态「${status}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 purchase_order 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function purchaseStatusLabelKey(status: string | undefined | null): string {
  const normalized = normalizePurchaseOrderStatus(status);
  return normalized ? PURCHASE_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function purchaseStatusTagType(status: string | undefined | null): PurchaseTagType {
  const normalized = normalizePurchaseOrderStatus(status);
  return normalized ? PURCHASE_STATUS_TAG_TYPES[normalized] : 'info';
}
