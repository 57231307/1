import { logger } from '@/utils/logger';

/**
 * 库存调整单状态（inventory_adjustments.status）的单一映射源。
 *
 * 取值与后端 models/status/purchase_inventory.rs:124 `inventory_adjustment` 常量逐字一致（全小写）：
 * 建单写 pending、审批通过写 approved（approve_adjustment）、驳回写 rejected（reject_adjustment）。
 * 词表里没有 submitted / completed，按它们筛选恒零命中、行内按钮恒不可达。
 *
 * Record<InventoryAdjustmentStatus, …> 让「后端词表新增状态必须补全映射」由编译器保证。
 */
export const INVENTORY_ADJUSTMENT_STATUS = {
  PENDING: 'pending',
  APPROVED: 'approved',
  REJECTED: 'rejected',
} as const;

export type InventoryAdjustmentStatus =
  (typeof INVENTORY_ADJUSTMENT_STATUS)[keyof typeof INVENTORY_ADJUSTMENT_STATUS];

/** 全部合法状态值（按业务流程排序）：筛选下拉的取值来源 */
export const INVENTORY_ADJUSTMENT_STATUSES: InventoryAdjustmentStatus[] = Object.values(
  INVENTORY_ADJUSTMENT_STATUS
);

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐） */
export type InventoryAdjustmentTagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

/** 状态 → i18n 文案键（locales 的 inventoryAdjustment.listTab.*，键名即后端原值） */
export const INVENTORY_ADJUSTMENT_STATUS_LABEL_KEYS: Record<InventoryAdjustmentStatus, string> = {
  pending: 'inventoryAdjustment.listTab.statusPending',
  approved: 'inventoryAdjustment.listTab.statusApproved',
  rejected: 'inventoryAdjustment.listTab.statusRejected',
};

/** 状态 → el-tag 配色 */
export const INVENTORY_ADJUSTMENT_STATUS_TAG_TYPES: Record<
  InventoryAdjustmentStatus,
  InventoryAdjustmentTagType
> = {
  pending: 'warning',
  approved: 'success',
  rejected: 'danger',
};

/**
 * 把后端返回的状态归一到已知枚举。
 * - null/undefined/空串（字段缺失或尚无状态）：返回 undefined，由调用方渲染中性占位；
 * - 词表外的非空取值（脏数据）：记错误日志并抛错。
 */
export function normalizeInventoryAdjustmentStatus(
  status: string | undefined | null
): InventoryAdjustmentStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((INVENTORY_ADJUSTMENT_STATUSES as readonly string[]).includes(status)) {
    return status as InventoryAdjustmentStatus;
  }
  const message =
    `未识别的库存调整单状态「${status}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 inventory_adjustment 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function inventoryAdjustmentStatusLabelKey(status: string | undefined | null): string {
  const normalized = normalizeInventoryAdjustmentStatus(status);
  return normalized ? INVENTORY_ADJUSTMENT_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function inventoryAdjustmentStatusTagType(
  status: string | undefined | null
): InventoryAdjustmentTagType {
  const normalized = normalizeInventoryAdjustmentStatus(status);
  return normalized ? INVENTORY_ADJUSTMENT_STATUS_TAG_TYPES[normalized] : 'info';
}
