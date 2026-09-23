import { logger } from '@/utils/logger';

/**
 * 财务分析对象（financial_indicators / 报告复用同表）状态词表。
 *
 * 事实来源：`backend/src/models/status/general.rs:52` 的 `master_data` 模块——
 * 小写 active/inactive/pending/approved/draft/retired/archived/rejected；
 * 指标/报告创建时 service 写 `master_data::ACTIVE`（= 小写 `active`，见
 * `services/financial_analysis_service.rs` 的 create_indicator）。
 * 此前的 draft/executed/failed 既非写入值也非词表成员，门控恒 false。
 */
export const FINANCIAL_STATUS = {
  ACTIVE: 'active',
  INACTIVE: 'inactive',
  PENDING: 'pending',
  APPROVED: 'approved',
  DRAFT: 'draft',
  RETIRED: 'retired',
  ARCHIVED: 'archived',
  REJECTED: 'rejected',
} as const;

export type FinancialStatus = (typeof FINANCIAL_STATUS)[keyof typeof FINANCIAL_STATUS];

export const FINANCIAL_STATUSES: FinancialStatus[] = Object.values(FINANCIAL_STATUS);

export type FinancialTagType = '' | 'success' | 'warning' | 'info' | 'danger';

const TAG: Record<FinancialStatus, FinancialTagType> = {
  active: 'success',
  approved: '',
  pending: 'warning',
  draft: 'info',
  inactive: 'info',
  retired: 'info',
  archived: 'info',
  rejected: 'danger',
};

export function normalizeFinancialStatus(status: string | undefined): FinancialStatus {
  if ((FINANCIAL_STATUSES as readonly string[]).includes(status ?? '')) {
    return status as FinancialStatus;
  }
  const message =
    `未识别的财务分析状态「${String(status)}」，` +
    '合法取值见后端 models/status/general.rs 的 master_data 模块';
  logger.error(message, { status });
  throw new Error(message);
}

/** 状态 → i18n 文案键（动态键名，走 finance.statusLabels.<token>，避免源码出现裸中文） */
export function financialStatusLabelKey(status: string | undefined): string {
  return `finance.statusLabels.${normalizeFinancialStatus(status)}`;
}

export function financialStatusTagType(status: string | undefined): FinancialTagType {
  return TAG[normalizeFinancialStatus(status)];
}
