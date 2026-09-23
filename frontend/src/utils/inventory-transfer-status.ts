import { logger } from '@/utils/logger';

/**
 * 库存调拨单状态（inventory_transfers.status）的单一映射源。
 *
 * 取值与后端 models/status/purchase_inventory.rs:63 `inventory_transfer` 常量逐字一致（全小写）：
 * 建单写 pending（services/inv/inventory_move.rs build_transfer_active_model）、
 * 审批写 approved/rejected（approve_transfer 按 `approved: bool` 分支）、
 * 发货写 shipped、收货写 completed（ship_transfer / receive_transfer）。
 * 词表里没有 draft / executed / cancelled 这类历史界面上出现过的值，按它们筛选恒零命中，
 * 行内操作按钮的状态门槛也永远不会命中。
 *
 * Record<InventoryTransferStatus, …> 让「后端词表新增状态必须补全映射」由编译器保证。
 */
export const INVENTORY_TRANSFER_STATUS = {
  PENDING: 'pending',
  APPROVED: 'approved',
  REJECTED: 'rejected',
  SHIPPED: 'shipped',
  COMPLETED: 'completed',
} as const;

export type InventoryTransferStatus =
  (typeof INVENTORY_TRANSFER_STATUS)[keyof typeof INVENTORY_TRANSFER_STATUS];

/** 全部合法状态值（按业务流程排序）：筛选下拉的取值来源 */
export const INVENTORY_TRANSFER_STATUSES: InventoryTransferStatus[] =
  Object.values(INVENTORY_TRANSFER_STATUS);

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐） */
export type InventoryTransferTagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

/** 状态 → i18n 文案键（locales 的 inventoryTransfer.transferList.status.*，键名即后端原值） */
export const INVENTORY_TRANSFER_STATUS_LABEL_KEYS: Record<InventoryTransferStatus, string> = {
  pending: 'inventoryTransfer.transferList.status.pending',
  approved: 'inventoryTransfer.transferList.status.approved',
  rejected: 'inventoryTransfer.transferList.status.rejected',
  shipped: 'inventoryTransfer.transferList.status.shipped',
  completed: 'inventoryTransfer.transferList.status.completed',
};

/** 状态 → el-tag 配色 */
export const INVENTORY_TRANSFER_STATUS_TAG_TYPES: Record<
  InventoryTransferStatus,
  InventoryTransferTagType
> = {
  pending: 'warning',
  approved: 'primary',
  rejected: 'danger',
  shipped: 'warning',
  completed: 'success',
};

/**
 * 把后端返回的状态归一到已知枚举。
 * 词表外的值（含大小写写错、空值）不回显裸枚举，而是记错误日志并抛错——
 * 映射缺口必须让页面和日志同时报错，而不是伪装成「看起来像个状态」。
 */
export function normalizeInventoryTransferStatus(
  status: string | undefined
): InventoryTransferStatus {
  if ((INVENTORY_TRANSFER_STATUSES as readonly string[]).includes(status ?? '')) {
    return status as InventoryTransferStatus;
  }
  const message =
    `未识别的库存调拨单状态「${String(status)}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 inventory_transfer 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function inventoryTransferStatusLabelKey(status: string | undefined): string {
  return INVENTORY_TRANSFER_STATUS_LABEL_KEYS[normalizeInventoryTransferStatus(status)];
}

export function inventoryTransferStatusTagType(
  status: string | undefined
): InventoryTransferTagType {
  return INVENTORY_TRANSFER_STATUS_TAG_TYPES[normalizeInventoryTransferStatus(status)];
}
