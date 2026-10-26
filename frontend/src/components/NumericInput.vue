<!--
  数值手写录入控件 项目内所有"数量/单价/税率/金额"类录入项的统一载体
  全项目禁止带加减按钮的数字输入框 本控件底层是 el-input 纯手写录入
  无步进按钮 无键盘上下键步进 无长按步进 范围与精度靠提交归一与校验器
  键入过程中本地文本原样保留（允许 12. 这类输入中间态不被打断）
  失焦或回车时经数值绑定器钳界与截位后写回 number 模型 模型类型契约不变
  清空写回 可选模型回 undefined 可空 null 模型回 null 必填模型回下界（同旧步进控件语义）

  调用方 各单据表单与表格行内的数值录入项（v-model 绑 number 或 number|undefined 字段）
  入参 modelValue 当前数值 与 min max precision integer clearValue placeholder disabled size
  传给谁 update:modelValue 写回绑定字段 change 提交后转发给调用方联动（合计重算等）
  存什么 不存储 仅录入边界归一
-->
<template>
  <el-input
    :model-value="displayText"
    :inputmode="integer ? 'numeric' : 'decimal'"
    :placeholder="placeholder"
    :disabled="disabled"
    :size="size"
    @update:model-value="onType"
    @change="commit"
    @blur="commit"
  />
</template>

<script setup lang="ts" generic="T extends number | null | undefined">
import { computed, ref } from 'vue';
import { useFormItem } from 'element-plus';
import { bindNumeric, numericView, type NumericBindOptions } from '@/utils/number-input';

interface Props {
  /** 下界 含 提交时钳制 */
  min?: number;
  /** 上界 含 提交时钳制 */
  max?: number;
  /** 小数位数 提交时按位截断 回显按位格式化 */
  precision?: number;
  /** 只允许整数 */
  integer?: boolean;
  /** 清空时写回的值 未填可选模型回 undefined 可空模型回 null 必填模型回下界 */
  clearValue?: number | null;
  /** 可选字段标记 清空写回 undefined（对应契约里可省略的数值键） */
  optional?: boolean;
  placeholder?: string;
  disabled?: boolean;
  size?: 'large' | 'default' | 'small';
}

const props = defineProps<Props>();
const emit = defineEmits<{ change: [value: T] }>();
const model = defineModel<T>();

// 键入中间态：本地文本只在输入过程中接管显示 提交后交还给模型回显
const typing = ref(false);
const localText = ref('');

const bindOptions = (): NumericBindOptions => ({
  min: props.min,
  max: props.max,
  precision: props.precision,
  integer: props.integer,
});

const displayText = computed(() =>
  typing.value
    ? localText.value
    : numericView(model.value === null ? undefined : (model.value as number | undefined), props.precision)
);

function onType(val: string | number): void {
  typing.value = true;
  localText.value = String(val ?? '');
}

const { formItem } = useFormItem();

function commit(): void {
  if (!typing.value) return;
  typing.value = false;
  const text = localText.value.trim();
  localText.value = '';
  const current = model.value;
  let next: number | null | undefined;
  if (text === '') {
    if (props.clearValue !== undefined) {
      next = props.clearValue;
    } else if (props.optional || current === null || current === undefined) {
      // 未填态保持未填（null/undefined），由 required 规则点名
      next = props.optional ? undefined : current;
    } else {
      next = bindNumeric('', current, bindOptions());
    }
  } else {
    const wasFilled = typeof current === 'number';
    const bound = bindNumeric(text, wasFilled ? current : undefined, bindOptions());
    // 可空 null 模型遇非法文本回 null（未填），可选 undefined 模型遇非法文本回 undefined
    next = bound === undefined && current === null ? null : bound;
  }
  if (next !== current) {
    model.value = next as T;
  }
  emit('change', model.value as T);
  // 自定义外层不接 el-form 的 change 广播 提交后主动触发本项校验
  formItem?.validate('change').catch(() => undefined);
}
</script>
