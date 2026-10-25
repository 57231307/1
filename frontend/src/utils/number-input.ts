/**
 * 数字录入校验器 供把 el-input-number 换成 el-input 的表单复用
 *
 * 背景 全项目禁止带加减按钮的数字输入框 el-input-number 一律换 el-input
 * 换控件不许把数字约束一起丢掉 原来控件上的 min max precision step 每一项都要在
 * rules 里有对应校验 失败消息点名是哪个字段
 * 这里只消费调用方给的边界词表 不做默认值兜底 不收任意文本
 *
 * 调用方 各单据表单里数量 单价 税率 金额等数值录入项的 el-form rules
 * 入参 一个字段的名字与边界（是否必填 范围 小数位 步长 正负）
 * 传给谁 返回 Element Plus FormItemRule 数组 挂到对应 el-form-item 的 rules 上
 * 存什么 不存储 只做前端提交前校验
 */
import type { FormItemRule } from 'element-plus';

export interface NumberInputRuleOptions {
  /** 字段中文名 用在失败消息里点名是哪个字段 */
  label: string;
  /** 必填 空值判负 */
  required?: boolean;
  /** 下界 含 值不得小于它 */
  min?: number;
  /** 上界 含 值不得大于它 */
  max?: number;
  /** 小数位数 等价于 el-input-number 的 precision 未给则不做位数限制 */
  precision?: number;
  /** 只允许整数 等价 precision 为 0 */
  integer?: boolean;
  /** 步长 取值须落在 min（无则 0）加步长整数倍的格点上 */
  step?: number;
  /** 必须严格大于 0 */
  positive?: boolean;
  /** 必须严格小于 0 */
  negative?: boolean;
  /**
   * 各条失败消息 调用方可传已本地化的文案 复用现有 i18n 键 避免把在用键变成孤儿
   * 未给的项回落到点名字段的中文默认文案
   */
  messages?: {
    empty?: string;
    notNumber?: string;
    mustPositive?: string;
    mustNegative?: string;
    tooSmall?: string;
    tooLarge?: string;
    tooPrecise?: string;
    badStep?: string;
  };
}

/**
 * 生成 el-input 数值字段的校验规则数组
 * el-input 的 v-model 是字符串 这里全程按字符串判空 按数值判界
 * 空值交给 required 规则拦截 非 required 时空值直接放过 交由上层决定是否必填
 */
export function numberInputRules(opts: NumberInputRuleOptions): FormItemRule[] {
  const m = opts.messages || {};
  const rules: FormItemRule[] = [];
  if (opts.required) {
    rules.push({
      required: true,
      message: m.empty || `${opts.label}不能为空`,
      trigger: 'blur',
    });
  }
  rules.push({
    validator: (_rule, value, callback) => {
      if (value === undefined || value === null || value === '') {
        // 空值由 required 规则处理 这里不再重复报
        callback();
        return;
      }
      const num = Number(value);
      if (!Number.isFinite(num)) {
        callback(new Error(m.notNumber || `${opts.label}必须是数字`));
        return;
      }
      if (opts.positive && num <= 0) {
        callback(new Error(m.mustPositive || `${opts.label}必须大于 0`));
        return;
      }
      if (opts.negative && num >= 0) {
        callback(new Error(m.mustNegative || `${opts.label}必须小于 0`));
        return;
      }
      if (opts.min !== undefined && num < opts.min) {
        callback(new Error(m.tooSmall || `${opts.label}不能小于 ${opts.min}`));
        return;
      }
      if (opts.max !== undefined && num > opts.max) {
        callback(new Error(m.tooLarge || `${opts.label}不能大于 ${opts.max}`));
        return;
      }
      // 按字符串原始形态数小数位 避开浮点表示误差
      const fractionDigits = (String(value).split('.')[1] || '').length;
      const maxFraction = opts.integer ? 0 : opts.precision;
      if (maxFraction !== undefined && fractionDigits > maxFraction) {
        callback(new Error(m.tooPrecise || `${opts.label}最多 ${maxFraction} 位小数`));
        return;
      }
      if (opts.step !== undefined && opts.step > 0) {
        const base = opts.min ?? 0;
        const ratio = (num - base) / opts.step;
        if (Math.abs(ratio - Math.round(ratio)) > 1e-9) {
          callback(new Error(m.badStep || `${opts.label}必须是步长 ${opts.step} 的整数倍`));
          return;
        }
      }
      callback();
    },
    trigger: 'blur',
  });
  return rules;
}
