/**
 * prRtnFmts.ts - 采购退货格式化工具
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
 *
 * 状态词表/配色/文案键统一出自 utils/purchase-return-status（与后端
 * models/status/purchase_inventory.rs 的 purchase_return 原值 draft/submitted/approved/rejected
 * 逐字一致）。历史手搓 map 把提交态写成 pending、并含后端从不产生的 completed，
 * 且用硬编码中文 + `|| status` 兜底掩盖，此处一并收敛到规范模块。
 */
import { i18n } from '@/i18n';
import {
  normalizePurchaseReturnStatus,
  purchaseReturnStatusLabelKey,
  purchaseReturnStatusTagType,
  type PurchaseReturnTagType,
} from '@/utils/purchase-return-status';

/** 采购退货状态 → el-tag 配色 */
export const getStatusType = (status: string | null | undefined): PurchaseReturnTagType =>
  purchaseReturnStatusTagType(status);

/** 采购退货状态 → 显示文案（i18n，键名即后端原值；词表外值抛错并记日志） */
export const getStatusText = (status: string | null | undefined): string =>
  i18n.global.t(purchaseReturnStatusLabelKey(normalizePurchaseReturnStatus(status)));
