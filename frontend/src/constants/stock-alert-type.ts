/**
 * 库存告警类型（`GET /inventory/stock/alerts` 的 alert_type）单一真相源。
 *
 * 取值来自后端 `services::stock_alert::AlertType`（派生逻辑 compute_alert_type），
 * 小写码值：normal / low_stock / out_of_stock / over_stock / slow_moving / expiring / discrepancy。
 * 前端此前用的是自造的 `alert_level: 'warning' | 'danger'`，后端从不返回该列，
 * 预警级别标签因此恒显示"警告"。
 */
export const STOCK_ALERT_TYPE = {
  normal: 'normal',
  lowStock: 'low_stock',
  outOfStock: 'out_of_stock',
  overStock: 'over_stock',
  slowMoving: 'slow_moving',
  expiring: 'expiring',
  discrepancy: 'discrepancy',
} as const;

export type StockAlertTypeValue = (typeof STOCK_ALERT_TYPE)[keyof typeof STOCK_ALERT_TYPE];

export const STOCK_ALERT_TYPE_VALUES: StockAlertTypeValue[] = [
  STOCK_ALERT_TYPE.normal,
  STOCK_ALERT_TYPE.lowStock,
  STOCK_ALERT_TYPE.outOfStock,
  STOCK_ALERT_TYPE.overStock,
  STOCK_ALERT_TYPE.slowMoving,
  STOCK_ALERT_TYPE.expiring,
  STOCK_ALERT_TYPE.discrepancy,
];

/** 告警类型 → i18n 文案键（越界值由调用方告警并原样显示，不猜含义） */
export const STOCK_ALERT_TYPE_LABEL_KEY: Record<StockAlertTypeValue, string> = {
  [STOCK_ALERT_TYPE.normal]: 'inventory.alertTab.alertTypeNormal',
  [STOCK_ALERT_TYPE.lowStock]: 'inventory.alertTab.alertTypeLowStock',
  [STOCK_ALERT_TYPE.outOfStock]: 'inventory.alertTab.alertTypeOutOfStock',
  [STOCK_ALERT_TYPE.overStock]: 'inventory.alertTab.alertTypeOverStock',
  [STOCK_ALERT_TYPE.slowMoving]: 'inventory.alertTab.alertTypeSlowMoving',
  [STOCK_ALERT_TYPE.expiring]: 'inventory.alertTab.alertTypeExpiring',
  [STOCK_ALERT_TYPE.discrepancy]: 'inventory.alertTab.alertTypeDiscrepancy',
};

/** 告警类型 → el-tag 颜色：缺货与盘点差异按危险处理，其余需关注项按警告 */
export const STOCK_ALERT_TYPE_TAG_TYPES: Record<
  StockAlertTypeValue,
  'success' | 'warning' | 'danger'
> = {
  [STOCK_ALERT_TYPE.normal]: 'success',
  [STOCK_ALERT_TYPE.lowStock]: 'warning',
  [STOCK_ALERT_TYPE.outOfStock]: 'danger',
  [STOCK_ALERT_TYPE.overStock]: 'warning',
  [STOCK_ALERT_TYPE.slowMoving]: 'warning',
  [STOCK_ALERT_TYPE.expiring]: 'warning',
  [STOCK_ALERT_TYPE.discrepancy]: 'danger',
};
