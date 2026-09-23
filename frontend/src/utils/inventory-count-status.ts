import { logger } from '@/utils/logger';

/**
 * 库存盘点单状态（inventory_counts.status）的单一映射源。
 *
 * 取值与后端 models/status/purchase_inventory.rs:82 `inventory_count` 常量逐字一致（全小写）：
 * 建单写 pending（inventory_count_service.rs:155）、提交审批写 in_review（:417，
 * 语义是「实盘已录入、等待复核确认」）、复核通过写 completed（:524）、驳回退回 pending（:663）。
 * 词表里没有 in_progress：写它的前端门控（编辑/完成按钮、状态筛选、统计卡）恒不可达。
 *
 * Record<InventoryCountStatus, …> 让「后端词表新增状态必须补全映射」由编译器保证。
 */
export const INVENTORY_COUNT_STATUS = {
  PENDING: 'pending',
  IN_REVIEW: 'in_review',
  COMPLETED: 'completed',
} as const;

export type InventoryCountStatus =
  (typeof INVENTORY_COUNT_STATUS)[keyof typeof INVENTORY_COUNT_STATUS];

/** 全部合法状态值（按业务流程排序）：筛选下拉的取值来源 */
export const INVENTORY_COUNT_STATUSES: InventoryCountStatus[] =
  Object.values(INVENTORY_COUNT_STATUS);

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐） */
export type InventoryCountTagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

/** 状态 → i18n 文案键（locales 的 inventoryCount.listTab.statusLabel.*，键名即后端原值） */
export const INVENTORY_COUNT_STATUS_LABEL_KEYS: Record<InventoryCountStatus, string> = {
  pending: 'inventoryCount.listTab.statusLabel.pending',
  in_review: 'inventoryCount.listTab.statusLabel.in_review',
  completed: 'inventoryCount.listTab.statusLabel.completed',
};

/** 状态 → el-tag 配色 */
export const INVENTORY_COUNT_STATUS_TAG_TYPES: Record<InventoryCountStatus, InventoryCountTagType> =
  {
    pending: 'warning',
    in_review: 'primary',
    completed: 'success',
  };

/**
 * 把后端返回的状态归一到已知枚举。
 * 词表外的值不回显裸枚举，而是记错误日志并抛错——映射缺口必须让页面和日志同时报错。
 */
export function normalizeInventoryCountStatus(status: string | undefined): InventoryCountStatus {
  if ((INVENTORY_COUNT_STATUSES as readonly string[]).includes(status ?? '')) {
    return status as InventoryCountStatus;
  }
  const message =
    `未识别的库存盘点单状态「${String(status)}」，` +
    '合法取值见后端 models/status/purchase_inventory.rs 的 inventory_count 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function inventoryCountStatusLabelKey(status: string | undefined): string {
  return INVENTORY_COUNT_STATUS_LABEL_KEYS[normalizeInventoryCountStatus(status)];
}

export function inventoryCountStatusTagType(status: string | undefined): InventoryCountTagType {
  return INVENTORY_COUNT_STATUS_TAG_TYPES[normalizeInventoryCountStatus(status)];
}
