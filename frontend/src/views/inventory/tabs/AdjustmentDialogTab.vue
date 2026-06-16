<!--
  AdjustmentDialogTab.vue - 库存调整对话框
  来源：原 inventory/index.vue 中 库存调整对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="库存调整"
    width="500px"
    :close-on-click-modal="false"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form :model="formData" label-width="100px">
      <el-form-item v-if="formData.product_name" label="产品">
        <el-input :value="formData.product_name" disabled />
      </el-form-item>
      <el-form-item v-if="formData.warehouse_name" label="仓库">
        <el-input :value="formData.warehouse_name" disabled />
      </el-form-item>
      <el-form-item v-if="formData.current_quantity" label="当前库存">
        <el-input :value="formData.current_quantity" disabled />
      </el-form-item>
      <el-form-item label="调整类型">
        <el-radio-group v-model="formData.adjustment_type">
          <el-radio value="increase">增加</el-radio>
          <el-radio value="decrease">减少</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="调整数量">
        <el-input-number v-model="formData.adjustment_quantity" :min="1" style="width: 100%" />
      </el-form-item>
      <el-form-item label="调整原因">
        <el-input
          v-model="formData.reason"
          type="textarea"
          :rows="3"
          placeholder="请输入调整原因"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { logger } from '@/utils/logger'

interface StockRow {
  id: number
  product_id: number
  warehouse_id: number
  product_name: string
  warehouse_name: string
  quantity: number
}

interface Props {
  modelValue: boolean
  currentRow: StockRow | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const formData = reactive({
  stock_id: null as number | null,
  product_id: null as number | null,
  warehouse_id: null as number | null,
  product_name: '',
  warehouse_name: '',
  current_quantity: 0,
  adjustment_type: 'increase' as 'increase' | 'decrease',
  adjustment_quantity: 0,
  reason: '',
})

const resetForm = () => {
  formData.stock_id = null
  formData.product_id = null
  formData.warehouse_id = null
  formData.product_name = ''
  formData.warehouse_name = ''
  formData.current_quantity = 0
  formData.adjustment_type = 'increase'
  formData.adjustment_quantity = 0
  formData.reason = ''
}

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        formData.stock_id = props.currentRow.id
        formData.product_id = props.currentRow.product_id
        formData.warehouse_id = props.currentRow.warehouse_id
        formData.product_name = props.currentRow.product_name
        formData.warehouse_name = props.currentRow.warehouse_name
        formData.current_quantity = props.currentRow.quantity
      } else {
        resetForm()
      }
    }
  }
)

const handleSubmit = async () => {
  if (!formData.adjustment_quantity || formData.adjustment_quantity <= 0) {
    ElMessage.warning('请输入有效的调整数量')
    return
  }
  if (!formData.reason) {
    ElMessage.warning('请输入调整原因')
    return
  }
  if (!formData.warehouse_id || !formData.product_id) {
    ElMessage.warning('请选择产品和仓库')
    return
  }

  try {
    const { inventoryApi } = await import('@/api/inventory')
    await inventoryApi.createStockAdjustment({
      warehouse_id: formData.warehouse_id,
      product_id: formData.product_id,
      adjustment_type: formData.adjustment_type,
      adjustment_quantity: formData.adjustment_quantity,
      reason: formData.reason,
    })
    ElMessage.success('库存调整成功')
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '库存调整失败')
    logger.error('库存调整失败', err.message)
  }
}
</script>
