/**
 * piFmts.ts - 采购验货格式化工具
 * 任务编号: P14 批 2 I-3 第 5 批（拆分原 purchase-inspection/index.vue）
 *
 * 状态词表/配色/文案键统一出自 utils/purchase-inspection-status（与后端
 * models/status/purchase_inventory.rs 的 purchase_inspection 原值 pending/completed 逐字一致）。
 * 检验结果（inspection_result）词表统一出自 utils/purchase-inspection-result（与后端
 * purchase_inspection_result 原值 pass/fail/partial 逐字一致，本列完成时已被白名单强校验）；
 * 展示层对强校验上线前的自由文本存量取值回显其本身（展示真实入库值，非掩盖缺键）。
 */
import { i18n } from '@/i18n';
import {
  purchaseInspectionStatusLabelKey,
  purchaseInspectionStatusTagType,
  type PurchaseInspectionTagType,
} from '@/utils/purchase-inspection-status';
import {
  purchaseInspectionResultLabelKey,
  purchaseInspectionResultTagType,
} from '@/utils/purchase-inspection-result';

/** 检验单状态 → el-tag 配色 */
export function getStatusType(status: string | null | undefined): PurchaseInspectionTagType {
  return purchaseInspectionStatusTagType(status);
}

/** 检验单状态 → 显示文案（i18n，键名即后端原值；缺失渲染中性占位，非空非法值抛错并记日志） */
export function getStatusText(status: string | null | undefined): string {
  return i18n.global.t(purchaseInspectionStatusLabelKey(status));
}

/** 检验结果 → el-tag 配色（词表外存量值回中性 info） */
export function getResultType(result: string | null | undefined): PurchaseInspectionTagType {
  return purchaseInspectionResultTagType(result);
}

/** 检验结果 → 显示文案（词表内取值走 i18n，词表外存量回显入库原值） */
export function getResultText(result: string | null | undefined): string {
  const key = purchaseInspectionResultLabelKey(result);
  return key ? i18n.global.t(key) : (result ?? '');
}
