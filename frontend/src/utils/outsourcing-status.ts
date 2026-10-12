import { logger } from '@/utils/logger';

/**
 * 委外订单状态（outsourcing_order.status）单一映射源。
 *
 * 取值与后端 models/status/wage_energy_chemical_business.rs::outsourcing_order_status 逐字一致
 * （draft/issued/processing/received/settled/closed/cancelled，全小写下划线，7 态）。
 * 状态机为 draft→issued→processing→received→settled→closed，任意非 closed→cancelled（同文件
 * 文档注释）。其中 received 由收回单 confirm 落库（services/outsourcing_ops/receipt.rs:444），
 * 结算是 received→settled（services/outsourcing_ops/order.rs:451-458）。词表缺任一 token，
 * 该状态列就会直接显示原始英文（旧缺陷：漏 received），故列表门槛与标签文案均须取自本词表原值。
 */
export const OUTSOURCING_ORDER_STATUS = {
  draft: 'draft',
  issued: 'issued',
  processing: 'processing',
  received: 'received',
  settled: 'settled',
  closed: 'closed',
  cancelled: 'cancelled',
} as const;

export type OutsourcingOrderStatus =
  (typeof OUTSOURCING_ORDER_STATUS)[keyof typeof OUTSOURCING_ORDER_STATUS];

/** 全部合法状态值（按状态机顺序） */
export const OUTSOURCING_ORDER_STATUS_VALUES: OutsourcingOrderStatus[] =
  Object.values(OUTSOURCING_ORDER_STATUS);

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐，'' 为默认主题色） */
export type OutsourcingStatusTagType = '' | 'primary' | 'success' | 'warning' | 'info' | 'danger';

/** 状态 → i18n 文案键（locales 的 outsourcing.statusLabels.*，键名即后端原值） */
export const OUTSOURCING_STATUS_LABEL_KEYS: Record<OutsourcingOrderStatus, string> = {
  draft: 'outsourcing.statusLabels.draft',
  issued: 'outsourcing.statusLabels.issued',
  processing: 'outsourcing.statusLabels.processing',
  received: 'outsourcing.statusLabels.received',
  settled: 'outsourcing.statusLabels.settled',
  closed: 'outsourcing.statusLabels.closed',
  cancelled: 'outsourcing.statusLabels.cancelled',
};

/** 状态 → el-tag 配色 */
export const OUTSOURCING_STATUS_TAG_TYPES: Record<
  OutsourcingOrderStatus,
  OutsourcingStatusTagType
> = {
  draft: 'info',
  issued: 'primary',
  processing: 'warning',
  // received：成品已收回入库的里程碑，与 issued 同级用 primary，避免与 settled(success) 混淆
  received: 'primary',
  settled: 'success',
  closed: 'info',
  cancelled: 'danger',
};

/**
 * 归一到已知枚举。词表外的非空取值（脏数据）记警告后返回 undefined——列表渲染路径不抛错，
 * 以免单条脏数据经 ErrorBoundary 崩掉整页。
 */
export function normalizeOutsourcingStatus(
  status: string | null | undefined
): OutsourcingOrderStatus | undefined {
  if (status != null && (OUTSOURCING_ORDER_STATUS_VALUES as readonly string[]).includes(status)) {
    return status as OutsourcingOrderStatus;
  }
  if (status != null && status !== '') {
    logger.warn(
      `未识别的委外订单状态「${status}」，合法取值见后端 outsourcing_order_status（7 态小写）`
    );
  }
  return undefined;
}

export function outsourcingStatusLabelKey(status: string | null | undefined): string {
  const normalized = normalizeOutsourcingStatus(status);
  return normalized ? OUTSOURCING_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function outsourcingStatusTagType(
  status: string | null | undefined
): OutsourcingStatusTagType {
  const normalized = normalizeOutsourcingStatus(status);
  return normalized ? OUTSOURCING_STATUS_TAG_TYPES[normalized] : 'info';
}
