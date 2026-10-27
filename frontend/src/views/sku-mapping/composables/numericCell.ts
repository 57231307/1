/**
 * SKU 对照录入的数值单元格判据（单条新建/编辑与批量添加两臂共用）
 *
 * 界面上的数值列一律用 el-input 承载文本（本仓禁用带加减按钮的数字输入框），
 * 范围与精度就全靠这里判定：
 * - 交期/优先级落库列是 integer，判「非负整数且不超 i32 上界」；
 * - 协议价/最小起订量落库列是 numeric(12,2)，判「非负小数，小数位不超过 2 位」；
 * - 空串表示「本列不填」（出参为 null），与 0 严格区分，不做默认值兜底。
 *
 * 后端对应判据：单条臂 `AppError::validation_displayable`（FieldValidator::decimal），
 * 批量臂按行归因进 errors；这里提前拦是让用户当场看到第几行哪一列要改，不是替代后端校验。
 */

/** i32 上界：交期与优先级两列都是 integer，超界会被数据库拒掉。 */
const I32_MAX = 2147483647;

const UNSIGNED_INT_PATTERN = /^\d+$/;
/** 非负、最多两位小数（与列定义 numeric(12,2) 同精度口径） */
const MONEY_PATTERN = /^\d+(?:\.\d{1,2})?$/;

/** 非负整数（用于交期、优先级）。空串不算合格，调用方按「必填列未填」另行提示。 */
export function isUnsignedIntText(value: string): boolean {
  const text = value.trim();
  if (text === '') return false;
  return UNSIGNED_INT_PATTERN.test(text) && Number(text) <= I32_MAX;
}

/** 非负且最多两位小数（用于协议价、最小起订量）。空串不算合格。 */
export function isMoneyText(value: string): boolean {
  const text = value.trim();
  if (text === '') return false;
  return MONEY_PATTERN.test(text);
}

/** 该数值单元格是否填了内容（用于区分「未填→null」与「填了但不合格」）。 */
export function hasCellText(value: string): boolean {
  return value.trim() !== '';
}
