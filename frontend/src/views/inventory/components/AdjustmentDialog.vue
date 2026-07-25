<!--
  AdjustmentDialog.vue - 库存调整对话框
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的库存调整对话框
  行为完全保持一致（仅结构重构）
  使用 props.initialForm 初始化 + 内部 localForm（不直接突变 prop）
  submit 时 emit submitWithForm(localForm) 把当前 form 回传
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('inventory.adjustmentDialog.title')"
    width="500px"
    :close-on-click-modal="false"
    :aria-label="t('inventory.adjustmentDialog.ariaLabel')"
    @update:model-value="onClose"
  >
    <el-form :model="localForm" label-width="100px" :aria-label="t('inventory.adjustmentDialog.formAria')">
      <el-form-item v-if="localForm.product_name" :label="t('inventory.adjustmentDialog.product')">
        <el-input :value="localForm.product_name" disabled />
      </el-form-item>
      <el-form-item v-if="localForm.warehouse_name" :label="t('inventory.adjustmentDialog.warehouse')">
        <el-input :value="localForm.warehouse_name" disabled />
      </el-form-item>
      <el-form-item v-if="localForm.current_quantity" :label="t('inventory.adjustmentDialog.currentQty')">
        <el-input :value="localForm.current_quantity" disabled />
      </el-form-item>
      <el-form-item :label="t('inventory.adjustmentDialog.adjustType')">
        <el-radio-group v-model="localForm.adjustment_type">
          <el-radio value="increase">{{ t('inventory.adjustmentDialog.typeIncrease') }}</el-radio>
          <el-radio value="decrease">{{ t('inventory.adjustmentDialog.typeDecrease') }}</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item :label="t('inventory.adjustmentDialog.adjustQty')">
        <el-input-number
          v-model="localForm.adjustment_quantity"
          :min="1"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="t('inventory.adjustmentDialog.reason')">
        <el-input
          v-model="localForm.reason"
          type="textarea"
          :rows="3"
          :placeholder="t('inventory.adjustmentDialog.reasonPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onClose(false)">{{ t('inventory.adjustmentDialog.cancel') }}</el-button>
      <el-button type="primary" @click="onSubmit">{{ t('inventory.adjustmentDialog.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { deepClone } from '@/utils'
import { reactive, watch } from 'vue'

// 接入 i18n，替换硬编码中文文案
const { t } = useI18n({ useScope: 'global' })

// 库存调整表单数据结构（字段与 inventory 父组件 initialForm 保持一致）
export interface AdjustmentForm {
  stock_id: number | null
  product_id: number | null
  warehouse_id: number | null
  product_name?: string
  warehouse_name?: string
  current_quantity?: number
  adjustment_type: 'increase' | 'decrease'
  adjustment_quantity: number
  reason: string
}

const props = defineProps<{
  visible: boolean
  initialForm: AdjustmentForm
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'submit', data: AdjustmentForm): void
}>()

// 浅拷贝 initialForm 同步初始值（不直接突变 prop）
const localForm = reactive<AdjustmentForm>({} as AdjustmentForm)
watch(
  () => props.initialForm,
  newVal => {
    // AdjustmentForm 字段固定，直接 Object.assign 覆盖即可（无需逐键 delete）
    Object.assign(localForm, deepClone(newVal))
  },
  { immediate: true, deep: true }
)

const onClose = (val: boolean) => {
  emit('update:visible', val)
}

const onSubmit = () => {
  emit('submit', deepClone(localForm))
}
</script>
