// 数值手输控件单源件（全项目禁步进数字输入框 对应校验规则与文本桥的唯一出处）
// 用户口径 创建单据的数量 单价 税率 金额一律纯手写输入框 范围与精度靠校验器不靠控件加减按钮
// 本件把 el-input 的输入文本与原数值模型桥接 模型类型 提交值与后端契约保持原样不改
// text 供 el-input v-model 绑定 输入过程用本地草稿保留 "1." 这类半成形态 避免小数点被吞
// normalize 供 el-input blur 调用 把显示收敛回规范数值形态
// rule 供 el-form 规则表或 el-form-item rules 使用 校验对象是数值模型本身
// watch 模型被外部改写（对话框回填 resetFields 编辑回显）时自动清草稿 防陈旧输入残留
// validateNumberValue 与 numberFieldRule 单独导出 供提交前手工守门与行内单元格复用

import { computed, effectScope, onScopeDispose, ref, toRef, watch, type Ref, type WritableComputedRef } from 'vue';
import type { FormItemRule } from 'element-plus';

export interface NumberFieldSpec {
  /** 用户可见字段名 用于拼装错误提示 可传 t() 结果 */
  label: string;
  required?: boolean;
  min?: number;
  max?: number;
  /** 小数位数 0 表示整数 省略则不限制小数位 */
  precision?: number;
  /** 必填拦截消息 沿用各页既有 i18n 文案 省略时按 label 生成 */
  requiredMessage?: string;
}

export interface NumberFieldBridge {
  /** 绑 el-input v-model 的文本形态 */
  text: WritableComputedRef<string>;
  /** 绑 el-input blur 收敛显示 */
  normalize: () => void;
  /** 进 el-form 规则表或 el-form-item rules */
  rule: FormItemRule;
}

type NumModel = number | null | undefined;

/** 输入侧字符约束 只放行数字 单个小数点 与允许负数时的前导负号 */
export function sanitizeNumberText(raw: string, spec: NumberFieldSpec): string {
  const allowNegative = spec.min === undefined || spec.min < 0;
  let rest = raw;
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
  if (spec.precision === 0) {
    rest = rest.replace(/\..*$/, '');
    if (rest === '.' || rest === '') negative = false;
  } else if (spec.precision !== undefined) {
    const parts = rest.split('.');
    if (parts.length > 1) {
      rest = parts[0] + '.' + parts.slice(1).join('').slice(0, spec.precision);
    }
  }
  if (!allowNegative) negative = false;
  return (negative ? '-' : '') + rest;
}

/** 把文本解析为数值 半成形态（空串 裸负号 裸小数点）返回 undefined */
export function parseNumberText(text: string): number | undefined {
  if (text === '' || text === '-' || text === '.' || text === '-.') return undefined;
  const n = Number(text);
  return Number.isFinite(n) ? n : undefined;
}

/** 数值形态到显示文本 */
export function displayNumberText(value: NumModel): string {
  return value == null ? '' : String(value);
}

/** 值校验单点实现 返回错误消息 合法返回 null */
export function validateNumberValue(value: unknown, spec: NumberFieldSpec): string | null {
  if (value === null || value === undefined || value === '') {
    return spec.required
      ? spec.requiredMessage ?? `请输入${spec.label}`
      : null;
  }
  const n = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(n)) {
    return `请输入正确的${spec.label}`;
  }
  if (spec.precision !== undefined) {
    if (spec.precision === 0 && !Number.isInteger(n)) {
      return `${spec.label}必须为整数`;
    }
    const decimals = String(n).split('.')[1]?.length ?? 0;
    if (decimals > spec.precision) {
      return `${spec.label}最多 ${spec.precision} 位小数`;
    }
  }
  if (spec.min !== undefined && n < spec.min) {
    return `${spec.label}不能小于 ${spec.min}`;
  }
  if (spec.max !== undefined && n > spec.max) {
    return `${spec.label}不能大于 ${spec.max}`;
  }
  return null;
}

/** el-form 规则工厂 校验数值模型 范围与精度与必填全部在此兜住 */
export function numberFieldRule(spec: NumberFieldSpec): FormItemRule {
  return {
    validator: (_rule, value, callback) => {
      const message = validateNumberValue(value, spec);
      if (message) {
        callback(new Error(message));
      } else {
        callback();
      }
    },
    trigger: ['blur', 'change'],
  };
}

/**
 * 文本桥接 把 reactive 表单字段与 el-input 之间的字符串输入收敛回数值模型
 * emptyAs 决定清空输入框时写回 null 还是 undefined 与各页既有字段类型保持同口径
 */
export function useNumberField(
  model: Ref<NumModel>,
  spec: NumberFieldSpec,
  emptyAs: 'null' | 'undefined' = 'null'
): NumberFieldBridge {
  const emptyValue: NumModel = emptyAs === 'undefined' ? undefined : null;
  const draft = ref<string | null>(null);

  const text = computed<string>({
    get: () => draft.value ?? displayNumberText(model.value),
    set: raw => {
      const cleaned = sanitizeNumberText(raw, spec);
      draft.value = cleaned;
      if (cleaned === '') {
        model.value = emptyValue;
        return;
      }
      const parsed = parseNumberText(cleaned);
      if (parsed !== undefined) {
        model.value = parsed;
      }
      // 半成形态不回写模型 由必填规则与 blur 收敛兜底
    },
  });

  // 模型被外部改写时 若草稿与外部值不同步即弃草稿 防编辑回显与对话框重置串值
  watch(model, next => {
    const cur = draft.value;
    if (cur === null) return;
    const parsed = parseNumberText(cur);
    const consistent = next == null ? cur === '' : parsed !== undefined && parsed === next;
    if (!consistent) draft.value = null;
  });

  const normalize = () => {
    draft.value = null;
  };

  return { text, normalize, rule: numberFieldRule(spec) };
}

/**
 * 表格行内单元格的桥接池 按行对象惰性建桥 同一行同一列共用一个草稿
 * 桥的 watch 挂在独立 effectScope 上 宿主组件销毁时统一回收
 * 行数组 splice 或整表重建时调用 forget 释放旧行桥（行对象身份失效后草稿无意义）
 */
export function createNumberFieldPool(
  spec: NumberFieldSpec,
  emptyAs: 'null' | 'undefined' = 'null'
) {
  const scope = effectScope();
  const pool = new Map<object, NumberFieldBridge>();
  onScopeDispose(() => scope.stop());
  const accessor = <T extends object>(row: T, key: keyof T): NumberFieldBridge => {
    const existing = pool.get(row);
    if (existing) return existing;
    const created = scope.run(() =>
      useNumberField(toRef(row, key) as Ref<NumModel>, spec, emptyAs)
    );
    if (!created) {
      throw new Error('number field pool scope stopped');
    }
    pool.set(row, created);
    return created;
  };
  accessor.forget = () => pool.clear();
  return accessor;
}
