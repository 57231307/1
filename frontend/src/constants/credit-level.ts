/**
 * 信用等级（customer_credit.credit_level）取值词表单一真相源。
 *
 * 载体与约束（逐条对过后端）：
 * - 列定义 backend/migration/src/domain/business/
 *   m0012_add_ap_ar_finance_analysis.rs：credit_level VARCHAR(20)，
 *   可空、无 CHECK、无 DEFAULT；
 * - 实体字段 backend/src/models/customer_credit.rs 的
 *   credit_level: Option<String>；
 * - 写入方 backend/src/services/customer_credit_limit.rs 的 set_credit_rating：
 *   未提供时后端自己落 B，不是前端该兜的值；
 * - 评估侧等级由分数换算（backend/src/services/customer_credit_evaluate.rs），
 *   取值恰为 AAA/AA/A/BBB/BB/B。
 *
 * 因此：
 * 1. null 是合法且可持久的库值（历史行/直接建行的场景），界面必须把它显示成
 *    可区分的"未评"文案，不得塌成 0 或 -——0 在信用域里另有"额度/分数为 0"
 *    的真实语义，把缺值画成 0 会让用户与排障者都读不出是哪种；
 * 2. 列上无 CHECK，库里可能出现词表外的任意不超过 20 字符的取值。这类值必须
 *    原样显示并标为异常（danger），不得静默归类、也不得因不在词表就替换成占位文案；
 * 3. 表单可选项与列表标签着色共用本表，避免"下拉给 8 档、着色只认 6 档"的两套词表漂移。
 */

/** 表单可选等级，顺序即由高到低（评估换算档位之外补 C/D 两档手工档） */
export const CREDIT_LEVEL_VALUES = ['AAA', 'AA', 'A', 'BBB', 'BB', 'B', 'C', 'D'] as const;

export type CreditLevelValue = (typeof CREDIT_LEVEL_VALUES)[number];

/** 受控词表内的等级 → el-tag 着色（沿用本页既有档位配色；D 档与"词表外"同色，都是危险档） */
export const CREDIT_LEVEL_TAG_TYPE: Record<CreditLevelValue, 'success' | 'warning' | 'danger'> = {
  AAA: 'success',
  AA: 'success',
  A: 'success',
  BBB: 'warning',
  BB: 'warning',
  B: 'warning',
  C: 'warning',
  D: 'danger',
};

export type CreditLevelTagType = 'success' | 'warning' | 'danger' | 'info';

/**
 * 等级单元格着色：null → info（未评，中性，不与"低等级"的 danger 混淆）；
 * 词表内 → 按档位；词表外（列无 CHECK 的任意存量值）→ danger 并原样显示取值。
 * 纯函数：不读全局、不改入参。
 */
export function creditLevelTagType(level: string | null): CreditLevelTagType {
  if (level === null) return 'info';
  return CREDIT_LEVEL_TAG_TYPE[level as CreditLevelValue] ?? 'danger';
}

/** 是否为"未评级"（后端显式回 null）；与"取不到键"的 undefined 不同，见 api/customer-credit.ts 的 CustomerCredit 声明 */
export function isUnratedCreditLevel(level: string | null): boolean {
  return level === null;
}
