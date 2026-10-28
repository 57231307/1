/**
 * 坯布页数值录入校验器。
 *
 * 功能：给坯布建档、编辑与出入库表单的数值项做格式与范围校验，取代带加减按钮的步进输入框
 * （全项目禁 el-input-number 与自造步进按钮，范围与精度靠校验器表达，不靠按钮步进）。
 * 调用方：components/GreigeFormDialog.vue 的库存与规格字段，components/GreigeStockDialog.vue
 * 的出入库数量字段。
 * 入参：Element Plus 表单校验回调形式的 value，以及文案 key 所需的字段名。
 * 传给谁：产出的错误文案经 i18n 取用户可见措辞，最终由 el-form-item 展示；提交值仍是
 *      字符串形态的十进制数，由页面在构造载荷时 Number() 归一后交给后端
 *      （后端 DTO 收 f64 再转 DECIMAL，列标度 2，见 models/greige_fabric.rs 的
 *      column_type = "Decimal(Some((10,2)))" 与 Decimal(Some((12,2)))）。
 * 存什么：不存任何状态，全部是纯函数。
 * 存哪里：无副作用、无持久化。
 */
import { i18n } from '@/i18n';

/** 本页面所有数值列的库内标度：DECIMAL(p,2)，超过两位小数会被后端静默舍位，故录入面直接拒 */
const MAX_FRACTION_DIGITS = 2;

/** 十进制数串（可带符号与小数点），与后端 f64 入参可解析的形态一致 */
const DECIMAL_PATTERN = /^-?\d+(\.\d+)?$/;

/**
 * 解析并判定用户输入。
 * @param raw 输入框原值，空串与纯空白表示未填
 * @returns 合法数值的 number；空值返回 undefined；格式非法返回 null（与 undefined 区分，
 *          调用方据此决定「未填」与「填了但不是数」两条不同文案）
 */
function parseInput(raw: unknown): number | undefined | null {
  const text = String(raw ?? '').trim();
  if (text === '') return undefined;
  if (!DECIMAL_PATTERN.test(text)) return null;
  const parsed = Number(text);
  return Number.isFinite(parsed) ? parsed : null;
}

/** 小数位数判定：超过库内标度即判非法，避免后端舍位后用户看到与录入不同的值 */
function fractionDigits(text: string): number {
  const dot = text.indexOf('.');
  return dot === -1 ? 0 : text.slice(dot + 1).length;
}

type ValidatorCallback = (error?: Error) => void;

/**
 * 「可空、非负、最多两位小数」校验器。
 * 空值合法：可空列的录入项留空即表达清空（由调用方把空值翻译成显式 null 提交），
 * 这里不判必填，必填项由调用方另挂 required 规则。
 */
export const optionalNonNegativeDecimal =
  (fieldKey: string) =>
  (_rule: unknown, value: unknown, callback: ValidatorCallback): void => {
    const text = String(value ?? '').trim();
    const parsed = parseInput(value);
    if (parsed === undefined) {
      callback();
      return;
    }
    if (parsed === null || fractionDigits(text) > MAX_FRACTION_DIGITS) {
      callback(
        new Error(
          i18n.global.t('greigeFabrics.rules.invalidDecimal', {
            field: i18n.global.t(fieldKey),
            digits: MAX_FRACTION_DIGITS,
          })
        )
      );
      return;
    }
    if (parsed < 0) {
      callback(
        new Error(
          i18n.global.t('greigeFabrics.rules.negative', {
            field: i18n.global.t(fieldKey),
          })
        )
      );
      return;
    }
    callback();
  };

/**
 * 「必填、正数、最多两位小数」校验器（出入库数量）。
 * 入库是累加、出库是扣减，0 与负数会把当前库存和累计入库量带偏，后端同样拒绝，
 * 这里前置拦一次，避免用户点了提交才拿到 400。
 */
export const requiredPositiveDecimal =
  (fieldKey: string) =>
  (_rule: unknown, value: unknown, callback: ValidatorCallback): void => {
    const text = String(value ?? '').trim();
    const parsed = parseInput(value);
    if (parsed === undefined) {
      callback(
        new Error(i18n.global.t('greigeFabrics.rules.required', { field: i18n.global.t(fieldKey) }))
      );
      return;
    }
    if (parsed === null || fractionDigits(text) > MAX_FRACTION_DIGITS) {
      callback(
        new Error(
          i18n.global.t('greigeFabrics.rules.invalidDecimal', {
            field: i18n.global.t(fieldKey),
            digits: MAX_FRACTION_DIGITS,
          })
        )
      );
      return;
    }
    if (parsed <= 0) {
      callback(
        new Error(
          i18n.global.t('greigeFabrics.rules.notPositive', {
            field: i18n.global.t(fieldKey),
          })
        )
      );
      return;
    }
    callback();
  };

/**
 * 表单值到载荷数值的归一：空串折成 null（可清空列的「清空」声明），合法数字折成 number，
 * 非法字符串折成 undefined 并由调用方的表单校验先行挡住，不在这里兜底造 0。
 */
export function toPayloadNumber(value: unknown): number | null | undefined {
  const parsed = parseInput(value);
  if (parsed === undefined) return null;
  if (parsed === null) return undefined;
  return parsed;
}
