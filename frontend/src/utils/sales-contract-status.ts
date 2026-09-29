import { logger } from '@/utils/logger';

/**
 * 销售合同状态的单一映射源。
 *
 * 词表出自写入方 `backend/src/models/status/bpm_crm_contract.rs` 的 `contract` 模块
 * （sales_contract_service.rs 仅写入 draft/active/cancelled 三态，无 pending / completed）。
 * 取值必须与写入方逐字符相同，故此处只有小写 draft/active/cancelled。
 */
export const SALES_CONTRACT_STATUSES = ['draft', 'active', 'cancelled'] as const;

export type SalesContractStatus = (typeof SALES_CONTRACT_STATUSES)[number];

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐） */
export type SalesContractTagType = 'success' | 'info' | 'danger';

/** i18n 文案键（locales 的 salesContract.table.status*） */
export const SALES_CONTRACT_STATUS_LABEL_KEYS: Record<SalesContractStatus, string> = {
  draft: 'salesContract.table.statusDraft',
  active: 'salesContract.table.statusActive',
  cancelled: 'salesContract.table.statusCancelled',
};

/** Element Plus el-tag 类型 */
export const SALES_CONTRACT_STATUS_TAG_TYPES: Record<SalesContractStatus, SalesContractTagType> = {
  draft: 'info',
  active: 'success',
  cancelled: 'danger',
};

/**
 * 把后端返回的合同状态归一到已知枚举。
 * 状态缺失（合同尚未加载）返回 undefined，由调用方渲染空标签；
 * 取值在词表外（拼错的大小写、已废弃的 pending/completed）不再被静默当作合法状态：
 * 这里记错误日志并抛错，禁止调用方用 `|| status` 把裸枚举回显给用户。
 */
export function normalizeSalesContractStatus(
  status: string | undefined
): SalesContractStatus | undefined {
  if (status === undefined) return undefined;
  if ((SALES_CONTRACT_STATUSES as readonly string[]).includes(status)) {
    return status as SalesContractStatus;
  }
  const message =
    `未识别的销售合同状态「${status}」，` +
    '合法取值见后端 models/status/bpm_crm_contract.rs 的 contract 模块';
  logger.error(message, { status });
  throw new Error(message);
}

export function salesContractStatusLabelKey(status: string | undefined): string | undefined {
  const normalized = normalizeSalesContractStatus(status);
  return normalized ? SALES_CONTRACT_STATUS_LABEL_KEYS[normalized] : undefined;
}

export function salesContractStatusTagType(status: string | undefined): SalesContractTagType {
  const normalized = normalizeSalesContractStatus(status);
  return normalized ? SALES_CONTRACT_STATUS_TAG_TYPES[normalized] : 'info';
}
