<template>
  <div class="color-item-editor">
    <el-form
      label-width="100px"
      :model="form"
      :rules="rules"
      :aria-label="t('components.colorItemEditor.formAriaLabel')"
    >
      <el-form-item :label="t('components.colorItemEditor.colorCode')" required>
        <el-input
          v-model="form.color_code"
          :placeholder="t('components.colorItemEditor.colorCodePlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.colorName')" required>
        <el-input
          v-model="form.color_name"
          :placeholder="t('components.colorItemEditor.colorNamePlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.colorPicker')">
        <el-color-picker v-model="hexValue" @change="applyHex" />
        <span style="margin-left: 12px">{{ hexValue }}</span>
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.rgb')">
        <!-- RGB 三分量手写录入 整数 0 到 255（原步进控件 min max 与默认精度 0 落到 rules） -->
        <el-form-item prop="rgb_r" class="rgb-cell" :show-message="false">
          <el-input v-model="form.rgb_r" inputmode="numeric" @change="onRgbChange" />
        </el-form-item>
        <el-form-item prop="rgb_g" class="rgb-cell" :show-message="false">
          <el-input v-model="form.rgb_g" inputmode="numeric" @change="onRgbChange" />
        </el-form-item>
        <el-form-item prop="rgb_b" class="rgb-cell" :show-message="false">
          <el-input v-model="form.rgb_b" inputmode="numeric" @change="onRgbChange" />
        </el-form-item>
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.hexValue')" required>
        <el-input v-model="hexValue" placeholder="#RRGGBB" @change="applyHex" />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.pantoneCode')">
        <el-input
          v-model="form.pantone_code"
          :placeholder="t('components.colorItemEditor.optionalPlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.cncsCode')">
        <el-input
          v-model="form.cncs_code"
          :placeholder="t('components.colorItemEditor.optionalPlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.sequence')" prop="sequence">
        <!-- 排序号手写录入 非负整数（原步进控件 min=0 落到 rules） -->
        <el-input v-model="form.sequence" inputmode="numeric" />
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormRules } from 'element-plus';
import type { ColorItemInfo } from '@/api/color-card';
import { numberInputRules } from '@/utils/number-input';

const { t } = useI18n({ useScope: 'global' });

const props = defineProps<{ modelValue: Partial<ColorItemInfo> }>();
const emit = defineEmits<{ (e: 'update:modelValue', v: Partial<ColorItemInfo>): void }>();

/**
 * 编辑态模型 数值字段（rgb 三分量 排序号）以字符串承载 el-input 手写录入
 * emit 边界把通过校验的取值显式转回契约数字（ColorItemInfo.rgb_r 等为 number）
 * 未通过校验的文本不上送 保留上一次有效值 界面上由 rules 出红字点名
 * 调用方 views/color-cards/detail.vue 新建色卡项表单 存什么 RGB 颜色分量与排序号
 */
interface EditForm {
  color_code: string;
  color_name: string;
  hex_value: string;
  pantone_code: string;
  cncs_code: string;
  rgb_r: string;
  rgb_g: string;
  rgb_b: string;
  sequence: string;
}

