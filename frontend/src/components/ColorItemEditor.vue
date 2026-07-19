<template>
  <div class="color-item-editor">
    <el-form label-width="100px" aria-label="色号编辑表单">
      <el-form-item label="色号编码" required>
        <el-input v-model="localValue.color_code" placeholder="例如: 18-1664 TPX" />
      </el-form-item>
      <el-form-item label="色号名称" required>
        <el-input v-model="localValue.color_name" placeholder="例如: 番茄红" />
      </el-form-item>
      <el-form-item label="颜色选择">
        <el-color-picker v-model="hexValue" @change="syncRgbFromHex" />
        <span style="margin-left: 12px">{{ hexValue }}</span>
      </el-form-item>
      <el-form-item label="RGB">
        <el-input-number v-model="localValue.rgb_r" :min="0" :max="255" @change="syncHexFromRgb" />
        <el-input-number v-model="localValue.rgb_g" :min="0" :max="255" @change="syncHexFromRgb" style="margin-left: 8px" />
        <el-input-number v-model="localValue.rgb_b" :min="0" :max="255" @change="syncHexFromRgb" style="margin-left: 8px" />
      </el-form-item>
      <el-form-item label="HEX 值" required>
        <el-input v-model="hexValue" @change="syncRgbFromHex" placeholder="#RRGGBB" />
      </el-form-item>
      <el-form-item label="PANTONE 编码">
        <el-input v-model="localValue.pantone_code" placeholder="可选" />
      </el-form-item>
      <el-form-item label="CNCS 编码">
        <el-input v-model="localValue.cncs_code" placeholder="可选" />
      </el-form-item>
      <el-form-item label="排序">
        <el-input-number v-model="localValue.sequence" :min="0" />
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { ColorItemInfo } from '@/api/color-card'

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
