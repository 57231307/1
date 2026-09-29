/**
 * 库存等级（inventory_stock.grade）单一真相源。
 *
 * 库里存的是中文稳定值：`models/inventory_stock.rs` 的列注释与批色降级流程
 * （一等品 → 二等品 → 等外品）写的都是这三个值，等级只有这三个，`A/B/C` 属
 * 验布/匹号另一套 grade 取值域。
 *
 * 界面必须提交值本身而不是译文：此前 el-option 的 label 与 value 都取译文，
 * 英文界面下会把 "First grade" 之类的字符串发去筛选，恒零命中。
 */
export const STOCK_GRADE = {
  first: '一等品',
  second: '二等品',
  offGrade: '等外品',
} as const;

export type StockGradeValue = (typeof STOCK_GRADE)[keyof typeof STOCK_GRADE];

/** 全部等级，顺序即由高到低（降级流程按此顺序下移一档） */
export const STOCK_GRADE_VALUES: StockGradeValue[] = [
  STOCK_GRADE.first,
  STOCK_GRADE.second,
  STOCK_GRADE.offGrade,
];

/** 等级 → i18n 文案键（沿用该页既有键，避免同一含义两套文案） */
export const STOCK_GRADE_LABEL_KEY: Record<string, string> = {
  [STOCK_GRADE.first]: 'inventoryBatch.batchListTab.optionGradeFirst',
  [STOCK_GRADE.second]: 'inventoryBatch.batchListTab.optionGradeSecond',
  [STOCK_GRADE.offGrade]: 'inventoryBatch.batchListTab.optionGradeThird',
};

export type StockGradeTagType = 'success' | 'warning' | 'danger';

export const STOCK_GRADE_TAG_TYPE: Record<string, StockGradeTagType> = {
  [STOCK_GRADE.first]: 'success',
  [STOCK_GRADE.second]: 'warning',
  [STOCK_GRADE.offGrade]: 'danger',
};