/** 解析手写文本 空串回 undefined 非数字或十六进制伪值回 null 交由调用方跳过该键 */
function parseField(v: string): number | undefined | null {
  if (v === '') return undefined;
  // Number('0x10') 会把十六进制伪值算成 16 这里只接受纯十进制写法
  if (!/^-?\d+(\.\d+)?$/.test(v)) return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

function inRangeRgb(n: number | undefined | null): boolean {
  return n !== undefined && n !== null && Number.isInteger(n) && n >= 0 && n <= 255;
}

const form = reactive<EditForm>({
  color_code: '',
  color_name: '',
  hex_value: '#000000',
  pantone_code: '',
  cncs_code: '',
  rgb_r: '0',
  rgb_g: '0',
  rgb_b: '0',
  sequence: '0',
});

const rules = reactive<FormRules>({
  rgb_r: numberInputRules({ label: 'R', min: 0, max: 255, integer: true }),
  rgb_g: numberInputRules({ label: 'G', min: 0, max: 255, integer: true }),
  rgb_b: numberInputRules({ label: 'B', min: 0, max: 255, integer: true }),
  sequence: numberInputRules({
    label: t('components.colorItemEditor.sequence'),
    min: 0,
    integer: true,
  }),
});

const hexValue = computed({
  get: () => form.hex_value,
  set: v => {
    form.hex_value = v ?? '#000000';
  },
});

// 最近一次父级对象 保留本表单不编辑的其它键（如 id）
const incoming = ref<Partial<ColorItemInfo>>({ ...props.modelValue });

function hydrate(val: Partial<ColorItemInfo>) {
  form.color_code = val.color_code ?? '';
  form.color_name = val.color_name ?? '';
  form.hex_value = val.hex_value ?? '#000000';
  form.pantone_code = val.pantone_code ?? '';
  form.cncs_code = val.cncs_code ?? '';
  form.rgb_r = val.rgb_r == null ? '' : String(val.rgb_r);
  form.rgb_g = val.rgb_g == null ? '' : String(val.rgb_g);
  form.rgb_b = val.rgb_b == null ? '' : String(val.rgb_b);
  form.sequence = val.sequence == null ? '' : String(val.sequence);
}

hydrate(incoming.value);

// 自身 emit 触发的父级回写不再反向灌表单 避免编辑中的文本被归一化洗掉
let lastEmitJson = '';

watch(
  () => props.modelValue,
  val => {
    incoming.value = { ...val };
    const json = JSON.stringify(val);
    if (json === lastEmitJson) return;
    hydrate(val);
  },
  { deep: true }
);

/** 编辑态字符串转回契约对象并上送 无效数值键跳过 保留父级上一次有效值 */
function emitModel() {
  const merged: Partial<ColorItemInfo> = { ...incoming.value };
  merged.color_code = form.color_code;
  merged.color_name = form.color_name;
  merged.hex_value = form.hex_value;
  merged.pantone_code = form.pantone_code;
  merged.cncs_code = form.cncs_code;

  const r = parseField(form.rgb_r);
  if (r !== null && inRangeRgb(r)) merged.rgb_r = r;
  const g = parseField(form.rgb_g);
  if (g !== null && inRangeRgb(g)) merged.rgb_g = g;
  const b = parseField(form.rgb_b);
  if (b !== null && inRangeRgb(b)) merged.rgb_b = b;
  const seq = parseField(form.sequence);
  if (seq !== null && seq !== undefined && seq >= 0 && Number.isInteger(seq)) merged.sequence = seq;

  lastEmitJson = JSON.stringify(merged);
  emit('update:modelValue', merged);
}

watch(form, () => {
  emitModel();
});

/** hex 驱动 RGB 三分量回显 由色板或 hex 文本框 change 触发 */
function applyHex(hex: string | null) {
  const h = hex ?? '';
  if (!/^#[0-9A-Fa-f]{6}$/.test(h)) return;
  form.rgb_r = String(parseInt(h.slice(1, 3), 16));
  form.rgb_g = String(parseInt(h.slice(3, 5), 16));
  form.rgb_b = String(parseInt(h.slice(5, 7), 16));
}

/** RGB 三分量驱动 hex 回显 仅当三分量都有效才联动 无效时 rules 已出红字 */
function onRgbChange() {
  const r = parseField(form.rgb_r);
  const g = parseField(form.rgb_g);
  const b = parseField(form.rgb_b);
  if (!inRangeRgb(r) || !inRangeRgb(g) || !inRangeRgb(b)) return;
  form.hex_value =
    '#' +
    [r, g, b]
      .map(v => (v as number).toString(16).padStart(2, '0'))
      .join('')
      .toUpperCase();
}
</script>

<style scoped>
.color-item-editor {
  padding: 0;
}
.rgb-cell {
  display: inline-block;
  width: 90px;
  margin-bottom: 0;
}
.rgb-cell + .rgb-cell {
  margin-left: 8px;
}
</style>
