import { logger } from '@/utils/logger';

/**
 * 销售退货状态的单一映射源。
 *
 * 取值与后端 `backend/src/models/status/sales.rs:56` 的 `sales_return` 常量逐字符对应
 * （该模块是大写词表，与销售订单的小写词表不同）：
 * 状态机 DRAFT → SUBMITTED → APPROVED → COMPLETED / REJECTED，没有 PENDING。
 *
 * 此前标签映射散落在表格、筛选下拉、详情弹窗三处且各自一套键，
 * 还带 `|| status` 兜底把裸枚举直接回显给用户。用 Record<SalesReturnStatus, …>
 * 让"新增状态必须补全映射"由编译器保证。
 */
export const SALES_RETURN_STATUSES = [
  'DRAFT',
  'SUBMITTED',
  'APPROVED',
  'REJECTED',
  'COMPLETED',
] as const;

export type SalesReturnStatus = (typeof SALES_RETURN_STATUSES)[number];

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐） */
export type SalesReturnTagType = 'success' | 'warning' | 'info' | 'primary' | 'danger';

/** i18n 文案键（locales 的 salesReturns.statusLabels.*） */
export const SALES_RETURN_STATUS_LABEL_KEYS: Record<SalesReturnStatus, string> = {
  DRAFT: 'salesReturns.statusLabels.DRAFT',
  SUBMITTED: 'salesReturns.statusLabels.SUBMITTED',
  APPROVED: 'salesReturns.statusLabels.APPROVED',
  REJECTED: 'salesReturns.statusLabels.REJECTED',
  COMPLETED: 'salesReturns.statusLabels.COMPLETED',
};

/** Element Plus el-tag 类型 */
export const SALES_RETURN_STATUS_TAG_TYPES: Record<SalesReturnStatus, SalesReturnTagType> = {
  DRAFT: 'info',
  SUBMITTED: 'warning',
  APPROVED: 'primary',
  REJECTED: 'danger',
  COMPLETED: 'success',
};

/**
 * 把后端返回的状态归一到已知枚举。
 * 未加载（undefined）返回 undefined，由调用方渲染空标签；
 * 词表外的取值记错误日志并抛错，不允许静默回显裸枚举。
 */
export function normalizeSalesReturnStatus(
  status: string | undefined
): SalesReturnStatus | undefined {
  if (status === undefined) return undefined;
  if ((SALES_RETURN_STATUSES as readonly string[]).includes(status)) {
    return status as SalesReturnStatus;
  }
  const message =
    `未识别的销售退货状态「${status}」，` +
    '合法取值见后端 models/status/sales.rs 的 sales_return 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function salesReturnStatusLabelKey(status: string | undefined): string | undefined {
  const normalized = normalizeSalesReturnStatus(status);
  return normalized ? SALES_RETURN_STATUS_LABEL_KEYS[normalized] : undefined;
}

export function salesReturnStatusTagType(status: string | undefined): SalesReturnTagType {
  const normalized = normalizeSalesReturnStatus(status);
  return normalized ? SALES_RETURN_STATUS_TAG_TYPES[normalized] : 'info';
}
