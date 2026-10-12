/**
 * 金额求和工具：后端 rust_decimal 列（未启 serde-floats）序列化为十进制字符串，
 * 直接以 `+` 累加会触发字符串拼接而非数值相加。求和前统一用 Number 归一再累加，
 * null/undefined 按 0 计（缺键不污染合计）。
 * 调用方：对账/报表等需要把出参金额字符串在前端聚合展示的视图（如 ap 对账汇总）。
 * 入参：一组后端金额出参值（十进制字符串，兼容历史 number 与可空）。
 * 传给谁：返回单个数值合计，交由调用方格式化或落库；不存储、不打印。
 */
export function sumDecimalAmounts(values: readonly (string | number | null | undefined)[]): number {
  // reduce 显式钉 <number> 累加器类型：入参数组元素是 string|number|null|undefined 联合，
  // 不钉类型时 TS 会把累加器推成元素联合，`+` 退化为字符串拼接（金额合计静默出错）
  return values.reduce<number>((total, value) => {
    if (value === null || value === undefined || value === '') return total;
    const num = Number(value);
    // 非数值脏数据（NaN/Infinity）不计入合计并显式抛出，避免静默把坏数据当 0 掩盖
    if (!Number.isFinite(num)) {
      throw new Error(`sumDecimalAmounts 收到非数值金额: ${String(value)}`);
    }
    return total + num;
  }, 0);
}

/**
 * 单值归一：后端 rust_decimal 出参（十进制字符串）转前端数值以参与四则运算/图表入参。
 * 与 sumDecimalAmounts 同纪律：null/undefined/'' 视为「无值」返回 0，非数值脏数据显式抛出。
 * 调用方：需要把后端 DecimalWire 字符串作为数值使用的视图（表格列计算、图表数据、比较）。
 * 用途窄于 sumDecimalAmounts（后者只做「合计」）；单值参与运算时用它，别在组件里各自 parseFloat。
 */
export function decimalWireToNumber(value: string | number | null | undefined): number {
  if (value === null || value === undefined || value === '') return 0;
  const num = Number(value);
  if (!Number.isFinite(num)) {
    throw new Error(`decimalWireToNumber 收到非数值金额: ${String(value)}`);
  }
  return num;
}

/**
 * 显示格式化：把后端 rust_decimal 出参（十进制字符串）按最小/最大小数位统一格式化。
 * 空值（null/undefined/''）显示为「0.00」（2 位默认，按 fractionDigits 缩放），避免因缺键崩溃。
 * 调用方：表格列 formatter、详情页金额展示；不参与落库回写。
 */
export function formatDecimalAmount(
  value: string | number | null | undefined,
  fractionDigits = 2
): string {
  return decimalWireToNumber(value).toLocaleString('zh-CN', {
    minimumFractionDigits: fractionDigits,
    maximumFractionDigits: fractionDigits,
  });
}

/**
 * 写线格式（编辑态 number → DecimalWire string）。
 * undefined = 缺值 ⇒ 返回 undefined，由载荷构造处省略键（严禁伪造 0/'' 上送）；
 * 0 是合法业务值，会被如实转成 "0" 与「缺值」可区分。
 * 后端 rust_decimal 的 visit_str 精确接收十进制与科学计数法两种字符串形态。
 * 调用方：表单提交前把 el-input-number 数值转字符串上送（见 api/quotation.ts CreateQuotationDto）。
 */
export function numberToDecimalWire(value: number | undefined): string | undefined {
  return value === undefined ? undefined : String(value);
}
