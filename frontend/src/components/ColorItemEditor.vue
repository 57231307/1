<template>
  <div class="color-item-editor">
    <el-form label-width="100px" :aria-label="t('components.colorItemEditor.formAriaLabel')">
      <el-form-item :label="t('components.colorItemEditor.colorCode')" required>
        <el-input v-model="localValue.color_code" :placeholder="t('components.colorItemEditor.colorCodePlaceholder')" />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.colorName')" required>
        <el-input v-model="localValue.color_name" :placeholder="t('components.colorItemEditor.colorNamePlaceholder')" />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.colorPicker')">
        <el-color-picker v-model="hexValue" @change="syncRgbFromHex" />
        <span style="margin-left: 12px">{{ hexValue }}</span>
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.rgb')">
        <el-input-number v-model="localValue.rgb_r" :min="0" :max="255" @change="syncHexFromRgb" />
        <el-input-number v-model="localValue.rgb_g" :min="0" :max="255" @change="syncHexFromRgb" style="margin-left: 8px" />
        <el-input-number v-model="localValue.rgb_b" :min="0" :max="255" @change="syncHexFromRgb" style="margin-left: 8px" />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.hexValue')" required>
        <el-input v-model="hexValue" @change="syncRgbFromHex" placeholder="#RRGGBB" />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.pantoneCode')">
        <el-input v-model="localValue.pantone_code" :placeholder="t('components.colorItemEditor.optionalPlaceholder')" />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.cncsCode')">
        <el-input v-model="localValue.cncs_code" :placeholder="t('components.colorItemEditor.optionalPlaceholder')" />
      </el-form-item>
      <el-form-item :label="t('components.colorItemEditor.sequence')">
        <el-input-number v-model="localValue.sequence" :min="0" />
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ColorItemInfo } from '@/api/color-card'

const { t } = useI18n({ useScope: 'global' })

const props = defineProps<{ modelValue: Partial<ColorItemInfo> }>()
const emit = defineEmits<{ (e: 'update:modelValue', v: Partial<ColorItemInfo>): void }>()

// 使用本地 ref 避免直接修改 modelValue prop（修复 ESLint no-mutating-props 错误）
const localValue = ref<Partial<ColorItemInfo>>({ ...props.modelValue })

// 父组件外部更新时同步本地副本
watch(
  () => props.modelValue,
  (val) => {
    localValue.value = { ...val }
  },
  { deep: true }
)

// 本地修改时向上抛出
watch(
  localValue,
  (val) => {
    emit('update:modelValue', { ...val })
  },
  { deep: true }
)

const hexValue = computed({
  get: () => localValue.value.hex_value || '#000000',
  set: (v) => {
    localValue.value = { ...localValue.value, hex_value: v }
  },
})

function syncRgbFromHex(hex: string) {
  if (!/^#[0-9A-Fa-f]{6}$/.test(hex)) return
  localValue.value = {
    ...localValue.value,
    hex_value: hex,
    rgb_r: parseInt(hex.slice(1, 3), 16),
    rgb_g: parseInt(hex.slice(3, 5), 16),
    rgb_b: parseInt(hex.slice(5, 7), 16),
  }
}

function syncHexFromRgb() {
  const r = (localValue.value.rgb_r ?? 0).toString(16).padStart(2, '0')
  const g = (localValue.value.rgb_g ?? 0).toString(16).padStart(2, '0')
  const b = (localValue.value.rgb_b ?? 0).toString(16).padStart(2, '0')
  localValue.value = {
    ...localValue.value,
    hex_value: `#${r}${g}${b}`.toUpperCase(),
  }
}
</script>

<style scoped>
.color-item-editor { padding: 0; }
</style>
