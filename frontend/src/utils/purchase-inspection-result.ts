import { logger } from '@/utils/logger';

/**
 * 采购质检结论（purchase_inspection.inspection_result）词表的单一映射源。
 *
 * 取值与后端 models/status/purchase_inventory.rs 的 purchase_inspection_result 常量
 * 逐字一致（英文小写码 pass/fail/partial）。词表的原始出处是写入方采集入口——
 * 「完成」三连 prompt 的结论录入 pattern（views/purchase-inspection/composables/
 * usePiProc.ts），本模块是该 pattern 与后端白名单校验共用的同一份常量（三端同源）：
 * 后端完成校验取不到这里任何一个 token 的中文/大小写变体，任何比较点禁止手写第二套。
 *
 * 与通用质检记录域 quality_inspection_records.inspection_result 的中文词表
 * （待检/合格/不合格，constants/quality-inspection-record.ts）分属两张表两套词表，
 * 禁止跨域借用：拿中文表比较本列会把合法生产数据判成非法，反之亦然。
 * 未质检时本列为 null（建单置 null，完成才写结论），null 是合法初值、空串不是取值。
 */
export const PURCHASE_INSPECTION_RESULT = {
  PASS: 'pass',
  FAIL: 'fail',
  PARTIAL: 'partial',
} as const;

export type PurchaseInspectionResult =
  (typeof PURCHASE_INSPECTION_RESULT)[keyof typeof PURCHASE_INSPECTION_RESULT];

/** 全部合法结论取值：筛选下拉、门控集合与 pattern 构造的取值来源 */
export const PURCHASE_INSPECTION_RESULTS: PurchaseInspectionResult[] = Object.values(
  PURCHASE_INSPECTION_RESULT
);

/**
 * ElMessageBox.prompt 的 inputPattern：由上方权威常量构造，禁止再手写正则字面量。
 * token 均为小写字母、无正则元字符，join('|') 构造 ^(…)$ 锚定式是安全的。
 */
export const PURCHASE_INSPECTION_RESULT_INPUT_PATTERN = new RegExp(
  `^(${PURCHASE_INSPECTION_RESULTS.join('|')})$`
);

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐，'' 为默认主题色） */
export type PurchaseInspectionResultTagType = '' | 'success' | 'warning' | 'info' | 'danger';

/** 结论 → i18n 文案键（复用既有筛选下拉文案键，键名即后端原值） */
export const PURCHASE_INSPECTION_RESULT_LABEL_KEYS: Record<PurchaseInspectionResult, string> = {
  pass: 'purchaseInspection.filter.result.pass',
  fail: 'purchaseInspection.filter.result.fail',
  partial: 'purchaseInspection.filter.result.partial',
};

/** 结论 → el-tag 配色 */
export const PURCHASE_INSPECTION_RESULT_TAG_TYPES: Record<
  PurchaseInspectionResult,
  PurchaseInspectionResultTagType
> = {
  pass: 'success',
  fail: 'danger',
  partial: 'warning',
};

/**
 * 判定结论是否在词表内（采集守门与展示映射前置）。
 * 大小写/中英变体（如 PASSED、合格）都是词表外——本列的英文码表禁止中文化。
 */
export function isPurchaseInspectionResult(value: string | null | undefined): boolean {
  return value != null && (PURCHASE_INSPECTION_RESULTS as readonly string[]).includes(value);
}

/**
 * 结论 → i18n 文案键；词表外返回 undefined。
 * 展示层对词表外历史值（本列强校验上线前的自由文本存量）必须回显入库原值本身
 * （诚实展示，不是掩盖缺键），因此这里不做归一抛错，由调用方决定占位形态。
 */
export function purchaseInspectionResultLabelKey(
  result: string | null | undefined
): string | undefined {
  if (result == null) return undefined;
  if (!isPurchaseInspectionResult(result)) {
    logger.warn('采购质检结论出现词表外的存量取值（强校验前的自由文本历史值）', {
      inspection_result: result,
    });
    return undefined;
  }
  return PURCHASE_INSPECTION_RESULT_LABEL_KEYS[result as PurchaseInspectionResult];
}

/** 结论 → el-tag 配色；词表外回中性 'info'（存量脏值不得伪装成合法配色语义） */
export function purchaseInspectionResultTagType(
  result: string | null | undefined
): PurchaseInspectionResultTagType {
  if (result == null || !isPurchaseInspectionResult(result)) return 'info';
  return PURCHASE_INSPECTION_RESULT_TAG_TYPES[result as PurchaseInspectionResult];
}
