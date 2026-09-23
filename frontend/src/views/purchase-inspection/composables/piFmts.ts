/**
 * piFmts.ts - 采购验货格式化工具
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 *
 * 状态词表/配色/文案键统一出自 utils/purchase-inspection-status（与后端
 * models/status/purchase_inventory.rs 的 purchase_inspection 原值 pending/completed 逐字一致）。
 * 检验结果（inspection_result）后端为自由文本、无状态词表，故不走 normalize，仅映射已知取值到
 * i18n 文案与配色，未映射值回显其本身（展示真实入库值，非掩盖缺键）。
 */
import { i18n } from '@/i18n';
import {
  normalizePurchaseInspectionStatus,
  purchaseInspectionStatusLabelKey,
  purchaseInspectionStatusTagType,
  type PurchaseInspectionTagType,
} from '@/utils/purchase-inspection-status';

/** 检验单状态 → el-tag 配色 */
export function getStatusType(status: string | null | undefined): PurchaseInspectionTagType {
  return purchaseInspectionStatusTagType(status);
}

/** 检验单状态 → 显示文案（i18n，键名即后端原值；词表外值抛错并记日志） */
export function getStatusText(status: string | null | undefined): string {
  return i18n.global.t(purchaseInspectionStatusLabelKey(normalizePurchaseInspectionStatus(status)));
}

/** 检验结果 → el-tag 配色 */
const RESULT_TYPE_MAP: Record<string, PurchaseInspectionTagType> = {
  pass: 'success',
  fail: 'danger',
  partial: 'warning',
};

/** 检验结果 → i18n 文案键 */
const RESULT_LABEL_KEY_MAP: Record<string, string> = {
  pass: 'purchaseInspection.filter.result.pass',
  fail: 'purchaseInspection.filter.result.fail',
  partial: 'purchaseInspection.filter.result.partial',
};

/** 检验结果 → el-tag 配色 */
export function getResultType(result: string | null | undefined): PurchaseInspectionTagType {
  return RESULT_TYPE_MAP[result ?? ''] ?? 'info';
}

/** 检验结果 → 显示文案（已知取值走 i18n，未知回显入库原值） */
export function getResultText(result: string | null | undefined): string {
  const key = RESULT_LABEL_KEY_MAP[result ?? ''];
  return key ? i18n.global.t(key) : (result ?? '');
}
