/**
 * scFmts.ts - 销售合同格式化工具
 * 任务编号: P14 批 2 I-3 第 1 批（拆分原 sales-contract/index.vue）
 * 提供状态标签/类型映射/货币格式化等纯函数
 *
 * 状态映射不在本文件维护：词表与配色统一出自 utils/sales-contract-status.ts
 * （它与后端 models/status/bpm_crm_contract.rs 的 contract 常量一一对应，
 * 词表外取值由 normalize 抛错，禁止 `|| status` 把裸枚举回显给用户）。
 */
import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';
import {
  salesContractStatusLabelKey,
  salesContractStatusTagType,
  type SalesContractTagType,
} from '@/utils/sales-contract-status';

/** 获取合同状态 el-tag 类型 */
export const getStatusType = (status: string): SalesContractTagType =>
  salesContractStatusTagType(status);

/**
 * 获取合同状态文案。
 * 词表外的值由 utils/sales-contract-status 抛错；这里也不再 `|| status` 回显裸枚举——
 * 取不到文案说明数据缺 status 字段，属真实缺陷，必须报错而不是伪装。
 */
export const getStatusLabel = (status: string): string => {
  const key = salesContractStatusLabelKey(status);
  if (!key) {
    const message = '销售合同行缺少 status 字段，无法映射状态文案';
    logger.error(message, { status });
    throw new Error(message);
  }
  return i18n.global.t(key);
};

/** 格式化货币（人民币） */
export const formatCurrency = (value?: number | null) => {
  return `¥${(value ?? 0).toFixed(2)}`;
};
