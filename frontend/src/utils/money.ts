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
