import { logger } from '@/utils/logger';

/**
 * 库存批次两条状态列的单一真相源（中文稳定值即库内取值）。
 *
 * 取值与后端 `backend/src/models/status/purchase_inventory.rs` 的
 * `inventory_stock_status`（正常/报废/已删除）与 `inventory_stock_quality_status`
 * （合格/待检/不合格）常量逐字一致；`models/inventory_stock.rs` 的 `stock_status` /
 * `quality_status` 两列只写入这几个值，比较点必须用原值而不是译文
 * （译文标签随语言变化，拿译名做门控会让判断恒 false）。
 */
export const INVENTORY_STOCK_STATUS = {
  NORMAL: '正常',
  SCRAPPED: '报废',
  DELETED: '已删除',
} as const;

export type InventoryStockStatusValue =
  (typeof INVENTORY_STOCK_STATUS)[keyof typeof INVENTORY_STOCK_STATUS];

export const INVENTORY_STOCK_STATUSES: InventoryStockStatusValue[] = [
  INVENTORY_STOCK_STATUS.NORMAL,
  INVENTORY_STOCK_STATUS.SCRAPPED,
  INVENTORY_STOCK_STATUS.DELETED,
];

export const INVENTORY_QUALITY_STATUS = {
  PASS: '合格',
  PENDING: '待检',
  FAIL: '不合格',
} as const;

export type InventoryQualityStatusValue =
  (typeof INVENTORY_QUALITY_STATUS)[keyof typeof INVENTORY_QUALITY_STATUS];

export const INVENTORY_QUALITY_STATUSES: InventoryQualityStatusValue[] = [
  INVENTORY_QUALITY_STATUS.PASS,
  INVENTORY_QUALITY_STATUS.PENDING,
  INVENTORY_QUALITY_STATUS.FAIL,
];

export type InventoryStatusTagType = 'success' | 'warning' | 'danger' | 'info';

/** 归一到已知库存状态；词表外的值记错误并抛错（映射缺口必须报错，不伪装成状态） */
export function normalizeInventoryStockStatus(
  status: string | undefined
): InventoryStockStatusValue {
  if ((INVENTORY_STOCK_STATUSES as readonly string[]).includes(status ?? '')) {
    return status as InventoryStockStatusValue;
  }
  const message =
    `未识别的库存状态「${String(status)}」，合法取值见后端 ` +
    'models/status/purchase_inventory.rs 的 inventory_stock_status 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function normalizeInventoryQualityStatus(
  status: string | undefined
): InventoryQualityStatusValue {
  if ((INVENTORY_QUALITY_STATUSES as readonly string[]).includes(status ?? '')) {
    return status as InventoryQualityStatusValue;
  }
  const message =
    `未识别的库存质量状态「${String(status)}」，合法取值见后端 ` +
    'models/status/purchase_inventory.rs 的 inventory_stock_quality_status 模块';
  logger.error(message, { status });
  throw new Error(message);
}

const STOCK_STATUS_TAG: Record<InventoryStockStatusValue, InventoryStatusTagType> = {
  [INVENTORY_STOCK_STATUS.NORMAL]: 'success',
  [INVENTORY_STOCK_STATUS.SCRAPPED]: 'danger',
  [INVENTORY_STOCK_STATUS.DELETED]: 'info',
};

const QUALITY_STATUS_TAG: Record<InventoryQualityStatusValue, InventoryStatusTagType> = {
  [INVENTORY_QUALITY_STATUS.PASS]: 'success',
  [INVENTORY_QUALITY_STATUS.PENDING]: 'warning',
  [INVENTORY_QUALITY_STATUS.FAIL]: 'danger',
};

/** 渲染期取色：未知值不抛错打断整表渲染，告警后回落 info 配色 */
export function stockStatusTagType(status: string | null | undefined): InventoryStatusTagType {
  if (!(INVENTORY_STOCK_STATUSES as readonly string[]).includes(status ?? '')) {
    logger.warn(`[stock-status] 未知库存状态「${String(status)}」，回落 info 配色`);
    return 'info';
  }
  return STOCK_STATUS_TAG[status as InventoryStockStatusValue];
}

export function qualityStatusTagType(status: string | null | undefined): InventoryStatusTagType {
  if (!(INVENTORY_QUALITY_STATUSES as readonly string[]).includes(status ?? '')) {
    logger.warn(`[stock-status] 未知库存质量状态「${String(status)}」，回落 info 配色`);
    return 'info';
  }
  return QUALITY_STATUS_TAG[status as InventoryQualityStatusValue];
}
