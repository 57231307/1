/**
 * pcFmts.ts - 采购合同格式化工具
 * 任务编号: P14 批 2 I-3 第 3 批（拆分原 purchase-contract/index.vue）
 *
 * 状态词表以写入方为准：建单写 contract::DRAFT，审批 draft→ACTIVE（purchase_contract_service.rs:263/271），
 * 取消 draft|active→CANCELLED（:312/320）。models/status/bpm_crm_contract.rs 的 contract 仅
 * draft/active/cancelled 三态——历史前端映射里的 pending/completed 后端永不产生，门控按钮恒不可达。
 * 本模块以三态原值为比较对象，未知 token 抛错并记日志，文案走 i18n 键（purchaseContract.status.*）。
 */
import { logger } from '@/utils/logger';
import { i18n } from '@/i18n';

export const PURCHASE_CONTRACT_STATUS = {
  DRAFT: 'draft',
  ACTIVE: 'active',
  CANCELLED: 'cancelled',
} as const;

export type PurchaseContractStatus =
  (typeof PURCHASE_CONTRACT_STATUS)[keyof typeof PURCHASE_CONTRACT_STATUS];

export const PURCHASE_CONTRACT_STATUSES: PurchaseContractStatus[] =
  Object.values(PURCHASE_CONTRACT_STATUS);

export type PurchaseContractTagType = '' | 'success' | 'warning' | 'info' | 'danger';

export const PURCHASE_CONTRACT_STATUS_LABEL_KEYS: Record<PurchaseContractStatus, string> = {
  draft: 'purchaseContract.status.draft',
  active: 'purchaseContract.status.active',
  cancelled: 'purchaseContract.status.cancelled',
};

export const PURCHASE_CONTRACT_STATUS_TAG_TYPES: Record<
  PurchaseContractStatus,
  PurchaseContractTagType
> = {
  draft: 'info',
  active: 'success',
  cancelled: 'danger',
};

export function normalizePurchaseContractStatus(
  status: string | undefined | null
): PurchaseContractStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((PURCHASE_CONTRACT_STATUSES as readonly string[]).includes(status)) {
    return status as PurchaseContractStatus;
  }
  const message =
    `未识别的采购合同状态「${status}」，` +
    '合法取值见后端 models/status/bpm_crm_contract.rs 的 contract 模块';
  logger.error(message, { status });
  throw new Error(message);
}

/** 获取采购合同状态 el-tag 类型 */
export const getStatusType = (status: string | undefined | null): PurchaseContractTagType => {
  const normalized = normalizePurchaseContractStatus(status);
  return normalized ? PURCHASE_CONTRACT_STATUS_TAG_TYPES[normalized] : 'info';
};

/** 获取采购合同状态显示文案（i18n） */
export const getStatusLabel = (status: string | undefined | null): string => {
  const normalized = normalizePurchaseContractStatus(status);
  const key = normalized ? PURCHASE_CONTRACT_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
  return i18n.global.t(key);
};

/** 格式化货币（人民币 2 位精度） */
export const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(2)}` : '¥0.00';
};
