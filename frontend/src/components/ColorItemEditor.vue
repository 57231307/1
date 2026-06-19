<template>
  <div class="color-item-editor">
    <el-form label-width="100px">
      <el-form-item label="色号编码" required>
        <el-input v-model="modelValue.color_code" placeholder="例如: 18-1664 TPX" />
      </el-form-item>
      <el-form-item label="色号名称" required>
        <el-input v-model="modelValue.color_name" placeholder="例如: 番茄红" />
      </el-form-item>
      <el-form-item label="颜色选择">
        <el-color-picker v-model="hexValue" @change="syncRgbFromHex" />
        <span style="margin-left: 12px">{{ hexValue }}</span>
      </el-form-item>
      <el-form-item label="RGB">
        <el-input-number v-model="modelValue.rgb_r" :min="0" :max="255" @change="syncHexFromRgb" />
        <el-input-number v-model="modelValue.rgb_g" :min="0" :max="255" @change="syncHexFromRgb" style="margin-left: 8px" />
        <el-input-number v-model="modelValue.rgb_b" :min="0" :max="255" @change="syncHexFromRgb" style="margin-left: 8px" />
      </el-form-item>
      <el-form-item label="HEX 值" required>
        <el-input v-model="hexValue" @change="syncRgbFromHex" placeholder="#RRGGBB" />
      </el-form-item>
      <el-form-item label="PANTONE 编码">
        <el-input v-model="modelValue.pantone_code" placeholder="可选" />
      </el-form-item>
      <el-form-item label="CNCS 编码">
        <el-input v-model="modelValue.cncs_code" placeholder="可选" />
      </el-form-item>
      <el-form-item label="排序">
        <el-input-number v-model="modelValue.sequence" :min="0" />
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { ColorItemInfo } from '@/api/color-card'

const props = defineProps<{ modelValue: Partial<ColorItemInfo> }>()
const emit = defineEmits<{ (e: 'update:modelValue', v: Partial<ColorItemInfo>): void }>()

const hexValue = computed({
  get: () => props.modelValue.hex_value || '#000000',
  set: (v) => emit('update:modelValue', { ...props.modelValue, hex_value: v }),
})

function syncRgbFromHex(hex: string) {
  if (!/^#[0-9A-Fa-f]{6}$/.test(hex)) return
  emit('update:modelValue', {
    ...props.modelValue,
    hex_value: hex,
    rgb_r: parseInt(hex.slice(1, 3), 16),
    rgb_g: parseInt(hex.slice(3, 5), 16),
    rgb_b: parseInt(hex.slice(5, 7), 16),
  })
}

function syncHexFromRgb() {
  const r = (props.modelValue.rgb_r ?? 0).toString(16).padStart(2, '0')
  const g = (props.modelValue.rgb_g ?? 0).toString(16).padStart(2, '0')
  const b = (props.modelValue.rgb_b ?? 0).toString(16).padStart(2, '0')
  emit('update:modelValue', {
    ...props.modelValue,
    hex_value: `#${r}${g}${b}`.toUpperCase(),
  })
}
</script>

<style scoped>
.color-item-editor { padding: 0; }
</style>
