/**
 * prcFmts.ts - 采购入库格式化工具
 * 任务编号: P14 批 2 第 4 批（拆分原 purchaseReceipt/index.vue）
 *
 * 状态标签文案/配色统一以 utils/purchase-receipt-status 为唯一事实源（与后端
 * models/status/purchase_inventory.rs 的 purchase_receipt / purchase_receipt_inspection
 * 原值逐字一致），本文件只负责把 i18n 键翻译为当前语言文案，不再本地维护裸中文词表。
 */
import { i18n } from '@/i18n';
import {
  purchaseReceiptStatusLabelKey,
  purchaseReceiptStatusTagType,
  purchaseReceiptInspectionStatusLabelKey,
  purchaseReceiptInspectionStatusTagType,
  type PurchaseReceiptTagType,
} from '@/utils/purchase-receipt-status';

/** 入库状态 → 显示文案（i18n，缺失渲染中性占位；词表外非空值抛错并记日志） */
export function getReceiptStatusLabel(status: string | null | undefined): string {
  return i18n.global.t(purchaseReceiptStatusLabelKey(status));
}

/** 入库状态 → el-tag 配色 */
export function getReceiptStatusTagType(status: string | null | undefined): PurchaseReceiptTagType {
  return purchaseReceiptStatusTagType(status);
}

/** 入库单质检状态 → 显示文案（i18n） */
export function getReceiptInspectionStatusLabel(status: string | null | undefined): string {
  return i18n.global.t(purchaseReceiptInspectionStatusLabelKey(status));
}

/** 入库单质检状态 → el-tag 配色 */
export function getReceiptInspectionStatusTagType(
  status: string | null | undefined
): PurchaseReceiptTagType {
  return purchaseReceiptInspectionStatusTagType(status);
}
