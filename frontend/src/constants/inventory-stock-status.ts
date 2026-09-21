/**
 * 库存台账状态（inventory_stocks.stock_status）单一真相源。
 *
 * 取值必须与后端主数据值一致：后端写入的是中文值（新建/收货为「正常」，批色报废为
 * 「报废」，删除为软删除标记「已删除」），此前前端筛选提交 normal/warning/frozen
 * 这类库里不存在的值，且查询参数名用的是后端不认识的 status，筛选恒零命中。
 */
export const INVENTORY_STOCK_STATUS = {
  normal: '正常',
  scrapped: '报废',
  deleted: '已删除',
} as const;

export type InventoryStockStatusValue =
  (typeof INVENTORY_STOCK_STATUS)[keyof typeof INVENTORY_STOCK_STATUS];

/** 台账状态 → i18n 文案键（未知取值由调用方告警并原样显示，不再静默掩盖） */
export const INVENTORY_STOCK_STATUS_LABEL_KEY: Record<string, string> = {
  [INVENTORY_STOCK_STATUS.normal]: 'inventory.stockTab.statusNormal',
  [INVENTORY_STOCK_STATUS.scrapped]: 'inventory.stockTab.statusScrapped',
  [INVENTORY_STOCK_STATUS.deleted]: 'inventory.stockTab.statusDeleted',
};

/** 可筛选的台账状态（含软删除，便于按追溯需要显式查回） */
export const INVENTORY_STOCK_STATUS_OPTIONS: InventoryStockStatusValue[] = [
  INVENTORY_STOCK_STATUS.normal,
  INVENTORY_STOCK_STATUS.scrapped,
  INVENTORY_STOCK_STATUS.deleted,
];
