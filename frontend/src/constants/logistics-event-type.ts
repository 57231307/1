/**
 * 物流轨迹事件类型（logistics_tracking_events.event_type）取值清单。
 *
 * 后端在写入入口按同一词表校验（`models/status/bpm_crm_contract.rs` 的
 * `logistics_event_type::ALL`），越界取值直接 400 并报出允许值——该列此前是自由文本，
 * 界面只能显示稳定码原文，无法判断哪条轨迹是合法登记的。
 *
 * 与运单主状态（`constants/waybill-status.ts`，后端 logistics_waybill 用
 * IN_TRANSIT/DELIVERED/SIGNED 大写码）是两套词表：事件是轨迹上的一个点，
 * 状态是运单当前所处阶段，二者不可互相赋值。
 */
export const LOGISTICS_EVENT_TYPE = {
  pickup: 'pickup',
  in_transit: 'in_transit',
  arrived: 'arrived',
  delivered: 'delivered',
} as const;

export type LogisticsEventTypeValue =
  (typeof LOGISTICS_EVENT_TYPE)[keyof typeof LOGISTICS_EVENT_TYPE];

export const LOGISTICS_EVENT_TYPE_VALUES: LogisticsEventTypeValue[] = [
  LOGISTICS_EVENT_TYPE.pickup,
  LOGISTICS_EVENT_TYPE.in_transit,
  LOGISTICS_EVENT_TYPE.arrived,
  LOGISTICS_EVENT_TYPE.delivered,
];

/** 事件类型 → i18n 文案键 */
export const LOGISTICS_EVENT_TYPE_LABEL_KEY: Record<LogisticsEventTypeValue, string> = {
  [LOGISTICS_EVENT_TYPE.pickup]: 'logistics.detail.events.typePickup',
  [LOGISTICS_EVENT_TYPE.in_transit]: 'logistics.detail.events.typeInTransit',
  [LOGISTICS_EVENT_TYPE.arrived]: 'logistics.detail.events.typeArrived',
  [LOGISTICS_EVENT_TYPE.delivered]: 'logistics.detail.events.typeDelivered',
};

/** 判断已存库的事件类型是否在本词表内（越界值界面按原样展示，不猜测含义） */
export const isLogisticsEventType = (value: string): value is LogisticsEventTypeValue =>
  LOGISTICS_EVENT_TYPE_VALUES.includes(value as LogisticsEventTypeValue);
