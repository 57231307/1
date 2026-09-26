/**
 * 运单状态（logistics_waybills.status）单一真相源。
 *
 * 取值必须与后端 `models/status/bpm_crm_contract.rs::logistics_waybill` 一致：
 * 状态为大写值，状态机 IN_TRANSIT → DELIVERED → SIGNED，
 * 其中 SIGNED 仅由签收接口写入（同时记录签收人/签收时间并触发应收确认）。
 */
export const WAYBILL_STATUS = {
  inTransit: 'IN_TRANSIT',
  delivered: 'DELIVERED',
  signed: 'SIGNED',
} as const;

export type WaybillStatus = (typeof WAYBILL_STATUS)[keyof typeof WAYBILL_STATUS];

/** 全部合法状态，顺序即状态机推进顺序 */
export const WAYBILL_STATUS_VALUES: WaybillStatus[] = [
  WAYBILL_STATUS.inTransit,
  WAYBILL_STATUS.delivered,
  WAYBILL_STATUS.signed,
];

/** 状态 → i18n 文案键（未知取值由调用方告警并原样显示，不静默掩盖） */
export const WAYBILL_STATUS_LABEL_KEY: Record<string, string> = {
  [WAYBILL_STATUS.inTransit]: 'logistics.common.status.inTransit',
  [WAYBILL_STATUS.delivered]: 'logistics.common.status.delivered',
  [WAYBILL_STATUS.signed]: 'logistics.common.status.signed',
};

/** el-tag 配色 */
export type WaybillTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger';

export const WAYBILL_STATUS_TAG_TYPE: Record<string, WaybillTagType> = {
  [WAYBILL_STATUS.inTransit]: 'primary',
  [WAYBILL_STATUS.delivered]: 'warning',
  [WAYBILL_STATUS.signed]: 'success',
};
