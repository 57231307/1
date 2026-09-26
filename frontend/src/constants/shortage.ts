/**
 * 缺料预警取值单一真相源（级别 + 状态机）。
 *
 * 必须与后端一致：
 * - level 取自 `services/material_shortage_service.rs::ShortageLevel::as_str`（首字母大写）；
 * - status 取自 `models/status/purchase_inventory.rs::shortage_alert_status`（小写），
 *   状态机 identified → purchase_request → purchase_order → received → resolved。
 * 后端对越界取值直接 400，前端不得再提交 pending / notified / critical 这类库里不存在的值。
 */
export const SHORTAGE_LEVEL = {
  critical: 'Critical',
  severe: 'Severe',
  warning: 'Warning',
  normal: 'Normal',
} as const;

export type ShortageLevelValue = (typeof SHORTAGE_LEVEL)[keyof typeof SHORTAGE_LEVEL];

/** 全部级别，顺序即严重度降序（与后端 sort_items_by_level 一致） */
export const SHORTAGE_LEVEL_VALUES: ShortageLevelValue[] = [
  SHORTAGE_LEVEL.critical,
  SHORTAGE_LEVEL.severe,
  SHORTAGE_LEVEL.warning,
  SHORTAGE_LEVEL.normal,
];

/** 级别 → i18n 文案键 */
export const SHORTAGE_LEVEL_LABEL_KEY: Record<string, string> = {
  [SHORTAGE_LEVEL.critical]: 'materialShortage.level.critical',
  [SHORTAGE_LEVEL.severe]: 'materialShortage.level.severe',
  [SHORTAGE_LEVEL.warning]: 'materialShortage.level.warning',
  [SHORTAGE_LEVEL.normal]: 'materialShortage.level.normal',
};

export type ShortageTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger';

export const SHORTAGE_LEVEL_TAG_TYPE: Record<string, ShortageTagType> = {
  [SHORTAGE_LEVEL.critical]: 'danger',
  [SHORTAGE_LEVEL.severe]: 'warning',
  [SHORTAGE_LEVEL.warning]: 'primary',
  [SHORTAGE_LEVEL.normal]: 'info',
};

/** 汇总接口按级别计数（ShortageSummary 只有 critical/severe/warning 三档，Normal 不入清单） */
export const SHORTAGE_COUNTED_LEVELS: ShortageLevelValue[] = [
  SHORTAGE_LEVEL.critical,
  SHORTAGE_LEVEL.severe,
  SHORTAGE_LEVEL.warning,
];

export const SHORTAGE_ALERT_STATUS = {
  identified: 'identified',
  purchaseRequest: 'purchase_request',
  purchaseOrder: 'purchase_order',
  received: 'received',
  resolved: 'resolved',
} as const;

export type ShortageAlertStatusValue =
  (typeof SHORTAGE_ALERT_STATUS)[keyof typeof SHORTAGE_ALERT_STATUS];

/** 全部合法状态，顺序即状态机推进顺序 */
export const SHORTAGE_ALERT_STATUS_VALUES: ShortageAlertStatusValue[] = [
  SHORTAGE_ALERT_STATUS.identified,
  SHORTAGE_ALERT_STATUS.purchaseRequest,
  SHORTAGE_ALERT_STATUS.purchaseOrder,
  SHORTAGE_ALERT_STATUS.received,
  SHORTAGE_ALERT_STATUS.resolved,
];

/** 状态 → i18n 文案键 */
export const SHORTAGE_ALERT_STATUS_LABEL_KEY: Record<string, string> = {
  [SHORTAGE_ALERT_STATUS.identified]: 'materialShortage.status.identified',
  [SHORTAGE_ALERT_STATUS.purchaseRequest]: 'materialShortage.status.purchaseRequest',
  [SHORTAGE_ALERT_STATUS.purchaseOrder]: 'materialShortage.status.purchaseOrder',
  [SHORTAGE_ALERT_STATUS.received]: 'materialShortage.status.received',
  [SHORTAGE_ALERT_STATUS.resolved]: 'materialShortage.status.resolved',
};

export const SHORTAGE_ALERT_STATUS_TAG_TYPE: Record<string, ShortageTagType> = {
  [SHORTAGE_ALERT_STATUS.identified]: 'danger',
  [SHORTAGE_ALERT_STATUS.purchaseRequest]: 'warning',
  [SHORTAGE_ALERT_STATUS.purchaseOrder]: 'primary',
  [SHORTAGE_ALERT_STATUS.received]: 'success',
  [SHORTAGE_ALERT_STATUS.resolved]: 'info',
};

/**
 * 补货建议优先级（后端 ReplenishmentSuggestion::priority 由缺料级别映射而来：
 * Critical→URGENT、Severe→HIGH、Warning→MEDIUM、Normal→LOW，大写值）
 */
export const REPLENISHMENT_PRIORITY = {
  urgent: 'URGENT',
  high: 'HIGH',
  medium: 'MEDIUM',
  low: 'LOW',
} as const;

export type ReplenishmentPriorityValue =
  (typeof REPLENISHMENT_PRIORITY)[keyof typeof REPLENISHMENT_PRIORITY];

export const REPLENISHMENT_PRIORITY_LABEL_KEY: Record<string, string> = {
  [REPLENISHMENT_PRIORITY.urgent]: 'materialShortage.priority.urgent',
  [REPLENISHMENT_PRIORITY.high]: 'materialShortage.priority.high',
  [REPLENISHMENT_PRIORITY.medium]: 'materialShortage.priority.medium',
  [REPLENISHMENT_PRIORITY.low]: 'materialShortage.priority.low',
};

export const REPLENISHMENT_PRIORITY_TAG_TYPE: Record<string, ShortageTagType> = {
  [REPLENISHMENT_PRIORITY.urgent]: 'danger',
  [REPLENISHMENT_PRIORITY.high]: 'warning',
  [REPLENISHMENT_PRIORITY.medium]: 'primary',
  [REPLENISHMENT_PRIORITY.low]: 'info',
};

/**
 * 状态机下一步：resolved 为终态返回 null。
 * 取值域外（脏数据）同样返回 null，由调用方告警，不猜测推进目标。
 */
export const nextShortageAlertStatus = (status: string): ShortageAlertStatusValue | null => {
  const idx = SHORTAGE_ALERT_STATUS_VALUES.indexOf(status as ShortageAlertStatusValue);
  if (idx < 0 || idx === SHORTAGE_ALERT_STATUS_VALUES.length - 1) {
    return null;
  }
  return SHORTAGE_ALERT_STATUS_VALUES[idx + 1];
};
