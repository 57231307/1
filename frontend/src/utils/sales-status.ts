import { logger } from '@/utils/logger';

/**
 * 销售订单状态的单一映射源。
 *
 * 状态取值与后端 models/status/sales.rs 的 so_status 常量一一对应；
 * 此前文案映射散落在 OrderViewDialog 的 textMap/typeMap、SalesOrderFilter 的
 * el-option 列表与 i18n sales.statusLabels 四处，且都只覆盖 5 个状态，
 * 导致 draft / partial_shipped / rejected 三个真实状态在界面上直接露出英文枚举、
 * 且无法在筛选下拉中选择；订单详情页更是完全没做映射。
 *
 * 用 Record<SalesOrderStatus, …> 让"新增状态必须补全映射"由编译器保证。
 */

export const SALES_ORDER_STATUSES = [
  'draft',
  'pending',
  'approved',
  'partial_shipped',
  'shipped',
  'completed',
  'cancelled',
  'rejected',
] as const;

export type SalesOrderStatus = (typeof SALES_ORDER_STATUSES)[number];

/** i18n 文案键（locales 的 sales.statusLabels.*） */
export const SALES_STATUS_LABEL_KEYS: Record<SalesOrderStatus, string> = {
  draft: 'sales.statusLabels.draft',
  pending: 'sales.statusLabels.pending',
  approved: 'sales.statusLabels.approved',
  partial_shipped: 'sales.statusLabels.partial_shipped',
  shipped: 'sales.statusLabels.shipped',
  completed: 'sales.statusLabels.completed',
  cancelled: 'sales.statusLabels.cancelled',
  rejected: 'sales.statusLabels.rejected',
};

/** Element Plus el-tag 类型 */
export const SALES_STATUS_TAG_TYPES: Record<SalesOrderStatus, string> = {
  draft: 'info',
  pending: 'warning',
  approved: 'primary',
  partial_shipped: 'warning',
  shipped: 'success',
  completed: 'info',
  cancelled: 'danger',
  rejected: 'danger',
};

/**
 * 把后端返回的状态归一到已知枚举。
 * 未知状态不再被静默当作合法状态（原 `typeMap[status] || 'info'` 会把拼错的状态
 * 伪装成正常展示），这里显式告警并返回 undefined，由调用方回退为"原样显示状态值"，
 * 让问题在界面与日志中同时可见。
 */
export function normalizeSalesOrderStatus(
  status: string | undefined
): SalesOrderStatus | undefined {
  if (!status) return undefined;
  if ((SALES_ORDER_STATUSES as readonly string[]).includes(status)) {
    return status as SalesOrderStatus;
  }
  logger.warn('未识别的销售订单状态', { status });
  return undefined;
}

export function salesStatusLabelKey(status: string | undefined): string | undefined {
  const normalized = normalizeSalesOrderStatus(status);
  return normalized ? SALES_STATUS_LABEL_KEYS[normalized] : undefined;
}

export function salesStatusTagType(status: string | undefined): string {
  const normalized = normalizeSalesOrderStatus(status);
  return normalized ? SALES_STATUS_TAG_TYPES[normalized] : 'info';
}
