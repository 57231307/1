/**
 * 字符串金额参与求和的行为锁。
 * 功能：验证生产工具 sumDecimalAmounts 把后端 rust_decimal 十进制字符串金额正确数值求和，
 * 并钉死"直接 `+` 拼接"这一历史崩溃点不得回归；对脏数据 fail-closed 抛错而非静默当 0。
 * 调用方：vitest（tests/unit，CI vitest job）。被测对象为生产函数（BudgetItemTab 期间合计等
 * 真实消费），非测试内影子实现。
 * 入参：字符串/数字/可空金额数组；传给谁：返回数值合计并断言；存什么/存哪里：不落盘。
 */
import { describe, expect, it } from 'vitest';

import { sumDecimalAmounts } from '@/utils/money';

describe('sumDecimalAmounts：后端十进制字符串金额求和', () => {
  it('两位小数字符串相加得到精确数值（非字符串拼接）', () => {
    expect(sumDecimalAmounts(['10.50', '20.25'])).toBe(30.75);
    expect(sumDecimalAmounts(['0.10', '0.20'])).toBeCloseTo(0.3, 10);
  });

  it('历史崩溃点回归锁：同样数据若用裸 `+` 会得到拼接字符串而非数值', () => {
    // 演示为何必须先 Number 归一：两个十进制字符串用 `+` 触发的是字符串拼接
    const first = '10.50';
    const second = '20.25';
    const naiveConcat = first + second;
    expect(naiveConcat).toBe('10.5020.25');
    expect(sumDecimalAmounts([first, second])).not.toBe(naiveConcat);
  });

  it('空串 / null / undefined 按不参与合计处理（缺键不污染求和）', () => {
    expect(sumDecimalAmounts(['100.00', null, undefined, '', '5.00'])).toBe(105);
  });

  it('整数与小数混合、含负数（红冲/调整）均正确', () => {
    expect(sumDecimalAmounts(['1000', '20.50', '-30.25'])).toBe(990.25);
  });

  it('空数组合计为 0', () => {
    expect(sumDecimalAmounts([])).toBe(0);
  });

  it('非数值脏数据 fail-closed 抛错，不静默当 0 掩盖', () => {
    expect(() => sumDecimalAmounts(['abc'])).toThrow(/非数值金额/);
    expect(() => sumDecimalAmounts([Number.NaN as unknown as string])).toThrow(/非数值金额/);
  });
});
