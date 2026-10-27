/**
 * 数字录入校验器 供把步进控件换成手写 el-input 的表单复用
 *
 * 背景 全项目禁止带加减按钮的数字输入框 步进数字控件一律换 el-input
 * 换控件不许把数字约束一起丢掉 原来控件上的 min max precision step 每一项都要在
 * rules 里有对应校验 失败消息点名是哪个字段
 * 这里只消费调用方给的边界词表 不做默认值兜底 不收任意文本
 *
 * 调用方 各单据表单里数量 单价 税率 金额等数值录入项的 el-form rules
 *        明细表格这类挂不进 el-form rules 的行内单元格 在提交前逐行走 validateNumberInput
 * 入参 一个字段的名字与边界（是否必填 范围 小数位 步长 正负）
 * 传给谁 numberInputRules 返回 Element Plus FormItemRule 数组 挂到对应 el-form-item 的 rules 上
 *        sanitizeNumberText 与 parseNumberText 服务行内录入的文本草稿 数值仍写回行模型
 *        numericView 负责模型到录入框回显文本的归一
 * 存什么 不存储 只做前端提交前校验与录入边界的归一
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
  /** 小数位数 等价于步进控件的 precision 未给则不做位数限制 */
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
 * 值校验单点实现 返回失败消息 合法返回 null
 * 必填 数字形态 正负 范围 小数位 步长全部在此判 rules 与表格行内录入共用这一份判定
 * 调用方 el-form 规则工厂 以及明细表格这类挂不进 el-form rules 的行内单元格（提交前逐行校验）
 * 入参 待判值与边界词表
 * 传给谁 返回值是面向用户的失败文案 由 rules 包成 Error 或由调用方外显
 */
export function validateNumberInput(value: unknown, opts: NumberInputRuleOptions): string | null {
  const m = opts.messages || {};
  if (value === undefined || value === null || value === '') {
    return opts.required ? m.empty || `${opts.label}不能为空` : null;
  }
  const num = Number(value);
  if (!Number.isFinite(num)) {
    return m.notNumber || `${opts.label}必须是数字`;
  }
  if (opts.positive && num <= 0) {
    return m.mustPositive || `${opts.label}必须大于 0`;
  }
  if (opts.negative && num >= 0) {
    return m.mustNegative || `${opts.label}必须小于 0`;
  }
  if (opts.min !== undefined && num < opts.min) {
    return m.tooSmall || `${opts.label}不能小于 ${opts.min}`;
  }
  if (opts.max !== undefined && num > opts.max) {
    return m.tooLarge || `${opts.label}不能大于 ${opts.max}`;
  }
  // 按字符串原始形态数小数位 避开浮点表示误差
  const fractionDigits = (String(value).split('.')[1] || '').length;
  const maxFraction = opts.integer ? 0 : opts.precision;
  if (maxFraction !== undefined && fractionDigits > maxFraction) {
    return m.tooPrecise || `${opts.label}最多 ${maxFraction} 位小数`;
  }
  if (opts.step !== undefined && opts.step > 0) {
    const base = opts.min ?? 0;
    const ratio = (num - base) / opts.step;
    if (Math.abs(ratio - Math.round(ratio)) > 1e-9) {
      return m.badStep || `${opts.label}必须是步长 ${opts.step} 的整数倍`;
    }
  }
  return null;
}

/**
 * 生成 el-input 数值字段的校验规则数组
 * el-input 的 v-model 是字符串 这里全程按字符串判空 按数值判界
 * 空值交给 required 规则拦截 非 required 时空值直接放过 交由上层决定是否必填
 */
export function numberInputRules(opts: NumberInputRuleOptions): FormItemRule[] {
  const rules: FormItemRule[] = [];
  if (opts.required) {
    rules.push({
      required: true,
      message: opts.messages?.empty || `${opts.label}不能为空`,
      trigger: 'blur',
    });
  }
  // 必填拦截已由上一条规则负责 这里按非必填判定同一份边界 避免空值被报两次
  const bounds: NumberInputRuleOptions = { ...opts, required: false };
  rules.push({
    validator: (_rule, value, callback) => {
      const message = validateNumberInput(value, bounds);
      if (message) {
        callback(new Error(message));
      } else {
        callback();
      }
    },
    trigger: 'blur',
  });
  return rules;
}

/**
 * 录入侧字符约束 只放行数字 单个小数点 与允许负数时的前导负号
 * 保留 "3." 与 "-" 这类输入中间态 免得逐键回写时把用户正在敲的小数点吞掉
 * 给定 precision 时按位截断 integer 时连小数点一起裁掉
 * 调用方 表格行内数值单元格（行模型保持数值 DOM 文本走草稿）的输入边界
 * 传给谁 返回值写回草稿文本 解析出的数值由调用方写回行模型
 */
export function sanitizeNumberText(raw: string, opts: NumberInputRuleOptions): string {
  const allowNegative = opts.integer !== true && (opts.min === undefined || opts.min < 0);
  let rest = raw ?? '';
  let negative = false;
  if (allowNegative && rest.startsWith('-')) {
    negative = true;
    rest = rest.slice(1);
  }
  rest = rest.replace(/[^\d.]/g, '');
  const dot = rest.indexOf('.');
  if (dot >= 0) {
    rest = rest.slice(0, dot + 1) + rest.slice(dot + 1).replace(/\./g, '');
  }
  const maxFraction = opts.integer ? 0 : opts.precision;
  if (maxFraction !== undefined) {
    const parts = rest.split('.');
    rest =
      parts.length > 1 ? parts[0] + '.' + parts.slice(1).join('').slice(0, maxFraction) : parts[0];
  }
  if (!allowNegative) negative = false;
  return (negative ? '-' : '') + rest;
}

/**
 * 把录入文本解析为数值 空串与 "3." 与裸负号这类半成形态返回 undefined
 * 半成形态不写回模型 由调用方保持模型原值 提交前的必填与边界判定负责外显
 */
export function parseNumberText(text: string): number | undefined {
  if (text === '' || text === '-' || text === '.' || text === '-.') return undefined;
  const n = Number(text);
  return Number.isFinite(n) ? n : undefined;
}

/** 数字模型到录入框回显文本的归一 空值出空串 给定精度时按小数位显示 供 :model-value 复用 */
export function numericView(v: number | null | undefined, precision?: number): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return '';
  return precision !== undefined ? v.toFixed(precision) : String(v);
}
